# AABB-Sphere Overlap

`aabb_intersects_sphere` in `bunny-query` decides whether an axis-aligned box and a sphere
meet, and returns the point of the box closest to the sphere's centre when they do. The
decision is exact for every representable input. The test plan is
[`test-plan.md`](test-plan.md).

## The Rule

Let `p = closest_point_aabb(aabb, sphere.center)`, `c` the centre and `r` the magnitude of
the radius, all raw integers (Q32.32 raw `i64`). The function returns `Some(p)` exactly when

```text
(p.x - c.x)^2 + (p.y - c.y)^2 + (p.z - c.z)^2 <= r^2
```

and `None` otherwise. Touching (the two sides equal) counts as meeting. A centre inside the
box has distance 0 and meets the box for every radius, zero included. A zero radius meets
only a box that contains the centre.

Nothing is rounded and nothing saturates, so there is no error path. The widths at the
extremes:

| Quantity | Largest value | Held in |
| --- | --- | --- |
| A gap `\|p - c\|` on one axis | `2^64 - 1` | `u64` (`i64::abs_diff`) |
| Its square | below `2^128` | `u128` |
| The sum of three squares | below `3 * 2^128` | `u128`; a sum that passes `u128::MAX` is `None` |
| `r` | `2^63` (radius `i64::MIN`) | `u64` |
| `r^2` | `2^126` | `u128` |

A sum of squares that does not fit `u128` is at least `2^128`, above every `r^2`, so `None`
is the exact answer for it.

## Degenerate Inputs

These readings are the ones the function had before flyingrobots/bunny#501; the uniform
degeneracy policy (flyingrobots/bunny#116) is undecided and may change them.

- **Inverted box** (minimum above maximum on an axis): read as given. `closest_point_aabb`
  returns the minimum when the coordinate is below it, else the maximum when the coordinate
  is above it, else the coordinate, so the box is neither swapped nor empty. For the axis
  `[10, 0]`, the coordinate 5 gives 10 and 20 gives 0.
- **Negative radius**: its sign is ignored; the sphere has radius `|r|`.

## What Changed in flyingrobots/bunny#501

Before, the squares were Q32.32 products (`diff.dot(diff)` and `radius * radius`), each rounded
to nearest with ties to even and saturated at `i64::MAX`, with saturating sums. Two kinds of
input now get a different answer; every other input gets the same one.

1. **Saturated radius.** The rounded square of a radius of magnitude `199_032_864_766_431` raw
   (about 46 340.95 units) or more exceeds `i64::MAX` and saturated, and a saturated squared
   distance is never above it, so every box was reported as met. Such a sphere now meets
   only the boxes within its radius. Below that radius the old routine never answered wrongly
   because of saturation: a saturated distance was above the rounded squared radius, which
   is `None`, the exact answer outside the band below.
2. **The rounding band.** Each rounded product is within `2^31` raw units squared of the
   exact square, so the old comparison could order the two sides wrongly only when the exact
   squared distance and squared radius differ by at most `2^33` raw units squared (four
   roundings). For example, a zero radius one raw unit from a point box was met before
   (`0 <= 0`) and is not now (`1 > 0`); three gaps of `1 049 600` raw against radius
   `1 817 961` raw were not met before (`771 > 770`) and are now
   (`3 304 980 480 000 <= 3 304 982 197 521`). At ordinary scales this band is about
   `2^-31` units squared wide.

The returned point is `closest_point_aabb`'s result in both versions.
