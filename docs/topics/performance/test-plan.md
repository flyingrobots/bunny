# Performance Test Plan

This document defines how Bunny proves its performance tooling and the fidelity rule
described in the [performance chapter](README.md).

## Scope

This plan covers:

- parsing, aggregation and comparison of `bunny-perf` output in `cargo xtask perf`
- the regression threshold used by CI
- building the harness against a revision that lacks newer feature groups
- the fidelity rule that every optimization must satisfy

Timing numbers themselves are measurements, not test oracles: no test asserts that a
kernel runs in a given number of nanoseconds.

## Test Goals

- Prove the report takes medians across rounds and computes change and speedup correctly.
- Prove the regression gate fails exactly when a case slows past the threshold.
- Prove cases present only in the candidate are reported as new, not as regressions.
- Prove the generated harness manifest points at the requested revision's crates.

## Non-Goals

- Asserting absolute timings or speedups.
- Testing GitHub Actions itself; the workflow is exercised by running it.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| PERF-REQ-001 | An optimization either preserves every output bit or ships as a new API with a stated error bound. | `README.md#the-fidelity-rule` |
| PERF-REQ-002 | Comparisons report the median of each percentile and the best p50 across interleaved rounds. | `README.md#measuring` |
| PERF-REQ-003 | The regression gate fails only when a case's best p50 slows past the threshold. | `README.md#continuous-integration` |
| PERF-REQ-004 | Cases the base revision cannot build are reported as new. | `README.md#measuring` |
| PERF-REQ-005 | The harness is built from the same source against each revision's own crates. | `README.md#measuring` |

```toml
[[requirement]]
id = "PERF-REQ-001"
summary = "An optimization either preserves every output bit or ships as a new API with a stated error bound."
status = "reserved"

[[requirement]]
id = "PERF-REQ-002"
summary = "Comparisons report the median of each percentile and the best p50 across interleaved rounds."
status = "active"

[[requirement]]
id = "PERF-REQ-003"
summary = "The regression gate fails only when a case's best p50 slows past the threshold."
status = "active"

[[requirement]]
id = "PERF-REQ-004"
summary = "Cases the base revision cannot build are reported as new."
status = "active"

[[requirement]]
id = "PERF-REQ-005"
summary = "The harness is built from the same source against each revision's own crates."
status = "active"
```

PERF-REQ-001 is reserved until the first optimization lands; each optimization adds its
equivalence or error-bound case here.

## Fixtures

No external fixtures. Tests build synthetic `bunny-perf` output and manifests in memory.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| PERF-TP-001 | Parsing | PERF-REQ-002 | Exact parsed timings; malformed lines are errors. | `xtask/src/perf_report.rs::tests::runs_parse_tab_separated_lines` |
| PERF-TP-002 | Aggregation | PERF-REQ-002 | Median of odd and even round counts. | `xtask/src/perf_report.rs::tests::median_damps_one_noisy_round` |
| PERF-TP-003 | Comparison | PERF-REQ-002, PERF-REQ-004 | Change, speedup and `new` rows in the rendered table. | `xtask/src/perf_report.rs::tests::comparison_reports_change_and_new_cases` |
| PERF-TP-004 | Gate | PERF-REQ-003 | Only cases past the threshold are reported. | `xtask/src/perf_report.rs::tests::regressions_respect_the_threshold` |
| PERF-TP-005 | Build | PERF-REQ-004 | Feature groups exclude `default` and comments. | `xtask/src/perf.rs::tests::feature_groups_skip_default_and_comments` |
| PERF-TP-007 | Gate | PERF-REQ-003 | A bimodal case whose medians land in different modes does not fail the gate. | `xtask/src/perf_report.rs::tests::the_gate_ignores_a_bimodal_case_whose_median_lands_in_different_modes` |
| PERF-TP-020 | Build | PERF-REQ-005 | Harness binary names follow the target (`.wasm`, `.exe`, none). | `xtask/src/perf.rs::tests::binary_names_follow_the_target` |
| PERF-TP-006 | Build | PERF-REQ-005 | The generated manifest is standalone and points at the given crates. | `xtask/src/perf.rs::tests::harness_manifest_points_at_the_given_crates` |

```toml
[[case]]
id = "PERF-TP-001"
requirements = ["PERF-REQ-002"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::runs_parse_tab_separated_lines"
oracle = "Exact parsed timings; malformed lines are errors."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-002"
requirements = ["PERF-REQ-002"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::median_damps_one_noisy_round"
oracle = "Median of odd and even round counts."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-003"
requirements = ["PERF-REQ-002", "PERF-REQ-004"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::comparison_reports_change_and_new_cases"
oracle = "Change, speedup and new rows in the rendered table."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-004"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::regressions_respect_the_threshold"
oracle = "Only cases past the threshold are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-005"
requirements = ["PERF-REQ-004"]
evidence = "test"
test = "xtask/src/perf.rs::tests::feature_groups_skip_default_and_comments"
oracle = "Feature groups exclude default and comments."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-006"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::harness_manifest_points_at_the_given_crates"
oracle = "The generated manifest is standalone and points at the given crates."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-007"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::the_gate_ignores_a_bimodal_case_whose_median_lands_in_different_modes"
oracle = "A bimodal case whose medians land in different modes does not fail the gate."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-020"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::binary_names_follow_the_target"
oracle = "Harness binary names follow the target (.wasm, .exe, none)."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The report logic is pure: it parses text, sorts with `f64::total_cmp` and renders
Markdown, with no clock, randomness or filesystem access in the tested functions. Timing
runs are deliberately outside the test oracle.

## Gaps

- The interleaving, worktree handling, cross-target runs and CI workflow are exercised by
  running `cargo xtask perf`, not by automated tests. The platform matrix has not run on
  GitHub yet.
