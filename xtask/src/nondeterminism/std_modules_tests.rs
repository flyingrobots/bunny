//! The root modules and macros of `std`, `core` and `alloc` checked against the pinned
//! toolchain's documentation and against what the gate reports for each.
//!
//! Oracle: `std_api_1.96.0.txt` lists the modules and macros the 1.96.0 documentation shows
//! at each root (unstable ones included, since a nightly build could name them). The
//! classification in `std_modules.rs` is written by hand from the rules' definitions in
//! `docs/CODE_DOJO.md`; these tests fail when the documentation lists a name the table
//! does not classify, when the table names one no root documents, and when the gate
//! reports anything but the classified rule for a name.

use std::collections::BTreeSet;

use super::std_modules::{Handling, MACROS, MODULES, ROOTS};
use super::tests::{reported, CORE_PATH, FAMILY};

const SNAPSHOT: &str = include_str!("std_api_1.96.0.txt");

/// The names of the `[<root> modules]` or `[<root> macros]` section of the snapshot.
fn documented(root: &str, kind: &str) -> Vec<&'static str> {
    section(&format!("[{root} {kind}]"))
}

/// The names (each line's second word) of one section of the snapshot.
fn section(header: &str) -> Vec<&'static str> {
    let names = SNAPSHOT
        .lines()
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(' ').map(|(_, name)| name))
        .collect::<Vec<_>>();
    assert!(!names.is_empty(), "the snapshot has no {header} section");
    names
}

fn classified(table: &[(&'static str, Handling)]) -> BTreeSet<&'static str> {
    table.iter().map(|(name, _)| *name).collect()
}

#[test]
fn every_documented_root_module_and_macro_is_classified() {
    for (kind, table) in [("modules", MODULES), ("macros", MACROS)] {
        let documented_names =
            ROOTS.iter().flat_map(|root| documented(root, kind)).collect::<BTreeSet<_>>();
        let names = classified(table);
        let unclassified = documented_names.difference(&names).collect::<Vec<_>>();
        let undocumented = names.difference(&documented_names).collect::<Vec<_>>();
        assert!(unclassified.is_empty(), "documented {kind} not classified: {unclassified:?}");
        assert!(undocumented.is_empty(), "classified {kind} no root documents: {undocumented:?}");
        assert_eq!(names.len(), table.len(), "a {kind} name is classified twice");
    }
}

/// The rule the classification expects for `name` under `root`, if any.
fn expected(handling: Handling, root: &str) -> Vec<String> {
    match handling {
        Handling::Banned(rule, roots) if roots.contains(&root) => vec![rule.to_string()],
        Handling::Banned(..) | Handling::Items | Handling::Permitted => Vec::new(),
    }
}

/// The family rules reported for `source`, without `float-boundary`, which judges float
/// names (`std::f64`) separately from this classification.
fn family_rules(source: &str) -> Vec<String> {
    let mut rules = reported(CORE_PATH, source)
        .into_iter()
        .map(|(_, rule)| rule)
        .filter(|rule| FAMILY.contains(&rule.as_str()) && rule != "float-boundary")
        .collect::<Vec<_>>();
    rules.sort();
    rules.dedup();
    rules
}

#[test]
fn each_root_module_and_macro_gets_its_classified_rule() {
    let mut wrong = Vec::new();
    for root in ROOTS {
        for (name, handling) in MODULES {
            if documented(root, "modules").contains(name) {
                let source = format!("use {root}::{name};");
                let found = family_rules(&source);
                if found != expected(*handling, root) {
                    wrong.push(format!(
                        "`{source}`: expected {handling:?}, gate reported {found:?}"
                    ));
                }
            }
        }
        for (name, handling) in MACROS {
            if documented(root, "macros").contains(name) {
                let source = format!("pub fn f() {{ {root}::{name}!(\"x\"); }}");
                let found = family_rules(&source);
                if found != expected(*handling, root) {
                    wrong.push(format!(
                        "`{source}`: expected {handling:?}, gate reported {found:?}"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{} names misreported:\n{}", wrong.len(), wrong.join("\n"));
}

#[test]
fn the_platform_modules_of_std_os_are_banned_except_raw() {
    let mut wrong = Vec::new();
    for module in section("[std::os]").iter().copied().chain(["raw"]) {
        let source = format!("use std::os::{module};");
        let found = family_rules(&source);
        let expected_rules = match module {
            "raw" => Vec::new(),
            "fd" => vec!["std-fs".to_string()],
            _ => vec!["host-platform".to_string()],
        };
        if found != expected_rules {
            wrong.push(format!("`{source}`: expected {expected_rules:?}, gate reported {found:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
