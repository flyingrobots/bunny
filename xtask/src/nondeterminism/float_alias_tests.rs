//! Floats under names `float-boundary` did not know (flyingrobots/bunny#348): the C type
//! aliases, the unstable float types and type aliases of a float, each seeded as a form
//! the gate must reject in a core crate, with look-alikes it must accept.
//!
//! Oracle: each snippet is written by hand from Rule 2 as `docs/CODE_DOJO.md` states it:
//! the ban applies to the float types, not to their spellings. The issue's two forms were
//! compiled by the reviewer in `bunny-num`; the expected rule and line are part of each
//! snippet.

use super::tests::{assert_rejects, reported, CORE_PATH};

const FLOAT: &str = "float-boundary";

#[test]
fn c_float_aliases_and_unstable_floats_are_float_boundary() {
    assert_rejects(
        FLOAT,
        &[
            (
                "issue: c_double cast and sqrt",
                "pub fn g5(x: i64) -> i64 {\n    (x as core::ffi::c_double).sqrt() as i64\n}",
                2,
            ),
            (
                "issue: c_float return type",
                "pub fn g5b(x: u16) -> std::os::raw::c_float {\n    x.into()\n}",
                1,
            ),
            (
                "issue: c_float path",
                "pub fn g5c(x: u16) -> u32 {\n    core::ffi::c_float::from(x).to_bits()\n}",
                2,
            ),
            ("imported c_double", "use core::ffi::c_double;", 1),
            ("std::ffi c_float", "pub fn f(x: std::ffi::c_float) -> u32 { x.to_bits() }", 1),
            ("libc c_double", "pub fn f(x: libc::c_double) -> u64 { x.to_bits() }", 1),
            ("core::primitive", "pub fn f(x: core::primitive::f64) -> u64 { x.to_bits() }", 1),
            ("std::primitive", "pub fn f(x: u16) -> std::primitive::f32 { x.into() }", 1),
            ("f16 type", "pub fn f(x: f16) -> u16 { x.to_bits() }", 1),
            ("f128 type", "pub fn f(x: f128) -> u128 { x.to_bits() }", 1),
            (
                "c_double in a macro",
                "pub fn f() -> usize { vec![core::ffi::c_double::default()].len() }",
                1,
            ),
        ],
    );
}

#[test]
fn uses_of_a_float_alias_are_float_boundary() {
    assert_rejects(
        FLOAT,
        &[
            (
                "a waived alias used in the next function",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 pub type Real = f64;\n\
                 pub fn f(x: Real) -> u64 { x.to_bits() }",
                3,
            ),
            (
                "an alias of an alias",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type A = f64;\n\
                 // dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type B = A;\n\
                 pub fn f(x: B) -> u64 { x.to_bits() }",
                5,
            ),
            (
                "an alias of an alias declared first",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type B = A;\n\
                 // dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type A = f64;\n\
                 pub fn f(x: B) -> u64 { x.to_bits() }",
                5,
            ),
            (
                "a c_double alias",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type Real = core::ffi::c_double;\n\
                 pub fn f(x: i32) -> i64 { (x as Real).sqrt() as i64 }",
                3,
            ),
            (
                "a renamed import used later",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 use std::ffi::c_float as Single;\n\
                 pub fn f(x: u16) -> Single { x.into() }",
                3,
            ),
            (
                "an alias inside a macro body",
                "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\n\
                 type Real = f32;\n\
                 pub fn f() -> usize { vec![Real::default()].len() }",
                3,
            ),
            (
                "an alias in an inline module",
                "mod dto {\n    // dojo: allow float-boundary -- reviewed DTO alias for the boundary\n    \
                 pub type Real = f64;\n}\n\
                 pub fn f(x: dto::Real) -> u64 { x.to_bits() }",
                5,
            ),
        ],
    );
}

#[test]
fn float_alias_look_alikes_are_accepted() {
    let accepted: [(&str, &str); 5] = [
        ("an integer C alias", "pub fn f(x: core::ffi::c_int) -> i32 { x }"),
        ("an integer type alias", "pub type Count = u64;\npub fn f(x: Count) -> Count { x + 1 }"),
        ("a name containing c_double", "pub fn f(c_double_count: u8) -> u8 { c_double_count }"),
        ("a struct named Real", "pub struct Real(i64);\npub fn f(x: Real) -> i64 { x.0 }"),
        ("a fixed-point alias", "pub type Real = Fixed;\npub fn f(x: Real) -> Real { x }"),
    ];
    let mut rejected = Vec::new();
    for (label, source) in accepted {
        let found = reported(CORE_PATH, source)
            .into_iter()
            .filter(|(_, rule)| rule == FLOAT)
            .collect::<Vec<_>>();
        if !found.is_empty() {
            rejected.push(format!("{label}: {found:?}"));
        }
    }
    assert!(rejected.is_empty(), "look-alikes were rejected:\n{}", rejected.join("\n"));
}

#[test]
fn a_float_alias_of_another_core_crate_is_float_boundary() {
    let files = [
        (
            "crates/bunny-num/src/lib.rs",
            "// dojo: allow float-boundary -- reviewed DTO alias for the boundary\npub type Scalar = f32;\n",
        ),
        (
            "crates/bunny-geom/src/lib.rs",
            "use bunny_num::Scalar;\npub fn negative(x: Scalar) -> bool {\n    x < Scalar::from_bits(0)\n}\n",
        ),
        (
            "crates/bunny-linalg/src/lib.rs",
            "pub struct Scalar(i64);\npub fn f(x: Scalar) -> i64 { x.0 }\n",
        ),
    ];
    let owned = files
        .iter()
        .map(|(path, text)| (std::path::PathBuf::from(path), (*text).to_string()))
        .collect::<Vec<_>>();
    let found = crate::code_dojo::tree_violations(&owned)
        .into_iter()
        .filter(|violation| violation.rule == FLOAT)
        .map(|violation| (violation.path.to_string_lossy().replace('\\', "/"), violation.line))
        .collect::<Vec<_>>();
    let expected = [1, 2, 3].map(|line| ("crates/bunny-geom/src/lib.rs".to_string(), line));
    assert_eq!(found, expected.to_vec(), "the use, the parameter and the comparison in bunny-geom");
}
