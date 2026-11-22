//! Build scripts, target paths and workspace members of the deterministic crates'
//! manifests (flyingrobots/bunny#349).
//!
//! Oracle: each manifest is written by hand; whether Cargo builds a build script or reads a
//! target from outside the crate follows from the Cargo manifest format (`build.rs` is
//! built unless `build = false`; a `build` or `path` value names a file relative to the
//! crate). The reviewer checked that `build = "../../scripts/zz_build.rs"` builds.

use std::path::{Path, PathBuf};

use super::{deterministic_manifest, unclassified_members};

fn rules(text: &str, build_rs: bool) -> Vec<(usize, &'static str)> {
    deterministic_manifest(text, build_rs)
        .expect("parses")
        .into_iter()
        .map(|finding| (finding.line, finding.rule))
        .collect()
}

/// A label, a manifest, whether `build.rs` exists, and the expected lines and rules.
type ManifestCase = (&'static str, String, bool, Vec<(usize, &'static str)>);

const PACKAGE: &str = "[package]\nname = \"bunny-num\"\n";

#[test]
fn a_deterministic_crate_has_no_build_script_and_no_target_outside_it() {
    let cases: [ManifestCase; 11] = [
        (
            "issue: build script outside the crate",
            format!("{PACKAGE}build = \"../../scripts/zz_build.rs\"\n"),
            false,
            vec![(3, "build-script")],
        ),
        (
            "build script inside the crate",
            format!("{PACKAGE}build = \"gen.rs\"\n"),
            false,
            vec![(3, "build-script")],
        ),
        ("build = true", format!("{PACKAGE}build = true\n"), true, vec![(3, "build-script")]),
        ("a build.rs Cargo finds by itself", PACKAGE.to_string(), true, vec![(0, "build-script")]),
        ("build.rs switched off", format!("{PACKAGE}build = false\n"), true, vec![]),
        (
            "lib path outside",
            format!("{PACKAGE}\n[lib]\npath = \"../shared/lib.rs\"\n"),
            false,
            vec![(5, "target-outside-crate")],
        ),
        (
            "test target outside",
            format!("{PACKAGE}\n[[test]]\nname = \"t\"\npath = \"../../tests/t.rs\"\n"),
            false,
            vec![(6, "target-outside-crate")],
        ),
        (
            "absolute bench path",
            format!("{PACKAGE}\n[[bench]]\nname = \"b\"\npath = \"/tmp/b.rs\"\n"),
            false,
            vec![(6, "target-outside-crate")],
        ),
        (
            "inline array of examples",
            format!("example = [{{ name = \"e\", path = \"../e.rs\" }}]\n{PACKAGE}"),
            false,
            vec![(1, "target-outside-crate")],
        ),
        (
            "paths that stay inside",
            format!(
                "{PACKAGE}\n[lib]\npath = \"src/root.rs\"\n\n\
                 [[bin]]\nname = \"x\"\npath = \"tools/../src/x.rs\"\n"
            ),
            false,
            vec![],
        ),
        ("a plain manifest", format!("{PACKAGE}version = \"0.7.1\"\n"), false, vec![]),
    ];
    let mut wrong = Vec::new();
    for (label, text, build_rs, expected) in cases {
        let found = rules(&text, build_rs);
        if found != expected {
            wrong.push(format!("{label}: expected {expected:?}, found {found:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(deterministic_manifest("[package\n", false).is_err(), "invalid TOML is an error");
}

#[test]
fn every_workspace_member_is_classified() {
    let manifests = |path: &Path| {
        let name = path.parent()?.file_name()?.to_str()?.to_string();
        Some(format!("[package]\nname = \"{name}\"\n"))
    };
    let dirs = |parent: &Path| {
        ["tool-a", "tool-b"].iter().map(|name| parent.join(name)).collect::<Vec<PathBuf>>()
    };
    let root = "[workspace]\nmembers = [\n  \"crates/bunny-num\",\n  \"crates/zz-probe-crate\",\n\
                \x20 \"xtask\",\n  \"tools/*\",\n]\n";
    let found = unclassified_members(root, &["bunny-num", "xtask", "tool-a"], &manifests, &dirs)
        .expect("parses")
        .into_iter()
        .map(|finding| {
            (finding.line, finding.message.split('`').nth(1).unwrap_or_default().to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(found, vec![(4, "zz-probe-crate".to_string()), (6, "tool-b".to_string())]);
}
