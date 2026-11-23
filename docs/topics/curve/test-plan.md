# Cubic Bezier Curve Test Plan

This document defines how Bunny proves [`bunny-curve/1`](README.md): part 1, `CubicBezier3`
with `at`, `derivative_at` and `split` (flyingrobots/bunny#308), and part 2, the uniform
Catmull-Rom spline `CatmullRom3` (flyingrobots/bunny#309).

## Scope

- `CubicBezier3::at`, `CubicBezier3::derivative_at` and `CubicBezier3::split` in
  `bunny-linalg`, and `CURVE_CONTRACT_ID`
- `CatmullRom3::new`, `segment_count`, `segment`, `to_beziers`, `at` and `derivative_at`,
  and `CatmullRomError`
- `CurveBounds3` and `CurveBounds3::contains`, `CubicBezier3::control_bounds`,
  `CubicBezier3::split_bounds` and `CatmullRom3::control_bounds` (flyingrobots/bunny#152)

Out of scope: arc length (flyingrobots/bunny#307), centripetal and chordal Catmull-Rom, and
a box tighter than the control box (from the exact cubic's extrema).

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
| CURVE-REQ-030 | `CubicBezier3::control_bounds` is the per-axis minimum and maximum of `p0` to `p3`, exact and never refused, and contains `at(t)` bit for bit at every `t` in `[0, 1]` with no widening; equal control points give that point. | `README.md#conservative-bounds` |
| CURVE-REQ-031 | `CurveBounds3::contains(point)` is `min <= point <= max` on every axis, faces included. | `README.md#conservative-bounds` |
| CURVE-REQ-032 | `CubicBezier3::split_bounds(t)` is each half's control box widened by 3 raw units per face and clipped to `control_bounds`; the first contains the original `at(s)` for every `s` in `[0, t]`, the second for every `s` in `[t, 1]`, each also its half's own evaluations; `None` for `t` outside `[0, 1]`. | `README.md#bounds-of-a-split` |
| CURVE-REQ-033 | `CatmullRom3::control_bounds` is the union of every segment's control box (`n - 1` segments open, `n` closed), without allocating, and contains `at(u)` at every `u` in `[0, segment_count]`, including where the curve leaves the box of its points. | `README.md#bounds-of-a-catmull-rom-spline` |

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

[[requirement]]
id = "CURVE-REQ-030"
summary = "CubicBezier3::control_bounds is the exact per-axis min and max of the control points and contains every at(t) with no widening."
status = "active"

[[requirement]]
id = "CURVE-REQ-031"
summary = "CurveBounds3::contains is min <= point <= max on every axis, faces included."
status = "active"

[[requirement]]
id = "CURVE-REQ-032"
summary = "split_bounds(t) widens each half's control box by 3 raw units, clipped to the control box, and contains the original at(s) on the half's range; None outside [0, 1]."
status = "active"

[[requirement]]
id = "CURVE-REQ-033"
summary = "CatmullRom3::control_bounds is the union of the segments' control boxes and contains every at(u), not the box of the points."
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

`crates/bunny-linalg/tests/catmull_rom_tests.rs` holds the Catmull-Rom references, also
sharing no code with `bunny-linalg` or `bunny-num`:

- **The rule port.** `port_segment` converts a segment with `i128` Euclidean division by 6
  and ties to even, with its own neighbour indexing, and `port_at` and `port_derivative`
  run de Casteljau on the result with their own `port_lerp`; `port_new` decides the
  refusals. The kernel must equal them bit for bit.
- **The exact Catmull-Rom cubic.** `exact` evaluates the power form
  `(2 P1 + (P2 - P0) t + (2 P0 - 5 P1 + 4 P2 - P3) t^2 + (3 P1 - P0 - 3 P2 + P3) t^3) / 2`
  and its derivative, not the Bezier form, as exact numerators in a 384-bit two's-complement
  integer, and `within` decides `|result - exact| <= bound` without rounding.

`crates/bunny-linalg/tests/curve_bounds_tests.rs` holds the bounds references, also sharing
no code with `bunny-linalg` or `bunny-num`: `port_box` (per-axis minimum and maximum of raw
points), `port_split_box` (a half from its own de Casteljau port, widened by 3 and clipped),
`port_segment` (its own Catmull-Rom conversion) and `inside`, which decides containment on raw
`i64` coordinates. Containment is checked against the kernel's own `at`, so the bound covers
the rounded evaluation, not only the exact curve.

The golden vectors of CURVE-TP-002 and CURVE-TP-021 come from Python models (an integer
port of the rule plus a `fractions` check of the bounds) posted in their pull requests.
Every test is a `wasm_bindgen_test`, so the WebAssembly job runs the golden vectors too.

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
| CURVE-TP-023 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-014 | A control point at exactly `MAX + 1/2` (ties to the even `MAX + 1`, refused) and `MIN - 1/2` (ties to the even `MIN`, accepted), with the inputs one raw unit either side; a knot tangent of `3 * 3 074 457 345 618 258 602 = MAX - 1` defined, and the neighbouring inputs whose tie goes the other way refused, arriving and leaving alike; a knot whose arriving segment's middle difference is `2^63` (refused) or `2^63 - 1` (defined), the leaving tangent defined for both. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_023_overflow_on_and_past_each_threshold` |
| CURVE-TP-024 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-012, CURVE-REQ-013, CURVE-REQ-014 | 20 000 seeded point lists of 1 to 6 points, open or closed (seed printed): `new` equals the port's outcome; for each valid spline the segments equal the port, four `u` each equal the port and are within 15/8 and 9/2 raw units of the exact power form; every point is hit exactly, tangents agree bit for bit at joints, knot tangents are within 3/2 of `(P[k + 1] - P[k - 1]) / 2`, closed seams are exact; both outcomes are reached and errors past 3/2 and 3 raw units occur. | `crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_024_seeded_splines_match_both_oracles` |
| CURVE-TP-025 | CURVE-REQ-010, CURVE-REQ-011, CURVE-REQ-012, CURVE-REQ-013 | Each Catmull-Rom rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |
| CURVE-TP-060 | CURVE-REQ-030, CURVE-REQ-031 | Hand-derived boxes: equal control points give that point and contain every evaluation; a flat curve has zero extent on its constant axes; a face set only by an inner control point (`y` up to 4, the curve peaking at 3); the ends of the raw range; `contains` on and one raw unit outside each face of each axis. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_060_hand_derived_boxes_and_contains` |
| CURVE-TP-061 | CURVE-REQ-030, CURVE-REQ-031 | 1 000 seeded polygons with the ends of the raw range often (seed printed), each at 324 `t` (257 multiples of 1/256 and every raw `t` within 16 of 0, 1/2 and 1): the box equals the independent minimum and maximum, and every `at(t)` lies in it, exactly per axis. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_061_seeded_control_boxes_contain_every_evaluation` |
| CURVE-TP-062 | CURVE-REQ-030 | Ties at a face: `x = (0, 1, 1, 0)` raw at `t = 1/2` ties three lerps and lands on the lower face (exact value 3/4 raw), and on and one raw unit either side of 1/2; `(MIN, MAX, MIN, MAX)` ties every first-level step at 1/2; faces one raw unit inside the range ends. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_062_ties_at_a_box_face_stay_inside` |
| CURVE-TP-063 | CURVE-REQ-032 | Constructed: the original at `s = 2 916 077 839` raw is 1 raw unit outside the right half's unwidened box (split at `t = 2 329 330 117` raw of `x = (-2, -3, 1, -4)`) and inside the widened one; `t` outside `[0, 1]` refused; 2 000 seeded polygons and `t` (seed printed): both boxes equal the independent port, the original at up to 69 `s` on each range and each half at 69 `u` lie inside. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_063_split_bounds_contain_the_original_on_each_range` |
| CURVE-TP-064 | CURVE-REQ-033 | Points `x = (0, 10, 10, 0)`: open, `at(1.5)` exceeds 10 (outside the points' box) and is inside the box, whose `x` maximum is `35 U / 3` rounded up; closed, `at(3.5)` dips below 0, outside the open box and inside the closed one; both boxes equal the port's union of segment boxes and contain 37 `u` per segment. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_064_catmull_rom_box_covers_the_overshoot` |
| CURVE-TP-065 | CURVE-REQ-033 | 2 000 seeded point lists of 2 to 6 points, open or closed (seed printed): for each one `new` accepts (more than half), the box equals the port's union of segment boxes and contains `at(u)` at 37 `u` per segment; evaluations outside the points' box occur. | `crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_065_seeded_catmull_rom_boxes_contain_every_evaluation` |
| CURVE-TP-066 | CURVE-REQ-030, CURVE-REQ-031, CURVE-REQ-032, CURVE-REQ-033 | Each bounds rule broken on purpose, and each face shrunk by one raw unit, fails at least one case above. | [Mutation checks](#mutation-checks) |

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
status = "implemented"

[[case]]
id = "CURVE-TP-021"
requirements = ["CURVE-REQ-011", "CURVE-REQ-013", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_021_golden_vectors"
oracle = "Raw golden vectors from an independent Python model of the rule, checked with fractions."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-022"
requirements = ["CURVE-REQ-010", "CURVE-REQ-012"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_022_refusals_and_the_parameter_range"
oracle = "Typed refusals and None on and one raw unit outside each end of [0, segment_count]."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-023"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_023_overflow_on_and_past_each_threshold"
oracle = "Hand-derived i64 thresholds of the control points and of the knot tangent, on ties."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-024"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-012", "CURVE-REQ-013", "CURVE-REQ-014"]
evidence = "test"
test = "crates/bunny-linalg/tests/catmull_rom_tests.rs::curve_tp_024_seeded_splines_match_both_oracles"
oracle = "Independent rule port and exact 384-bit power form over 20 000 seeded point lists."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-025"
requirements = ["CURVE-REQ-010", "CURVE-REQ-011", "CURVE-REQ-012", "CURVE-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/curve/test-plan.md#mutation-checks"
oracle = "Each Catmull-Rom rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "CURVE-TP-060"
requirements = ["CURVE-REQ-030", "CURVE-REQ-031"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_060_hand_derived_boxes_and_contains"
oracle = "Hand-derived raw boxes of constant, flat and bumped curves; contains on and one raw unit outside each face."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-061"
requirements = ["CURVE-REQ-030", "CURVE-REQ-031"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_061_seeded_control_boxes_contain_every_evaluation"
oracle = "Independent min and max over 1 000 seeded polygons, each evaluated at 324 parameters."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-062"
requirements = ["CURVE-REQ-030"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_062_ties_at_a_box_face_stay_inside"
oracle = "Constructed ties to even landing on a face, and ties at the ends of the raw range."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-063"
requirements = ["CURVE-REQ-032"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_063_split_bounds_contain_the_original_on_each_range"
oracle = "A constructed escape from the unwidened half box; independent de Casteljau port over 2 000 seeded splits."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-064"
requirements = ["CURVE-REQ-033"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_064_catmull_rom_box_covers_the_overshoot"
oracle = "Hand-derived overshoot of an open and a closed spline; independent segment conversion port."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-065"
requirements = ["CURVE-REQ-033"]
evidence = "test"
test = "crates/bunny-linalg/tests/curve_bounds_tests.rs::curve_tp_065_seeded_catmull_rom_boxes_contain_every_evaluation"
oracle = "Independent segment conversion port over 2 000 seeded point lists."
tier = "fast"
status = "implemented"

[[case]]
id = "CURVE-TP-066"
requirements = ["CURVE-REQ-030", "CURVE-REQ-031", "CURVE-REQ-032", "CURVE-REQ-033"]
evidence = "manual-audit"
artifact = "docs/topics/curve/test-plan.md#mutation-checks"
oracle = "Each bounds rule broken on purpose, and each face shrunk by one raw unit, makes at least one implemented case fail."
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

The Catmull-Rom rules were broken the same way in
`crates/bunny-linalg/src/catmull_rom3.rs`, with `cargo test --locked --no-fail-fast -p
bunny-linalg --test catmull_rom_tests`:

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| Ties round up (`rest >= 3`) | Ties to even | 023, 024 |
| `P1 + round((P2 - P0) / 6)`, rounding the sixth instead of the control point | One rounding of the exact control point | 023, 024 |
| An open spline's phantom neighbour is the next point, not the end point | Duplicated end points | 020, 021, 023, 024 |
| A closed spline clamps its last neighbour instead of wrapping | Closed neighbours wrap | 021, 024 |
| `u = segment_count` refused | The end belongs to the last segment | 020, 021, 022, 024 |
| A whole `u = k` owned by the arriving segment at `t = 1.0` | A point belongs to the segment leaving it | 023 |
| The control-point range not checked in `new` | `ControlPointOutOfRange` | 023, 024 |
| `AllPointsEqual` not checked | All points equal is refused | 022 |
| `b2` from `P3` toward `P1` (`P2 + (P3 - P1) / 6`) | The second inner control point | 020, 021, 023, 024 |
| A closed spline of 2 points accepted | At least 3 points closed | 022, 024 |

Owning a point by the arriving segment changes no position and, by the exact tangent
continuity, no defined derivative; only CURVE-TP-023's one-sided overflow sees it.

The bounds were broken the same way in `crates/bunny-linalg/src/curve_bounds3.rs`, with
`cargo test --locked --no-fail-fast -p bunny-linalg --test curve_bounds_tests`:

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| The control box's `min.x` raised by one raw unit | The box is the control points' minimum | 060, 061, 062, 063, 064, 065 |
| The control box's `min.y` raised by one raw unit | The box is the control points' minimum | 060, 061, 062, 063, 064, 065 |
| The control box's `min.z` raised by one raw unit | The box is the control points' minimum | 060, 061, 062, 063, 064, 065 |
| The control box's `max.x` lowered by one raw unit | The box is the control points' maximum | 060, 061, 062, 063, 064, 065 |
| The control box's `max.y` lowered by one raw unit | The box is the control points' maximum | 060, 061, 062, 063, 064, 065 |
| The control box's `max.z` lowered by one raw unit | The box is the control points' maximum | 060, 061, 062, 063, 064, 065 |
| The split widening 1 raw unit | Widened by 3 | 063 |
| The split widening 2 raw units | Widened by 3 | 063 |
| The split widening 0 | Widened by 3 | 063 (the constructed escape) |
| The widened split box not clipped to the control box | Clipped to `control_bounds` | 063 |
| The right half not widened | Both halves widened | 063 |
| The spline box from its points | The union of the segments' control boxes | 064, 065 |
| A closed spline's closing segment left out | Every segment, `n` when closed | 064, 065 |
| `contains` excludes the lower face | Closed on every face | 060, 061, 063, 064 |
| `contains` excludes the upper face | Closed on every face | 060, 061, 063, 064 |

A shrunk face reached by `p0` or `p3` fails containment, since `at(0)` and `at(1.0)` are
those points; a shrunk face set only by `p1` or `p2` fails the equality with the independent
minimum and maximum. A split widening of 1 or 2 raw units is caught only by the equality with
the port, which states the rule: no evaluation found leaves a half's box by more than 1 raw
unit, so containment alone cannot tell 1, 2 and 3 apart.
