//! Constructions from an independent review (flyingrobots/bunny#347) that bring host access
//! or unscanned code into a deterministic crate through macros, prelude paths and std
//! items no rule named, each seeded as a form the gate must reject, with the look-alikes
//! next to them that it must still accept.
//!
//! Oracle: each snippet is written by hand from the rule's definition in
//! `docs/CODE_DOJO.md`; the expected rule and line are part of the snippet. The five forms
//! of the issue were compiled and run by the reviewer as a `bunny-num` integration test.

use super::tests::{assert_rejects, reported, CORE_PATH, FAMILY};

const METAVARIABLE: &str = "macro-metavariable";

#[test]
fn metavariables_in_unresolvable_positions_are_rejected() {
    assert_rejects(
        METAVARIABLE,
        &[
            (
                "issue: segment below std::time",
                "macro_rules! clock { ($t:ident) => { std::time::$t::now() }; }",
                1,
            ),
            (
                "segment below the root",
                "macro_rules! read {\n    ($m:ident) => { std::$m::read(\"x\") };\n}",
                2,
            ),
            (
                "first segment",
                "macro_rules! clock { ($root:ident) => { $root::time::Instant::now() }; }",
                1,
            ),
            (
                "issue: macro name",
                "macro_rules! call { ($m:ident, $p:literal) => { $m!($p) }; }",
                1,
            ),
            (
                "macro name with brackets",
                "macro_rules! call { ($m:ident) => { $m![\"x\"] }; }",
                1,
            ),
            (
                "issue: path attribute value",
                "macro_rules! module_at {\n    ($p:literal) => {\n        #[path = $p]\n\
                 mod outside;\n    };\n}",
                3,
            ),
            (
                "whole attribute",
                "macro_rules! module_at { ($a:meta) => { #[$a] mod outside; }; }",
                1,
            ),
            (
                "path inside cfg_attr",
                "macro_rules! m { ($p:literal) => { #[cfg_attr(all(), path = $p)] mod outside; }; }",
                1,
            ),
            (
                "inner attribute",
                "macro_rules! m { ($a:meta) => { mod inner { #![$a] } }; }",
                1,
            ),
            (
                "method name",
                "macro_rules! probe { ($p:expr, $m:ident) => { $p.$m() }; }",
                1,
            ),
            (
                "segments joined by a repetition",
                "macro_rules! path { ($($s:ident)::+) => { $($s)::+ :: now() }; }",
                1,
            ),
            (
                "repetition after a separator",
                "macro_rules! m { ($($s:ident),*) => { std::$($s)* }; }",
                1,
            ),
            (
                "inside a nested macro call",
                "macro_rules! show {\n    ($t:ident) => { format!(\"{:?}\", core::$t::now()) };\n}",
                2,
            ),
            (
                "in a use declaration",
                "macro_rules! import { ($m:ident) => { use std::$m::read; }; }",
                1,
            ),
            (
                "turbofish method",
                "macro_rules! m { ($x:expr, $f:ident) => { $x.$f::<u8>() }; }",
                1,
            ),
        ],
    );
}

#[test]
fn metavariable_look_alikes_are_accepted() {
    let accepted: [(&str, &str); 9] = [
        ("path separators in a matcher", "macro_rules! m { ($($s:ident)::+) => { 0 }; }"),
        ("$crate path", "macro_rules! m { () => { $crate::fixed::ONE }; }"),
        ("qualified type", "macro_rules! m { ($t:ty) => { <$t>::MAX }; }"),
        ("comparison", "macro_rules! m { ($a:expr, $b:expr) => { $a != $b }; }"),
        ("field access", "macro_rules! m { ($s:ident, $f:ident) => { $s.$f + 1 }; }"),
        ("doc attribute", "macro_rules! m { ($d:literal) => { #[doc = $d] pub struct S; }; }"),
        ("matcher fragments", "macro_rules! m { ($t:ident, $($x:expr),*) => { 0 }; }"),
        ("expression argument", "macro_rules! m { ($e:expr) => { $e + 1 }; }"),
        ("repetition of values", "macro_rules! m { ($($x:expr),*) => { [$($x),*] }; }"),
    ];
    let mut rejected = Vec::new();
    for (label, source) in accepted {
        let found = family(source);
        if !found.is_empty() {
            rejected.push(format!("{label}: {found:?}"));
        }
    }
    assert!(rejected.is_empty(), "look-alikes were rejected:\n{}", rejected.join("\n"));
}

#[test]
fn banned_macros_are_found_through_every_std_path() {
    assert_rejects(
        "include-source",
        &[
            (
                "issue: include through std::prelude::v1",
                "use std::prelude::v1::include as inc;\npub fn f() -> u32 { inc!(\"x.txt\") }",
                2,
            ),
            ("the import itself", "use std::prelude::v1::include as inc;", 1),
            (
                "core prelude, no import",
                "pub fn f() -> u32 { core::prelude::v1::include!(\"x\") }",
                1,
            ),
            (
                "module alias of a prelude",
                "use core::prelude::rust_2021 as p;\npub fn f() -> u32 { p::include!(\"x\") }",
                2,
            ),
            ("in a group", "use std::prelude::rust_2024::{include, vec};", 1),
        ],
    );
    assert_rejects(
        "std-env",
        &[
            (
                "issue: env through core::prelude::rust_2021",
                "use core::prelude::rust_2021::env as build_env;\n\
                 pub fn f() -> &'static str { build_env!(\"X\") }",
                2,
            ),
            (
                "issue: option_env with no import",
                "pub fn f() -> Option<&'static str> { core::prelude::v1::option_env!(\"PATH\") }",
                1,
            ),
            ("file!, bare", "pub fn f() -> &'static str { file!() }", 1),
            ("file! under core", "pub fn f() -> &'static str { core::file!() }", 1),
            (
                "file! beside a function imported as file (review)",
                "mod local { pub fn file() {} }\nuse local::file;\npub fn f() -> &'static str { file!() }",
                3,
            ),
            (
                "option_env imported inside a macro body (review)",
                "macro_rules! read_host {\n    () => {{\n        use std::prelude::v1::option_env as read;\n        \
                 read!(\"HOME\")\n    }};\n}",
                3,
            ),
            (
                "file! through the prelude",
                "use std::prelude::v1::file as here;\npub fn f() -> &'static str { here!() }",
                1,
            ),
        ],
    );
    assert_rejects(
        "include-data",
        &[(
            "include_str through a prelude, host file",
            "pub fn f() -> &'static str { std::prelude::rust_2024::include_str!(\"/etc/hostname\") }",
            1,
        )],
    );
    assert_rejects(
        "host-cpu",
        &[
            ("feature detection", "pub fn f() -> bool { is_x86_feature_detected!(\"avx2\") }", 1),
            (
                "aarch64 detection",
                "pub fn f() -> bool { std::arch::is_aarch64_feature_detected!(\"neon\") }",
                1,
            ),
        ],
    );
}

#[test]
fn host_state_beyond_the_earlier_lists_is_rejected() {
    assert_rejects(
        "std-env",
        &[
            (
                "issue: Backtrace::capture",
                "pub fn f() -> String { format!(\"{}\", std::backtrace::Backtrace::capture()) }",
                1,
            ),
            ("imported backtrace", "use std::backtrace::Backtrace;", 1),
            ("force_capture by name", "pub fn f() { let _ = Backtrace::force_capture(); }", 1),
            ("caller location", "pub fn f() -> u32 { std::panic::Location::caller().line() }", 1),
            ("core location", "use core::panic::Location;", 1),
            (
                "backtrace style",
                "pub fn f() -> bool { std::panic::get_backtrace_style().is_some() }",
                1,
            ),
        ],
    );
    assert_rejects(
        "host-cpu",
        &[
            ("std::arch", "use std::arch::x86_64::_rdtsc;", 1),
            ("core::arch", "pub fn f() { let _ = core::arch::aarch64::__nop; }", 1),
        ],
    );
    assert_rejects(
        "random",
        &[("std::random", "pub fn f() -> u64 { std::random::random(..) }", 1)],
    );
    assert_rejects(
        "host-platform",
        &[
            ("platform thread extension", "use std::os::unix::thread::JoinHandleExt;", 1),
            ("platform ffi extension", "use std::os::windows::ffi::OsStrExt;", 1),
            ("the module itself", "use std::os;", 1),
        ],
    );
    assert_rejects(
        "std-process",
        &[(
            "last OS error",
            "pub fn f() -> Option<i32> { std::io::Error::last_os_error().raw_os_error() }",
            1,
        )],
    );
}

#[test]
fn an_unclassified_std_module_is_reported() {
    assert_rejects(
        "unclassified-std",
        &[
            ("unknown std module", "pub fn f() { let _ = std::hostinfo::name(); }", 1),
            ("unknown core module", "use core::hostinfo;", 1),
            ("unknown alloc module", "pub fn f(_x: alloc::hostinfo::Name) {}", 1),
            ("unknown std macro", "pub fn f() { std::hostinfo!(); }", 1),
        ],
    );
}

#[test]
fn look_alikes_of_the_host_items_are_accepted() {
    let accepted: [(&str, &str); 7] = [
        ("os raw integer", "pub fn f(x: std::os::raw::c_int) -> i32 { x }"),
        ("panic unwinding", "pub fn f() -> bool { std::panic::catch_unwind(|| 1).is_ok() }"),
        ("line and column", "pub fn f() -> (u32, u32) { (line!(), column!()) }"),
        ("module path", "pub fn f() -> &'static str { module_path!() }"),
        ("another crate's file macro", "pub fn f() -> u8 { other::file!() }"),
        ("io error from a kind", "pub fn f() -> std::io::Error { std::io::ErrorKind::Other.into() }"),
        (
            "alloc collections",
            "pub fn f() -> alloc::collections::BTreeMap<u8, u8> { alloc::collections::BTreeMap::new() }",
        ),
    ];
    let mut rejected = Vec::new();
    for (label, source) in accepted {
        let found = family(source);
        if !found.is_empty() {
            rejected.push(format!("{label}: {found:?}"));
        }
    }
    assert!(rejected.is_empty(), "look-alikes were rejected:\n{}", rejected.join("\n"));
}

fn family(source: &str) -> Vec<(usize, String)> {
    reported(CORE_PATH, source)
        .into_iter()
        .filter(|(_, rule)| FAMILY.contains(&rule.as_str()))
        .collect()
}
