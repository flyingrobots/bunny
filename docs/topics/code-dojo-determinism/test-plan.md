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
| ND-REQ-002 | A waiver is a comment (never literal, doc or macro text) holding exactly one `dojo: allow <rule> -- <reason>` clause with a reason of at least eight characters; it covers its own line when it follows code, and otherwise the comment lines below it and the whole outermost item, statement, field, variant, arm or struct-literal field starting on the next code line; a refused waiver does not hide a valid one, and a manifest waiver is a whole-line TOML comment. | `README.md#waivers` |
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
summary = "A waiver is a comment (never literal, doc or macro text) holding exactly one `dojo: allow <rule> -- <reason>` clause with a reason of at least eight characters; it covers its own line when it follows code, and otherwise the comment lines below it and the whole outermost item, statement, field, variant, arm or struct-literal field starting on the next code line; a refused waiver does not hide a valid one, and a manifest waiver is a whole-line TOML comment."
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
| ND-TP-030 | extern crate aliases | ND-REQ-001 | `extern crate std as host;` makes `host::env::var`, `use host::fs::read` and `host::time::Instant` findings under `std-env`, `std-fs` and `time-instant`; `extern crate rand;` and a renamed `rand` are `random` findings. | `xtask/src/nondeterminism/evasion_tests.rs::extern_crate_aliases_resolve_like_imports` |
| ND-TP-031 | raw identifiers | ND-REQ-001 | `r#std::fs::read`, `std::r#env::var`, `use std::collections::r#HashMap`, a raw `HashSet` in a macro body, a raw spelling of an alias, `x.r#unwrap()` and `r#panic!` are reported under their rules at the stated line. | `xtask/src/nondeterminism/evasion_tests.rs::raw_identifiers_match_their_plain_names` |
| ND-TP-032 | raw float types | ND-REQ-001 | `r#f64` as a type and as a path segment is reported as `float-boundary`. | `xtask/src/nondeterminism/evasion_tests.rs::raw_float_types_are_float_boundary` |
| ND-TP-033 | self in a use group | ND-REQ-001 | `use std::{self as s};` then `s::fs::read` is `std-fs` at the call line, and `use std::{self as host, fmt};` then `host::env::var` is `std-env`; treating `self` as a path segment fails this case. | `xtask/src/nondeterminism/evasion_tests.rs::self_in_a_use_group_names_the_module_itself` |
| ND-TP-034 | evasion look-alikes | ND-REQ-003 | An identifier `r#type`, `HashMap` in a comment and a string, and `core` and `alloc` renamed by `extern crate` produce no finding in the rule family. | `xtask/src/nondeterminism/evasion_tests.rs::look_alikes_of_the_evasions_are_accepted` |
| ND-TP-035 | waiver outside a comment | ND-REQ-002 | Waiver text in a string, raw string, byte string, a line of a multi-line raw string, an outer, inner or block doc comment, a doc attribute or macro tokens leaves the `std-fs` finding reported. | `xtask/src/waivers/tests.rs::waiver_text_outside_a_comment_waives_nothing` |
| ND-TP-036 | comment waivers | ND-REQ-002 | A line comment or block comment above the finding, or a line comment on its line, suppresses it. | `xtask/src/waivers/tests.rs::line_and_block_comments_waive` |
| ND-TP-037 | one rule per waiver | ND-REQ-002 | A comment with a `std-fs` and a `std-env` clause leaves both findings reported. | `xtask/src/waivers/tests.rs::a_comment_with_two_clauses_waives_nothing` |
| ND-TP-038 | refused then valid waiver | ND-REQ-002 | A short-reason waiver followed by a valid one for the same rule suppresses the finding. | `xtask/src/waivers/tests.rs::a_valid_waiver_after_a_refused_one_applies` |
| ND-TP-039 | next statement only | ND-REQ-002 | A waiver above the first of two one-line statements leaves exactly the second one's finding (line 4) reported. | `xtask/src/waivers/tests.rs::a_waiver_covers_only_the_next_statement` |
| ND-TP-040 | multi-line statement | ND-REQ-002 | A finding on the fourth line of a waived statement is suppressed; the next statement's finding (line 8) is reported. | `xtask/src/waivers/tests.rs::a_waiver_covers_every_line_of_a_multi_line_statement` |
| ND-TP-041 | float waiver scope | ND-REQ-002 | A `float-boundary` waiver above a function covers a float use five lines into it; the same use in the next function (line 8) is reported. | `xtask/src/waivers/tests.rs::a_float_waiver_covers_its_item_and_nothing_after_it` |
| ND-TP-042 | comment lines below a waiver | ND-REQ-002 | A `hand-hashed-table` finding on a comment line between the waiver and the item is suppressed. | `xtask/src/waivers/tests.rs::a_waiver_covers_the_comment_lines_below_it` |
| ND-TP-043 | stacked waivers | ND-REQ-002 | Two waiver comments for two rules above one item suppress both findings. | `xtask/src/waivers/tests.rs::stacked_waivers_each_cover_the_item` |
| ND-TP-044 | exact rule name | ND-REQ-002 | A waiver for `std-fs-extra` leaves a `std-fs` finding reported. | `xtask/src/waivers/tests.rs::a_waiver_names_its_rule_exactly` |
| ND-TP-066 | block comment over lines | ND-REQ-002 | A `/* */` comment whose two lines hold a `std-fs` and a `std-env` clause waives neither: both findings (lines 4 and 5) are reported. | `xtask/src/waivers/tests.rs::a_block_comment_over_two_lines_with_two_clauses_waives_nothing` |
| ND-TP-067 | block comment with one clause | ND-REQ-002 | A `/* */` comment over two lines with one clause, its reason running onto the second line, waives the item below it. | `xtask/src/waivers/tests.rs::a_block_comment_over_two_lines_with_one_clause_waives_the_item_below` |
| ND-TP-068 | manifest multi-line strings | ND-REQ-002, ND-REQ-005 | A waiver line inside a multi-line basic or literal TOML string waives nothing; a whole-line comment after a multi-line string closed on its own line waives. | `xtask/src/waivers/tests.rs::manifest_waiver_text_inside_a_multi_line_string_is_not_a_comment` |
| ND-TP-045 | manifest waiver form | ND-REQ-002, ND-REQ-005 | A whole-line manifest comment waives `random`; the same text inside a TOML string, or a comment with two clauses, does not. | `xtask/src/waivers/tests.rs::manifest_waivers_are_whole_line_comments` |
| ND-TP-046 | trailing waiver | ND-REQ-002 | A waiver after code on a line covers that line only; the next statement's finding (line 3) is reported. | `xtask/src/waivers/tests.rs::a_waiver_after_code_covers_only_its_line` |

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

[[case]]
id = "ND-TP-030"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/evasion_tests.rs::extern_crate_aliases_resolve_like_imports"
oracle = "`extern crate std as host;` makes `host::env::var`, `use host::fs::read` and `host::time::Instant` findings under `std-env`, `std-fs` and `time-instant`; `extern crate rand;` and a renamed `rand` are `random` findings."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-031"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/evasion_tests.rs::raw_identifiers_match_their_plain_names"
oracle = "`r#std::fs::read`, `std::r#env::var`, `use std::collections::r#HashMap`, a raw `HashSet` in a macro body, a raw spelling of an alias, `x.r#unwrap()` and `r#panic!` are reported under their rules at the stated line."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-032"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/evasion_tests.rs::raw_float_types_are_float_boundary"
oracle = "`r#f64` as a type and as a path segment is reported as `float-boundary`."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-033"
requirements = ["ND-REQ-001"]
evidence = "test"
test = "xtask/src/nondeterminism/evasion_tests.rs::self_in_a_use_group_names_the_module_itself"
oracle = "`use std::{self as s};` then `s::fs::read` is `std-fs` at the call line, and `use std::{self as host, fmt};` then `host::env::var` is `std-env`; treating `self` as a path segment fails this case."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-034"
requirements = ["ND-REQ-003"]
evidence = "test"
test = "xtask/src/nondeterminism/evasion_tests.rs::look_alikes_of_the_evasions_are_accepted"
oracle = "An identifier `r#type`, `HashMap` in a comment and a string, and `core` and `alloc` renamed by `extern crate` produce no finding in the rule family."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-035"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::waiver_text_outside_a_comment_waives_nothing"
oracle = "Waiver text in a string, raw string, byte string, a line of a multi-line raw string, an outer, inner or block doc comment, a doc attribute or macro tokens leaves the `std-fs` finding reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-036"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::line_and_block_comments_waive"
oracle = "A line comment or block comment above the finding, or a line comment on its line, suppresses it."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-037"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_comment_with_two_clauses_waives_nothing"
oracle = "A comment with a `std-fs` and a `std-env` clause leaves both findings reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-038"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_valid_waiver_after_a_refused_one_applies"
oracle = "A short-reason waiver followed by a valid one for the same rule suppresses the finding."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-039"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_waiver_covers_only_the_next_statement"
oracle = "A waiver above the first of two one-line statements leaves exactly the second one's finding (line 4) reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-040"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_waiver_covers_every_line_of_a_multi_line_statement"
oracle = "A finding on the fourth line of a waived statement is suppressed; the next statement's finding (line 8) is reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-041"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_float_waiver_covers_its_item_and_nothing_after_it"
oracle = "A `float-boundary` waiver above a function covers a float use five lines into it; the same use in the next function (line 8) is reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-042"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_waiver_covers_the_comment_lines_below_it"
oracle = "A `hand-hashed-table` finding on a comment line between the waiver and the item is suppressed."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-043"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::stacked_waivers_each_cover_the_item"
oracle = "Two waiver comments for two rules above one item suppress both findings."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-044"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_waiver_names_its_rule_exactly"
oracle = "A waiver for `std-fs-extra` leaves a `std-fs` finding reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-066"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_block_comment_over_two_lines_with_two_clauses_waives_nothing"
oracle = "A `/* */` comment whose two lines hold a `std-fs` and a `std-env` clause waives neither: both findings (lines 4 and 5) are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-067"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_block_comment_over_two_lines_with_one_clause_waives_the_item_below"
oracle = "A `/* */` comment over two lines with one clause, its reason running onto the second line, waives the item below it."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-068"
requirements = ["ND-REQ-002", "ND-REQ-005"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::manifest_waiver_text_inside_a_multi_line_string_is_not_a_comment"
oracle = "A waiver line inside a multi-line basic or literal TOML string waives nothing; a whole-line comment after a multi-line string closed on its own line waives."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-045"
requirements = ["ND-REQ-002", "ND-REQ-005"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::manifest_waivers_are_whole_line_comments"
oracle = "A whole-line manifest comment waives `random`; the same text inside a TOML string, or a comment with two clauses, does not."
tier = "fast"
status = "implemented"

[[case]]
id = "ND-TP-046"
requirements = ["ND-REQ-002"]
evidence = "test"
test = "xtask/src/waivers/tests.rs::a_waiver_after_code_covers_only_its_line"
oracle = "A waiver after code on a line covers that line only; the next statement's finding (line 3) is reported."
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
`extern crate` renames, raw identifiers (`r#std`, `r#HashMap`, `r#f64`),
module aliases, glob imports, leading `::`, turbofish generics, type
positions, macro invocation paths and macro bodies, comments, waivers with
short reasons and waivers for other rules, waiver text in literals, doc
comments and macro tokens, two clauses in one comment, a banned crate reached through
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
| `float-op` is enforced through float types and literals, not method names, because Bunny's fixed-point types have `sqrt` and `sin` methods. A float admitted by a waived item can leave it (as a return value) and be operated on elsewhere where its type is only inferred. | Review of each `float-boundary` waiver; a type-aware check would close it. |

