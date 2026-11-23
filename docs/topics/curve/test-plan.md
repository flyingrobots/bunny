# Cubic Bezier Curve Test Plan

This document defines how Bunny proves [`bunny-curve/1`](README.md): part 1, `CubicBezier3`
with `at`, `derivative_at` and `split` (flyingrobots/bunny#308), and part 2, the uniform
Catmull-Rom spline `CatmullRom3` (flyingrobots/bunny#309).

## Scope

- `CubicBezier3::at`, `CubicBezier3::derivative_at` and `CubicBezier3::split` in
  `bunny-linalg`, and `CURVE_CONTRACT_ID`
- `CatmullRom3::new`, `segment_count`, `segment`, `to_beziers`, `at` and `derivative_at`,
  and `CatmullRomError`

Out of scope: arc length (flyingrobots/bunny#307), and centripetal and chordal Catmull-Rom.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| CURVE-REQ-001 | `at(t)` is de Casteljau over `FixedVec3::lerp` in the stated order (three levels, six lerps per component), for `t` in `[0, 1]` (raw `0..=2^32`); it is never refused there and is within 3/2 raw units per component of the exact Bernstein value. | `README.md#evaluation` |
| CURVE-REQ-002 | `at(0)` is exactly `p0` and `at(1.0)` exactly `p3`; equal control points give that point at every `t`. | `README.md#evaluation` |
| CURVE-REQ-003 | `derivative_at(t)` is `3` times the de Casteljau quadratic of the differences `p1 - p0`, `p2 - p1`, `p3 - p2`, with the differences and the multiplication by 3 checked; `None` when either leaves Q32.32; within 3 raw units of the exact derivative; exactly `3 (p1 - p0)` at `t = 0` and `3 (p3 - p2)` at `t = 1.0`. | `README.md#derivative` |
| CURVE-REQ-004 | `split(t)` returns the halves `(p0, a, d, m)` and `(m, e, c, p3)` of the de Casteljau points; `m` is bit for bit `at(t)`; each half at `u` is within 3 raw units of the exact original at the matching parameter. | `README.md#split` |
| CURVE-REQ-005 | `t` outside `[0, 1]` is `None` for all three functions. | `README.md#domain-and-refusals` |
| CURVE-REQ-010 | `CatmullRom3::new` borrows its points and refuses, in this order, fewer than 2 points (3 closed) as `TooFewPoints`, all points equal as `AllPointsEqual`, and a segment with an inner control point outside Q32.32 as `ControlPointOutOfRange` naming the first such segment; `segment_count` is `n - 1` open and `n` closed. | `README.md#catmull-rom-points-and-refusals` |
| CURVE-REQ-011 | `segment(i)` is the `CubicBezier3` `(P1, round((6 P1 + P2 - P0) / 6), round((6 P2 + P1 - P3) / 6), P2)`, each inner component rounded once to nearest, ties to even, with neighbours wrapped when closed and end points duplicated when open; `to_beziers` yields every segment in order. | `README.md#catmull-rom-segments` |
| CURVE-REQ-012 | `u` in `[0, segment_count]` (raw `0..=count * 2^32`) is segment `floor(u)` at the fraction, with `u = segment_count` the last segment at `t = 1.0`; any other `u` is `None` for `at` and `derivative_at`. | `README.md#catmull-rom-parameter` |
| CURVE-REQ-013 | `at(u)` is `CubicBezier3::at` of the segment, within 15/8 raw units of the exact Catmull-Rom cubic, exactly point `k` at `u = k`, and `at(0) == at(segment_count)` when closed. | `README.md#catmull-rom-evaluation` |
| CURVE-REQ-014 | `derivative_at(u)` is `CubicBezier3::derivative_at` of the segment, `None` when that overflows, within 9/2 raw units of the exact derivative; at a point it is within 3/2 raw units of `(P[k + 1] - P[k - 1]) / 2`, and equals the previous segment's derivative at `t = 1.0` bit for bit where both are defined. | `README.md#catmull-rom-derivative` |

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
id = "CURVE-REQ-010"
summary = "CatmullRom3::new borrows its points and refuses too few points, all points equal, and an out-of-range control point, in that order."
status = "active"

[[requirement]]
id = "CURVE-REQ-011"
summary = "segment(i) is the Bezier with inner points (6 P1 + P2 - P0) / 6 and (6 P2 + P1 - P3) / 6 rounded once, ties to even; open ends duplicate, closed wraps."
status = "active"

[[requirement]]
id = "CURVE-REQ-012"
summary = "u in [0, segment_count] is segment floor(u) at the fraction, the end owned by the last segment; other u is None."
status = "active"

[[requirement]]
id = "CURVE-REQ-013"
summary = "at(u) is the segment's CubicBezier3::at, within 15/8 raw units of the exact cubic, exact at every point, closed at(0) == at(count)."
status = "active"

[[requirement]]
id = "CURVE-REQ-014"
summary = "derivative_at(u) is the segment's derivative, within 9/2 raw units, within 3/2 of (P[k+1] - P[k-1]) / 2 at a point, continuous bit for bit at joints."
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
| CURVE-TP-020 | CURVE-REQ-011, CURVE-REQ-013, CURVE-REQ-014 | Hand-derived: six collinear points `P_i = i d`, `d = (6, 12, -6)`, give `at(u) = u d` and derivative `d` exactly at `u = k/8` on the interior segments, `d g(t)` with `g(t) = (t + 2 t^2 - t^3) / 2` on the first segment and its mirror on the last, end tangents `d / 2`, and every point at integer `u`. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_020_hand_derived_collinear_and_through_points` |
| CURVE-TP-021 | CURVE-REQ-011, CURVE-REQ-013, CURVE-REQ-014 | Raw golden control points of every segment of an open five-point and a closed three-point spline, and raw golden points and derivatives at ten `u` (one 1.4999999979 raw from exact), the closed seam's point and tangent. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_021_golden_vectors` |
| CURVE-TP-022 | CURVE-REQ-010, CURVE-REQ-012 | Zero and one point open, two closed: `TooFewPoints`; two and three equal points: `AllPointsEqual`; two points open, three closed and repeated-but-not-all-equal points accepted; `u` of `i64::MIN`, `-1.0`, -1 raw, the end plus 1 raw, the end plus `1.0` and `i64::MAX` refused, 0 and the end accepted, the end owned by the last segment at `t = 1.0`. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_022_refusals_and_the_parameter_range` |
| CURVE-TP-023 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-014 | A control point at exactly `MAX + 1/2` (ties to the even `MAX + 1`, refused) and `MIN - 1/2` (ties to the even `MIN`, accepted), with the inputs one raw unit either side; a knot tangent of `3 * 3 074 457 345 618 258 602 = MAX - 1` defined, and the neighbouring inputs whose tie goes the other way refused, arriving and leaving alike. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_023_overflow_on_and_past_each_threshold` |
| CURVE-TP-024 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-012, CURVE-REQ-013, CURVE-REQ-014 | 20 000 seeded point lists of 1 to 6 points, open or closed (seed printed): `new` equals the port's outcome; for each valid spline the segments equal the port, four `u` each equal the port and are within 15/8 and 9/2 raw units of the exact power form; every point is hit exactly, tangents agree bit for bit at joints, knot tangents are within 3/2 of `(P[k + 1] - P[k - 1]) / 2`, closed seams are exact; both outcomes are reached and errors past 3/2 and 3 raw units occur. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_024_seeded_splines_match_both_oracles` |
| CURVE-TP-025 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-012, CURVE-REQ-013 | Each Catmull-Rom rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

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
id = "CURVE-TP-020"
requirements = ["CURVE-REQ-011", "CURVE-REQ-013", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_020_hand_derived_collinear_and_through_points"
oracle = "Hand-derived raw values of collinear equally spaced points."
tier = "fast"
status = "planned"

[[case]]
id = "CURVE-TP-021"
requirements = ["CURVE-REQ-011", "CURVE-REQ-013", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_021_golden_vectors"
oracle = "Raw golden vectors from an independent Python model of the rule, checked with fractions."
tier = "fast"
status = "planned"

[[case]]
id = "CURVE-TP-022"
requirements = ["CURVE-REQ-010", "CURVE-REQ-012"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_022_refusals_and_the_parameter_range"
oracle = "Typed refusals and None on and one raw unit outside each end of [0, segment_count]."
tier = "fast"
status = "planned"

[[case]]
id = "CURVE-TP-023"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_023_overflow_on_and_past_each_threshold"
oracle = "Hand-derived i64 thresholds of the control points and of the knot tangent, on ties."
tier = "fast"
status = "planned"

[[case]]
id = "CURVE-TP-024"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-012", "CURVE-REQ-013", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_024_seeded_splines_match_both_oracles"
oracle = "Independent rule port and exact 384-bit power form over 20 000 seeded point lists."
tier = "fast"
status = "planned"

[[case]]
id = "CURVE-TP-025"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-012", "CURVE-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/curve/test-plan.md#mutation-checks"
oracle = "Each Catmull-Rom rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
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
