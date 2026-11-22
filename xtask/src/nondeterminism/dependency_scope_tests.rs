//! The dependency graph of a deterministic crate holds only scanned crates and reviewed
//! registry crates (flyingrobots/bunny#349): a workspace or path crate outside the
//! deterministic set, a crates.io crate missing from the allow list and a crate from any
//! other source are findings, on `cargo tree --prefix depth --format {p}` listings.
//!
//! Oracle: each listing is written by hand in the format `cargo tree` prints (a path crate
//! carries its directory in parentheses, a crates.io crate none, a git crate its URL); the
//! expected rule follows from the rule in `docs/topics/code-dojo-determinism/README.md`.

use crate::code_dojo::dependency_violations;

fn no_manifest(_crate_name: &str) -> String {
    String::new()
}

fn rules_and_dependencies(tree: &str) -> Vec<(String, String)> {
    dependency_violations(tree, &no_manifest)
        .iter()
        .map(|violation| {
            let text = violation.to_string();
            let named = text.split('`').nth(1).unwrap_or_default().to_string();
            (violation.rule.to_string(), named)
        })
        .collect()
}

/// A label, a `cargo tree` listing, and the expected rules with the crates they name.
type TreeCase = (&'static str, &'static str, Vec<(String, String)>);

fn pair(rule: &str, dependency: &str) -> (String, String) {
    (rule.to_string(), dependency.to_string())
}

#[test]
fn crates_the_gate_does_not_scan_are_findings() {
    let cases: [TreeCase; 10] = [
        (
            "issue: a workspace crate outside the list",
            "0bunny-num v0.7.1 (/work/crates/bunny-num)\n1zz-probe-crate v0.1.0 (/work/crates/zz-probe-crate)\n",
            vec![pair("unscanned-dependency", "zz-probe-crate")],
        ),
        (
            "issue: the tooling crate",
            "0bunny-geom v0.7.1 (/work/crates/bunny-geom)\n1bunny-perf v0.7.1 (/work/crates/bunny-perf)\n",
            vec![pair("unscanned-dependency", "bunny-perf")],
        ),
        (
            "issue: an unlisted crates.io crate",
            "0bunny-num v0.7.1 (/work/crates/bunny-num)\n1chrono v0.4.42\n",
            vec![pair("unlisted-dependency", "chrono")],
        ),
        (
            "a path crate outside the workspace",
            "0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n1helper v0.1.0 (/elsewhere/helper)\n",
            vec![pair("unscanned-dependency", "helper")],
        ),
        (
            "a crate named like a deterministic one at another path",
            "0bunny-geom v0.7.1 (/work/crates/bunny-geom)\n1bunny-num v0.7.1 (/work/vendor/bunny-num)\n",
            vec![pair("unscanned-dependency", "bunny-num")],
        ),
        (
            "an allow-listed name from git",
            "0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n1sha2 v0.10.9 (https://github.com/x/sha2#0123abcd)\n",
            vec![pair("unlisted-dependency", "sha2")],
        ),
        (
            "an alternative registry",
            "0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n1digest v0.10.7 (registry `mirror`)\n",
            vec![pair("unlisted-dependency", "digest")],
        ),
        (
            "an unlisted procedural macro",
            "0bunny-codec v0.7.1 (/work/crates/bunny-codec)\n1serde_derive v1.0.228 (proc-macro)\n",
            vec![pair("unlisted-dependency", "serde_derive")],
        ),
        (
            "deep below an allow-listed crate",
            "0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n1sha2 v0.10.9\n2digest v0.10.7\n3uuid v1.18.0\n",
            vec![pair("unlisted-dependency", "uuid")],
        ),
        (
            "a banned crate keeps its own rule",
            "0bunny-num v0.7.1 (/work/crates/bunny-num)\n1ahash v0.8.12\n1fastrand v2.3.0\n",
            vec![pair("unlisted-dependency", "ahash"), pair("random", "fastrand")],
        ),
    ];
    let mut wrong = Vec::new();
    for (label, tree, expected) in cases {
        let found = rules_and_dependencies(tree);
        if found != expected {
            wrong.push(format!("{label}: expected {expected:?}, found {found:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_graph_on_main_is_accepted() {
    let tree = "\
0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)
1bunny-geom v0.7.1 (/work/crates/bunny-geom)
2bunny-num v0.7.1 (/work/crates/bunny-num)
1sha2 v0.10.9
2cfg-if v1.0.4
2cpufeatures v0.2.17
3libc v0.2.186
2digest v0.10.7
3block-buffer v0.10.4
4generic-array v0.14.7
5typenum v1.20.1
5version_check v0.9.5
3crypto-common v0.1.7
4generic-array v0.14.7
4typenum v1.20.1
0bunny-codec v0.7.1 (C:\\work\\crates\\bunny-codec)
1bunny-mesh v0.7.1 (C:\\work\\crates\\bunny-mesh)
";
    assert_eq!(rules_and_dependencies(tree), Vec::new());
}

#[test]
fn the_new_dependency_rules_cannot_be_waived_in_a_manifest() {
    let tree =
        "0bunny-num v0.7.1 (/work/crates/bunny-num)\n1chrono v0.4.42\n1helper v0.1.0 (/x/helper)\n";
    let manifest = |_crate_name: &str| {
        "# dojo: allow unlisted-dependency -- reviewed clock-free use of chrono\n\
         # dojo: allow unscanned-dependency -- reviewed helper crate for the probe\n"
            .to_string()
    };
    let rules = dependency_violations(tree, &manifest)
        .iter()
        .map(|violation| violation.rule)
        .collect::<Vec<_>>();
    assert_eq!(rules, vec!["unlisted-dependency", "unscanned-dependency"]);
}
