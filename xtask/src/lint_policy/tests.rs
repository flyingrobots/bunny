//! The `blanket-nursery` check: seeded crate roots and manifests whose findings, with their
//! lines, follow from Rule 10 as `docs/topics/lint-policy/README.md` states it, and the
//! repository itself.

use std::path::{Path, PathBuf};

use super::{attribute_lines, manifest_lines};

fn lines(source: &str) -> Vec<usize> {
    attribute_lines(source).expect("parses")
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
        // The group on an item rather than the crate, or only mentioned.
        ("#[deny(clippy::nursery)]\nfn one() {}\n", &[]),
        ("// #![deny(clippy::nursery)]\nconst TEXT: &str = \"#![deny(clippy::nursery)]\";\n", &[]),
        ("#![deny(nursery)]\n#![deny(other::nursery)]\n", &[]),
    ];
    for (source, expected) in cases {
        assert_eq!(lines(source), expected, "{source:?}");
    }
    assert!(attribute_lines("#![deny(").is_err(), "a crate root that does not parse is an error");
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
    for (manifest, expected) in cases {
        assert_eq!(manifest_lines(manifest), expected, "{manifest:?}");
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
