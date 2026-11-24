# Closest Point On A Triangle

`bunny_query::closest_point_triangle(v0, v1, v2, p)` returns a point of the triangle
`(v0, v1, v2)` closest to `p`. It has no error type and never refuses an input. This
chapter states what it does for non-degenerate and for degenerate triangles, and the
accuracy of each answer. The other closest-point helpers of `bunny-query`
(`closest_point_aabb`, `aabb_intersects_sphere` and `closest_points_segments`) are not
specified here.

The evidence is [`test-plan.md`](test-plan.md) (CLOSEST-REQ-001 to -004, CLOSEST-TP-001 to
-009).

## Which Triangles Are Degenerate

A triangle is degenerate when the exact cross product `(v1 - v0) x (v2 - v0)` of its raw
vertices is zero: two or three vertices are equal, or all three lie on one line. This is
`FixedTriangle3::is_degenerate` from `bunny-geom`, computed without rounding over the whole
raw range. See the [shape library](../shape-library/README.md). A triangle one raw unit off
a line is not degenerate.

## Non-Degenerate Triangles

A non-degenerate triangle goes through the region tests in a fixed order: the vertex
regions of `v0`, `v1` and `v2`, the edge regions of `v0 v1`, `v0 v2` and `v1 v2`, then the
face. The dot products and ratios use the Q32.32 operators of `bunny-num`. Each product and
quotient is rounded to nearest, ties to even, and each sum saturates. No error bound is
stated for these triangles.

Their results did not change when degenerate triangles were fixed
(flyingrobots/bunny#500). CLOSEST-TP-007 compares 100 000 seeded non-degenerate triangles
bit for bit against a frozen copy of the routine as it was before.

A triangle a raw unit or so off a line is a sliver. The region tests can misroute a sliver
and return a point well away from the true closest one. That behaviour is kept unchanged on
purpose (flyingrobots/bunny#507).

## Degenerate Triangles

A degenerate triangle is the segment between its two farthest vertices, or a point when all
three vertices are equal. The answer is the closest point of that segment:

1. **The axis.** Take the first axis, in the order x, y, z, on which the three raw
   coordinates are not all equal. If there is none, all three vertices are equal and the
   answer is `v0`.
2. **The ends.** The ends of the segment are the vertex with the least coordinate on that
   axis and the vertex with the greatest. On a tie, the lower vertex index wins. The two
   ends are taken in vertex order as `(a, b)`. The vertices lie on one line that changes on
   this axis, so their order along the axis is their order along the line, and these are
   the two farthest vertices.
3. **The point.** The answer is the second point of `closest_points_segments(p, p, a, b)`.
   For a point query this is `a + (b - a) * t`. Here `t` is the ratio of `(b - a) . (p - a)`
   to `(b - a) . (b - a)`, clamped to `[0, 1]`. Each product in the dot products, the ratio
   and each component of `(b - a) * t` is rounded to nearest, ties to even.

Two consequences:

- Three equal vertices give that vertex exactly, for every `p` and over the whole raw range.
- The answer is `a` or `b` exactly when `p` projects onto or beyond that end by more than
  the rounding of the dot products.

For the triangle `(0, 0, 0), (0, 0, 0), (2, 0, 0)` and the point `(1, 1, 0)`, the segment is
`(0, 0, 0)` to `(2, 0, 0)`, `t = 2 / 4`, and the answer is `(1, 0, 0)`.

## Accuracy Of The Degenerate Answer

Let `d = b - a` in raw units. When every coordinate of the vertices and of `p` is below
`2^45` raw in magnitude, no intermediate saturates. Then each component `k` of the answer is
within

```text
1 + min(|d_k|, ceil(|d_k| / 2^33) + ceil(2^35 |d_k| / |d|^2))
```

raw units of the exact closest point. The bound comes from three error sources:

- The two dot products are each off by at most 3/2 raw from three roundings.
- The ratio adds half a raw unit of `t`.
- The final product adds half a raw unit.

The first term covers the case where the computed `t` is anywhere in `[0, 1]`. CLOSEST-TP-005
checks the bound against an exact `i128` oracle for coordinates below `2^40` raw, on 168 000
seeded cases. The largest error observed there is 24 raw units, on segments a few raw
units long.

A segment shorter than about `2^16` raw units is too short for the Q32.32 dot products: its
squared length rounds to a few raw units or to zero. There the bound is the segment's own
extent, and the answer can be anywhere on the segment, give or take one raw unit.

## The Ends Of The Raw Range

Above `2^45` raw the edge `b - a`, the offset `p - a` and the dot products saturate. The
answer is then pinned, not endorsed. For the segment from `i64::MIN` to `i64::MAX` on x, the
point `(MIN / 2, 1, 0)` gives `(-1, 0, 0)` raw, while the exact answer is `(MIN / 2, 0, 0)`
(CLOSEST-TP-008). Non-degenerate triangles saturate the same way at these magnitudes. Whether
a saturating query should refuse, compute exactly or document its range is decision D5 of
flyingrobots/bunny#116, which is open.

## What Is Not Decided Here

The degeneracy policy of flyingrobots/bunny#116 is open. This function answers every
degenerate triangle with its geometric closest point and refuses none, because its signature
has no error type. That answer is correct under every option of that policy that answers a
degenerate triangle at all.
