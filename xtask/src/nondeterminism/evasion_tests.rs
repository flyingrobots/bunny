//! Constructions that an independent review (flyingrobots/bunny#321) found getting banned
//! code past the source rules, each seeded as a form the gate must reject, and the
//! look-alikes next to them that it must still accept.
//!
//! Oracle: each snippet is written by hand from the rule's definition in
//! `docs/CODE_DOJO.md`, and each compiles as Rust (the raw and `extern crate` forms were
//! checked with `rustc` 1.96.0); the expected rule and line are part of the snippet.

use super::tests::{assert_rejects, reported, CORE_PATH, FAMILY};

#[test]
fn extern_crate_aliases_resolve_like_imports() {
    assert_rejects(
        "std-env",
        &[(
            "std renamed by extern crate",
            "extern crate std as host;\npub fn f() { let _ = host::env::var(\"HOME\"); }",
            2,
        )],
    );
    assert_rejects(
        "std-fs",
        &[("use through the alias", "extern crate std as host;\nuse host::fs::read;", 2)],
    );
    assert_rejects(
        "time-instant",
        &[(
            "type through the alias",
            "extern crate std as host;\npub fn f(_at: host::time::Instant) {}",
            2,
        )],
    );
    assert_rejects(
        "random",
        &[
            ("extern crate of a banned crate", "extern crate rand;", 1),
            (
                "renamed banned crate",
                "extern crate rand as dice;\npub fn f() -> u8 { dice::random() }",
                2,
            ),
        ],
    );
}

#[test]
fn raw_identifiers_match_their_plain_names() {
    assert_rejects(
        "std-fs",
        &[("raw first segment", "pub fn f() { let _ = r#std::fs::read(\"x\"); }", 1)],
    );
    assert_rejects(
        "std-env",
        &[("raw module segment", "pub fn f() { let _ = std::r#env::var(\"X\"); }", 1)],
    );
    assert_rejects(
        "unordered-container",
        &[
            ("raw imported name", "use std::collections::r#HashMap;", 1),
            (
                "raw name in a macro body",
                "pub fn f() { let _ = vec![std::collections::r#HashSet::<u8>::new()]; }",
                1,
            ),
        ],
    );
    assert_rejects(
        "std-process",
        &[(
            "raw spelling of an alias",
            "use std::process as host;\npub fn f() -> u32 { r#host::id() }",
            2,
        )],
    );
    assert_rejects(
        "panic-path",
        &[
            ("raw method name", "pub fn f(x: Option<u8>) -> u8 { x.r#unwrap() }", 1),
            ("raw macro name", "pub fn f() { r#panic!(\"x\") }", 1),
        ],
    );
}

#[test]
fn raw_float_types_are_float_boundary() {
    assert_rejects(
        "float-boundary",
        &[
            (
                "raw float type",
                "pub fn f(x: u32) -> u64 { let y: r#f64 = x.into(); y.to_bits() }",
                1,
            ),
            (
                "raw float path",
                "pub fn f(bits: u64) -> u64 { r#f64::from_bits(bits).to_bits() }",
                1,
            ),
        ],
    );
}

#[test]
fn self_in_a_use_group_names_the_module_itself() {
    assert_rejects(
        "std-fs",
        &[(
            "crate root renamed through self",
            "use std::{self as s};\npub fn f() { let _ = s::fs::read(\"x\"); }",
            2,
        )],
    );
    assert_rejects(
        "std-env",
        &[(
            "self beside another leaf",
            "use std::{self as host, fmt};\npub fn f() { let _ = host::env::var(\"X\"); }",
            2,
        )],
    );
}

#[test]
fn include_of_source_is_rejected_however_it_is_named() {
    assert_rejects(
        "include-source",
        &[
            ("item include", "include!(\"payload.inc\");", 1),
            ("expression include", "pub fn f() -> u64 { include!(\"payload.inc\") }", 1),
            ("include of a .rs file", "include!(\"generated.rs\");", 1),
            ("computed path", "include!(concat!(\"pay\", \"load.inc\"));", 1),
            ("std path", "std::include!(\"payload.inc\");", 1),
            ("core path", "core::include!(\"payload.inc\");", 1),
            ("renamed import", "use std::include as inc;\ninc!(\"payload.inc\");", 2),
            ("macro body", "pub fn f() -> Vec<u64> { vec![include!(\"payload.inc\")] }", 1),
        ],
    );
}

#[test]
fn path_attributes_must_name_a_scanned_file() {
    assert_rejects(
        "include-source",
        &[
            ("not a .rs file", "#[path = \"payload.inc\"]\nmod hidden;", 1),
            ("outside the crate", "#[path = \"../../../outside/hidden.rs\"]\nmod hidden;", 1),
            ("absolute", "#[path = \"/tmp/hidden.rs\"]\nmod hidden;", 1),
            ("backslash separators", "#[path = \"..\\\\..\\\\hidden.rs\"]\nmod hidden;", 1),
            ("under cfg_attr", "#[cfg_attr(test, path = \"payload.inc\")]\nmod hidden;", 1),
        ],
    );
}

#[test]
fn path_attributes_in_macros_nested_cfg_attr_and_unscanned_directories_are_rejected() {
    assert_rejects(
        "include-source",
        &[
            (
                "emitted by a macro",
                "macro_rules! load { () => { #[path = \"payload.inc\"] mod hidden; } }\nload!();",
                1,
            ),
            (
                "nested cfg_attr",
                "#[cfg_attr(all(), cfg_attr(all(), path = \"payload.inc\"))]\nmod hidden;",
                1,
            ),
            (
                "second path in a cfg_attr",
                "#[cfg_attr(all(), cfg_attr(any(), path = \"safe.rs\"), path = \"payload.inc\")]\nmod hidden;",
                1,
            ),
            ("vendor directory", "#[path = \"vendor/payload.rs\"]\nmod hidden;", 1),
            ("third_party directory", "#[path = \"third_party/payload.rs\"]\nmod hidden;", 1),
            ("target directory", "#[path = \"../target/payload.rs\"]\nmod hidden;", 1),
        ],
    );
}

#[test]
fn included_data_must_come_from_inside_the_crate() {
    assert_rejects(
        "include-data",
        &[
            (
                "string outside the crate",
                "pub const X: &str = include_str!(\"../../../generated/x.json\");",
                1,
            ),
            ("absolute bytes", "pub const X: &[u8] = include_bytes!(\"/etc/hostname\");", 1),
            (
                "computed path",
                "pub const X: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/x.bin\"));",
                1,
            ),
            (
                "renamed import",
                "use std::include_str as text;\npub const X: &str = text!(\"/etc/hostname\");",
                2,
            ),
            (
                "macro body",
                "pub fn f() -> Vec<&'static str> { vec![include_str!(\"/etc/hostname\")] }",
                1,
            ),
        ],
    );
}

#[test]
fn compile_time_environment_macros_are_std_env() {
    assert_rejects(
        "std-env",
        &[
            ("env", "pub fn f() -> &'static str { env!(\"HOME\") }", 1),
            ("option_env", "pub fn f() -> Option<&'static str> { option_env!(\"HOME\") }", 1),
            ("std path", "pub fn f() -> &'static str { std::env!(\"HOME\") }", 1),
            ("core path", "pub fn f() -> Option<&'static str> { core::option_env!(\"HOME\") }", 1),
            ("cargo variable", "pub fn f() -> &'static str { env!(\"CARGO_PKG_VERSION\") }", 1),
            ("macro body", "pub fn f() -> String { format!(\"{}\", env!(\"HOME\")) }", 1),
        ],
    );
}

/// Deterministic code next to the forms above that the rules must accept.
const ACCEPTED: &[(&str, &str)] = &[
    ("raw identifier `r#type`", "pub fn f(r#type: u8) -> u8 { r#type }"),
    (
        "`HashMap` in a comment and a string",
        "// A HashMap would reorder these.\npub fn f() -> &'static str { \"HashMap\" }",
    ),
    (
        "core renamed by extern crate",
        "extern crate core as c;\npub fn f() -> c::time::Duration { c::time::Duration::ZERO }",
    ),
    (
        "alloc renamed by extern crate",
        "extern crate alloc as a;\npub fn f() -> a::collections::BTreeMap<u8, u8> { a::collections::BTreeMap::new() }",
    ),
    ("module path inside the crate", "#[path = \"support/oracle.rs\"]\nmod oracle;"),
    ("data inside the crate", "pub const X: &str = include_str!(\"fixtures/case.txt\");"),
    (
        "data in the crate's tests",
        "pub const X: &[u8] = include_bytes!(\"../tests/fixtures/case.bin\");",
    ),
    ("a function named include", "fn include(x: u8) -> u8 { x }\npub fn f() -> u8 { include(1) }"),
    ("a binding named env", "pub fn f(env: u8) -> u8 { env }"),
    ("another crate's macro named include", "pub fn f() -> u8 { local::include!(1) }"),
];

#[test]
fn look_alikes_of_the_evasions_are_accepted() {
    let mut rejected = Vec::new();
    for (label, source) in ACCEPTED {
        let found = reported(CORE_PATH, source)
            .into_iter()
            .filter(|(_, rule)| FAMILY.contains(&rule.as_str()))
            .collect::<Vec<_>>();
        if !found.is_empty() {
            rejected.push(format!("{label}: {found:?}"));
        }
    }
    assert!(rejected.is_empty(), "look-alikes were rejected:\n{}", rejected.join("\n"));
}
