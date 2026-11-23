# Cubic Bezier Curves (`bunny-curve/1`)

This chapter states how Bunny evaluates, differentiates and splits a cubic Bezier curve in
Q32.32. Every rounding is a [`bunny-lerp/1`](../lerp/README.md) lerp, applied in a stated
order, so the result is fixed bit for bit. The identity string is `bunny-curve/1`
(`bunny_linalg::CURVE_CONTRACT_ID`). The [test plan](test-plan.md) holds the oracles,
golden vectors and mutation checks. It answers flyingrobots/bunny#308 (part 1 of
`bunny-curve/1`).

| Function | Result | Error against the exact curve | Refused (`None`) |
| --- | --- | --- | --- |
| `CubicBezier3::at(t)` | de Casteljau, six lerps per component | at most 3/2 raw units | `t` outside `[0, 1]` |
| `CubicBezier3::derivative_at(t)` | 3 times the de Casteljau quadratic of the differences | at most 3 raw units | `t` outside `[0, 1]`; a difference or the product by 3 outside Q32.32 |
| `CubicBezier3::split(t)` | the de Casteljau halves | each half at most 3 raw units from the original | `t` outside `[0, 1]` |

`CubicBezier3 { p0, p1, p2, p3 }` holds four `FixedVec3` control points. The exact curve is

```text
B(t) = (1 - t)^3 p0 + 3 t (1 - t)^2 p1 + 3 t^2 (1 - t) p2 + t^3 p3
```

and the errors below are per component, in raw units (`2^-32`), against `B` at the exact
rational `t`.

## Domain and Refusals

`t` is a Q32.32 value in `[0, 1]`: raw `0` to `2^32` inclusive. Every function returns
`None` for a raw `t` below `0` or above `2^32`, so `-1` raw and `2^32 + 1` raw are refused;
the curve is not extrapolated. Inside `[0, 1]`, `at` and `split` are never refused: a lerp
with `t` in `[0, 1]` lies between its two ends, so no de Casteljau point leaves the range of
the control points. Only `derivative_at` can overflow (see [derivative](#derivative)).

## Evaluation

`at(t)` is defined as the de Casteljau recurrence over `FixedVec3::lerp`, each lerp from the
earlier point to the later one:

```text
a = lerp(p0, p1, t)   b = lerp(p1, p2, t)   c = lerp(p2, p3, t)
d = lerp(a, b, t)     e = lerp(b, c, t)
at(t) = lerp(d, e, t)
```

The result is whatever these six lerps give, bit for bit; it is not one rounding of `B(t)`.
The direction matters: `lerp(b, a, 1 - t)` has the same exact value as `lerp(a, b, t)`, but
ties to even can round it differently, so the order above is part of the contract.

**Bound.** Each lerp is `x + round((y - x) t)`, within half a raw unit of the exact
`(1 - t) x + t y`. If its inputs are within `E` of their exact de Casteljau values, its
exact value is within `(1 - t) E + t E = E` of the exact point, because `t` is in
`[0, 1]`, and its rounding adds at most `1/2`. The three levels give `1/2`, `1` and `3/2`, so
`|at(t) - B(t)| <= 3/2` raw units. The bound is nearly reached: in the golden vectors one
component is 1.4999999993 raw units from exact. The issue's stated budget of 3 raw units
holds with room to spare.

**Ends.** A lerp returns exactly its first end at `t = 0` and its second at `t = 1.0`, so
`at(0)` is exactly `p0` and `at(1.0)` exactly `p3`. Equal control points give that point at
every `t`, and control points `0, 1, 2, 3` on an axis give exactly `3t` there, because every
lerp's step is then `t` times a whole unit, with nothing to round.

## Derivative

`derivative_at(t)` is the derivative of `B` computed from the differences:

```text
d0 = p1 - p0   d1 = p2 - p1   d2 = p3 - p2          exact, each checked against i64
q = lerp(lerp(d0, d1, t), lerp(d1, d2, t), t)        three lerps per component
derivative_at(t) = 3 q                               exact, checked against i64
```

It is not `3 (e - d)` from the second-level points of evaluation: that has the same exact
value but different roundings.

**Bound.** `q` is two levels of lerp, within 1 raw unit of the exact
`(1 - t)^2 d0 + 2 t (1 - t) d1 + t^2 d2`, and the exact multiplication by 3 makes that
`|derivative_at(t) - B'(t)| <= 3` raw units. The result is always a multiple of 3 raw units.
The bound is reached: a golden vector is exactly 3 raw units from exact.

**Ends.** `derivative_at(0)` is exactly `3 (p1 - p0)` and `derivative_at(1.0)` exactly
`3 (p3 - p2)`, whenever the result is defined.

**Overflow.** `None` when any of `d0`, `d1`, `d2` is outside the `i64` raw range, at every
`t`, even where that difference has no weight (`d0` at `t = 1.0`): control points more than
`2^63 - 1` raw apart on an axis, such as `-1` raw and `i64::MAX`. Also `None` when `3 q` is
outside `i64`: `3 q` fits exactly when `|q| <= 3 074 457 345 618 258 602` raw
(`i64::MAX = 3 * 3 074 457 345 618 258 602 + 1`). The lerps in between are never refused.

## Split

`split(t)` returns the two halves from the same de Casteljau points as `at(t)`:

```text
left  = (p0, a, d, m)      right = (m, e, c, p3)      m = at(t)
```

What holds exactly:

- The halves share `m`, and `m` is bit for bit `at(t)`, so
  `left.at(1.0) == right.at(0) == at(t)`.
- `left` starts at exactly `p0` and `right` ends at exactly `p3`.
- `split(0)` is `((p0, p0, p0, p0), self)` and `split(1.0)` is `(self, (p3, p3, p3, p3))`.

What does not: evaluating a half is not evaluating the original. The inner points `a`, `d`,
`m`, `e`, `c` are rounded, so `left.at(u)` is generally not `at(t u)`, even when `t u` is a
Q32.32 value. The bound: the exact left half through the rounded points differs from the
exact original at `t u` by a weighted average of the point errors, at most `3/2`, and
evaluating it adds at most `3/2`. So

```text
|left.at(u)  - B(t u)|           <= 3 raw units
|right.at(u) - B(t + (1 - t) u)| <= 3 raw units
```

against `B` at the exact rational parameter. The seeded test reaches errors above `3/2`.

## Widths

Every lerp holds its step exactly in `i128` (`bunny-lerp/1`). The differences of the
derivative are checked in `i64` (`checked_sub`), and the multiplication by 3 too
(`checked_mul`); nothing else is computed outside the lerps.

## Arc Length

Part 3 of `bunny-curve/1` (flyingrobots/bunny#307) measures a curve by the polyline through
equal parameter steps and inverts that table for constant-speed motion. The table is an
approximation: what is proven and what is only measured are kept apart below.

### The `Curve3` Interface

`Curve3` has two methods: `span_count()` and `at(u)` for `u` in `[0, span_count]`, `None`
outside. `CubicBezier3` implements it with one span, delegating to `CubicBezier3::at`
unchanged. Any deterministic implementation can be measured; the bounds that mention
`3/2` raw units hold for `CubicBezier3` and must be restated for another curve with its own
point error.

### Building the Table

`ArcLengthTable::build(curve, segments_per_span, storage)` takes
`segments_per_span` `m` in `1..=4096` and writes into the front of a caller-provided
`&mut [FixedQ32_32]`; it never allocates. With `K = span_count * m`:

- Sample `k`, for `k` in `0..=K`, is `P_k = curve.at(u_k)` with
  `u_k = FixedQ32_32::from_ratio(k, m)` (`bunny-lerp/1`: `k * 2^32 / m` rounded once to
  nearest). `u_0 = 0` and `u_K = span_count` exactly.
- Each chord `c_k = P_{k+1} - P_k` is formed exactly; its squared length is exact in `u128`
  (below `3 * 2^126`), and its length is kept as `q_k = floor(2^16 |c_k|)`, so
  `0 <= 2^16 |c_k| - q_k < 1`. `q_k` is `r * 2^16 + j`: `r` is the floor square root of the
  squared length (`FixedQ32_32::sqrt_u128`) and the 16 bits `j` are chosen bit by bit as the
  largest with `2^17 r j + j^2 <= 2^32 (S - r^2)`, every term below `2^98`.
- The `q_k` are summed exactly, and entry `L_k` is that sum divided by `2^16` and rounded
  once to nearest, ties to even. `L_0 = 0`, and `total()` is `L_K`.

So `L_k` never decreases, and against the exact length `D_k` of the polyline through the
sampled points

```text
D_k - 1/2 - k / 2^16  <=  L_k  <=  D_k + 1/2        (raw units)
```

which is within `0.5625` raw units for a single span at `m = 4096`.

This does not use `FixedVec3::length`, which the issue proposed. That function floors each
chord to a whole raw unit, so the sum can lose up to one raw unit per chord: with floored
chords the straight curve of CURVE-TP-040 is 1618 raw units short at `m = 4095`, against
the criterion of 2. The extra 16 bits keep the loss below `2^-16` per chord.

### What the Total Is a Bound Of

The polyline through the exact curve at any increasing parameters is inscribed, so it is
never longer than the true arc length `A`. The table's points are not exact:
`CubicBezier3::at` is within `3/2` raw units per component, `(3/2) sqrt 3` in distance, so
each sampled chord is at most `3 sqrt 3` longer than the exact one. Proven for
`CubicBezier3`:

```text
total()  <=  A + 3 sqrt(3) K + 1/2            (about A + 2661 raw units at K = 512)
```

That bound allows the table to exceed `A`; the issue's "never exceeds the true arc length"
holds for the exact inscribed polyline, not for the rounded table. In every measured case
below, the table was under `A`.

For a straight curve whose ends are exact, the triangle inequality gives
`total() >= |p3 - p0| - 1/2 - K / 2^16`; the upper side is measured, not proven.

Measured against the arc length of a Python `decimal` composite Simpson integration over
100 000 steps (the reference, posted in the pull request; its step-halving change is below
`3e-9` raw units), `A - total()` in raw units:

| Curve | `m = 8` | `m = 64` | `m = 512` |
| --- | --- | --- | --- |
| Quarter-circle-like arc, `A` = 6 747 465 339 | 10 851 892 | 169 586 | 2 649 |
| S curve rising in z, `A` = 17 070 500 551 | 194 220 052 | 3 011 720 | 47 057 |
| Planar loop crossing itself, `A` = 31 749 351 003 | 338 169 008 | 5 298 611 | 82 794 |

The error falls by a factor near 64 for each factor of 8 in `m`, as the square of the step
predicts. These figures cover these three curves only; they are not a bound for others.

### Distance to Parameter

`param_at_distance(s)` returns `None` for `s` below zero or above `total()`. Otherwise:

- `s = total()` returns exactly `end()`, which is `span_count`;
- the bracket is the first `j` with `L_j >= s`, by binary search; when `L_j = s` the result is
  exactly `u_j`, so `s = 0` gives exactly 0, and a distance on a run of equal entries (where
  the curve did not move between samples) gives the first parameter of the run;
- otherwise `L_{j-1} < s < L_j`, and the result is
  `u_{j-1} + round_half_even((s - L_{j-1}) (u_j - u_{j-1}) / (L_j - L_{j-1}))`, the numerator
  below `2^95`, exact in `u128`, rounded once. Composing `from_ratio` and `lerp` would round
  twice, and `from_ratio` takes only `u32` operands.

The result is within 1/2 raw unit of the exact linear interpolation of the table, lies in
`[u_{j-1}, u_j]`, and never decreases as `s` grows: the bracket index never decreases and,
within one bracket, a rounded non-decreasing function stays non-decreasing.

How far the result is from the parameter at true arc length `s` is measured, not proven.
At `s = k total() / 16` for `k` in `0..=16`, the largest distance in raw parameter units from
the parameter at which a 100 000-step reference polyline reaches `s`, and the spread
(longest minus shortest) of the 16 chords between the resulting points:

| Curve | `m = 8` | `m = 64` | `m = 512` |
| --- | --- | --- | --- |
| Arc, parameter error | 6 551 585 | 102 354 | 1 599 |
| Arc, chord spread | 7 843 847 | 53 183 | 7 728 |
| S curve, parameter error | 44 872 345 | 736 481 | 10 943 |
| S curve, chord spread | 285 549 379 | 17 658 873 | 14 073 735 |
| Loop, parameter error | 56 322 716 | 898 567 | 13 799 |
| Loop, chord spread | 579 021 637 | 101 158 459 | 97 916 333 |

The chord spread does not fall to zero: equal arc lengths have unequal chords where the
curvature varies, which is what dominates the S curve and the loop at `m = 512`.

### Refusals

`build` returns `Err(ArcLengthError)`, checked in this order:

| Variant | Threshold |
| --- | --- |
| `SegmentsOutOfRange` | `m` is 0 or above 4096 |
| `SpanCountOutOfRange` | `span_count()` is 0 or above `2^31 - 1` |
| `TooManySamples` | `span_count * m` is above `u32::MAX` |
| `StorageTooSmall { required }` | the slice is shorter than `K + 1` |
| `CurveRefused { sample }` | `at(u_k)` returned `None` |
| `ChordOverflow { chord }` | a chord component is outside `i64`: a difference of `i64::MAX` raw is accepted, `2^63` refused |
| `LengthOverflow { entry }` | an entry rounds above `i64::MAX` raw: a total of exactly `i64::MAX` is accepted |
| `ZeroLength` | the total is zero, as for a curve with four equal control points |

The last four are found while sampling, at the first failing sample, chord or entry.
