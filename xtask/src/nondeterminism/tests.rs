//! Seeded violations for every Echo `ban-nondeterminism` rule, driven through the gate's
//! per-file entry point (`code_dojo::file_violations`) and dependency entry point
//! (`code_dojo::dependency_violations`).
//!
//! Oracle: each seeded snippet is written by hand from the rule's definition in
//! `docs/CODE_DOJO.md`; the expected rule ID and line are part of the snippet, not derived
//! from the implementation.

use std::path::PathBuf;

use crate::code_dojo::{dependency_violations, file_violations};

pub(super) const CORE_PATH: &str = "crates/bunny-num/src/lib.rs";

/// One seeded form of a rule: a label, the source, and the line that must be reported.
pub(super) type Form = (&'static str, &'static str, usize);

pub(super) fn reported(path: &str, source: &str) -> Vec<(usize, String)> {
    file_violations(PathBuf::from(path), source)
        .iter()
        .map(|violation| (violation.line, violation.rule.to_string()))
        .collect()
}

pub(super) fn reported_rules(path: &str, source: &str) -> Vec<String> {
    reported(path, source).into_iter().map(|(_, rule)| rule).collect()
}

/// Asserts that every seeded form is reported under `rule` at its stated line.
///
/// On failure the message lists every form that slipped through together with what the
/// gate did report for it, so a red run records the exact coverage gap.
pub(super) fn assert_rejects(rule: &str, forms: &[Form]) {
    let mut missed = Vec::new();
    for (label, source, line) in forms {
        let found = reported(CORE_PATH, source);
        if !found.iter().any(|(at, name)| at == line && name == rule) {
            missed.push(format!(
                "{label}: expected [{rule}] at line {line}, gate reported {found:?}"
            ));
        }
    }
    assert!(
        missed.is_empty(),
        "{} of {} forms slipped through:\n{}",
        missed.len(),
        forms.len(),
        missed.join("\n")
    );
}

#[test]
fn time_system_rejects_every_seeded_form() {
    assert_rejects(
        "time-system",
        &[
            ("qualified call", "pub fn f() { let _ = std::time::SystemTime::now(); }", 1),
            ("use", "use std::time::SystemTime;", 1),
            ("use group", "use std::time::{Duration, SystemTime};", 1),
            ("renamed import", "use std::time::SystemTime as WallClock;", 1),
            (
                "module alias",
                "use std::time as clock;\npub fn f() { let _ = clock::SystemTime::now(); }",
                2,
            ),
            ("epoch constant", "pub fn f() { let _ = std::time::UNIX_EPOCH; }", 1),
            ("type position", "pub fn f(_at: std::time::SystemTime) {}", 1),
            (
                "macro body",
                "pub fn f() -> String { format!(\"{:?}\", std::time::SystemTime::now()) }",
                1,
            ),
            ("glob import", "use std::time::*;", 1),
        ],
    );
}

#[test]
fn time_instant_rejects_every_seeded_form() {
    assert_rejects(
        "time-instant",
        &[
            ("qualified call", "pub fn f() { let _ = std::time::Instant::now(); }", 1),
            ("use", "use std::time::Instant;", 1),
            ("use group", "use std::time::{Instant};", 1),
            ("renamed import", "use std::time::Instant as Tick;", 1),
            (
                "renamed import call",
                "use std::time::Instant as Tick;\npub fn f() { let _ = Tick::now(); }",
                2,
            ),
            (
                "module alias",
                "use std::time as clock;\npub fn f() { let _ = clock::Instant::now(); }",
                2,
            ),
            ("type position", "pub fn f(_at: std::time::Instant) {}", 1),
            (
                "macro body",
                "pub fn f() -> String { format!(\"{:?}\", std::time::Instant::now()) }",
                1,
            ),
            ("glob import", "use std::time::*;", 1),
        ],
    );
}

#[test]
fn thread_sleep_rejects_every_seeded_form() {
    assert_rejects(
        "thread-sleep",
        &[
            ("qualified call", "pub fn f() { std::thread::sleep(core::time::Duration::ZERO); }", 1),
            (
                "module import",
                "use std::thread;\npub fn f() { thread::sleep(core::time::Duration::ZERO); }",
                2,
            ),
            ("renamed import", "use std::thread::sleep as nap;", 1),
            ("tokio sleep", "pub fn f() { let _ = tokio::time::sleep; }", 1),
            ("async-std sleep", "pub fn f() { let _ = async_std::time::sleep; }", 1),
            ("macro body", "pub fn f() { let _ = vec![std::thread::sleep]; }", 1),
            ("glob import", "use std::thread::*;", 1),
        ],
    );
}

#[test]
fn host_parallelism_rejects_every_seeded_form() {
    assert_rejects(
        "host-parallelism",
        &[
            ("qualified call", "pub fn f() { let _ = std::thread::available_parallelism(); }", 1),
            ("use", "use std::thread::available_parallelism;", 1),
            ("bare call", "pub fn f() { let _ = available_parallelism(); }", 1),
            (
                "module alias",
                "use std::thread as host;\npub fn f() { let _ = host::available_parallelism(); }",
                2,
            ),
            ("glob import", "use std::thread::*;", 1),
        ],
    );
}

#[test]
fn random_rejects_every_seeded_form() {
    assert_rejects(
        "random",
        &[
            ("rand call", "pub fn f() -> u8 { rand::random() }", 1),
            ("rand use", "use rand::Rng;", 1),
            ("renamed thread_rng", "use rand::{thread_rng as rng};", 1),
            ("bare thread_rng", "pub fn f() { let _ = thread_rng(); }", 1),
            ("getrandom", "pub fn f() { let _ = getrandom::getrandom; }", 1),
            ("fastrand", "pub fn f() -> u8 { fastrand::u8(..) }", 1),
            ("macro body", "pub fn f() { let _ = vec![fastrand::u8(..)]; }", 1),
        ],
    );
}

#[test]
fn host_callback_rejects_every_seeded_form() {
    assert_rejects(
        "host-callback",
        &[
            ("js function type", "pub fn f(_callback: js_sys::Function) {}", 1),
            ("closure use", "use wasm_bindgen::closure::Closure;", 1),
            ("closure type", "pub fn f(_callback: wasm_bindgen::closure::Closure<dyn Fn()>) {}", 1),
            ("web-sys", "pub fn f(_window: web_sys::Window) {}", 1),
        ],
    );
}

#[test]
fn network_rejects_every_seeded_form() {
    assert_rejects(
        "network",
        &[
            ("qualified call", "pub fn f() { let _ = std::net::TcpStream::connect(\"x\"); }", 1),
            ("use group", "use std::net::{UdpSocket};", 1),
            (
                "module alias",
                "use std::net as wire;\npub fn f() { let _ = wire::TcpListener::bind(\"x\"); }",
                2,
            ),
            ("reqwest", "pub fn f() { let _ = reqwest::get(\"x\"); }", 1),
            ("ureq", "pub fn f() { let _ = ureq::get(\"x\"); }", 1),
            (
                "macro body",
                "pub fn f() -> String { format!(\"{:?}\", std::net::Ipv4Addr::LOCALHOST) }",
                1,
            ),
        ],
    );
}

#[test]
fn unordered_container_rejects_every_seeded_form() {
    assert_rejects(
        "unordered-container",
        &[
            (
                "qualified constructor",
                "pub fn f() { let _ = std::collections::HashMap::<u8, u8>::new(); }",
                1,
            ),
            ("use group", "use std::collections::{BTreeMap, HashSet};", 1),
            ("renamed import", "use std::collections::HashMap as Map;", 1),
            (
                "renamed import call",
                "use std::collections::HashMap as Map;\n\
                 pub fn f() { let _ = Map::<u8, u8>::new(); }",
                2,
            ),
            ("hash_map module", "use std::collections::hash_map::Entry;", 1),
            ("random state", "pub fn f() { let _ = std::hash::RandomState::new(); }", 1),
            ("fx map", "pub fn f(_map: rustc_hash::FxHashMap<u8, u8>) {}", 1),
            ("hashbrown", "pub fn f(_set: hashbrown::HashSet<u8>) {}", 1),
            (
                "macro body",
                "pub fn f() { let _ = vec![std::collections::HashMap::<u8, u8>::new()]; }",
                1,
            ),
            ("glob import", "use std::collections::*;", 1),
        ],
    );
}

#[test]
fn hand_hashed_table_rejects_every_seeded_form() {
    assert_rejects(
        "hand-hashed-table",
        &[
            ("line comment", "// Open addressing table keyed by vertex.\npub struct Table;", 1),
            ("doc comment", "/// open-addressing probe sequence\npub fn probe() {}", 1),
        ],
    );
}

#[test]
fn json_serialization_rejects_every_seeded_form() {
    assert_rejects(
        "json-serialization",
        &[
            ("qualified call", "pub fn f() { let _ = serde_json::to_vec(&0); }", 1),
            ("use", "use serde_json::Value;", 1),
            (
                "crate alias",
                "use serde_json as json;\npub fn f() { let _ = json::to_string(&0); }",
                2,
            ),
            ("json macro", "pub fn f() { let _ = serde_json::json!({}); }", 1),
            ("macro body", "pub fn f() { let _ = vec![serde_json::Value::Null]; }", 1),
        ],
    );
}

#[test]
fn wasm_serde_rejects_every_seeded_form() {
    assert_rejects(
        "wasm-serde",
        &[
            ("qualified call", "pub fn f() { let _ = serde_wasm_bindgen::to_value(&0); }", 1),
            ("use", "use serde_wasm_bindgen::Serializer;", 1),
        ],
    );
}

#[test]
fn float_sentinels_are_rejected_as_float_boundary() {
    assert_rejects(
        "float-boundary",
        &[
            ("nan constant", "pub fn f() -> u32 { f32::NAN.to_bits() }", 1),
            ("infinity constant", "pub fn f() -> u64 { f64::INFINITY.to_bits() }", 1),
            (
                "legacy module constant",
                "pub fn f() -> u64 { core::f64::NEG_INFINITY.to_bits() }",
                1,
            ),
            ("consts module", "pub fn f() -> u64 { core::f64::consts::PI.to_bits() }", 1),
            ("macro body", "pub fn f() -> String { format!(\"{}\", f64::NAN) }", 1),
        ],
    );
}

#[test]
fn float_operations_are_rejected_as_float_boundary() {
    assert_rejects(
        "float-boundary",
        &[
            ("method on typed float", "pub fn f(x: f64) -> u64 { x.sqrt().to_bits() }", 1),
            (
                "associated function",
                "pub fn f() -> u64 { f64::powi(core::f64::consts::E, 2).to_bits() }",
                1,
            ),
            ("float literal in macro", "pub fn f() -> String { format!(\"{}\", 0.5) }", 1),
        ],
    );
}

#[test]
fn std_env_rejects_every_seeded_form() {
    assert_rejects(
        "std-env",
        &[
            ("var", "pub fn f() { let _ = std::env::var(\"X\"); }", 1),
            ("current_dir", "pub fn f() { let _ = std::env::current_dir(); }", 1),
            ("module import call", "use std::env;\npub fn f() { let _ = env::vars(); }", 2),
            ("use group", "use std::{env, fmt};", 1),
            ("renamed import", "use std::env::var as read_setting;", 1),
            ("macro body", "pub fn f() -> String { format!(\"{:?}\", std::env::args()) }", 1),
        ],
    );
}

#[test]
fn std_fs_rejects_every_seeded_form() {
    assert_rejects(
        "std-fs",
        &[
            ("qualified call", "pub fn f() { let _ = std::fs::read(\"x\"); }", 1),
            (
                "imported function",
                "use std::fs::read_to_string;\npub fn f() { let _ = read_to_string(\"x\"); }",
                1,
            ),
            (
                "renamed module in group",
                "use std::{fs as disk};\npub fn f() { let _ = disk::write(\"x\", b\"\"); }",
                2,
            ),
            ("file open", "pub fn f() { let _ = std::fs::File::open(\"x\"); }", 1),
            (
                "macro body",
                "pub fn f() -> String { format!(\"{:?}\", std::fs::metadata(\"x\")) }",
                1,
            ),
        ],
    );
}

#[test]
fn std_process_rejects_every_seeded_form() {
    assert_rejects(
        "std-process",
        &[
            ("qualified call", "pub fn f() -> u32 { std::process::id() }", 1),
            ("use", "use std::process::Command;", 1),
            ("use group", "use std::{process};", 1),
            ("module alias", "use std::process as host;\npub fn f() { host::abort(); }", 2),
            ("macro body", "pub fn f() -> String { format!(\"{}\", std::process::id()) }", 1),
        ],
    );
}

/// One offending line per rule, used for the waiver contract tests.
const WAIVABLE: &[(&str, &str)] = &[
    ("time-system", "pub fn f() { let _ = std::time::SystemTime::now(); }"),
    ("time-instant", "pub fn f() { let _ = std::time::Instant::now(); }"),
    ("thread-sleep", "pub fn f() { std::thread::sleep(core::time::Duration::ZERO); }"),
    ("host-parallelism", "pub fn f() { let _ = std::thread::available_parallelism(); }"),
    ("random", "pub fn f() -> u8 { rand::random() }"),
    ("host-callback", "pub fn f(_callback: js_sys::Function) {}"),
    ("network", "pub fn f() { let _ = std::net::TcpStream::connect(\"x\"); }"),
    ("unordered-container", "pub fn f() { let _ = std::collections::HashSet::<u8>::new(); }"),
    ("hand-hashed-table", "pub struct Table; // open addressing over vertex keys"),
    ("json-serialization", "pub fn f() { let _ = serde_json::to_vec(&0); }"),
    ("wasm-serde", "pub fn f() { let _ = serde_wasm_bindgen::to_value(&0); }"),
    ("float-boundary", "pub fn f() -> u32 { f32::NAN.to_bits() }"),
    ("std-env", "pub fn f() { let _ = std::env::var(\"X\"); }"),
    ("std-fs", "pub fn f() { let _ = std::fs::read(\"x\"); }"),
    ("std-process", "pub fn f() -> u32 { std::process::id() }"),
];

fn waived_source(waiver: &str, offending: &str) -> String {
    format!("{waiver}\n{offending}\n")
}

#[test]
fn waiver_with_reason_suppresses_each_rule() {
    let mut still_reported = Vec::new();
    for (rule, offending) in WAIVABLE {
        let unwaived = waived_source("// no waiver here", offending);
        assert!(
            reported_rules(CORE_PATH, &unwaived).iter().any(|found| found == rule),
            "{rule}: the unwaived witness must be reported first"
        );
        let waiver = format!("// dojo: allow {rule} -- reviewed boundary adapter, issue 279");
        if reported_rules(CORE_PATH, &waived_source(&waiver, offending))
            .iter()
            .any(|found| found == rule)
        {
            still_reported.push(*rule);
        }
    }
    assert!(still_reported.is_empty(), "reasoned waivers did not suppress: {still_reported:?}");
}

#[test]
fn waiver_without_reason_is_refused_for_each_rule() {
    let mut suppressed = Vec::new();
    for (rule, offending) in WAIVABLE {
        for waiver in [format!("// dojo: allow {rule}"), format!("// dojo: allow {rule} -- ok")] {
            let source = waived_source(&waiver, offending);
            if !reported_rules(CORE_PATH, &source).iter().any(|found| found == rule) {
                suppressed.push(format!("{rule} by `{waiver}`"));
            }
        }
    }
    assert!(suppressed.is_empty(), "reasonless waivers were honoured: {suppressed:?}");
}

#[test]
fn waiver_for_another_rule_does_not_suppress() {
    let source = waived_source(
        "// dojo: allow std-env -- reviewed boundary adapter, issue 279",
        "pub fn f() { let _ = std::fs::read(\"x\"); }",
    );

    assert!(reported_rules(CORE_PATH, &source).iter().any(|rule| rule == "std-fs"));
}

/// Nearby code that the rules must accept: deterministic look-alikes of each banned form.
const ACCEPTED: &[(&str, &str)] = &[
    ("time-system and time-instant", "use std::time::Duration;\npub fn f() -> Duration { Duration::ZERO }"),
    ("thread-sleep", "pub fn f() -> core::time::Duration { core::time::Duration::from_secs(1) }"),
    ("random", "pub fn f(seed: u64) -> u64 { seed.wrapping_mul(6_364_136_223_846_793_005) }"),
    ("host-callback", "use wasm_bindgen_test::wasm_bindgen_test;\n#[wasm_bindgen_test]\nfn runs_in_wasm() {}"),
    ("network", "pub fn f() -> core::net::Ipv4Addr { core::net::Ipv4Addr::LOCALHOST }"),
    (
        "unordered-container",
        "use std::collections::{BTreeMap, BTreeSet};\n\
         pub fn f() -> (BTreeMap<u8, u8>, BTreeSet<u8>) { (BTreeMap::new(), BTreeSet::new()) }",
    ),
    ("hand-hashed-table", "// Sorted vector addressed by binary search.\npub struct Table;"),
    (
        "default hasher",
        "use std::collections::hash_map::DefaultHasher;\n\
         pub fn f() -> DefaultHasher { DefaultHasher::new() }",
    ),
    ("json-serialization", "pub fn f(bytes: &[u8]) -> usize { bytes.len() }"),
    ("float-op on fixed point", "pub fn f(x: Fixed, a: Angle) -> (Option<Fixed>, Fixed) { (x.sqrt(), a.sin()) }"),
    ("std-env", "mod env { pub fn name() -> &'static str { \"bunny\" } }\npub fn f() -> &'static str { env::name() }"),
    ("compile-time env macro", "pub fn f() -> &'static str { env!(\"CARGO_PKG_VERSION\") }"),
    ("std-process", "mod process { pub fn run() {} }\npub fn f() { process::run(); }"),
];

/// Every rule ID that maps an Echo `ban-nondeterminism` rule.
pub(super) const FAMILY: &[&str] = &[
    "time-system",
    "time-instant",
    "thread-sleep",
    "host-parallelism",
    "random",
    "host-callback",
    "network",
    "unordered-container",
    "hand-hashed-table",
    "json-serialization",
    "wasm-serde",
    "float-boundary",
    "std-env",
    "std-fs",
    "std-process",
];

#[test]
fn deterministic_look_alikes_are_accepted() {
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

const STD_FS_CALL: &str = "pub fn f() { let _ = std::fs::read(\"x\"); }";

#[test]
fn nondeterminism_rules_cover_core_tests_codec_and_contract() {
    for path in [
        "crates/bunny-num/tests/determinism.rs",
        "crates/bunny-broadphase/src/bvh/sah/tests/split.rs",
        "crates/bunny-codec/src/lib.rs",
        "crates/bunny-contract/src/lib.rs",
    ] {
        assert!(
            reported_rules(path, STD_FS_CALL).iter().any(|rule| rule == "std-fs"),
            "{path} was not scanned"
        );
    }
}

#[test]
fn tooling_and_generator_crates_are_outside_nondeterminism_scope() {
    let timing = "use std::time::Instant;\npub fn f() { let _ = Instant::now(); let _ = std::fs::read(\"x\"); }";
    for path in
        ["xtask/src/perf.rs", "crates/bunny-perf/src/timing.rs", "crates/bunny-wesley/src/main.rs"]
    {
        let rules = reported_rules(path, timing);
        assert!(
            rules.iter().all(|rule| rule != "time-instant" && rule != "std-fs"),
            "{path}: {rules:?}"
        );
    }
}

#[test]
fn float_rules_stay_core_only() {
    let parser = "pub fn f() -> f32 { f32::NAN }";

    assert!(!reported_rules("crates/bunny-codec/src/obj/float.rs", parser)
        .iter()
        .any(|rule| rule == "float-boundary"));
}

#[test]
fn each_rule_is_reported_once_per_line() {
    let source = "use std::collections::{HashMap, HashSet};";

    let count = reported(CORE_PATH, source)
        .iter()
        .filter(|(_, rule)| rule == "unordered-container")
        .count();

    assert_eq!(count, 1);
}

fn no_manifest(_crate_name: &str) -> String {
    String::new()
}

fn dependency_findings(tree: &str, manifest: &dyn Fn(&str) -> String) -> Vec<(String, String)> {
    dependency_violations(tree, manifest)
        .iter()
        .map(|violation| (violation.rule.to_string(), violation.to_string()))
        .collect()
}

/// A `cargo tree --prefix depth --format {p} --no-dedupe -e normal` listing of two
/// deterministic crates where `bunny-geom` reaches `rand` only through `bunny-num`.
const TREE: &str = "\
0bunny-num v0.7.1 (/work/crates/bunny-num)
1rand v0.8.5
2getrandom v0.2.15
3cfg-if v1.0.0
0bunny-geom v0.7.1 (/work/crates/bunny-geom)
1bunny-num v0.7.1 (/work/crates/bunny-num)
2rand v0.8.5
3getrandom v0.2.15
1sha2 v0.10.9
2digest v0.10.7
1serde_json v1.0.140
";

#[test]
fn banned_dependencies_are_attributed_to_the_crate_that_declares_them() {
    let manifest = |name: &str| PathBuf::from("crates").join(name).join("Cargo.toml");

    let found = dependency_violations(TREE, &no_manifest)
        .iter()
        .map(|violation| {
            let named = ["rand", "getrandom", "serde_json"]
                .into_iter()
                .find(|crate_name| violation.to_string().contains(&format!("`{crate_name}`")));
            (violation.rule, violation.path.clone(), named)
        })
        .collect::<Vec<_>>();

    // Compared as paths, not rendered text, so the separator is the platform's own.
    assert_eq!(
        found,
        vec![
            ("random", manifest("bunny-num"), Some("rand")),
            ("random", manifest("bunny-num"), Some("getrandom")),
            ("json-serialization", manifest("bunny-geom"), Some("serde_json")),
        ]
    );
}

#[test]
fn every_banned_dependency_maps_to_its_rule() {
    let cases = [
        ("rand", "random"),
        ("getrandom", "random"),
        ("fastrand", "random"),
        ("js-sys", "host-callback"),
        ("wasm-bindgen", "host-callback"),
        ("web-sys", "host-callback"),
        ("reqwest", "network"),
        ("ureq", "network"),
        ("hashbrown", "unordered-container"),
        ("rustc-hash", "unordered-container"),
        ("serde_json", "json-serialization"),
        ("serde-wasm-bindgen", "wasm-serde"),
        ("tokio", "thread-sleep"),
        ("async-std", "thread-sleep"),
    ];
    for (dependency, rule) in cases {
        let tree = format!("0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n1{dependency} v1.0.0\n");
        let rules = dependency_findings(&tree, &no_manifest)
            .into_iter()
            .map(|(found, _)| found)
            .collect::<Vec<_>>();
        assert_eq!(rules, vec![rule.to_string()], "{dependency}");
    }
}

#[test]
fn permitted_dependencies_are_accepted() {
    let tree = "0bunny-mesh v0.7.1 (/work/crates/bunny-mesh)\n\
                1sha2 v0.10.9\n2digest v0.10.7\n2cpufeatures v0.2.17 (*)\n";

    assert!(dependency_findings(tree, &no_manifest).is_empty());
}

#[test]
fn manifest_waiver_with_reason_suppresses_a_dependency_rule() {
    let manifest = |crate_name: &str| {
        if crate_name == "bunny-num" {
            "# dojo: allow random -- seeded generator behind a reviewed feature\n".to_string()
        } else {
            String::new()
        }
    };

    let rules =
        dependency_findings(TREE, &manifest).into_iter().map(|(rule, _)| rule).collect::<Vec<_>>();

    assert_eq!(rules, vec!["json-serialization".to_string()]);
}

#[test]
fn manifest_waiver_without_reason_is_refused() {
    let manifest = |_crate_name: &str| {
        "# dojo: allow random\n# dojo: allow json-serialization -- ok\n".to_string()
    };

    assert_eq!(dependency_findings(TREE, &manifest).len(), 3);
}
