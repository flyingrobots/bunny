//! The time and thread rules checked item by item against the std documentation of the
//! pinned toolchain.
//!
//! Oracle: `std_api_1.96.0.txt` lists every item of `std::thread` and `std::time` and every
//! method of `Duration` as the 1.96.0 documentation shows them (unstable items included,
//! since a nightly build could name them). The tables below classify each item by hand
//! from the rules' definitions in `docs/CODE_DOJO.md`; the tests compare that
//! classification with what the gate reports, and fail when the documentation lists an
//! item the tables do not classify or the toolchain moves without the list being renewed.

use std::collections::BTreeSet;

use super::tests::{reported, CORE_PATH, FAMILY};

const SNAPSHOT: &str = include_str!("std_api_1.96.0.txt");
const TOOLCHAIN_FILE: &str = include_str!("../../../rust-toolchain.toml");

/// Every documented item of `std::thread` and `std::time`, with the rule that bans it or
/// `None` for an item deterministic code may name.
const MODULE_ITEMS: &[(&str, &str, Option<&str>)] = &[
    // Waiting on the host scheduler or clock.
    ("thread", "sleep", Some("thread-sleep")),
    ("thread", "sleep_ms", Some("thread-sleep")),
    ("thread", "sleep_until", Some("thread-sleep")),
    ("thread", "park", Some("thread-sleep")),
    ("thread", "park_timeout", Some("thread-sleep")),
    ("thread", "park_timeout_ms", Some("thread-sleep")),
    ("thread", "yield_now", Some("thread-sleep")),
    ("thread", "available_parallelism", Some("host-parallelism")),
    // Threads themselves are allowed: CODE_STANDARDS.md Rule 6 lets parallelism in as long
    // as it does not change results, which tests show, not the gate.
    ("thread", "spawn", None),
    ("thread", "scope", None),
    ("thread", "Builder", None),
    ("thread", "JoinHandle", None),
    ("thread", "Scope", None),
    ("thread", "ScopedJoinHandle", None),
    ("thread", "add_spawn_hook", None),
    ("thread", "Thread", None),
    ("thread", "ThreadId", None),
    ("thread", "current", None),
    ("thread", "current_id", None),
    ("thread", "panicking", None),
    ("thread", "LocalKey", None),
    ("thread", "AccessError", None),
    ("thread", "Result", None),
    ("time", "Instant", Some("time-instant")),
    ("time", "SystemTime", Some("time-system")),
    ("time", "SystemTimeError", Some("time-system")),
    ("time", "UNIX_EPOCH", Some("time-system")),
    // A span of time as a value; its float methods are `float-boundary` in core.
    ("time", "Duration", None),
    ("time", "TryFromFloatSecsError", None),
];

/// The items of one `[std::<module>]` section of the snapshot.
fn documented_items(module: &str) -> BTreeSet<String> {
    section(&format!("[std::{module}]"))
        .into_iter()
        .filter_map(|line| line.split_once(' ').map(|(_, name)| name.to_string()))
        .collect()
}

fn section(header: &str) -> Vec<&'static str> {
    SNAPSHOT
        .lines()
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.is_empty())
        .collect()
}

fn family_rules(source: &str) -> Vec<String> {
    let mut rules = reported(CORE_PATH, source)
        .into_iter()
        .map(|(_, rule)| rule)
        .filter(|rule| FAMILY.contains(&rule.as_str()))
        .collect::<Vec<_>>();
    rules.dedup();
    rules
}

#[test]
fn the_std_snapshot_is_of_the_pinned_toolchain() {
    let version = SNAPSHOT
        .lines()
        .find_map(|line| line.strip_prefix("toolchain "))
        .expect("the snapshot names its toolchain");

    assert!(
        TOOLCHAIN_FILE.contains(&format!("channel = \"{version}\"")),
        "rust-toolchain.toml no longer pins {version}: renew std_api_{version}.txt and the \
         tables in std_api_tests.rs as docs/CODE_DOJO.md describes"
    );
}

#[test]
fn every_documented_thread_and_time_item_is_classified() {
    for module in ["thread", "time"] {
        let classified = MODULE_ITEMS
            .iter()
            .filter(|(owner, _, _)| *owner == module)
            .map(|(_, name, _)| (*name).to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            documented_items(module),
            classified,
            "std::{module}: the documented items and the classification differ"
        );
    }
}

#[test]
fn each_thread_and_time_item_gets_its_classified_rule() {
    let mut wrong = Vec::new();
    for (module, name, rule) in MODULE_ITEMS {
        let expected = rule.map(str::to_string).into_iter().collect::<Vec<_>>();
        for source in [
            format!("pub fn f() {{ let _ = std::{module}::{name}; }}"),
            format!("use std::{module}::{name};"),
        ] {
            let found = family_rules(&source);
            if found != expected {
                wrong.push(format!("`{source}`: expected {expected:?}, gate reported {found:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{} misclassified:\n{}", wrong.len(), wrong.join("\n"));
}

#[test]
fn every_float_method_of_duration_is_float_boundary() {
    let methods = section("[std::time::Duration methods]");
    let float_methods =
        methods.iter().filter(|name| name.ends_with("_f32") || name.ends_with("_f64")).count();
    assert_eq!(float_methods, 14, "witness: 1.96.0 documents 14 float methods of Duration");
    let mut wrong = Vec::new();
    for name in methods {
        let float = name.ends_with("_f32") || name.ends_with("_f64");
        for source in [
            format!("pub fn f(d: core::time::Duration) {{ let _ = d.{name}(); }}"),
            format!("pub fn f() {{ let _ = core::time::Duration::{name}; }}"),
        ] {
            let flagged = family_rules(&source).contains(&"float-boundary".to_string());
            if flagged != float {
                wrong.push(format!("`{source}`: float-boundary {flagged}, expected {float}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{} misclassified:\n{}", wrong.len(), wrong.join("\n"));
}

/// The documented `Path` methods that read the filesystem or the current directory, with
/// their rule; every other documented method is lexical and deterministic.
const PATH_HOST_METHODS: &[(&str, &str)] = &[
    ("canonicalize", "std-fs"),
    ("exists", "std-fs"),
    ("is_dir", "std-fs"),
    ("is_file", "std-fs"),
    ("is_symlink", "std-fs"),
    ("metadata", "std-fs"),
    ("read_dir", "std-fs"),
    ("read_link", "std-fs"),
    ("symlink_metadata", "std-fs"),
    ("try_exists", "std-fs"),
    ("absolute", "std-env"),
];

#[test]
fn every_host_method_of_path_gets_its_rule() {
    let methods = section("[std::path::Path methods]");
    assert_eq!(methods.len(), 65, "witness: 1.96.0 documents 65 methods of Path");
    let mut wrong = Vec::new();
    for name in methods {
        let expected = PATH_HOST_METHODS
            .iter()
            .find(|(method, _)| *method == name)
            .map(|(_, rule)| (*rule).to_string())
            .into_iter()
            .collect::<Vec<_>>();
        for source in [
            format!("pub fn f(p: &std::path::Path) {{ let _ = p.{name}(); }}"),
            format!("pub fn f() {{ let _ = std::path::Path::{name}; }}"),
        ] {
            let found = family_rules(&source);
            if found != expected {
                wrong.push(format!("`{source}`: expected {expected:?}, gate reported {found:?}"));
            }
        }
    }
    let documented = section("[std::path::Path methods]");
    let missing = PATH_HOST_METHODS
        .iter()
        .filter(|(method, _)| !documented.contains(method))
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "classified but not documented: {missing:?}");
    assert!(wrong.is_empty(), "{} misclassified:\n{}", wrong.len(), wrong.join("\n"));
}
