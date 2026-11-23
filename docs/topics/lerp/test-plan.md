# Exact Lerp and Ratio Test Plan

This document defines how Bunny proves [`bunny-lerp/1`](README.md): `FixedQ32_32::lerp`,
`FixedQ32_32::from_ratio`, `FixedVec3::lerp` and `lerp_channels`. It answers
flyingrobots/bunny#303.

## Scope

- `FixedQ32_32::lerp` and `FixedQ32_32::from_ratio` in `bunny-num`
- `FixedVec3::lerp` and `lerp_channels` in `bunny-linalg`

Out of scope: `inverse_lerp`, `clamp` and `remap` (flyingrobots/bunny#111).

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| LERP-REQ-001 | `lerp(a, b, t)` is `a + round_half_even((b - a) * t / 2^32)` on raw values, computed exactly, for every `t` (extrapolating outside `[0, 1]`), and `None` only when that result is outside Q32.32. | `README.md#lerp` |
| LERP-REQ-002 | `lerp` returns exactly `a` at `t = 0` and exactly `b` at `t = 1.0` for every `a` and `b`, and is monotone in `t`, strictly when `\|b - a\| >= 1.0`. | `README.md#lerp` |
| LERP-REQ-003 | `from_ratio(n, d)` is `round_half_even(n * 2^32 / d)`, `None` for `d = 0` and for a result of `2^31` or more; `from_ratio(d, d)` is exactly `ONE`. | `README.md#from-ratio` |
| LERP-REQ-004 | `FixedVec3::lerp` and `lerp_channels` apply the scalar `lerp` to each component and return `None` when any component is `None`. | `README.md#vectors-and-channels` |

```toml
[[requirement]]
id = "LERP-REQ-001"
summary = "lerp is a + round_half_even((b - a) * t / 2^32) on raw values, exact, extrapolating, None only outside Q32.32."
status = "active"

[[requirement]]
id = "LERP-REQ-002"
summary = "lerp is exact at t = 0 and t = 1.0 and monotone in t, strictly when |b - a| >= 1.0."
status = "active"

[[requirement]]
id = "LERP-REQ-003"
summary = "from_ratio is round_half_even(n * 2^32 / d), None for d = 0 or a result of 2^31 or more; from_ratio(d, d) is ONE."
status = "active"

[[requirement]]
id = "LERP-REQ-004"
summary = "FixedVec3::lerp and lerp_channels are the scalar lerp per component, None when any component is None."
status = "active"
```

## Oracles

Each test file writes its own exact reference and shares no code with the kernel:
`oracle_lerp` computes `(b - a) * t` in `i128` and rounds the step by `div_euclid`,
`rem_euclid` and a comparison of twice the remainder with `2^32`; `oracle_ratio` computes
`floor((2 n 2^32 + d) / 2d)` in `u128` and moves a tie that went to an odd value back by one.
The kernel rounds by an arithmetic shift and a mask (`lerp`) and by `u64` quotient and
remainder (`from_ratio`). The hand-derived vectors are also checked against the oracle.

The golden vectors of LERP-TP-001 and LERP-TP-006 are raw `i64` values inline in
`crates/bunny-num/tests/lerp_tests.rs`, each with its derivation in a comment. Every test
in both files is a `wasm_bindgen_test`, so the WebAssembly job (`wasm-pack test --node` on
`bunny-num` and `bunny-linalg`) runs them all, golden vectors included.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| LERP-TP-001 | LERP-REQ-001 | The identity string; hand-derived golden vectors: `lerp(0, 1.0, 0.5) = 0.5`, `lerp(-1.0, 1.0, 0.25) = -0.5`; steps of 1 and 3 raw at `t = 0.5` (ties to 0 and 2), one raw unit of `t` either side, and the negative ties; the step, not the sum, is rounded (`lerp(5, 6, 0.5) = 5` raw); `lerp(0, 1.0, 2.0) = 2.0` and `t = -1.0`; results on and one raw unit past `i64::MAX` and `i64::MIN`; `b - a = 2^64 - 1` with ties at the widest step; the widest products refused. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_001_hand_derived_lerps_and_ties_round_to_even` |
| LERP-TP-002 | LERP-REQ-002 | `lerp(a, b, 0) == a` and `lerp(a, b, 1.0) == b` for all 4 096 pairs of the 64-value grid (0, `+-0.5`, `+-1.0` and `+-2^30`, each with three raw units either side, and the ends of the raw range). | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_002_the_ends_are_exact_for_every_grid_pair` |
| LERP-TP-003 | LERP-REQ-001 | Every grid pair at all 4 097 `t = k 2^20` in `[0, 1]` (16 781 312 cases) equals `oracle_lerp`. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_003_the_grid_matches_the_oracle_exhaustively` |
| LERP-TP-004 | LERP-REQ-001 | 100 000 seeded triples (seed printed; `a`, `b` of every magnitude; `t` in `[0, 1]`, in `[-4, 4)`, of every magnitude, and multiples of 1/2) equal `oracle_lerp`; at least 10 000 defined, 10 000 refused and 1 000 ties. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_004_seeded_triples_match_the_oracle` |
| LERP-TP-005 | LERP-REQ-002 | For 256 pairs from the grid and 12 291 `t` (`k 2^22` and one raw unit either side, `k` in `-2048..=2048`), the defined outputs are ordered as `b` against `a`, and never repeat when `\|b - a\| >= 1.0`; `lerp(0, 1 raw, 0) == lerp(0, 1 raw, 1 raw)` shows it is not strict below. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_005_lerp_is_monotone_in_t` |
| LERP-TP-006 | LERP-REQ-003 | Hand-derived golden ratios: `1/3` and `2/3` (raw 1 431 655 765 and 2 863 311 531), `7/7` and `(2^32 - 1)/(2^32 - 1)` are `ONE`; division by zero; `2^31 - 1` and `(2^32 - 1)/2` fit and `2^31` does not; ratios just under and over a half raw unit; each equals `oracle_ratio`. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_006_hand_derived_ratios` |
| LERP-TP-007 | LERP-REQ-003 | A 13 by 13 grid of boundary `u32` values, `from_ratio(d, d) == ONE` for `d` in `1..=4096`, and 100 000 seeded pairs (seed printed, `d` of every magnitude) equal `oracle_ratio`; at least 10 000 defined and 1 000 refused. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_007_ratios_match_the_oracle_on_a_grid_and_seeded_pairs` |
| LERP-TP-008 | LERP-REQ-004 | Hand-derived vector and channel lerps at `t = 0.5`, the exact ends, `t = 2.0`, and an overflowing component in each position refusing the whole result. | `crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_008_vector_and_channel_lerps_are_component_wise` |
| LERP-TP-009 | LERP-REQ-004 | 20 000 seeded vector triples (seed printed) equal a per-component `i128` oracle written in the test, for `FixedVec3::lerp` and `lerp_channels`; at least 5 000 defined and 1 000 refused. | `crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_009_seeded_vectors_and_channels_match_the_oracle` |
| LERP-TP-010 | LERP-REQ-001, LERP-REQ-002, LERP-REQ-003, LERP-REQ-004 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "LERP-TP-001"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_001_hand_derived_lerps_and_ties_round_to_even"
oracle = "Hand-derived raw golden vectors, each also checked against the independent i128 oracle."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-002"
requirements = ["LERP-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_002_the_ends_are_exact_for_every_grid_pair"
oracle = "a at t = 0 and b at t = 1.0 for every pair of the 64-value grid."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-003"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_003_the_grid_matches_the_oracle_exhaustively"
oracle = "Independent i128 oracle over 64 x 64 grid pairs and 4 097 values of t."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-004"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_004_seeded_triples_match_the_oracle"
oracle = "Independent i128 oracle over 100 000 seeded triples."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-005"
requirements = ["LERP-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_005_lerp_is_monotone_in_t"
oracle = "Order of consecutive outputs over a boundary grid of t."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-006"
requirements = ["LERP-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_006_hand_derived_ratios"
oracle = "Hand-derived raw golden ratios, each also checked against the independent u128 oracle."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-007"
requirements = ["LERP-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_007_ratios_match_the_oracle_on_a_grid_and_seeded_pairs"
oracle = "Independent u128 oracle over a boundary grid and 100 000 seeded pairs."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-008"
requirements = ["LERP-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_008_vector_and_channel_lerps_are_component_wise"
oracle = "Hand-derived raw component values."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-009"
requirements = ["LERP-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_009_seeded_vectors_and_channels_match_the_oracle"
oracle = "Per-component i128 oracle written in the test, 20 000 seeded triples."
tier = "fast"
status = "planned"

[[case]]
id = "LERP-TP-010"
requirements = ["LERP-REQ-001", "LERP-REQ-002", "LERP-REQ-003", "LERP-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/lerp/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
```

## Mutation Checks

Planned: each rule of the kernel is broken on purpose, the tests are run, and the failing
cases are recorded here.
