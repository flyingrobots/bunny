# Exact Lerp and Ratio Test Plan

This document defines how Bunny proves [`bunny-lerp/1`](README.md): `FixedQ32_32::lerp`,
`FixedQ32_32::from_ratio`, `FixedQ32_32::inverse_lerp`, `FixedQ32_32::remap`,
`FixedQ32_32::checked_clamp`, `FixedVec3::lerp` and `lerp_channels`. It answers
flyingrobots/bunny#303 and flyingrobots/bunny#111.

## Scope

- `FixedQ32_32::lerp` and `FixedQ32_32::from_ratio` in `bunny-num`
- `FixedQ32_32::inverse_lerp`, `FixedQ32_32::remap` and `FixedQ32_32::checked_clamp` in
  `bunny-num`, and `Ord::min` and `Ord::max` on `FixedQ32_32`
- `FixedVec3::lerp` and `lerp_channels` in `bunny-linalg`

Out of scope: vector and channel forms of `inverse_lerp`, `remap` and `checked_clamp`,
which do not exist.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| LERP-REQ-001 | `lerp(a, b, t)` is `a + round_half_even((b - a) * t / 2^32)` on raw values, computed exactly, for every `t` (extrapolating outside `[0, 1]`), and `None` only when that result is outside Q32.32. | `README.md#lerp` |
| LERP-REQ-002 | `lerp` returns exactly `a` at `t = 0` and exactly `b` at `t = 1.0` for every `a` and `b`, and is monotone in `t`, strictly when `\|b - a\| >= 1.0`. | `README.md#lerp` |
| LERP-REQ-003 | `from_ratio(n, d)` is `round_half_even(n * 2^32 / d)`, `None` for `d = 0` and for a result of `2^31` or more; `from_ratio(d, d)` is exactly `ONE`. | `README.md#from-ratio` |
| LERP-REQ-004 | `FixedVec3::lerp` and `lerp_channels` apply the scalar `lerp` to each component and return `None` when any component is `None`. | `README.md#vectors-and-channels` |
| LERP-REQ-010 | `x.checked_clamp(lo, hi)` is exactly `lo` below `lo`, `hi` above `hi` and `x` otherwise, `lo` when `lo == hi`, and `None` when `lo > hi`; `Ord::min` and `Ord::max` follow the raw order. | `README.md#clamp-min-and-max` |
| LERP-REQ-011 | `inverse_lerp(a, b, x)` is `round_half_even((x - a) * 2^32 / (b - a))` on raw values, computed exactly, for every `x` (extrapolating), exactly `0` at `x = a` and `1.0` at `x = b`, and `None` when `a == b` or the result is outside Q32.32. | `README.md#inverse-lerp` |
| LERP-REQ-012 | With `d = \|b - a\|` raw and both calls defined, `lerp(a, b, inverse_lerp(a, b, x))` is `x` when `d <= 2^32` and within `floor(1/2 + d / 2^33)` raw units otherwise; `inverse_lerp(a, b, lerp(a, b, t))` is `t` when `d >= 2^32` and within `floor(1/2 + 2^31 / d)` otherwise. | `README.md#round-trips` |
| LERP-REQ-013 | `remap(x, [in_lo, in_hi], [out_lo, out_hi])` is `out_lo + round_half_even((x - in_lo) (out_hi - out_lo) / (in_hi - in_lo))` on raw values, computed exactly, exactly `out_lo` and `out_hi` at the ends, monotone in `x`, equal to `lerp` and `inverse_lerp` on unit ranges, and `None` when `in_lo == in_hi` or the result is outside Q32.32. | `README.md#remap` |

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

[[requirement]]
id = "LERP-REQ-010"
summary = "checked_clamp is exact, lo when lo == hi, None when lo > hi; Ord::min and Ord::max follow the raw order."
status = "active"

[[requirement]]
id = "LERP-REQ-011"
summary = "inverse_lerp is round_half_even((x - a) * 2^32 / (b - a)), exact at the ends, extrapolating, None for a == b or outside Q32.32."
status = "active"

[[requirement]]
id = "LERP-REQ-012"
summary = "Each lerp/inverse_lerp round trip is exact on one side of |b - a| = 1.0 and within a stated floor bound on the other."
status = "active"

[[requirement]]
id = "LERP-REQ-013"
summary = "remap rounds its exact step once, is exact at the ends, monotone, equals lerp and inverse_lerp on unit ranges, None for a degenerate input range or outside Q32.32."
status = "active"
```

## Oracles

Each test file writes its own exact reference and shares no code with the kernel:
`oracle_lerp` computes `(b - a) * t` in `i128` and rounds the step by `div_euclid`,
`rem_euclid` and a comparison of twice the remainder with `2^32`; `oracle_ratio` computes
`floor((2 n 2^32 + d) / 2d)` in `u128` and moves a tie that went to an odd value back by one.
The kernel rounds by an arithmetic shift and a mask (`lerp`) and by `u64` quotient and
remainder (`from_ratio`). The hand-derived vectors are also checked against the oracle.

For `inverse_lerp` and `remap`, `crates/bunny-num/tests/inverse_lerp_remap_tests.rs` has two
oracles of its own. `oracle_inverse` rounds `(x - a) 2^32 / (b - a)` by Euclidean division
of the signed value in `i128`. `oracle_remap` never forms the 128-bit product: `mul_div`
walks the bits of one factor, doubling and adding the other modulo the divisor, so it holds
`floor(m n / d)` and its remainder without a wide multiply. The kernel instead multiplies
the magnitudes in `u128`, divides once and compares the remainder with its complement. The
two oracles are checked against each other on every grid case of LERP-TP-023. The
hand-derived vectors of LERP-TP-020, -022 and -026 are inline raw values with their
derivations, each also checked against the oracle.

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
| LERP-TP-004 | LERP-REQ-001 | 100 000 seeded triples (seed printed; `a`, `b` of every magnitude; `t` in `[0, 1]`, in `[-4, 4)`, of every magnitude, and multiples of 1/2) equal `oracle_lerp`; at least 10 000 defined, 5 000 refused and 1 000 ties. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_004_seeded_triples_match_the_oracle` |
| LERP-TP-005 | LERP-REQ-002 | For 256 pairs from the grid and 12 291 `t` (`k 2^22` and one raw unit either side, `k` in `-2048..=2048`), the defined outputs are ordered as `b` against `a`, and never repeat when `\|b - a\| >= 1.0`; `lerp(0, 1 raw, 0) == lerp(0, 1 raw, 1 raw)` shows it is not strict below. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_005_lerp_is_monotone_in_t` |
| LERP-TP-006 | LERP-REQ-003 | Hand-derived golden ratios: `1/3` and `2/3` (raw 1 431 655 765 and 2 863 311 531), `7/7` and `(2^32 - 1)/(2^32 - 1)` are `ONE`; division by zero; `2^31 - 1` and `(2^32 - 1)/2` fit and `2^31` does not; ratios just under and over a half raw unit; each equals `oracle_ratio`. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_006_hand_derived_ratios` |
| LERP-TP-007 | LERP-REQ-003 | A 13 by 13 grid of boundary `u32` values, `from_ratio(d, d) == ONE` for `d` in `1..=4096`, and 100 000 seeded pairs (seed printed, `d` of every magnitude) equal `oracle_ratio`; at least 10 000 defined and 1 000 refused. | `crates/bunny-num/tests/lerp_tests.rs::lerp_tp_007_ratios_match_the_oracle_on_a_grid_and_seeded_pairs` |
| LERP-TP-008 | LERP-REQ-004 | Hand-derived vector and channel lerps at `t = 0.5`, the exact ends, `t = 2.0`, and an overflowing component in each position refusing the whole result. | `crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_008_vector_and_channel_lerps_are_component_wise` |
| LERP-TP-009 | LERP-REQ-004 | 20 000 seeded vector triples (seed printed) equal a per-component `i128` oracle written in the test, for `FixedVec3::lerp` and `lerp_channels`; at least 5 000 defined and 1 000 refused. | `crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_009_seeded_vectors_and_channels_match_the_oracle` |
| LERP-TP-010 | LERP-REQ-001, LERP-REQ-002, LERP-REQ-003, LERP-REQ-004 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |
| LERP-TP-020 | LERP-REQ-010 | Hand-derived clamps: `lo == hi` with `x` on it and one raw unit either side; `x` on each bound and one raw unit either side; `lo > hi` refused, including `lo = hi + 1` raw; the ends of the raw range. Then all 262 144 grid triples against `lo <= hi` then `max(lo, min(x, hi))` on raw `i64`, and `Ord::clamp` agreeing where it does not panic. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_020_checked_clamp_is_exact_and_refuses_an_inverted_range` |
| LERP-TP-021 | LERP-REQ-010 | `min`, `max` and `cmp` on all 4 096 grid pairs equal those of the raw `i64` values. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_021_min_and_max_follow_the_raw_order` |
| LERP-TP-022 | LERP-REQ-011 | Hand-derived golden vectors: `0.5`, `1/3` and `2/3`, a reversed range; ties at `b - a = 2.0` (`+-1/2` and `+-3/2` raw of `t`) and at `b - a = 4.0` with one raw unit of `x` either side; extrapolation both ways; four degenerate ranges refused; the ends of the widest range; `t` on and one raw unit past `i64::MAX` and `i64::MIN`; a tie rounding to `2^63` refused, with its two neighbours; `b - a = 1` raw. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_022_hand_derived_inverse_lerps` |
| LERP-TP-023 | LERP-REQ-011 | Every triple of the 64-value grid (262 144 cases) equals `oracle_inverse`, and `oracle_remap` with `[0, 1.0]` agrees; the ends are exact for every pair, `None` when degenerate. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_023_inverse_lerp_matches_the_oracle_on_the_grid` |
| LERP-TP-024 | LERP-REQ-011 | 100 000 seeded triples (seed printed; a quarter with `b - a = 2^(33 + k)` and `x` near `a`, a quarter degenerate, half of every magnitude) equal `oracle_inverse`; at least 10 000 defined, 1 000 refused for overflow, 1 000 ties and 25 000 degenerate. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_024_seeded_inverse_lerps_match_the_oracle` |
| LERP-TP-025 | LERP-REQ-012 | `lerp(0, 1 raw, 0) == lerp(0, 1 raw, 1 raw)`, so the round trip is not exact; both round trips within their bounds for all grid triples with `a != b` and 100 000 seeded triples (seed printed), at least 10 000 with both defined, and neither round trip exact over the run (each bound's largest value, `2^31` raw, is reached). | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_025_round_trips_hold_within_their_bounds` |
| LERP-TP-026 | LERP-REQ-013 | Hand-derived golden vectors: `0.5` from `[0, 1]` to `[10, 20]`, `1/3`, a reversed output range, extrapolation; ties of `+-1/2` and `+-3/2` raw and one raw unit either side; the step, not the sum, is rounded; three degenerate input ranges refused; a degenerate output range; exact ends of the widest ranges, both reversed; results on and one raw unit past `i64::MAX` and `i64::MIN`, by a tie and without; a step rounding to `2^64` and products near `2^128` and above `2^127` refused. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_026_hand_derived_remaps` |
| LERP-TP-027 | LERP-REQ-013 | Every 5-tuple of a 16-value grid (1 048 576 cases) equals `oracle_remap`. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_027_remap_matches_the_oracle_on_the_grid` |
| LERP-TP-028 | LERP-REQ-013 | 100 000 seeded 5-tuples (seed printed; a quarter with small even input ranges, a quarter degenerate) equal `oracle_remap`, `remap(x, [0, 1.0], [a, b])` equals `lerp` and `remap(x, [a, b], [0, 1.0])` equals `inverse_lerp`; at least 10 000 defined, 1 000 refused for overflow and 1 000 ties. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_028_seeded_remaps_match_the_oracle_and_lerp` |
| LERP-TP-029 | LERP-REQ-013 | The ends are exact for 262 144 range pairs from the grid (`None` when the input range is degenerate), and over 4 096 range pairs the defined outputs for the sorted grid of `x` are ordered as the two ranges run. | `crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_029_remap_ends_are_exact_and_the_map_is_monotone` |
| LERP-TP-030 | LERP-REQ-010, LERP-REQ-011, LERP-REQ-012, LERP-REQ-013 | Each rule of `crates/bunny-num/src/remap.rs` broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "LERP-TP-001"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_001_hand_derived_lerps_and_ties_round_to_even"
oracle = "Hand-derived raw golden vectors, each also checked against the independent i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-002"
requirements = ["LERP-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_002_the_ends_are_exact_for_every_grid_pair"
oracle = "a at t = 0 and b at t = 1.0 for every pair of the 64-value grid."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-003"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_003_the_grid_matches_the_oracle_exhaustively"
oracle = "Independent i128 oracle over 64 x 64 grid pairs and 4 097 values of t."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-004"
requirements = ["LERP-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_004_seeded_triples_match_the_oracle"
oracle = "Independent i128 oracle over 100 000 seeded triples."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-005"
requirements = ["LERP-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_005_lerp_is_monotone_in_t"
oracle = "Order of consecutive outputs over a boundary grid of t."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-006"
requirements = ["LERP-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_006_hand_derived_ratios"
oracle = "Hand-derived raw golden ratios, each also checked against the independent u128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-007"
requirements = ["LERP-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/lerp_tests.rs::lerp_tp_007_ratios_match_the_oracle_on_a_grid_and_seeded_pairs"
oracle = "Independent u128 oracle over a boundary grid and 100 000 seeded pairs."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-008"
requirements = ["LERP-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_008_vector_and_channel_lerps_are_component_wise"
oracle = "Hand-derived raw component values."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-009"
requirements = ["LERP-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/lerp_tests.rs::lerp_tp_009_seeded_vectors_and_channels_match_the_oracle"
oracle = "Per-component i128 oracle written in the test, 20 000 seeded triples."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-010"
requirements = ["LERP-REQ-001", "LERP-REQ-002", "LERP-REQ-003", "LERP-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/lerp/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "LERP-TP-020"
requirements = ["LERP-REQ-010"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_020_checked_clamp_is_exact_and_refuses_an_inverted_range"
oracle = "Hand-derived raw clamps and every grid triple against raw i64 comparisons."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-021"
requirements = ["LERP-REQ-010"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_021_min_and_max_follow_the_raw_order"
oracle = "min, max and cmp on raw i64 for every grid pair."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-022"
requirements = ["LERP-REQ-011"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_022_hand_derived_inverse_lerps"
oracle = "Hand-derived raw golden vectors, each also checked against the independent i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-023"
requirements = ["LERP-REQ-011"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_023_inverse_lerp_matches_the_oracle_on_the_grid"
oracle = "Independent i128 oracle over every triple of the 64-value grid, cross-checked by the bit-serial remap oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-024"
requirements = ["LERP-REQ-011"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_024_seeded_inverse_lerps_match_the_oracle"
oracle = "Independent i128 oracle over 100 000 seeded triples."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-025"
requirements = ["LERP-REQ-012"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_025_round_trips_hold_within_their_bounds"
oracle = "The derived floor bounds over the grid and 100 000 seeded triples."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-026"
requirements = ["LERP-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_026_hand_derived_remaps"
oracle = "Hand-derived raw golden vectors, each also checked against the independent bit-serial oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-027"
requirements = ["LERP-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_027_remap_matches_the_oracle_on_the_grid"
oracle = "Independent bit-serial oracle over every 5-tuple of a 16-value grid."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-028"
requirements = ["LERP-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_028_seeded_remaps_match_the_oracle_and_lerp"
oracle = "Independent bit-serial oracle over 100 000 seeded 5-tuples, and lerp and inverse_lerp on unit ranges."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-029"
requirements = ["LERP-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/inverse_lerp_remap_tests.rs::lerp_tp_029_remap_ends_are_exact_and_the_map_is_monotone"
oracle = "Exact ends and the order of consecutive outputs over grid ranges."
tier = "fast"
status = "implemented"

[[case]]
id = "LERP-TP-030"
requirements = ["LERP-REQ-010", "LERP-REQ-011", "LERP-REQ-012", "LERP-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/lerp/test-plan.md#mutation-checks"
oracle = "Each rule of remap.rs broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-num/src/lerp.rs` or
`crates/bunny-linalg/src/lerp.rs`, and `cargo test --locked --no-fail-fast -p bunny-num
-p bunny-linalg --test lerp_tests` was run; the source was restored after each. The
unmutated source passed every case. "Half up" means a tie goes toward positive infinity.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| A tie of the step goes half up | Ties to even | 001 (the tie `b - a = 1` raw at `t = 0.5`), 003, 004, 008 |
| A tie goes away from zero | Ties to even | 001, 003, 004, 008 |
| A tie goes toward zero | Ties to even | 001, 003, 004 |
| A tie goes to the odd neighbour | Ties to even | 001, 003, 004, 008 |
| The step is floored, not rounded | Round to nearest | 001, 003, 004, 009 |
| `b - a` checked in `i64`, `None` when it overflows | `b - a` exact in `i128` | 001, 002, 003, 009 |
| `a 2^32 + (b - a) t` rounded as a whole | The step, not the sum, is rounded | 001 (`lerp(5, 6, 0.5)`), 003, 004, 008 |
| The `i64` range check removed (the sum wraps) | `None` outside Q32.32 | 001, 004, 005, 008, 009 |
| `from_ratio` truncates | Round to nearest | 006 (`2/3`), 007 |
| `from_ratio` range check removed (wraps) | `None` for `2^31` or more | 006, 007 |
| `lerp_channels` returns its last two channels swapped | Per component | 008, 009 |
| `lerp_channels` keeps `b`'s third channel when it is refused | `None` when any channel is | 008, 009 |
| `from_ratio` ties go half up | Ties to even | none, as proved: no `u32` ratio is a tie ([from ratio](README.md#from-ratio)) |

`FixedVec3::lerp` calls `lerp_channels`, so the two channel mutations change it too and
LERP-TP-008 and -009 check both.

The rules of `crates/bunny-num/src/remap.rs` were broken the same way, one at a time, and
`cargo test --locked --no-fail-fast -p bunny-num --test inverse_lerp_remap_tests` was run;
the source was restored after each and the unmutated source passed every case. Each
mutation of the shared rounding changes `inverse_lerp` as well, since it calls `remap`.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| A tie of the step's magnitude goes away from zero | Ties to even | 022, 023, 024, 026, 027, 028 |
| A tie goes toward zero | Ties to even | 022, 023, 024, 026, 027, 028 |
| A tie goes to the odd neighbour | Ties to even | 022, 023, 024, 026, 027, 028 |
| The step is truncated, not rounded | Round to nearest | 022, 023, 024, 025, 026, 027, 028 |
| A step of exactly `2^64 - 1` refused | The widest step is accepted | 026, 029 |
| The `2^64 - 1` step cap removed | Refused before signed arithmetic | 026 (the product above `2^127` overflows `i128`, a panic in a test build) |
| The degenerate-range check removed | `None` for `in_lo == in_hi` | 022, 023, 024, 026, 027, 028, 029 (division by zero panics) |
| The sign of `in_hi - in_lo` ignored | Reversed ranges | 022 to 029 |
| The `i64` range check removed (the sum wraps) | `None` outside Q32.32 | 022 to 029 |
| `checked_clamp` refuses `lo == hi` | `lo == hi` returns `lo` | 020 |
| `checked_clamp` swaps an inverted range | `None` when `lo > hi` | 020 |
| `checked_clamp` ignores `hi` | Exact clamp | 020 |
