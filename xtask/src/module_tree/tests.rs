//! Which files are test code, decided by how each module is declared, and the gate's
//! limits applied by that decision (flyingrobots/bunny#348).
//!
//! Oracle: each fixture is a small crate written by hand, and the expected test files
//! follow from the rule in `docs/topics/code-dojo-determinism/README.md#scope` and from how
//! `rustc` finds module files (`foo.rs` or `foo/mod.rs`, `#[path]` relative to the
//! declaring file's directory, inline modules adding a directory).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::ModuleTree;
use crate::code_dojo::tree_violations;

fn tree(files: &[(&str, &str)]) -> ModuleTree {
    let paths = files.iter().map(|(path, _)| PathBuf::from(path)).collect::<Vec<_>>();
    ModuleTree::new(&paths, &|path| {
        files
            .iter()
            .find(|(known, _)| Path::new(known) == path)
            .map(|(_, text)| (*text).to_string())
    })
}

fn tests_of(files: &[(&str, &str)]) -> BTreeSet<String> {
    let tree = tree(files);
    files
        .iter()
        .filter(|(path, _)| tree.is_test(&PathBuf::from(path)))
        .map(|(path, _)| (*path).to_string())
        .collect()
}

/// A labelled fixture crate and the files the walk must call test code.
type Case = (&'static str, &'static [(&'static str, &'static str)], &'static [&'static str]);

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_string()).collect()
}

const LIB: &str = "crates/bunny-num/src/lib.rs";

#[test]
fn a_tests_directory_compiled_into_the_library_is_library_code() {
    let files = [
        (LIB, "pub mod tests;\n"),
        ("crates/bunny-num/src/tests/mod.rs", "pub mod float;\n"),
        ("crates/bunny-num/src/tests/float.rs", "pub fn root(x: f64) -> f64 { x.sqrt() * 0.5 }\n"),
    ];
    assert_eq!(tests_of(&files), set(&[]));
}

#[test]
fn the_issue_form_is_reported_under_the_core_rules() {
    let files = [
        (LIB, "pub mod tests;\n"),
        ("crates/bunny-num/src/tests/mod.rs", "pub mod float;\n"),
        (
            "crates/bunny-num/src/tests/float.rs",
            "pub fn root(x: f64) -> f64 { x.sqrt() * 0.5 }\npub fn first(v: &[u8]) -> u8 { v[0] }\n",
        ),
    ];
    let owned = files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), (*text).to_string()))
        .collect::<Vec<_>>();
    let found = tree_violations(&owned)
        .into_iter()
        .filter(|violation| violation.path == Path::new("crates/bunny-num/src/tests/float.rs"))
        .map(|violation| (violation.line, violation.rule))
        .collect::<BTreeSet<_>>();
    assert!(found.contains(&(1, "float-boundary")), "{found:?}");
    assert!(found.contains(&(2, "indexing")), "{found:?}");
}

#[test]
fn a_cfg_test_module_gets_the_test_limits() {
    let files = [
        (LIB, "#[cfg(test)]\nmod checks;\n"),
        ("crates/bunny-num/src/checks.rs", "pub fn first(v: &[u8]) -> u8 { v[0] }\n"),
    ];
    let owned = files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), (*text).to_string()))
        .collect::<Vec<_>>();
    let found = tree_violations(&owned)
        .into_iter()
        .map(|violation| (violation.path, violation.rule))
        .collect::<Vec<_>>();
    assert_eq!(found, Vec::new(), "indexing is allowed in test code");
}

#[test]
fn a_module_is_test_code_only_when_every_declaration_is_under_cfg_test() {
    let cases: [Case; 12] = [
        (
            "cfg(test) directory module",
            &[(LIB, "#[cfg(test)]\nmod tests;\n"), ("crates/bunny-num/src/tests/mod.rs", "")],
            &["crates/bunny-num/src/tests/mod.rs"],
        ),
        (
            "cfg(test) tests.rs below a file module",
            &[
                (LIB, "mod fixed;\n"),
                ("crates/bunny-num/src/fixed.rs", "#[cfg(test)]\nmod tests;\n"),
                ("crates/bunny-num/src/fixed/tests.rs", ""),
            ],
            &["crates/bunny-num/src/fixed/tests.rs"],
        ),
        (
            "tests.rs declared without cfg",
            &[
                (LIB, "mod fixed;\n"),
                ("crates/bunny-num/src/fixed.rs", "pub mod tests;\n"),
                ("crates/bunny-num/src/fixed/tests.rs", ""),
            ],
            &[],
        ),
        (
            "the file's own #![cfg(test)]",
            &[(LIB, "mod checks;\n"), ("crates/bunny-num/src/checks.rs", "#![cfg(test)]\n")],
            &["crates/bunny-num/src/checks.rs"],
        ),
        (
            "an enclosing inline module under cfg(test)",
            &[
                (LIB, "#[cfg(test)]\nmod outer {\n    mod inner;\n}\n"),
                ("crates/bunny-num/src/outer/inner.rs", ""),
            ],
            &["crates/bunny-num/src/outer/inner.rs"],
        ),
        (
            "cfg(all(test, ..))",
            &[
                (LIB, "#[cfg(all(test, feature = \"slow\"))]\nmod slow;\n"),
                ("crates/bunny-num/src/slow.rs", ""),
            ],
            &["crates/bunny-num/src/slow.rs"],
        ),
        (
            "cfg(any(test, ..)) is also compiled outside tests",
            &[
                (LIB, "#[cfg(any(test, feature = \"x\"))]\nmod maybe;\n"),
                ("crates/bunny-num/src/maybe.rs", ""),
            ],
            &[],
        ),
        (
            "cfg(not(test))",
            &[(LIB, "#[cfg(not(test))]\nmod real;\n"), ("crates/bunny-num/src/real.rs", "")],
            &[],
        ),
        (
            "declared twice, once outside tests",
            &[
                (LIB, "#[cfg(test)]\nmod tests;\n#[path = \"tests.rs\"]\npub mod shipped;\n"),
                ("crates/bunny-num/src/tests.rs", ""),
            ],
            &[],
        ),
        (
            "a child of a test module inherits it",
            &[
                (LIB, "#[cfg(test)]\nmod tests;\n"),
                ("crates/bunny-num/src/tests/mod.rs", "mod helpers;\n"),
                ("crates/bunny-num/src/tests/helpers.rs", ""),
            ],
            &["crates/bunny-num/src/tests/mod.rs", "crates/bunny-num/src/tests/helpers.rs"],
        ),
        (
            "a #[path] file is a mod.rs-like file",
            &[
                (LIB, "#[cfg(test)]\n#[path = \"checks/root.rs\"]\nmod checks;\n"),
                ("crates/bunny-num/src/checks/root.rs", "mod more;\n"),
                ("crates/bunny-num/src/checks/more.rs", ""),
            ],
            &["crates/bunny-num/src/checks/root.rs", "crates/bunny-num/src/checks/more.rs"],
        ),
        (
            "an unreached file under src is library code",
            &[(LIB, ""), ("crates/bunny-num/src/tests/orphan.rs", "")],
            &[],
        ),
    ];
    let mut wrong = Vec::new();
    for (label, files, expected) in cases {
        let found = tests_of(files);
        if found != set(expected) {
            wrong.push(format!("{label}: expected {expected:?}, found {found:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_crate_tests_directory_is_test_code_and_other_roots_are_not() {
    let files = [
        (LIB, ""),
        ("crates/bunny-num/tests/golden.rs", "mod common;\n"),
        ("crates/bunny-num/tests/common/mod.rs", ""),
        ("crates/bunny-num/tests/support/unreached.rs", ""),
        ("crates/bunny-num/benches/speed.rs", ""),
        ("crates/bunny-num/examples/demo.rs", ""),
        ("crates/bunny-num/src/bin/tool.rs", ""),
        ("crates/bunny-num/build.rs", ""),
    ];
    assert_eq!(
        tests_of(&files),
        set(&[
            "crates/bunny-num/tests/golden.rs",
            "crates/bunny-num/tests/common/mod.rs",
            "crates/bunny-num/tests/support/unreached.rs",
        ])
    );
}

#[test]
fn the_repository_test_modules_inside_src_are_still_tests() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the repository root");
    let read = |path: &std::path::Path| std::fs::read_to_string(root.join(path)).ok();
    let paths = [
        "crates/bunny-broadphase/src/bvh/sah/tests/mod.rs",
        "crates/bunny-optics/src/shading/vertex_normals/tests/mod.rs",
    ];
    let mut listing = crate::code_dojo::listed_rust_files(root);
    listing.sort();
    let tree = ModuleTree::new(&listing, &read);
    for path in paths {
        assert!(tree.is_test(&PathBuf::from(path)), "{path} is declared under cfg(test)");
    }
    assert!(!tree.is_test(&PathBuf::from("crates/bunny-broadphase/src/bvh/sah.rs")));
}
