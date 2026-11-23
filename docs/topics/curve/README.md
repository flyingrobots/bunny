# Cubic Bezier Curves (`bunny-curve/1`)

This chapter states how Bunny evaluates, differentiates and splits a cubic Bezier curve in
Q32.32, and how it builds a uniform Catmull-Rom spline from cubic Bezier segments. Every rounding is a [`bunny-lerp/1`](../lerp/README.md) lerp, applied in a stated
order, so the result is fixed bit for bit. The identity string is `bunny-curve/1`
(`bunny_linalg::CURVE_CONTRACT_ID`). The [test plan](test-plan.md) holds the oracles,
golden vectors and mutation checks. It answers flyingrobots/bunny#308 (part 1 of
`bunny-curve/1`, the Bezier curve) and flyingrobots/bunny#309 (part 2, the
[Catmull-Rom spline](#catmull-rom-splines)).

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

## Catmull-Rom Splines

`CatmullRom3` is a uniform Catmull-Rom spline through a list of points, open or closed. It
borrows the caller's `&[FixedVec3]` and allocates nothing. Each segment is a
`CubicBezier3` whose two inner control points are the only new roundings, and evaluation
goes through `CubicBezier3::at` and `CubicBezier3::derivative_at` above.

| Function | Result | Error against the exact spline | Refused |
| --- | --- | --- | --- |
| `CatmullRom3::new(points, closed)` | the spline | none | `CatmullRomError`, see [refusals](#catmull-rom-points-and-refusals) |
| `segment(i)`, `to_beziers()` | the Bezier form of segment `i`, every segment in order | inner control points at most 1/2 raw unit | `None` for `i >= segment_count()` |
| `at(u)` | `CubicBezier3::at` of the segment | at most 15/8 raw units | `u` outside `[0, segment_count]` |
| `derivative_at(u)` | `CubicBezier3::derivative_at` of the segment | at most 9/2 raw units | `u` outside `[0, segment_count]`; the segment's derivative overflows |

Uniform Catmull-Rom is the plain parametrisation: every segment takes one unit of `u`,
however far apart its points are. With unevenly spaced points it overshoots, and it can
form cusps and loops. Centripetal and chordal parametrisations, which avoid that, need
distances and a power function with a stated bound, and are not provided.

## Catmull-Rom Points and Refusals

`new` checks, in this order:

1. Fewer than 2 points (open) or 3 points (closed): `CatmullRomError::TooFewPoints`.
2. Every point the same point: `CatmullRomError::AllPointsEqual`. Repeated points that are
   not all equal are accepted; a segment between two equal points stays on that point.
3. An inner control point of a segment outside the Q32.32 range:
   `CatmullRomError::ControlPointOutOfRange { segment }`, naming the first such segment. The
   check runs over every segment, so a spline that is built can be converted and evaluated
   everywhere.

An open spline of `n` points has `n - 1` segments and a closed one `n`, the last from point
`n - 1` back to point 0. `segment_count()` returns that number.

## Catmull-Rom Segments

Segment `i` runs from `P1 = point i` to `P2 = point i + 1` (point 0 after the last, when
closed), with neighbours `P0 = point i - 1` and `P3 = point i + 2`. A closed spline wraps
the indices modulo `n`. An open spline duplicates its end points as phantom neighbours:
point `-1` is point 0 and point `n` is point `n - 1`. The segment is the cubic Bezier

```text
b0 = P1
b1 = round((6 P1 + P2 - P0) / 6)      the exact value is P1 + (P2 - P0) / 6
b2 = round((6 P2 + P1 - P3) / 6)      the exact value is P2 - (P3 - P1) / 6
b3 = P2
```

per component. Each `round` is one rounding to nearest, ties to even, of the exact rational:
the numerator is at most `8 * 2^63 = 2^66` in magnitude and is held exactly in `i128`, and
the quotient is taken by Euclidean division by 6. So `|b1 - exact| <= 1/2` and
`|b2 - exact| <= 1/2` raw units, and `b0`, `b3` are the points bit for bit. Ties happen
(a numerator `6k + 3`) and go to the even neighbour of the whole control point, not of the
sixth: `P1 + round((P2 - P0) / 6)` would differ when `P1` is odd. A control point outside
`i64` is the refusal above. The tie rule shows at the edges of the range: an exact
`i64::MAX + 1/2` ties to the even `i64::MAX + 1` and is refused, while an exact
`i64::MIN - 1/2` ties to the even `i64::MIN` and is accepted.

`segment(i)` returns this Bezier, and `to_beziers()` returns `segment_count()` of them in
order, borrowing the points.

## Catmull-Rom Parameter

The spline's parameter `u` is a Q32.32 value in `[0, segment_count]`, raw `0` to
`segment_count * 2^32` inclusive. Its integer part (raw `>> 32`) is the segment and its
fraction (the low 32 bits) the local `t`; this split is exact. A whole `u = k` therefore
belongs to segment `k` at `t = 0`, the segment leaving point `k`, except `u = segment_count`,
which is the last segment at `t = 1.0`. Any raw `u` below 0 or above `segment_count * 2^32`
is `None` for `at` and `derivative_at`: the spline is not extrapolated. A spline of `2^31` or
more segments has an end beyond the largest Q32.32 value, and its parameter range is cut off
there.

## Catmull-Rom Evaluation

`at(u)` is `segment(i).at(t)`, bit for bit. Inside the parameter range it is never refused.

**Exact Catmull-Rom cubic.** On segment `i` the exact spline is

```text
C(t) = (2 P1 + (P2 - P0) t + (2 P0 - 5 P1 + 4 P2 - P3) t^2 + (3 P1 - P0 - 3 P2 + P3) t^3) / 2
```

which is the Bezier with the exact inner points `P1 + (P2 - P0) / 6` and `P2 - (P3 - P1) / 6`.

**Bound.** The rounded inner points move the exact Bezier by
`3 t (1 - t)^2 e1 + 3 t^2 (1 - t) e2` with `|e1|, |e2| <= 1/2`, at most
`(1/2) 3 t (1 - t) <= 3/8`, and `CubicBezier3::at` adds at most `3/2` (part 1). So
`|at(u) - C(t)| <= 15/8` raw units per component, inside the issue's budget of 3 raw units
plus the division's rounding. The seeded test reaches errors above `3/2`, so the control
points' share is real.

**Exact regardless of rounding.** `at(k)` is point `k` bit for bit, at every whole `k` (and
`at(segment_count)` is the last point, or point 0 when closed), because `CubicBezier3::at`
returns its end points exactly. So a closed spline has `at(0) == at(segment_count)`, and
the two segments meeting at a point agree on it bit for bit.

**Collinear points.** Points `P_i = P_0 + i d` with every component of `d` a multiple of 24
raw units give exactly `P_0 + u d` at every `u = k/8` on the interior segments: the inner
control points `P1 + d/3` and `P2 - d/3` are exact, and every lerp's step is `k/8` of a
multiple of 8 raw units, with nothing to round. The end segments see a duplicated phantom,
so the first follows `P_0 + d (t + 2 t^2 - t^3) / 2` instead (and the last its mirror):
still on the line, but not linear in `u`.

## Catmull-Rom Derivative

`derivative_at(u)` is `segment(i).derivative_at(t)`, the derivative with respect to `u`
(each segment takes one unit of `u`, so it is also the derivative in `t`). It is `None`
when that derivative is (part 1: a difference of the segment's control points, or the
product by 3, outside Q32.32), and otherwise never refused inside the parameter range.

**Bound.** The rounded inner points change the exact Bezier derivative by
`3 (((1 - t)^2 - 2 t (1 - t)) e1 + (2 t (1 - t) - t^2) e2)`. The two weights' magnitudes sum
to at most 1 on `[0, 1]` (1 at the ends), so that is at most `3/2`, and
`CubicBezier3::derivative_at` adds at most 3. So the result is within `9/2` raw units of the
exact `C'(t)`. The seeded test reaches errors above 3.

**At a point.** At `u = k` the derivative is that of segment `k` at `t = 0`, exactly
`3 (b1 - P1)` (part 1). The exact value is `(P[k + 1] - P[k - 1]) / 2`, with the phantom
neighbours at an open spline's ends, and the rounding of `b1` is at most 1/2, so the result
is within `3/2` raw units of it, and a multiple of 3 raw units.

**Continuity at a joint.** The segment arriving at point `k` ends with
`3 (P[k] - b2)`, and `b2 = round(P[k] - D / 6)` while the leaving segment has
`b1 = round(P[k] + D / 6)`, with the same `D = P[k + 1] - P[k - 1]`. Ties to even are
symmetric about an integer: `round(P + x) + round(P - x) = 2 P` for integer `P`, ties
included, because `P + x` and `P - x` tie together and their even neighbours are mirror
images. So `b1 - P[k] = P[k] - b2` exactly, and where both segments' derivatives are
defined, `segment(k - 1).derivative_at(1.0)` equals `segment(k).derivative_at(0)` bit for
bit: the tangent is continuous exactly. A closed spline's seam is a joint like any other.
Only definedness can differ: a difference elsewhere in the arriving segment can overflow
while the leaving one is defined, and `derivative_at(k)` is then the leaving derivative.

## Catmull-Rom Widths

The inner control points' numerators are at most `2^66` and are held in `i128`; the result
is checked against `i64`. The parameter split is a shift and a mask of the raw value.
Everything else is part 1's `CubicBezier3::at` and `CubicBezier3::derivative_at`.
