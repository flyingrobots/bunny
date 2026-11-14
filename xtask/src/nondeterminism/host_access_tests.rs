//! Host access outside Echo's rule list that `CODE_STANDARDS.md` Rule 4 still forbids in
//! deterministic crates (filesystem, environment, process and terminal input), and the
//! doc tests, which run under `cargo test` like any other test.
//!
//! Oracle: each snippet is written by hand from the rule's definition in
//! `docs/CODE_DOJO.md`; the expected rule and line are part of the snippet. The `Path`
//! methods are the ones the std 1.96.0 documentation lists that read the filesystem
//! (`std_api_1.96.0.txt`).

use super::tests::{assert_rejects, reported, CORE_PATH, FAMILY};

#[test]
fn path_methods_that_read_the_filesystem_are_std_fs() {
    assert_rejects(
        "std-fs",
        &[
            ("exists", "pub fn f(p: &std::path::Path) -> bool { p.exists() }", 1),
            ("try_exists", "pub fn f(p: &Path) -> bool { p.try_exists().unwrap_or(false) }", 1),
            ("is_file and is_dir", "pub fn f(p: &Path) -> bool { p.is_file() || p.is_dir() }", 1),
            ("metadata", "pub fn f(p: &Path) -> bool { p.metadata().is_ok() }", 1),
            ("canonicalize", "pub fn f(p: &Path) -> bool { p.canonicalize().is_ok() }", 1),
            ("read_dir", "pub fn f(p: &Path) -> bool { p.read_dir().is_ok() }", 1),
            ("qualified call", "pub fn f(p: &std::path::Path) -> bool { std::path::Path::exists(p) }", 1),
            ("PathBuf call", "pub fn f(p: &PathBuf) -> bool { PathBuf::is_symlink(p) }", 1),
            ("macro body", "pub fn f(p: &Path) -> String { format!(\"{}\", p.exists()) }", 1),
        ],
    );
}

#[test]
fn platform_filesystem_modules_are_banned() {
    assert_rejects(
        "std-fs",
        &[
            ("unix symlink", "use std::os::unix::fs::symlink;", 1),
            (
                "windows symlink",
                "pub fn f() { let _ = std::os::windows::fs::symlink_file(\"a\", \"b\"); }",
                1,
            ),
            ("linux metadata", "use std::os::linux::fs::MetadataExt;", 1),
            ("file descriptors", "use std::os::fd::AsRawFd;", 1),
            ("platform prelude glob", "use std::os::unix::prelude::*;", 1),
        ],
    );
    assert_rejects("network", &[("unix socket", "use std::os::unix::net::UnixStream;", 1)]);
    assert_rejects(
        "std-process",
        &[
            ("unix command extension", "use std::os::unix::process::CommandExt;", 1),
            ("pipe", "pub fn f() { let _ = std::io::pipe(); }", 1),
        ],
    );
    assert_rejects(
        "std-env",
        &[
            ("absolute path", "pub fn f(p: &Path) -> bool { std::path::absolute(p).is_ok() }", 1),
            ("absolute method", "pub fn f(p: &Path) -> bool { p.absolute().is_ok() }", 1),
        ],
    );
}

#[test]
fn terminal_input_is_host_input() {
    assert_rejects(
        "host-input",
        &[
            ("stdin", "pub fn f() { let _ = std::io::stdin(); }", 1),
            ("Stdin in a group", "use std::io::{self, Stdin};", 1),
            ("IsTerminal", "use std::io::IsTerminal;", 1),
            ("is_terminal method", "pub fn f(out: &std::io::Stdout) -> bool { out.is_terminal() }", 1),
            ("module alias", "use std::io as host;\npub fn f() { let _ = host::stdin().lock(); }", 2),
            ("macro body", "pub fn f() -> String { format!(\"{:?}\", std::io::stdin()) }", 1),
        ],
    );
}

#[test]
fn doc_tests_are_scanned() {
    assert_rejects(
        "std-fs",
        &[(
            "outer doc test",
            "/// ```\n/// let _ = std::fs::read(\"x\");\n/// ```\npub fn f() {}",
            2,
        )],
    );
    assert_rejects(
        "time-instant",
        &[("inner doc test", "//! ```\n//! let _ = std::time::Instant::now();\n//! ```", 2)],
    );
    assert_rejects(
        "unordered-container",
        &[(
            "hidden line",
            "/// ```\n/// # use std::collections::HashMap;\n/// let m: HashMap<u8, u8> = HashMap::new();\n\
             /// ```\npub fn f() {}",
            3,
        )],
    );
    assert_rejects(
        "thread-sleep",
        &[(
            "rust,no_run block",
            "/// ```rust,no_run\n/// std::thread::sleep(core::time::Duration::ZERO);\n/// ```\n\
             pub fn f() {}",
            2,
        )],
    );
    assert_rejects(
        "std-env",
        &[(
            "block doc comment",
            "/**\n```\nlet _ = std::env::var(\"X\");\n```\n*/\npub fn f() {}",
            3,
        )],
    );
}

/// Deterministic code next to the forms above that the rules must accept.
const ACCEPTED: &[(&str, &str)] = &[
    ("a method named exists with an argument", "pub fn f(s: &Set) -> bool { s.exists(3) }"),
    ("a field named metadata", "pub fn f(m: &Mesh) -> u8 { m.metadata }"),
    ("lexical path operations", "pub fn f() -> std::path::PathBuf { std::path::Path::new(\"x\").join(\"y\") }"),
    ("writing to stdout", "pub fn f() { let _ = std::io::stdout(); }"),
    ("C integer types", "pub fn f(x: std::os::raw::c_int) -> i32 { x }"),
    ("a text block", "/// ```text\n/// std::fs::read(\"x\")\n/// ```\npub fn f() {}"),
    ("prose naming a hash map", "/// A HashMap would reorder these.\npub fn f() {}"),
];

#[test]
fn host_access_look_alikes_are_accepted() {
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
