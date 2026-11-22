//! The `blanket-nursery` check: seeded sources, manifests and Cargo configurations whose
//! findings, with their lines, follow from Rule 10 as `docs/topics/lint-policy/README.md`
//! states it, and the repository itself.
//!
//! Oracle: each input is written by hand; whether it sets the group follows from the TOML
//! grammar (dotted keys, whitespace around dots, inline tables are one table) and from
//! where Rust lint attributes apply. The reviewer checked that `nursery.level = "warn"` in
//! `[workspace.lints.clippy]` turns the group on in `cargo clippy` (flyingrobots/bunny#350).

use std::path::{Path, PathBuf};

use super::{attribute_lines, config_lines, manifest_lines};

fn lines(source: &str) -> Vec<usize> {
    attribute_lines(source).expect("parses")
}

fn manifest(text: &str) -> Vec<usize> {
    manifest_lines(text).expect("parses")
}

#[test]
fn lint_tp_001_a_blanket_nursery_attribute_is_found_at_its_line() {
    let cases: [(&str, &[usize]); 11] = [
        ("#![deny(clippy::nursery)]\n", &[1]),
        ("#![deny(unsafe_code)]\n#![warn(clippy::nursery)]\n", &[2]),
        ("//! Docs.\n\n#![forbid(clippy::nursery)]\n", &[3]),
        ("#![deny(clippy::all, clippy::nursery, missing_docs)]\n", &[1]),
        ("#![cfg_attr(not(test), deny(clippy::nursery))]\n", &[1]),
        ("#![cfg_attr(test, allow(dead_code), warn(clippy::nursery))]\n", &[1]),
        // Named nursery lints, `allow` and `expect` of the group, other groups.
        ("#![deny(clippy::use_self)]\n#![deny(clippy::missing_const_for_fn)]\n", &[]),
        (
            "#![allow(clippy::nursery)]\n#![expect(clippy::nursery)]\n#![deny(clippy::pedantic)]\n",
            &[],
        ),
        // The group only mentioned, or another tool's `nursery`.
        ("// #![deny(clippy::nursery)]\nconst TEXT: &str = \"#![deny(clippy::nursery)]\";\n", &[]),
        ("#![deny(nursery)]\n#![deny(other::nursery)]\n", &[]),
        ("#[allow(clippy::nursery)]\nmod quiet {}\n#[deny(clippy::use_self)]\nmod named {}\n", &[]),
    ];
    for (source, expected) in cases {
        assert_eq!(lines(source), expected, "{source:?}");
    }
    assert!(attribute_lines("#![deny(").is_err(), "a source that does not parse is an error");
}

#[test]
fn lint_tp_005_the_group_on_a_module_item_or_statement_is_found() {
    let cases: [(&str, &str, &[usize]); 9] = [
        (
            "issue: outer attribute on a module",
            "#![forbid(unsafe_code)]\n#[deny(clippy::nursery)]\nmod inner {}\n",
            &[2],
        ),
        (
            "inner attribute of an inline module",
            "mod inner {\n    #![warn(clippy::nursery)]\n}\n",
            &[2],
        ),
        (
            "inner attribute of a module file",
            "//! A module.\n#![deny(clippy::nursery)]\npub fn f() {}\n",
            &[2],
        ),
        ("on a function", "#[deny(clippy::nursery)]\nfn one() {}\n", &[1]),
        ("on an impl", "struct S;\n#[forbid(clippy::nursery)]\nimpl S {}\n", &[2]),
        (
            "on a method",
            "struct S;\nimpl S {\n    #[warn(clippy::nursery)]\n    fn f(&self) {}\n}\n",
            &[3],
        ),
        ("on a statement", "fn f() {\n    #[deny(clippy::nursery)]\n    let _x = 1;\n}\n", &[2]),
        (
            "through cfg_attr on a module",
            "#[cfg_attr(test, deny(clippy::nursery))]\nmod checks {}\n",
            &[1],
        ),
        (
            "nested in a module",
            "mod a {\n    mod b {\n        #[deny(clippy::nursery)]\n        fn f() {}\n    }\n}\n",
            &[3],
        ),
    ];
    for (label, source, expected) in cases {
        assert_eq!(lines(source), expected, "{label}");
    }
}

#[test]
fn lint_tp_002_a_blanket_nursery_manifest_level_is_found_at_its_line() {
    let cases: [(&str, &[usize]); 8] = [
        ("[workspace.lints.clippy]\nall = \"deny\"\nnursery = \"deny\"\n", &[3]),
        ("[workspace.lints.clippy]\nnursery = { level = \"warn\", priority = -1 }\n", &[2]),
        ("[lints.clippy]\n\"nursery\" = { priority = -1, level = 'forbid' }\n", &[2]),
        ("[package]\nname = \"x\"\n\n[lints]\nclippy.nursery = \"forbid\"\n", &[5]),
        // `allow`, a named lint, a comment, and a `nursery` key of another table.
        ("[workspace.lints.clippy]\nnursery = { level = \"allow\", priority = -1 }\n", &[]),
        ("[lints.clippy]\nuse_self = \"deny\"\n# nursery = \"deny\"\n", &[]),
        ("[features]\nnursery = \"deny\"\n[lints]\nworkspace = true\n", &[]),
        ("[workspace.lints.rust]\nnursery = \"deny\"\n", &[]),
    ];
    for (text, expected) in cases {
        assert_eq!(manifest(text), expected, "{text:?}");
    }
}

#[test]
fn lint_tp_006_every_toml_spelling_of_the_table_is_read() {
    let cases: [(&str, &str, &[usize]); 12] = [
        (
            "issue: dotted level",
            "[workspace.lints.clippy]\nnursery.level = \"deny\"\nnursery.priority = -1\n",
            &[2],
        ),
        ("issue: spaced header", "[ workspace . lints . clippy ]\nnursery = \"deny\"\n", &[2]),
        (
            "issue: inline clippy table",
            "[workspace.lints]\nclippy = { nursery = \"deny\" }\n",
            &[2],
        ),
        (
            "issue: inline level table",
            "[lints.clippy]\nnursery = { level = \"deny\", priority = -1 }\n",
            &[2],
        ),
        ("inline lints table", "lints = { clippy = { nursery = \"warn\" } }\n", &[1]),
        (
            "inline lints in the workspace table",
            "[workspace]\nlints = { clippy = { nursery = \"forbid\" } }\n",
            &[2],
        ),
        ("nested inline level", "[lints]\nclippy = { nursery = { level = \"forbid\" } }\n", &[2]),
        ("quoted dotted key", "[workspace.lints.clippy]\n\"nursery\".level = \"deny\"\n", &[2]),
        ("literal-quoted header part", "[lints.'clippy']\nnursery = 'warn'\n", &[2]),
        ("spaced dotted key", "[lints]\nclippy . nursery = \"deny\"\n", &[2]),
        (
            "header with a comment",
            "[workspace.lints.clippy] # strict\nnursery = \"deny\" # all of it\n",
            &[2],
        ),
        (
            "dotted table header below lints",
            "[workspace.lints.clippy.nursery]\nlevel = \"warn\"\npriority = -1\n",
            &[1],
        ),
    ];
    for (label, text, expected) in cases {
        assert_eq!(manifest(text), expected, "{label}");
    }
    let accepted = [
        "[workspace.lints.clippy]\nnursery.level = \"allow\"\n",
        "[package.metadata.lints.clippy]\nnursery = \"deny\"\n",
        "[lints.clippy]\nnursery_x = \"deny\"\n",
        "[lints.clippy]\nuse_self = { level = \"deny\", priority = 1 }\n",
        "[package]\ndescription = \"nursery = deny\"\n",
    ];
    for text in accepted {
        assert_eq!(manifest(text), Vec::<usize>::new(), "{text:?}");
    }
    assert!(
        manifest_lines("[lints.clippy\nnursery = \"deny\"\n").is_err(),
        "invalid TOML is an error"
    );
}

#[test]
fn lint_tp_007_rustflags_in_a_cargo_configuration_are_read() {
    let cases: [(&str, &str, &[usize]); 6] = [
        ("separate flag", "[build]\nrustflags = [\"-D\", \"clippy::nursery\"]\n", &[2]),
        ("joined flag", "[build]\nrustflags = [\"-Dclippy::nursery\"]\n", &[2]),
        ("long flag", "[build]\nrustflags = [\"--warn=clippy::nursery\"]\n", &[2]),
        (
            "one string",
            "[target.x86_64-unknown-linux-gnu]\nrustflags = \"-F clippy::nursery\"\n",
            &[2],
        ),
        (
            "cfg target",
            "[target.'cfg(all())']\nrustflags = [\"--deny\", \"clippy::nursery\"]\n",
            &[2],
        ),
        ("dotted key", "build.rustflags = [\"-W\", \"clippy::nursery\"]\n", &[1]),
    ];
    for (label, text, expected) in cases {
        assert_eq!(config_lines(text).expect("parses"), expected, "{label}");
    }
    for text in [
        "[build]\nrustflags = [\"-A\", \"clippy::nursery\"]\n",
        "[build]\nrustflags = [\"-D\", \"clippy::all\", \"-D\", \"clippy::use_self\"]\n",
        "[env]\nNOTE = \"-D clippy::nursery\"\n",
    ] {
        assert_eq!(config_lines(text).expect("parses"), Vec::<usize>::new(), "{text:?}");
    }
}

#[test]
fn lint_tp_003_the_core_crates_name_their_nursery_lints() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the repository root");
    let found: Vec<(PathBuf, usize)> = crate::code_dojo::lint_policy_violations(root)
        .into_iter()
        .map(|violation| (violation.path, violation.line))
        .collect();
    assert_eq!(found, Vec::<(PathBuf, usize)>::new());
}

#[test]
fn lint_tp_008_every_source_file_and_configuration_of_a_core_crate_is_read() {
    let root = std::env::temp_dir().join(format!("bunny-lint-policy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let files = [
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/bunny-num\"]\n"),
        (".cargo/config.toml", "[build]\nrustflags = [\"-Dclippy::nursery\"]\n"),
        (
            "crates/bunny-num/Cargo.toml",
            "[package]\nname = \"bunny-num\"\n\n[lib]\npath = \"src/root.rs\"\n",
        ),
        ("crates/bunny-num/src/root.rs", "#![warn(clippy::nursery)]\nmod inner;\n"),
        ("crates/bunny-num/src/inner.rs", "//! Inner.\n#![deny(clippy::nursery)]\n"),
        ("crates/bunny-num/tests/strict.rs", "#![forbid(clippy::nursery)]\n"),
        ("crates/bunny-num/target/debug/build/out.rs", "#![deny(clippy::nursery)]\n"),
        ("crates/bunny-other/src/lib.rs", "#![deny(clippy::nursery)]\n"),
    ];
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture directory");
        std::fs::write(path, text).expect("fixture file");
    }
    let mut found = super::findings(&root, &["bunny-num"])
        .into_iter()
        .map(|finding| (finding.path.to_string_lossy().replace('\\', "/"), finding.line))
        .collect::<Vec<_>>();
    found.sort();
    let _ = std::fs::remove_dir_all(&root);
    let expected = [
        (".cargo/config.toml", 2),
        ("crates/bunny-num/src/inner.rs", 2),
        ("crates/bunny-num/src/root.rs", 1),
        ("crates/bunny-num/tests/strict.rs", 1),
    ]
    .map(|(path, line)| (path.to_string(), line));
    assert_eq!(found, expected.to_vec());
}
