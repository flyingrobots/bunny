//! The committed contract artifacts are exactly what `bunny-wesley` writes now.
//!
//! The check runs the generator with the arguments `cargo run -p xtask -- generate` uses, from a
//! scratch directory that holds a copy of the schema at its repository path, so every path the
//! generator records in its output is the same as in the repository. It then compares each
//! output with the committed file. See
//! `docs/topics/contract-generation/README.md#regeneration-check`.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type DynError = Box<dyn Error>;

/// The schema the artifacts are generated from, relative to the repository root.
pub(super) const SCHEMA: &str = "schemas/bunny/v0/graphics.graphql";

/// Each output flag of `bunny-wesley` and the committed file it writes.
pub(super) const OUTPUTS: [(&str, &str); 3] = [
    ("--rust", "crates/bunny-contract/src/generated/graphics.rs"),
    ("--typescript", "generated/typescript/bunny-graphics.ts"),
    ("--manifest", "generated/bunny-graphics.manifest.json"),
];

/// The command that rewrites the committed artifacts.
pub(super) const FIX_COMMAND: &str = "cargo run --locked -p xtask -- generate";

/// Where, under the workspace's `target/`, the check regenerates.
const SCRATCH: &str = "target/dojo-generated";

/// The arguments of `bunny-wesley` after `--`: the schema, then each output flag and path.
pub(super) fn generator_args() -> Vec<&'static str> {
    let mut args = vec![SCHEMA];
    for (flag, path) in OUTPUTS {
        args.extend([flag, path]);
    }
    args
}

/// A committed artifact that differs from the regenerated one: its path and the 1-based
/// line where the two first differ (one past the shorter text when one is a prefix of the
/// other), or `None` when the committed file is missing.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Drift {
    pub(super) path: &'static str,
    pub(super) line: Option<usize>,
}

/// The 1-based line where `committed` and `fresh` first differ, or `None` when they are equal.
pub(super) fn first_difference(committed: &[u8], fresh: &[u8]) -> Option<usize> {
    if committed == fresh {
        return None;
    }
    let mut committed_lines = committed.split_inclusive(|byte| *byte == b'\n');
    let mut fresh_lines = fresh.split_inclusive(|byte| *byte == b'\n');
    let mut line = 1;
    loop {
        match (committed_lines.next(), fresh_lines.next()) {
            (Some(left), Some(right)) if left == right => line += 1,
            _ => return Some(line),
        }
    }
}

/// Regenerates into `root/target/dojo-generated` and returns every committed artifact that
/// differs, in the order of [`OUTPUTS`].
pub(super) fn drifts(root: &Path) -> Result<Vec<Drift>, DynError> {
    let scratch = root.join(SCRATCH);
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }
    let schema_copy = scratch.join(SCHEMA);
    if let Some(parent) = schema_copy.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(root.join(SCHEMA), &schema_copy)?;
    let manifest_path = root.join("Cargo.toml");
    let status = Command::new("cargo")
        .args(["run", "--locked", "-q", "--manifest-path"])
        .arg(&manifest_path)
        .args(["-p", "bunny-wesley", "--"])
        .args(generator_args())
        .current_dir(&scratch)
        .status()?;
    if !status.success() {
        return Err("Code Dojo: bunny-wesley failed to regenerate the contract artifacts".into());
    }
    let mut found = Vec::new();
    for (_, path) in OUTPUTS {
        let fresh = fs::read(scratch.join(path))?;
        match fs::read(root.join(path)) {
            Ok(committed) => {
                if let Some(line) = first_difference(&committed, &fresh) {
                    found.push(Drift { path, line: Some(line) });
                }
            }
            Err(_) => found.push(Drift { path, line: None }),
        }
    }
    Ok(found)
}

/// The gate step: fails, naming each drifted file and the fix, unless every committed artifact
/// equals the regenerated one.
pub(super) fn check(root: &Path) -> Result<(), DynError> {
    let found = drifts(root)?;
    if found.is_empty() {
        println!("Code Dojo: generated contract artifacts match bunny-wesley");
        return Ok(());
    }
    eprintln!("Code Dojo: generated contract artifacts differ from what bunny-wesley writes");
    for drift in &found {
        match drift.line {
            Some(line) => eprintln!("  {}:{line}: differs from the regenerated file", drift.path),
            None => eprintln!("  {}: missing; bunny-wesley writes it", drift.path),
        }
    }
    eprintln!("Regenerate with `{FIX_COMMAND}` and commit the result.");
    Err("generated artifacts drifted".into())
}

#[cfg(test)]
mod tests;
