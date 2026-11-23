# Cubic Bezier Curve Test Plan

This document defines how Bunny proves [`bunny-curve/1`](README.md), part 1: `CubicBezier3`
with `at`, `derivative_at` and `split`. It answers flyingrobots/bunny#308. Part 3, the
arc-length table (flyingrobots/bunny#307), has CURVE-REQ-020 onward and CURVE-TP-040 onward.

## Scope

- `CubicBezier3::at`, `CubicBezier3::derivative_at` and `CubicBezier3::split` in
  `bunny-linalg`, and `CURVE_CONTRACT_ID`
- `Curve3`, `ArcLengthTable::build`, `total`, `lengths`, `end` and `param_at_distance`, and
  `ArcLengthError`

Out of scope: Catmull-Rom splines (flyingrobots/bunny#309).

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| CURVE-REQ-001 | `at(t)` is de Casteljau over `FixedVec3::lerp` in the stated order (three levels, six lerps per component), for `t` in `[0, 1]` (raw `0..=2^32`); it is never refused there and is within 3/2 raw units per component of the exact Bernstein value. | `README.md#evaluation` |
| CURVE-REQ-002 | `at(0)` is exactly `p0` and `at(1.0)` exactly `p3`; equal control points give that point at every `t`. | `README.md#evaluation` |
| CURVE-REQ-003 | `derivative_at(t)` is `3` times the de Casteljau quadratic of the differences `p1 - p0`, `p2 - p1`, `p3 - p2`, with the differences and the multiplication by 3 checked; `None` when either leaves Q32.32; within 3 raw units of the exact derivative; exactly `3 (p1 - p0)` at `t = 0` and `3 (p3 - p2)` at `t = 1.0`. | `README.md#derivative` |
| CURVE-REQ-004 | `split(t)` returns the halves `(p0, a, d, m)` and `(m, e, c, p3)` of the de Casteljau points; `m` is bit for bit `at(t)`; each half at `u` is within 3 raw units of the exact original at the matching parameter. | `README.md#split` |
| CURVE-REQ-005 | `t` outside `[0, 1]` is `None` for all three functions. | `README.md#domain-and-refusals` |
| CURVE-REQ-020 | `Curve3` has `span_count()` and `at(u)`; `CubicBezier3` implements it with one span by its unchanged `at`. | `README.md#the-curve3-interface` |
| CURVE-REQ-021 | The table samples `u_k = from_ratio(k, m)` for `k` in `0..=K`, keeps each chord as `floor(2^16 \|c_k\|)` from its exact squared length, sums exactly and rounds each entry once to nearest, ties to even; entries never decrease and are within `1/2 + k / 2^16` raw of the sampled polyline. | `README.md#building-the-table` |
| CURVE-REQ-022 | A straight curve's total is its endpoint distance within 2 raw units for every `m`; against the true arc length the total is bounded by `A + 3 sqrt(3) K + 1/2` (proven) and its error is measured and recorded for three curves at `m` = 8, 64, 512, falling as `m` grows. | `README.md#what-the-total-is-a-bound-of` |
| CURVE-REQ-023 | `param_at_distance(s)` is `None` outside `[0, total]`, exactly `end()` at `total`, exactly `u_j` on entry `j` (the first of a run of equal entries), otherwise one half-even rounding of the linear interpolation in the bracket; it never decreases in `s`. | `README.md#distance-to-parameter` |
| CURVE-REQ-024 | At `s = k total / 16` the parameter error and chord spread are measured and recorded for the three curves. | `README.md#distance-to-parameter` |
| CURVE-REQ-025 | `build` refuses each case of `ArcLengthError` at its stated threshold, including chord and total overflow one raw unit either side and a zero-length curve. | `README.md#refusals` |

```toml
[[requirement]]
id = "CURVE-REQ-001"
summary = "at(t) is de Casteljau over FixedVec3::lerp in the stated order, never refused in [0, 1], within 3/2 raw units of the exact cubic."
status = "active"

[[requirement]]
id = "CURVE-REQ-002"
summary = "at(0) is p0 and at(1.0) is p3 exactly; equal control points give a constant."
status = "active"

[[requirement]]
id = "CURVE-REQ-003"
summary = "derivative_at is 3 times the de Casteljau quadratic of the checked differences, None on overflow, within 3 raw units, exact at the ends."
status = "active"

[[requirement]]
id = "CURVE-REQ-004"
summary = "split(t) returns the de Casteljau halves sharing at(t) bit for bit; each half is within 3 raw units of the exact original."
status = "active"

[[requirement]]
id = "CURVE-REQ-005"
summary = "t outside [0, 1] is None for at, derivative_at and split."
status = "active"

[[requirement]]
id = "CURVE-REQ-020"
summary = "Curve3 has span_count and at; CubicBezier3 implements it with one span by its unchanged at."
status = "active"

[[requirement]]
id = "CURVE-REQ-021"
summary = "Entries are the exact sum of chords floored to 2^-16 raw, rounded once half to even; non-decreasing, within 1/2 + k / 2^16 raw of the sampled polyline."
status = "active"

[[requirement]]
id = "CURVE-REQ-022"
summary = "Straight curve within 2 raw units of its endpoint distance; proven bound against the arc length; measured errors recorded and falling with m."
status = "active"

[[requirement]]
id = "CURVE-REQ-023"
summary = "param_at_distance: None outside [0, total], exact ends and entries, one half-even rounding inside a bracket, non-decreasing."
status = "active"

[[requirement]]
id = "CURVE-REQ-024"
summary = "Constant-speed parameter error and chord spread measured and recorded for three curves."
status = "active"

[[requirement]]
id = "CURVE-REQ-025"
summary = "build refuses each ArcLengthError case at its stated threshold."
status = "active"
```

## Oracles

`crates/bunny-linalg/tests/curve_tests.rs` holds two references, sharing no code with
`bunny-linalg` or `bunny-num`:

- **The rule port.** `port_lerp` rounds each step by `i128` Euclidean division and a
  comparison of twice the remainder with `2^32`; `port_levels` and `port_derivative` run the
  stated recurrence on raw coordinates. The kernel must equal them bit for bit.
- **The exact cubic.** `exact_within` evaluates the Bernstein form
  `sum C(n, i) c_i s^i (1 - s)^(n - i)` as an exact numerator in a 384-bit two's-complement
  integer written in the test, and decides `|result - exact| <= bound` without rounding.

The golden vectors of CURVE-TP-002 come from a Python model (an integer port of the rule
plus a `fractions` check of the bounds) posted in the pull request. Every test is a
`wasm_bindgen_test`, so the WebAssembly job runs the golden vectors too.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| CURVE-TP-001 | CURVE-REQ-001, CURVE-REQ-002, CURVE-REQ-003, CURVE-REQ-004 | The identity string; hand-derived: control points 0, 1, 2, 3 on x give `at(t) = 3t` and derivative `(3, 0, 0)` exactly at `t = k/8`, and halves at 0.5 of `(0, 0.5, 1, 1.5)` and `(1.5, 2, 2.5, 3)`; a flat curve keeps zero y and z; equal points give a constant, a zero derivative and constant halves. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_001_hand_derived_curves` |
| CURVE-TP-002 | CURVE-REQ-001, CURVE-REQ-003, CURVE-REQ-004 | Raw golden points, derivatives and halves of an asymmetric polygon at `t` = 0.5, `1/3`, 1 raw and `1 - 1` raw (one 1.4999999993 raw from exact), the same polygon with `p1` and `p2` swapped differing, and a polygon at the ends of the raw range. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_002_golden_vectors` |
| CURVE-TP-003 | CURVE-REQ-005 | `t` of `i64::MIN`, `-1.0`, -1 raw, `1.0 + 1` raw, `2.0` and `i64::MAX` refused by all three functions; 0 and `1.0` accepted. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_003_t_outside_zero_to_one_is_refused` |
| CURVE-TP-004 | CURVE-REQ-003 | The multiplication by 3 on and one raw unit past `+-(i64::MAX / 3)`; a difference on and one raw unit past `i64::MAX` and `i64::MIN` refusing even where the derivative would be zero; evaluation and split still defined. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_004_derivative_overflow_on_and_past_each_threshold` |
| CURVE-TP-005 | CURVE-REQ-001, CURVE-REQ-002, CURVE-REQ-003 | 100 000 seeded polygons and `t` (seed printed): `at` and `derivative_at` equal the rule port, are within 3/2 and 3 raw units of the exact cubic, errors above 1 and 2 raw units are reached, both derivative classes are reached, and the ends are exact for every polygon. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_005_seeded_evaluation_and_derivative_match_both_oracles` |
| CURVE-TP-006 | CURVE-REQ-004 | 20 000 seeded polygons, `t` and `u` (seed printed): the halves equal the port's de Casteljau points, share `at(t)` bit for bit, are exact at `t = 0` and `1.0`, and each half at `u` is within 3 raw units of the exact original at `t u` or `t + (1 - t) u`. | `crates/bunny-linalg/tests/curve_tests.rs::curve_tp_006_seeded_splits_share_their_points_and_track_the_original` |
| CURVE-TP-007 | CURVE-REQ-001, CURVE-REQ-003, CURVE-REQ-004, CURVE-REQ-005 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |
| CURVE-TP-040 | CURVE-REQ-020, CURVE-REQ-022, CURVE-REQ-023 | Hand-derived: the straight curve (0,0,0)-(3,4,0) has total 5.0 within 2 raw units and `param_at_distance(2.5)` 0.5 within 2 raw units for `m` in 1, 2, 3, 7, 8, 64, 512, 4095, 4096; `Curve3::at` equals `CubicBezier3::at` on and past both ends. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_040_hand_derived_straight_curve` |
| CURVE-TP-041 | CURVE-REQ-021 | The rule port with its own square root, exact sums and rounding: every entry of three curves at 8 values of `m`, non-decreasing; two chords found by search whose scaled length is an exact half, one rounding down to even, one up. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_041_entries_equal_the_rule_port` |
| CURVE-TP-042 | CURVE-REQ-022 | Python `decimal` Simpson arc lengths (pinned) and an in-test 100 000-step exact-point polyline (a reference, within 4 raw units below Python); `total()` under each and within the recorded error at `m` = 8, 64, 512, strictly falling. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_042_total_against_the_reference_arc_length` |
| CURVE-TP-043 | CURVE-REQ-023 | The lookup port over every entry, one raw unit either side and a 4096-step grid, for six curves (three with stationary samples at the start, middle and end) at `m` = 1, 3, 64, 4096: equal to the port, non-decreasing, exact at 0 and `total`, `None` one raw unit outside; runs of equal entries reached at the start, inside and at the end. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_043_lookup_rule_ends_runs_and_monotonicity` |
| CURVE-TP-044 | CURVE-REQ-024 | At `k total / 16`: the parameter against the 100 000-step reference inverted linearly, and the spread of the 16 chords by the test's square root, each within the recorded value. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_044_constant_speed_sampling` |
| CURVE-TP-045 | CURVE-REQ-020, CURVE-REQ-025 | Each refusal on and past its threshold: `m` 0, 1, 4096, 4097; storage `K` and `K + 1`; span counts 0 and `2^31`; `K` above `u32::MAX`; a refused sample; a chord of `i64::MAX` and `2^63` raw; totals of `i64::MAX` and `2^63`; four equal control points; two spans end at exactly 2.0. | `crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_045_refusals_on_and_past_each_threshold` |
| CURVE-TP-046 | CURVE-REQ-021, CURVE-REQ-023, CURVE-REQ-025 | Each arc-length rule broken on purpose fails at least one case above. | [Arc-length mutation checks](#arc-length-mutation-checks) |

```toml
[[case]]
id = "CURVE-TP-001"
requirements = ["CURVE-REQ-001", "CURVE-REQ-002", "CURVE-REQ-003", "CURVE-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_001_hand_derived_curves"
oracle = "Hand-derived raw values of straight, flat and constant curves."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-002"
requirements = ["CURVE-REQ-001", "CURVE-REQ-003", "CURVE-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_002_golden_vectors"
oracle = "Raw golden vectors from an independent Python model of the rule, checked with fractions."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-003"
requirements = ["CURVE-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_003_t_outside_zero_to_one_is_refused"
oracle = "None on and one raw unit outside each end of [0, 1]."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-004"
requirements = ["CURVE-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_004_derivative_overflow_on_and_past_each_threshold"
oracle = "Hand-derived i64 thresholds of the differences and of the multiplication by 3."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-005"
requirements = ["CURVE-REQ-001", "CURVE-REQ-002", "CURVE-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_005_seeded_evaluation_and_derivative_match_both_oracles"
oracle = "Independent rule port and exact 384-bit Bernstein form over 100 000 seeded cases."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-006"
requirements = ["CURVE-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_tests.rs::curve_tp_006_seeded_splits_share_their_points_and_track_the_original"
oracle = "Independent rule port and exact 384-bit Bernstein form over 20 000 seeded splits."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-007"
requirements = ["CURVE-REQ-001", "CURVE-REQ-003", "CURVE-REQ-004", "CURVE-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/curve/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "CURVE-TP-040"
requirements = ["CURVE-REQ-020", "CURVE-REQ-022", "CURVE-REQ-023"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_040_hand_derived_straight_curve"
oracle = "Hand-derived length and midpoint of a straight Bezier."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-041"
requirements = ["CURVE-REQ-021"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_041_entries_equal_the_rule_port"
oracle = "Independent port of the table rule with its own integer square root, and exact ties."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-042"
requirements = ["CURVE-REQ-022"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_042_total_against_the_reference_arc_length"
oracle = "Python decimal Simpson arc lengths and an in-test 100 000-step reference polyline."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-043"
requirements = ["CURVE-REQ-023"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_043_lookup_rule_ends_runs_and_monotonicity"
oracle = "Independent port of the lookup rule over entries, neighbours and a grid."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-044"
requirements = ["CURVE-REQ-024"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_044_constant_speed_sampling"
oracle = "The 100 000-step reference inverted linearly, and the test's own chord lengths."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-045"
requirements = ["CURVE-REQ-020", "CURVE-REQ-025"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_arc_length_tests.rs::curve_tp_045_refusals_on_and_past_each_threshold"
oracle = "Hand-derived thresholds one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-046"
requirements = ["CURVE-REQ-021", "CURVE-REQ-023", "CURVE-REQ-025"]
evidence = "manual-audit"
artifact = "docs/topics/curve/test-plan.md#arc-length-mutation-checks"
oracle = "Each arc-length rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of
`crates/bunny-linalg/src/cubic_bezier3.rs`, and `cargo test --locked --no-fail-fast -p
bunny-linalg --test curve_tests` was run; the source was restored after each. The unmutated
source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| `p1` and `p2` swapped in evaluation | The control-point order | 001, 002 (the asymmetric vector at `t = 1/3`), 005, 006 |
| The second level lerps from the later point, `lerp(b, a, 1 - t)` | The stated lerp direction | 002, 005, 006 |
| The third level lerps from the later point, `lerp(e, d, 1 - t)` | The stated lerp direction | 002, 005, 006 |
| `t` outside `[0, 1]` accepted (extrapolated) | `None` outside `[0, 1]` | 003 |
| `t = 1.0` refused (the upper end made exclusive) | `[0, 1]` is closed | 001, 003, 004, 005, 006 |
| The multiplication by 3 wraps | Checked multiplication by 3 | 004, 005 |
| The differences wrap | Checked differences | 002, 004, 005 |
| The derivative as `3 (e - d)` from the second-level points | 3 times the quadratic of the differences | 002, 005 |
| The second half's inner points swapped | The halves are the de Casteljau points | 001, 002, 006 |
| The first half's inner points swapped | The halves are the de Casteljau points | 001, 002, 006 |

The lerp-direction mutations change results only on ties, which is why the seeded
parameters include multiples of 1/8.

## Arc-Length Mutation Checks

Each rule below was broken on purpose in a local copy of
`crates/bunny-linalg/src/curve_arc_length.rs`, and `cargo test --locked --no-fail-fast -p
bunny-linalg --test curve_arc_length_tests` was run; the source was restored after each. The
unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| Each chord floored to a whole raw unit, as `FixedVec3::length` does | Chords kept to `2^-16` raw | 040, 041, 042, 044 |
| Each entry floored | One rounding to nearest | 041, 042, 044 |
| Ties round up | Ties to even | 041 |
| Sample parameters floored | `u_k = from_ratio(k, m)` | 041, 043 |
| The bracket is the last entry `<= s` | The first entry `>= s` | 043 |
| No exact `end()` at `s = total` | Exact end | 043 |
| The interpolation truncates | One half-even rounding | 043, 044 |
| `s` above `total` clamped to `end()` | `None` above `total` | 043 |
| `s` below zero clamped to 0 | `None` below zero | 043 |
| Chord differences wrap | Checked chord components | 045 |
| An entry above `i64::MAX` wraps | `LengthOverflow` | 045 |
| A zero total accepted | `ZeroLength` | 045 |
| `m = 4096` refused | `m` in `1..=4096` | 040, 041, 043, 045 |
