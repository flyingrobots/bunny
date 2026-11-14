# Lint Policy Test Plan

This plan defines how Bunny verifies Rule 10 of `CODE_STANDARDS.md` for the
core crates. The current chapter is [`README.md`](README.md).

## Scope

It covers the Code Dojo check `blanket-nursery`
(`xtask/src/lint_policy.rs`) and the named nursery lints of the core crates.
It does not cover which Clippy lints other crates enable.

## Test Goals

- Prove that a blanket `clippy::nursery` level in any place that reaches a
  core crate fails the gate, naming the file and line.
- Prove that named nursery lints, `allow`, and text that only mentions the
  group pass.
- Prove that the repository passes, and that the named lints still catch the
  patterns they were chosen for.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| LINT-REQ-001 | No core crate sets `clippy::nursery` as a group to `deny`, `warn` or `forbid`: not in an inner attribute of its `src/lib.rs` (inside `cfg_attr` included), not in its manifest's `[lints.clippy]` table and not in the workspace's `[workspace.lints.clippy]` table. Code Dojo's full gate and pre-commit gate fail on one with rule `blanket-nursery`, naming the file and line. | `README.md#the-check` |
| LINT-REQ-002 | Each core crate denies the nursery lints it relies on by name, each with its reason in a comment, and builds warning-free with them. | `README.md#named-nursery-lints` |

```toml
[[requirement]]
id = "LINT-REQ-001"
summary = "No core crate sets clippy::nursery as a group to deny, warn or forbid: not in an inner attribute of its src/lib.rs (inside cfg_attr included), not in its manifest's [lints.clippy] table and not in the workspace's [workspace.lints.clippy] table. Code Dojo's full gate and pre-commit gate fail on one with rule blanket-nursery, naming the file and line."
status = "active"

[[requirement]]
id = "LINT-REQ-002"
summary = "Each core crate denies the nursery lints it relies on by name, each with its reason in a comment, and builds warning-free with them."
status = "active"
```

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| LINT-TP-001 | Seeded attributes | LINT-REQ-001 | Constructed `lib.rs` sources: `deny`, `warn` and `forbid` of the group, the group among other lints, and inside `cfg_attr`, are each one finding at the stated line; named nursery lints, `allow(clippy::nursery)`, an attribute on an item and the group named in a comment or string are none. | `xtask/src/lint_policy/tests.rs::lint_tp_001_a_blanket_nursery_attribute_is_found_at_its_line` |
| LINT-TP-002 | Seeded manifests | LINT-REQ-001 | Constructed manifests: `nursery = "deny"` and `nursery = { level = "warn", priority = -1 }` in `[workspace.lints.clippy]` or `[lints.clippy]`, and `clippy.nursery = "forbid"` under `[lints]`, are each one finding at the stated line; `allow`, a named lint, a comment and another table's `nursery` key are none. | `xtask/src/lint_policy/tests.rs::lint_tp_002_a_blanket_nursery_manifest_level_is_found_at_its_line` |
| LINT-TP-003 | Repository | LINT-REQ-001, LINT-REQ-002 | The check over this repository's core crates finds nothing; on main before this change it found the six `#![deny(clippy::nursery)]` lines. | `xtask/src/lint_policy/tests.rs::lint_tp_003_the_core_crates_name_their_nursery_lints` |
| LINT-TP-004 | Named lints still bite | LINT-REQ-002 | In a container copy, one seeded regression per named lint (a `const fn` made non-`const`, a crate-private item in a private module made `pub(crate)`, a `Self` written as the type's name) fails Clippy with that lint. | [Seeded lint checks](#seeded-lint-checks) |

```toml
[[case]]
id = "LINT-TP-001"
requirements = ["LINT-REQ-001"]
evidence = "test"
test = "xtask/src/lint_policy/tests.rs::lint_tp_001_a_blanket_nursery_attribute_is_found_at_its_line"
oracle = "Constructed sources with known findings and their lines."
tier = "fast"
status = "implemented"

[[case]]
id = "LINT-TP-002"
requirements = ["LINT-REQ-001"]
evidence = "test"
test = "xtask/src/lint_policy/tests.rs::lint_tp_002_a_blanket_nursery_manifest_level_is_found_at_its_line"
oracle = "Constructed manifests with known findings and their lines."
tier = "fast"
status = "implemented"

[[case]]
id = "LINT-TP-003"
requirements = ["LINT-REQ-001", "LINT-REQ-002"]
evidence = "test"
test = "xtask/src/lint_policy/tests.rs::lint_tp_003_the_core_crates_name_their_nursery_lints"
oracle = "The repository's own files: no finding; six before the change."
tier = "fast"
status = "implemented"

[[case]]
id = "LINT-TP-004"
requirements = ["LINT-REQ-002"]
evidence = "manual-audit"
artifact = "docs/topics/lint-policy/test-plan.md#seeded-lint-checks"
oracle = "Clippy names the seeded lint in each crate copy."
tier = "manual"
status = "implemented"
```

## Seeded Lint Checks

In a container copy of each core crate, one seed at a time was appended to its
`src/lib.rs`, the file touched, and `cargo clippy --locked -q -p <crate> --lib`
run; the seed counts as caught when Clippy's output names the lint.

| Seed | Lint | Caught in |
| --- | --- | --- |
| `pub fn seeded_const_candidate() -> u8 { 1 }` | `missing_const_for_fn` | all six crates |
| `mod seeded_private { pub(crate) const SEEDED: u8 = 1; }` and a public use of it | `redundant_pub_crate` | all six crates |
| `impl SeededType { pub const fn new() -> SeededType { SeededType(1) } .. }` | `use_self` | all six crates |

The gate check itself was red before the crates changed: LINT-TP-003 named
line 4 of the `src/lib.rs` of `bunny-num`, `bunny-linalg`, `bunny-geom`,
`bunny-query`, `bunny-broadphase` and `bunny-mesh`.
