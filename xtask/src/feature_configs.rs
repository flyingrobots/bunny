//! The no-default-features gate.
//!
//! `bunny-mesh` and `bunny-codec` reach crates.io only through default features.
//! The full gate lists each one's dependency graph with default features off and
//! fails on any crate that is not a local path crate, then lints, tests and
//! wasm-checks both crates in that configuration. Every command names only these
//! packages, so another workspace member cannot re-enable a feature through
//! feature unification.

use std::error::Error;
use std::path::Path;
use std::process::Command;

type DynError = Box<dyn Error>;

/// Crates whose build with default features off must reach no third-party crate.
const NO_DEFAULT_PACKAGES: &[&str] = &["bunny-mesh", "bunny-codec"];

/// Fails if a crate in [`NO_DEFAULT_PACKAGES`] reaches a third-party crate through
/// its normal or build dependencies with default features off.
pub(super) fn check_trees(root: &Path) -> Result<(), DynError> {
    let mut failures = Vec::new();
    for package in NO_DEFAULT_PACKAGES {
        let output = Command::new("cargo").args(tree_args(package)).current_dir(root).output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo tree for {package} failed: {stderr}").into());
        }
        let tree = String::from_utf8(output.stdout)?;
        failures.extend(
            third_party_packages(&tree)
                .into_iter()
                .map(|found| format!("{package} --no-default-features reaches `{found}`")),
        );
    }
    if failures.is_empty() {
        println!("Code Dojo: no-default-features dependency graphs hold only path crates");
        return Ok(());
    }
    eprintln!("Code Dojo: third-party crates without default features");
    for failure in &failures {
        eprintln!("  {failure}");
    }
    Err("no-default-features dependency policy violations found".into())
}

/// Clippy, tests (doctests included) and a wasm32 check of [`NO_DEFAULT_PACKAGES`]
/// with default features off. The wasm32 target must already be installed.
pub(super) fn commands() -> Vec<Vec<String>> {
    vec![
        command(&["clippy", "--locked", "--all-targets"], &["--", "-D", "warnings"]),
        command(&["test", "--locked"], &[]),
        command(&["check", "--locked", "--target", "wasm32-unknown-unknown"], &[]),
    ]
}

/// `cargo <head> -p bunny-mesh -p bunny-codec --no-default-features <tail>`.
fn command(head: &[&str], tail: &[&str]) -> Vec<String> {
    let mut args = vec!["cargo".to_owned()];
    args.extend(head.iter().map(|arg| (*arg).to_owned()));
    for package in NO_DEFAULT_PACKAGES {
        args.extend(["-p".to_owned(), (*package).to_owned()]);
    }
    args.push("--no-default-features".to_owned());
    args.extend(tail.iter().map(|arg| (*arg).to_owned()));
    args
}

fn tree_args(package: &str) -> Vec<String> {
    [
        "tree",
        "--locked",
        "-p",
        package,
        "--no-default-features",
        "--edges",
        "normal,build",
        "--target",
        "all",
        "--prefix",
        "none",
        "--format",
        "{p}",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Returns `name version` for each crate in a `cargo tree --prefix none --format {p}`
/// listing that is not a local path crate, once each, in listing order.
pub(super) fn third_party_packages(tree: &str) -> Vec<String> {
    let mut found = Vec::<String>::new();
    for line in tree.lines().map(str::trim) {
        if line.is_empty() || is_path_package(line) {
            continue;
        }
        let package = line.split(' ').take(2).collect::<Vec<_>>().join(" ");
        if !found.contains(&package) {
            found.push(package);
        }
    }
    found
}

/// `bunny-num v0.7.1 (/work/crates/bunny-num)` and its deduplicated form ending in
/// `(*)` are path crates. `sha2 v0.10.9` (crates.io prints no source), `sha2 v0.10.9 (*)`
/// and `x v1.0.0 (https://github.com/o/x#abc)` are not.
fn is_path_package(line: &str) -> bool {
    let Some(source) = line.splitn(3, ' ').nth(2).and_then(|rest| rest.strip_prefix('(')) else {
        return false;
    };
    source.starts_with('/') || source.get(1..3) == Some(":\\")
}

#[cfg(test)]
mod tests {
    use super::third_party_packages;

    #[test]
    fn registry_and_git_crates_are_reported() {
        let tree = "\
bunny-codec v0.7.1 (/work/crates/bunny-codec)
bunny-mesh v0.7.1 (/work/crates/bunny-mesh)
sha2 v0.10.9
typenum v1.20.1 (*)
bunny-num v0.7.1 (/work/crates/bunny-num) (*)
sha2 v0.10.9
outside v1.0.0 (https://github.com/example/outside#0123abc)
";
        assert_eq!(
            third_party_packages(tree),
            ["sha2 v0.10.9", "typenum v1.20.1", "outside v1.0.0"],
            "each crate that is not a local path crate is reported once, in listing order"
        );
    }

    #[test]
    fn path_crates_are_accepted() {
        let tree = "\
bunny-codec v0.7.1 (/work/crates/bunny-codec)
bunny-geom v0.7.1 (/work/crates/bunny-geom)
bunny-linalg v0.7.1 (/work/crates/bunny-linalg) (*)

bunny-num v0.7.1 (C:\\work\\crates\\bunny-num)
";
        assert_eq!(third_party_packages(tree), Vec::<String>::new());
    }
}
