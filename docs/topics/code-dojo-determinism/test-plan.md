# Code Dojo Determinism Rules Test Plan

This document defines how Bunny verifies the Code Dojo rules that map Echo's
`ban-nondeterminism` scan.

The current topic chapter is [`README.md`](README.md). The rule-by-rule mapping
table lives in [`docs/CODE_DOJO.md`](../../CODE_DOJO.md#nondeterminism-rules).

## Scope

This plan covers the source rules in `xtask/src/nondeterminism.rs`, their waiver
contract, their crate scope, the `float-boundary` rule's expression-path and
macro coverage, and the dependency-level rule run by the full gate.

It does not cover type inference. The source rules read syntax and `use`
declarations; they cannot see a banned type that reaches a core crate only
through the inferred return type of another crate's function. The dependency
rule exists for that reason.

## Test Goals

- Prove that every Echo rule ID is enforced by a named Code Dojo rule.
- Prove each rule against every syntactic form a contributor could write,
  not only the one form a text pattern would match.
- Prove that waivers need a reason and are scoped to one rule.
- Prove that deterministic look-alikes are not rejected.
- Prove which crates are in scope and which are not.
- Prove that banned crates in the normal dependency graph are found and
  attributed to the crate that declares the path.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| ND-REQ-001 | Every rule of Echo's `ban-nondeterminism` scan maps to a Code Dojo rule that rejects its seeded forms in deterministic library crates: qualified paths, `use` items, `use` groups, renamed imports, module aliases, glob imports, type positions and macro bodies. | `README.md#rules` |
| ND-REQ-002 | Waivers are local and rule-scoped and must carry a reason of at least eight characters; a waiver without a reason, with a too-short reason or naming another rule does not suppress a finding. | `README.md#waivers` |
| ND-REQ-003 | Deterministic look-alikes of banned forms are accepted. | `README.md#rules` |
| ND-REQ-004 | The source rules cover the core crates, `bunny-codec` and `bunny-contract`, including their tests; `xtask`, `bunny-perf` and `bunny-wesley` are outside them; `float-boundary` stays core-only. | `README.md#scope` |
| ND-REQ-005 | The normal dependency graph of each deterministic library crate contains no banned crate; a finding names the deterministic crate that declares the path to it and can be waived only in that crate's manifest with a reason. | `README.md#dependency-rules` |

```toml
[[requirement]]
id = "ND-REQ-001"
summary = "Every rule of Echo's `ban-nondeterminism` scan maps to a Code Dojo rule that rejects its seeded forms in deterministic library crates: qualified paths, `use` items, `use` groups, renamed imports, module aliases, glob imports, type positions and macro bodies."
status = "active"

[[requirement]]
id = "ND-REQ-002"
summary = "Waivers are local and rule-scoped and must carry a reason of at least eight characters; a waiver without a reason, with a too-short reason or naming another rule does not suppress a finding."
status = "active"

[[requirement]]
id = "ND-REQ-003"
summary = "Deterministic look-alikes of banned forms are accepted."
status = "active"

[[requirement]]
id = "ND-REQ-004"
summary = "The source rules cover the core crates, `bunny-codec` and `bunny-contract`, including their tests; `xtask`, `bunny-perf` and `bunny-wesley` are outside them; `float-boundary` stays core-only."
status = "active"

[[requirement]]
id = "ND-REQ-005"
summary = "The normal dependency graph of each deterministic library crate contains no banned crate; a finding names the deterministic crate that declares the path to it and can be waived only in that crate's manifest with a reason."
status = "active"
```

## Fixtures

Every source case is a hand-written Rust snippet passed to
`code_dojo::file_violations`, the same per-file entry point the gate uses for
each tracked file, at a repository-relative path that selects the crate scope.
Each rejecting form states the rule ID and the line that must be reported; the
failure message lists every form that slipped through together with what the
gate did report, so a red run records the exact gap.

Dependency cases pass a hand-written `cargo tree --prefix depth --format {p}
--no-dedupe -e normal` listing and a manifest lookup to
`code_dojo::dependency_violations`, the function the full gate calls with the
real `cargo tree` output and the real manifests.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| ND-TP-001 | time-system forms | ND-REQ-001 | Nine seeded forms of wall-clock access are each reported as `time-system` at the stated line. | `xtask/src/nondeterminism/tests.rs::time_system_rejects_every_seeded_form` |
| ND-TP-002 | time-instant forms | ND-REQ-001 | Nine seeded forms of monotonic clock access are each reported as `time-instant`. | `xtask/src/nondeterminism/tests.rs::time_instant_rejects_every_seeded_form` |
| ND-TP-003 | thread-sleep forms | ND-REQ-001 | Seven seeded sleep forms (std, tokio, async-std, alias, macro, glob) are each reported as `thread-sleep`. | `xtask/src/nondeterminism/tests.rs::thread_sleep_rejects_every_seeded_form` |
| ND-TP-004 | host-parallelism forms | ND-REQ-001 | Five seeded forms of `available_parallelism` are each reported as `host-parallelism`. | `xtask/src/nondeterminism/tests.rs::host_parallelism_rejects_every_seeded_form` |
| ND-TP-005 | random forms | ND-REQ-001 | Seven seeded forms naming `rand`, `getrandom`, `fastrand` or `thread_rng` are each reported as `random`. | `xtask/src/nondeterminism/tests.rs::random_rejects_every_seeded_form` |
| ND-TP-006 | host-callback forms | ND-REQ-001 | Seeded `js_sys`, `wasm_bindgen` closure and `web_sys` paths are each reported as `host-callback`. | `xtask/src/nondeterminism/tests.rs::host_callback_rejects_every_seeded_form` |
| ND-TP-007 | network forms | ND-REQ-001 | Seeded `std::net`, `reqwest` and `ureq` forms are each reported as `network`. | `xtask/src/nondeterminism/tests.rs::network_rejects_every_seeded_form` |
| ND-TP-008 | unordered-container forms | ND-REQ-001 | Ten seeded hash-container forms (std, `hash_map` module, `RandomState`, `FxHashMap`, `hashbrown`, alias, macro, glob) are each reported as `unordered-container`. | `xtask/src/nondeterminism/tests.rs::unordered_container_rejects_every_seeded_form` |
| ND-TP-009 | hand-hashed-table forms | ND-REQ-001 | Comments and doc comments naming open addressing are reported as `hand-hashed-table`. | `xtask/src/nondeterminism/tests.rs::hand_hashed_table_rejects_every_seeded_form` |
| ND-TP-010 | json-serialization forms | ND-REQ-001 | Seeded `serde_json` paths, aliases and macros are each reported as `json-serialization`. | `xtask/src/nondeterminism/tests.rs::json_serialization_rejects_every_seeded_form` |
| ND-TP-011 | wasm-serde forms | ND-REQ-001 | Seeded `serde_wasm_bindgen` paths are each reported as `wasm-serde`. | `xtask/src/nondeterminism/tests.rs::wasm_serde_rejects_every_seeded_form` |
| ND-TP-012 | float-sentinel forms | ND-REQ-001 | `NAN`, `INFINITY`, `NEG_INFINITY` and float `consts` paths, also inside a macro, are reported as `float-boundary`. | `xtask/src/nondeterminism/tests.rs::float_sentinels_are_rejected_as_float_boundary` |
| ND-TP-013 | float-op forms | ND-REQ-001 | A float method on a typed float, a float associated function and a float literal inside a macro are reported as `float-boundary`. | `xtask/src/nondeterminism/tests.rs::float_operations_are_rejected_as_float_boundary` |
| ND-TP-014 | std-env forms | ND-REQ-001 | Six seeded environment forms are each reported as `std-env`. | `xtask/src/nondeterminism/tests.rs::std_env_rejects_every_seeded_form` |
| ND-TP-015 | std-fs forms | ND-REQ-001 | Five seeded filesystem forms, including an imported function called by its bare name, are each reported as `std-fs`. | `xtask/src/nondeterminism/tests.rs::std_fs_rejects_every_seeded_form` |
| ND-TP-016 | std-process forms | ND-REQ-001 | Five seeded process forms are each reported as `std-process`. | `xtask/src/nondeterminism/tests.rs::std_process_rejects_every_seeded_form` |
| ND-TP-017 | one finding per line | ND-REQ-001 | A `use` group naming two hash containers yields exactly one `unordered-container` finding for that line. | `xtask/src/nondeterminism/tests.rs::each_rule_is_reported_once_per_line` |
| ND-TP-018 | reasoned waiver | ND-REQ-002 | For every rule, a waiver with a reason on the line above suppresses the finding. | `xtask/src/nondeterminism/tests.rs::waiver_with_reason_suppresses_each_rule` |
| ND-TP-019 | reasonless waiver | ND-REQ-002 | For every rule, a waiver with no reason or the reason `ok` leaves the finding reported. | `xtask/src/nondeterminism/tests.rs::waiver_without_reason_is_refused_for_each_rule` |
| ND-TP-020 | rule-scoped waiver | ND-REQ-002 | A `std-env` waiver does not suppress a `std-fs` finding. | `xtask/src/nondeterminism/tests.rs::waiver_for_another_rule_does_not_suppress` |
| ND-TP-021 | look-alikes | ND-REQ-003 | Duration, BTree containers, `hash_map::DefaultHasher`, `core::net`, fixed-point `sqrt`/`sin`, `wasm_bindgen_test`, local `env`/`process` modules and `env!` produce no finding in the rule family. | `xtask/src/nondeterminism/tests.rs::deterministic_look_alikes_are_accepted` |
| ND-TP-022 | library scope | ND-REQ-004 | A `std::fs` call is reported in a core test file, a core `src/**/tests` file, `bunny-codec` and `bunny-contract`. | `xtask/src/nondeterminism/tests.rs::nondeterminism_rules_cover_core_tests_codec_and_contract` |
| ND-TP-023 | tooling scope | ND-REQ-004 | Clock and filesystem use in `xtask`, `bunny-perf` and `bunny-wesley` is not reported by the rule family. | `xtask/src/nondeterminism/tests.rs::tooling_and_generator_crates_are_outside_nondeterminism_scope` |
| ND-TP-024 | float scope | ND-REQ-004 | `f32::NAN` in `bunny-codec` is not reported as `float-boundary`. | `xtask/src/nondeterminism/tests.rs::float_rules_stay_core_only` |
| ND-TP-025 | dependency attribution | ND-REQ-005 | A two-root tree yields exactly three findings: `rand` and `getrandom` against `bunny-num`, `serde_json` against `bunny-geom`; `rand` reached through `bunny-num` is not charged to `bunny-geom`. | `xtask/src/nondeterminism/tests.rs::banned_dependencies_are_attributed_to_the_crate_that_declares_them` |
| ND-TP-026 | banned dependency table | ND-REQ-005 | Each of the fourteen banned crate names yields exactly one finding under its stated rule. | `xtask/src/nondeterminism/tests.rs::every_banned_dependency_maps_to_its_rule` |
| ND-TP-027 | permitted dependencies | ND-REQ-005 | `sha2`, `digest` and `cpufeatures` produce no finding. | `xtask/src/nondeterminism/tests.rs::permitted_dependencies_are_accepted` |
| ND-TP-028 | manifest waiver | ND-REQ-005 | A reasoned `random` waiver in the `bunny-num` manifest leaves only the `json-serialization` finding. | `xtask/src/nondeterminism/tests.rs::manifest_waiver_with_reason_suppresses_a_dependency_rule` |
| ND-TP-029 | reasonless manifest waiver | ND-REQ-005 | Manifest waivers without a reason or with the reason `ok` leave all three findings reported. | `xtask/src/nondeterminism/tests.rs::manifest_waiver_without_reason_is_refused` |

```toml
[[case]]
id = "ND-TP-001"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::time_system_rejects_every_seeded_form"
oracle = "Nine seeded forms of wall-clock access are each reported as `time-system` at the stated line."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-002"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::time_instant_rejects_every_seeded_form"
oracle = "Nine seeded forms of monotonic clock access are each reported as `time-instant`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-003"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::thread_sleep_rejects_every_seeded_form"
oracle = "Seven seeded sleep forms (std, tokio, async-std, alias, macro, glob) are each reported as `thread-sleep`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-004"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::host_parallelism_rejects_every_seeded_form"
oracle = "Five seeded forms of `available_parallelism` are each reported as `host-parallelism`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-005"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::random_rejects_every_seeded_form"
oracle = "Seven seeded forms naming `rand`, `getrandom`, `fastrand` or `thread_rng` are each reported as `random`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-006"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::host_callback_rejects_every_seeded_form"
oracle = "Seeded `js_sys`, `wasm_bindgen` closure and `web_sys` paths are each reported as `host-callback`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-007"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::network_rejects_every_seeded_form"
oracle = "Seeded `std::net`, `reqwest` and `ureq` forms are each reported as `network`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-008"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::unordered_container_rejects_every_seeded_form"
oracle = "Ten seeded hash-container forms (std, `hash_map` module, `RandomState`, `FxHashMap`, `hashbrown`, alias, macro, glob) are each reported as `unordered-container`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-009"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::hand_hashed_table_rejects_every_seeded_form"
oracle = "Comments and doc comments naming open addressing are reported as `hand-hashed-table`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-010"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::json_serialization_rejects_every_seeded_form"
oracle = "Seeded `serde_json` paths, aliases and macros are each reported as `json-serialization`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-011"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::wasm_serde_rejects_every_seeded_form"
oracle = "Seeded `serde_wasm_bindgen` paths are each reported as `wasm-serde`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-012"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::float_sentinels_are_rejected_as_float_boundary"
oracle = "`NAN`, `INFINITY`, `NEG_INFINITY` and float `consts` paths, also inside a macro, are reported as `float-boundary`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-013"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::float_operations_are_rejected_as_float_boundary"
oracle = "A float method on a typed float, a float associated function and a float literal inside a macro are reported as `float-boundary`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-014"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::std_env_rejects_every_seeded_form"
oracle = "Six seeded environment forms are each reported as `std-env`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-015"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::std_fs_rejects_every_seeded_form"
oracle = "Five seeded filesystem forms, including an imported function called by its bare name, are each reported as `std-fs`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-016"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::std_process_rejects_every_seeded_form"
oracle = "Five seeded process forms are each reported as `std-process`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-017"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::each_rule_is_reported_once_per_line"
oracle = "A `use` group naming two hash containers yields exactly one `unordered-container` finding for that line."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-018"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::waiver_with_reason_suppresses_each_rule"
oracle = "For every rule, a waiver with a reason on the line above suppresses the finding."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-019"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::waiver_without_reason_is_refused_for_each_rule"
oracle = "For every rule, a waiver with no reason or the reason `ok` leaves the finding reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-020"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::waiver_for_another_rule_does_not_suppress"
oracle = "A `std-env` waiver does not suppress a `std-fs` finding."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-021"
requirements = ["ND-REQ-003"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::deterministic_look_alikes_are_accepted"
oracle = "Duration, BTree containers, `hash_map::DefaultHasher`, `core::net`, fixed-point `sqrt`/`sin`, `wasm_bindgen_test`, local `env`/`process` modules and `env!` produce no finding in the rule family."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-022"
requirements = ["ND-REQ-004"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::nondeterminism_rules_cover_core_tests_codec_and_contract"
oracle = "A `std::fs` call is reported in a core test file, a core `src/**/tests` file, `bunny-codec` and `bunny-contract`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-023"
requirements = ["ND-REQ-004"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::tooling_and_generator_crates_are_outside_nondeterminism_scope"
oracle = "Clock and filesystem use in `xtask`, `bunny-perf` and `bunny-wesley` is not reported by the rule family."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-024"
requirements = ["ND-REQ-004"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::float_rules_stay_core_only"
oracle = "`f32::NAN` in `bunny-codec` is not reported as `float-boundary`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-025"
requirements = ["ND-REQ-005"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::banned_dependencies_are_attributed_to_the_crate_that_declares_them"
oracle = "A two-root tree yields exactly three findings: `rand` and `getrandom` against `bunny-num`, `serde_json` against `bunny-geom`; `rand` reached through `bunny-num` is not charged to `bunny-geom`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-026"
requirements = ["ND-REQ-005"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::every_banned_dependency_maps_to_its_rule"
oracle = "Each of the fourteen banned crate names yields exactly one finding under its stated rule."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-027"
requirements = ["ND-REQ-005"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::permitted_dependencies_are_accepted"
oracle = "`sha2`, `digest` and `cpufeatures` produce no finding."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-028"
requirements = ["ND-REQ-005"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::manifest_waiver_with_reason_suppresses_a_dependency_rule"
oracle = "A reasoned `random` waiver in the `bunny-num` manifest leaves only the `json-serialization` finding."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-029"
requirements = ["ND-REQ-005"]
evidence = "test"
test = "xtask/src/nondeterminism/tests.rs::manifest_waiver_without_reason_is_refused"
oracle = "Manifest waivers without a reason or with the reason `ok` leave all three findings reported."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The rules are pure functions of source text, `cargo tree` text and manifest
text. The tests pass those texts in directly: no filesystem, Git repository,
clock, network or subprocess is involved, and expected findings are compared
as rule IDs and line numbers, not as rendered messages.

## Known Failures

The current executable surface has no known failing cases.

## Edge Cases And Unusual Inputs

Covered: `use` groups, nested groups, `self` in a group, renamed imports,
module aliases, glob imports, leading `::`, turbofish generics, type
positions, macro invocation paths and macro bodies, comments, waivers with
short reasons and waivers for other rules, a banned crate reached through
another deterministic crate.

## Stress And Fuzz

No fuzz target exists. The rules walk `syn` syntax trees and token streams; a
generated-snippet property test (random nesting of `use` groups and aliases
around a banned leaf) would be the next instrument if a form slips through.

## Open Gaps

| Gap | Blocking Work |
| --- | --- |
| Alias resolution is file-wide, not per module scope: an import in one module resolves a same-named path in a sibling module. This can only over-report. | Scope-aware name resolution; not needed while no false positive is reported on `main`. |
| A banned type returned by another crate's function and never named in the file is invisible to the source rules. | Covered by the dependency rule for third-party crates; a type-aware check would need `rustc` (for example a lint driver). |
| `float-op` is enforced through float types and literals, not method names, because Bunny's fixed-point types have `sqrt` and `sin` methods. A float binding admitted by a `float-boundary` waiver can call float methods further than two lines below that waiver. | Review of each `float-boundary` waiver; a type-aware check would close it. |

