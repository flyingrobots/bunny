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

/// The crate whose build with default features off must not use `std`.
const NO_STD_PACKAGE: &str = "bunny-num";

/// A target that has no `std` at all, so a `std` path in [`NO_STD_PACKAGE`], or a
/// dependency that links `std`, is a build error there.
const NO_STD_TARGET: &str = "thumbv7em-none-eabihf";

/// The `no_std` gate of [`NO_STD_PACKAGE`] (flyingrobots/bunny#198): install the bare-metal
/// target, then build the library with default features off for it, for
/// `wasm32-unknown-unknown` and natively, and lint and test it in that configuration. Each
/// command selects only that package, so no other workspace member re-enables `std` through
/// feature unification. The tests themselves still link `std`.
pub(super) fn no_std_commands() -> Vec<Vec<String>> {
    let rustup = ["rustup", "target", "add", NO_STD_TARGET, "wasm32-unknown-unknown"];
    let mut commands = vec![rustup.map(str::to_owned).to_vec()];
    commands.extend(no_std_package_commands(NO_STD_PACKAGE));
    commands
}

/// The Bunny crates above [`NO_STD_PACKAGE`] that also build without `std`
/// (flyingrobots/bunny#156), in dependency order: each depends only on the ones before it.
const NO_STD_STACK_PACKAGES: &[&str] = &["bunny-linalg", "bunny-geom", "bunny-query"];

/// The `no_std` gate of every crate in [`NO_STD_STACK_PACKAGES`]: the same five commands as
/// [`no_std_commands`] runs for [`NO_STD_PACKAGE`], for one package at a time, so no other
/// workspace member re-enables `std` through feature unification. It runs after
/// [`no_std_commands`], which installs the targets.
pub(super) fn no_std_stack_commands() -> Vec<Vec<String>> {
    NO_STD_STACK_PACKAGES.iter().flat_map(|package| no_std_package_commands(package)).collect()
}

/// Builds `package` with default features off for [`NO_STD_TARGET`], for
/// `wasm32-unknown-unknown` and natively, then lints and tests it in that configuration.
fn no_std_package_commands(package: &str) -> Vec<Vec<String>> {
    let no_std = |head: &[&str], tail: &[&str]| {
        let mut args = vec!["cargo".to_owned()];
        args.extend(head.iter().map(|arg| (*arg).to_owned()));
        args.extend(["--locked", "-p", package, "--no-default-features"].map(str::to_owned));
        args.extend(tail.iter().map(|arg| (*arg).to_owned()));
        args
    };
    vec![
        no_std(&["build"], &["--target", NO_STD_TARGET]),
        no_std(&["build"], &["--target", "wasm32-unknown-unknown"]),
        no_std(&["build"], &[]),
        no_std(&["clippy"], &["--all-targets", "--", "-D", "warnings"]),
        no_std(&["test"], &[]),
    ]
}

/// The rustdoc gate (flyingrobots/bunny#467): document the whole workspace with every
/// feature on and fail on any rustdoc warning, so a broken, ambiguous or private intra-doc
/// link is a build error. Only `-D warnings` is passed, so the pinned toolchain and a newer
/// one run the same flags.
pub(super) fn rustdoc_command() -> Vec<String> {
    ["cargo", "doc", "--locked", "--workspace", "--no-deps", "--all-features"]
        .map(str::to_owned)
        .to_vec()
}

/// The value of `RUSTDOCFLAGS` for [`rustdoc_command`].
pub(super) const RUSTDOCFLAGS: &str = "-D warnings";

/// One `cargo check` of the timing harness per feature group alone, then one with every group.
///
/// `cargo xtask perf` falls back to building each group alone when the base revision cannot
/// build them all, and reports a group it cannot build as new. A helper that only some groups
/// use must therefore compile away with the others, or that fallback silently stops comparing
/// cases. The workspace denies warnings, so a helper left behind by a group is a build error.
pub(super) fn perf_group_commands(manifest: &str) -> Vec<Vec<String>> {
    let groups = crate::perf::feature_groups(manifest);
    let mut sets: Vec<String> = groups.clone();
    sets.push(groups.join(","));
    sets.into_iter()
        .map(|features| {
            ["cargo", "check", "--locked", "-p", "bunny-perf", "--no-default-features"]
                .into_iter()
                .map(str::to_owned)
                .chain(["--features".to_owned(), features])
                .collect()
        })
        .collect()
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
    use super::{
        no_std_commands, no_std_stack_commands, perf_group_commands, rustdoc_command,
        third_party_packages, RUSTDOCFLAGS,
    };

    #[test]
    fn no_std_tp_001_bunny_num_is_built_without_std_for_a_bare_metal_target() {
        let commands = no_std_commands();
        let (rustup, cargo) = commands.split_first().expect("the target install comes first");
        assert_eq!(rustup[..3], ["rustup", "target", "add"]);
        assert!(rustup.iter().any(|arg| arg == "thumbv7em-none-eabihf"));
        assert!(rustup.iter().any(|arg| arg == "wasm32-unknown-unknown"));
        for command in cargo {
            assert_eq!(command[0], "cargo");
            assert!(command.iter().any(|arg| arg == "--no-default-features"), "{command:?}");
            assert!(!command.iter().any(|arg| arg == "--all-features" || arg == "--features"));
            let packages: Vec<&String> = command
                .windows(2)
                .filter(|pair| pair[0] == "-p" || pair[0] == "--package")
                .map(|pair| &pair[1])
                .collect();
            assert_eq!(packages, ["bunny-num"], "only bunny-num is selected: {command:?}");
        }
        let builds_for = |target: Option<&str>| {
            cargo.iter().any(|command| {
                command[1] == "build"
                    && match target {
                        Some(target) => command.windows(2).any(|pair| pair == ["--target", target]),
                        None => !command.iter().any(|arg| arg == "--target"),
                    }
            })
        };
        assert!(builds_for(Some("thumbv7em-none-eabihf")), "a target with no std");
        assert!(builds_for(Some("wasm32-unknown-unknown")), "the issue's wasm32 build");
        assert!(builds_for(None), "the issue's native build");
        assert!(cargo.iter().any(|command| command[1] == "test"));
        assert!(cargo.iter().any(|command| command[1] == "clippy"));
    }

    #[test]
    fn no_std_tp_010_linalg_geom_and_query_are_built_without_std_one_package_at_a_time() {
        let stack = no_std_stack_commands();
        let package_of = |command: &[String]| -> Vec<String> {
            command
                .windows(2)
                .filter(|pair| pair[0] == "-p" || pair[0] == "--package")
                .map(|pair| pair[1].clone())
                .collect()
        };
        // bunny-num's commands, without the target install, are the template for each crate.
        let template: Vec<Vec<String>> = no_std_commands()[1..].to_vec();
        let mut expected = Vec::new();
        for package in ["bunny-linalg", "bunny-geom", "bunny-query"] {
            for command in &template {
                let replaced: Vec<String> = command
                    .iter()
                    .map(|arg| if arg == "bunny-num" { package.to_owned() } else { arg.clone() })
                    .collect();
                expected.push(replaced);
            }
        }
        assert_eq!(stack, expected, "five commands per crate, in dependency order");
        for command in &stack {
            assert_eq!(command[0], "cargo");
            assert!(command.iter().any(|arg| arg == "--no-default-features"), "{command:?}");
            assert!(!command.iter().any(|arg| arg == "--all-features" || arg == "--features"));
            assert_eq!(package_of(command).len(), 1, "one package per command: {command:?}");
        }
        let bare_metal: Vec<Vec<String>> = stack
            .iter()
            .filter(|command| {
                command.windows(2).any(|pair| pair == ["--target", "thumbv7em-none-eabihf"])
            })
            .map(|command| package_of(command))
            .collect();
        assert_eq!(bare_metal, [["bunny-linalg"], ["bunny-geom"], ["bunny-query"]]);
    }

    #[test]
    fn rustdoc_documents_the_whole_workspace_and_denies_warnings() {
        let command = rustdoc_command();
        assert_eq!(command[..2], ["cargo", "doc"]);
        for flag in ["--locked", "--workspace", "--no-deps", "--all-features"] {
            assert!(command.iter().any(|arg| arg == flag), "{flag}: {command:?}");
        }
        assert!(!command.iter().any(|arg| arg == "--document-private-items"));
        assert_eq!(RUSTDOCFLAGS, "-D warnings", "only -D warnings, nothing toolchain-specific");
    }

    fn features_of(command: &[String]) -> &str {
        let at = command.iter().position(|arg| arg == "--features").expect("a --features flag");
        &command[at + 1]
    }

    #[test]
    fn perf_harness_is_checked_once_per_group_alone_and_once_with_all() {
        let manifest = "[features]\ndefault = [\"num\"]\n# note\nnum = []\nangle = []\n";
        let commands = perf_group_commands(manifest);
        let features: Vec<&str> = commands.iter().map(|c| features_of(c)).collect();
        assert_eq!(features, ["num", "angle", "num,angle"]);
        for command in &commands {
            assert!(command.iter().any(|arg| arg == "--no-default-features"));
            assert!(command.windows(2).any(|pair| pair == ["-p", "bunny-perf"]));
        }
    }

    #[test]
    fn perf_gate_covers_every_group_the_harness_manifest_declares() {
        // Normalise CRLF checkouts: the parser reads lines, so a trailing CR must not leak
        // into a group name.
        let manifest = include_str!("../../crates/bunny-perf/Cargo.toml").replace("\r\n", "\n");
        let groups = crate::perf::feature_groups(&manifest);
        assert!(groups.len() > 10, "the harness declares its groups: {groups:?}");
        let commands = perf_group_commands(&manifest);
        assert_eq!(commands.len(), groups.len() + 1);
        for group in &groups {
            assert!(
                commands.iter().any(|c| features_of(c) == group),
                "group {group} has no build of its own"
            );
        }
    }

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
