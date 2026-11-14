//! The committed contract artifacts are exactly what `bunny-wesley` writes now.
//!
//! The check runs the generator with the arguments `cargo run -p xtask -- generate` uses, from a
//! scratch directory that holds a copy of the schema at its repository path, so every path the
//! generator records in its output is the same as in the repository. It then compares each
//! output with the committed file. See
//! `docs/topics/contract-generation/README.md#regeneration-check`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
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

/// Which copy of the schema and of the committed artifacts the check reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Snapshot {
    /// The files on disk: the full gate and `code-dojo-generated`.
    WorkTree,
    /// The staged content: the pre-commit gate, so the check judges what the commit records.
    Index,
}

/// The bytes of `path` (relative to `root`) in `snapshot`, or `None` when it is absent there.
pub(super) fn read_snapshot(
    root: &Path,
    snapshot: Snapshot,
    path: &str,
) -> Result<Option<Vec<u8>>, DynError> {
    match snapshot {
        Snapshot::WorkTree | Snapshot::Index => match fs::read(root.join(path)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        },
    }
}

/// The scratch directory of the run identified by `run`, under `root/target/`.
pub(super) fn scratch_dir(root: &Path, _run: u32) -> PathBuf {
    root.join(SCRATCH)
}

/// Runs `body` with a fresh scratch directory for run `run`, then removes the directory,
/// whatever `body` returned.
pub(super) fn with_scratch<T>(
    root: &Path,
    run: u32,
    body: impl FnOnce(&Path) -> Result<T, DynError>,
) -> Result<T, DynError> {
    let scratch = scratch_dir(root, run);
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }
    fs::create_dir_all(&scratch)?;
    let result = body(&scratch);
    let removed = fs::remove_dir_all(&scratch);
    let value = result?;
    removed?;
    Ok(value)
}

/// Runs the generator in `scratch` on the schema of `snapshot`, copied to its repository path.
fn regenerate(root: &Path, snapshot: Snapshot, scratch: &Path) -> Result<(), DynError> {
    let schema = read_snapshot(root, snapshot, SCHEMA)?
        .ok_or("Code Dojo: the contract schema is missing")?;
    let schema_copy = scratch.join(SCHEMA);
    if let Some(parent) = schema_copy.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&schema_copy, schema)?;
    let status = Command::new("cargo")
        .args(["run", "--locked", "-q", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .args(["-p", "bunny-wesley", "--"])
        .args(generator_args())
        .current_dir(scratch)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err("Code Dojo: bunny-wesley failed to regenerate the contract artifacts".into())
    }
}

/// Every artifact of `snapshot` that differs from the regenerated one in `scratch`, in the
/// order of [`OUTPUTS`].
pub(super) fn compare(
    root: &Path,
    snapshot: Snapshot,
    scratch: &Path,
) -> Result<Vec<Drift>, DynError> {
    let mut found = Vec::new();
    for (_, path) in OUTPUTS {
        let fresh = fs::read(scratch.join(path))?;
        match read_snapshot(root, snapshot, path)? {
            Some(committed) => {
                if let Some(line) = first_difference(&committed, &fresh) {
                    found.push(Drift { path, line: Some(line) });
                }
            }
            None => found.push(Drift { path, line: None }),
        }
    }
    Ok(found)
}

/// Regenerates from the schema of `snapshot` in a scratch directory of this process and
/// returns every artifact of `snapshot` that differs.
pub(super) fn drifts(root: &Path, snapshot: Snapshot) -> Result<Vec<Drift>, DynError> {
    with_scratch(root, std::process::id(), |scratch| {
        regenerate(root, snapshot, scratch)?;
        compare(root, snapshot, scratch)
    })
}

/// The gate step: fails, naming each drifted file and the fix, unless every committed artifact
/// equals the regenerated one.
pub(super) fn check(root: &Path, snapshot: Snapshot) -> Result<(), DynError> {
    let found = drifts(root, snapshot)?;
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
