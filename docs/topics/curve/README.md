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
