# Closest-Point Queries

This chapter states what `bunny_query::closest_points_segments(p1, q1, p2, q2)` returns for
every arrangement of two segments, and how accurate each answer is. The function returns a
pair `(C1, C2)` with `C1` on the segment `p1 q1` and `C2` on `p2 q2`. It has no error type
and never refuses an input. The other closest-point helpers of `bunny-query`
(`closest_point_aabb`, `aabb_intersects_sphere` and `closest_point_triangle`) are not
specified here.

The evidence is [`test-plan.md`](test-plan.md) (CLOSEST-REQ-010 to -014, CLOSEST-TP-020 to
-028).

## Arithmetic

Write `d1 = q1 - p1`, `d2 = q2 - p2` and `r = p1 - p2`, and the dot products `a = d1 . d1`,
`b = d1 . d2`, `e = d2 . d2`, `c = d1 . r` and `f = d2 . r`. All of them, their products and
the parameters `s` and `t` use the Q32.32 operators of `bunny-num`:

- each product and each ratio is rounded to nearest, ties to even;
- each sum and difference saturates;
- a ratio with a zero divisor is `0`;
- `s` and `t` are clamped to `[0, 1]`.

The answer is `p1 + d1 * s` and `p2 + d2 * t`, each component rounded the same way.

## Zero-Length Segments

A segment with `p == q` (raw equality) is a point.

- **First segment a point** (`p1 == q1`): the answer is `p1` and its closest point on the
  second segment, `t = f / e` clamped to `[0, 1]`. This is the point-to-segment path.
- **Second segment a point** (`p2 == q2`): the answer is the mirror of the call with the
  point first. If `closest_points_segments(p2, q2, p1, q1)` is `(p2, C)`, the answer is
  `(C, p2)`.
- **Both points**: the answer is `(p1, p2)` exactly.

So the answer does not depend on which argument holds the point. For the segment
`(0, 0, 0)-(2, 0, 0)` and the point `(1, 1, 0)`, both
`closest_points_segments((0,0,0), (2,0,0), (1,1,0), (1,1,0))` and the swapped call give the
points `(1, 0, 0)` and `(1, 1, 0)`. Before flyingrobots/bunny#506 the first call returned
`(0, 0, 0)` and `(1, 1, 0)`: with `d2 = 0` the parallel branch fixed `s = 0`, and nothing
moved it.

## Parallel And Collinear Segments

Two segments of nonzero length are parallel when the exact cross product of their raw
directions is zero. Collinear segments are included. The test is computed without rounding
over the whole raw range. A pair of parallel segments can have many closest pairs, and the
function returns this one:

1. If `0 <= f <= e`, `p1` projects onto the second segment. The answer is `p1` and that
   projection: `s = 0`, `t = f / e`.
2. If `f < 0`, the answer is `p2` and its projection onto the first segment: `t = 0`,
   `s = -c / a`, clamped.
3. If `f > e`, the answer is `q2` and its projection onto the first segment: `t = 1`,
   `s = (b - c) / a`, clamped.

This is the rule the function always applied when the rounded `a e - b^2` was zero, and
those pairs keep their bits. Parallel pairs whose rounded `a e - b^2` was not zero used to
take the non-parallel formulas below, which then divide rounding noise by rounding noise.
In the seeded sweep (CLOSEST-TP-025) some of those answers were up to 0.8 units from every
closest pair. They now take the same rule.

## Segments Of Nonzero Length

Segments of nonzero length whose directions are not parallel solve the two-line system
with the rounded denominator `a e - b^2`. Then `s` is clamped, and `t` with it; then `t` is
clamped, and `s` is recomputed from it. If the rounded denominator is zero (very short or
very nearly parallel segments), the parallel rule above is used. No error bound is stated
for these segments.

Their results did not change when degenerate and parallel segments were fixed
(flyingrobots/bunny#506). CLOSEST-TP-024 compares 100 000 seeded pairs bit for bit against a
frozen copy of the routine as it was before. The sweep also measures them against the exact
oracle, for coordinates below `2^40` raw:

- crossing segments are within 1 865 raw units of the exact pair;
- skew segments are within 92 raw units;
- one pair with a shared end was 99 716 raw units off. It has a short, nearly parallel
  first segment.

These are unchanged on purpose, as the contract of this fix.

## Accuracy

Let `d` be the raw direction of a segment, in raw units, and

```text
bound_k(d) = 1 + min(|d_k|, ceil(|d_k| / 2^33) + ceil(2^35 |d_k| / |d|^2))
```

When one segment has zero length and every coordinate is below `2^45` raw in magnitude, no
intermediate saturates. Then:

- the point is returned exactly;
- each component `k` of the other answer is within `bound_k(d)` raw units of the exact
  closest point, where `d` is the direction of the segment of nonzero length.

The bound comes from three error sources:

- the dot products `f` and `e` are each off by at most 3/2 raw;
- the ratio `t` adds half a raw unit of `t`;
- the final product adds half a raw unit.

The first term of the minimum covers a computed `t` anywhere in `[0, 1]`. A segment shorter
than about `2^16` raw units is too short for the Q32.32 dot products: its squared length
rounds to a few raw units or to zero. There the bound is the segment's own extent.

For parallel segments below `2^45` raw, each component of each answer is within
`max(bound_k(d1), bound_k(d2))` raw units of some exact closest pair. This bound is checked,
not derived.

CLOSEST-TP-025 checks both bounds against an exact big-integer oracle for coordinates below
`2^40` raw, on 36 000 seeded pairs. The largest error observed on those arrangements is 24
raw units.

## The Ends Of The Raw Range

Above `2^45` raw the differences and the dot products saturate. The answer is then pinned,
not endorsed. For the segment from `i64::MIN` to `i64::MAX` on x and the point `(0, 1, 0)`,
the answer on the segment is `(-1, 0, 0)` raw in either argument order, while the exact
answer is the origin (CLOSEST-TP-027). Whether a saturating query should refuse, compute
exactly or document its range is decision D5 of flyingrobots/bunny#116, which is open.

## What Is Not Decided Here

The degeneracy policy of flyingrobots/bunny#116 is open. This function answers every
degenerate and parallel pair with a geometric closest pair and refuses none, because its
signature has no error type. That answer is correct under every option of that policy that
answers such inputs at all.
