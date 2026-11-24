# 2D Primitives And Queries (`bunny-planar/1`)

This chapter states Bunny's 2D primitives in `bunny-geom` and their queries: the segment,
ray, axis-aligned box, circle and polygon types, exact containment, overlap,
segment-intersection and closest-point tests, ray queries against a box, a circle and a
segment, and point in polygon. The identity string is `bunny-planar/1`
(`bunny_geom::PLANAR_CONTRACT_ID`). The [test plan](test-plan.md) holds the oracles, the
constructed cases, the seeded sweeps and the mutation checks. It answers the first two
slices of flyingrobots/bunny#149; segment-to-segment closest points, intersection points,
closest points on a circle and float constructors are not part of the contract yet.

A Q32.32 point is an integer on the raw grid, so every yes-or-no answer here is decided
exactly, with no rounding, for every representable input, `i64::MIN` and `i64::MAX`
coordinates included. Two kinds of result are rounded: the closest point on a segment, each
coordinate rounded once, and the first hit parameter of a ray, rounded once. No hit decision
reads a rounded value.

| Query | Exact rule, on raw values | Rounded? | Degenerate input |
| --- | --- | --- | --- |
| `FixedAabb2::contains_point(p)` | `min <= p <= max` on both axes | never | inverted box: `false` |
| `FixedAabb2::overlaps(other)` | intervals intersect on both axes | never | either box inverted: `false` |
| `FixedAabb2::closest_point(p)` | `p` clamped to `[min, max]` per axis | never | inverted box: `None` |
| `FixedCircle2::contains_point(p)` | `\|p - c\|^2 <= r^2` | never | negative radius: `false` |
| `FixedCircle2::overlaps_circle(other)` | `\|c1 - c2\|^2 <= (r1 + r2)^2` | never | either radius negative: `false` |
| `FixedCircle2::overlaps_aabb(aabb)` | the box's closest point to `c` is in the disc | never | negative radius or inverted box: `false` |
| `FixedSegment2::contains_point(p)` | `orientation2(a, b, p)` is `Collinear` and `p` is in the endpoints' box | never | zero length: `p == a` |
| `FixedSegment2::intersects(other)` | a proper crossing, or an endpoint of one on the other | never | zero length: a point |
| `FixedSegment2::closest_point(p)` | the exact nearest point, rounded once per axis | once, ties to even | zero length: `a` |
| `FixedRay2::intersects_aabb(aabb)` | exact slab intervals meet at some `t >= 0` | never | inverted box: `false` |
| `FixedRay2::intersects_circle(c)` | origin in the disc, or `w . d > 0` and `\|d\|^2 r^2 >= (w x d)^2` | never | negative radius: `false` |
| `FixedRay2::intersects_segment(s)` | Cramer's rule, or the collinear case | never | zero length: a point |
| `FixedRay2::hit_aabb`, `hit_circle`, `hit_segment` | the least exact `t >= 0`, as a `RayHit2` | once, ties to even | as the `intersects_` form |
| `FixedPolygon2::classify_point(p)` | on an edge, else the winding number's sign | never | see [below](#degenerate-and-self-intersecting-polygons) |
| `FixedPolygon2::contains_point(p)` | `classify_point(p)` is not `Outside` | never | as `classify_point` |

All of it is `no_std` with no allocation; a polygon borrows its vertex slice. It depends on
`FixedVec2`, `FixedUnitVec2`, the
exact `orientation2` of the [predicates](../predicates/README.md#2d-orientation), and the
256-bit integer the predicates use (`crates/bunny-geom/src/wide.rs`).

## Boundary Rule

Every shape is closed: a box contains its edges and corners, a disc its circle, a segment
both of its ends, a polygon its edges and vertices, and a ray its origin. Two shapes that
touch at a single point overlap, and a ray that grazes a shape hits it. This is the rule of the 3D
box overlap test (`aabbs_overlap` in `bunny-broadphase`) and of `contains_closed` in the
predicates. No query has an open or half-open form.

## Types And Constructors

Each type keeps its fields public and has a `new` that holds its input unchanged, and a
`try_new` that refuses what the type does not represent, as the 3D types do. The refusals are
the existing `GeomError` variants of the 3D constructors; no variant is added.

| Type | Value | `try_new` refuses | Error |
| --- | --- | --- | --- |
| `FixedSegment2 { start, end }` | every point between the endpoints, both included | nothing (only `new`) | none |
| `FixedRay2 { origin, direction }` | origin and a `FixedUnitVec2` | a direction `FixedUnitVec2::new` refuses (zero, overflowing, or not unit after normalization) | `GeomError::InvalidRayDirection`, as `FixedRay3::try_new` |
| `FixedAabb2 { min, max }` | `min.x <= x <= max.x`, `min.y <= y <= max.y` | `min.x > max.x` or `min.y > max.y`, raw comparison | `GeomError::InvalidAabbBounds`, as `FixedAabb3::try_new` |
| `FixedCircle2 { center, radius }` | the closed disc | a radius below raw zero | `GeomError::NegativeSphereRadius`, as `FixedSphere3::try_new` |
| `FixedPolygon2<'a> { vertices }` | the closed region bounded by the edges `v[i]`-`v[(i + 1) mod n]` | nothing (only `new`, which borrows any slice) | none |

Accepted degenerate values: a zero-length segment is a point; a flat box (`min == max` on
one axis) is a segment and a point box a point; a zero radius is the centre alone.

Values `new` accepts and `try_new` refuses are the empty set to every query: an inverted box
(`is_ordered()` false) contains no point, overlaps no box (itself included) and has no
closest point, and a circle with a negative radius contains and overlaps nothing. A query
never reads an inverted box's bounds as if they were swapped.

A segment's endpoint order is kept and matters to `==`, but no query depends on it.

A polygon's vertex order and starting vertex are kept, but no query depends on them.

## Boxes

`contains_point` and `overlaps` compare raw `i64` values; nothing is computed, so nothing
can overflow. For the box `(0, 0)` to `(10, 10)` raw, `(10, 5)` and `(10, 10)` are
contained and `(11, 5)` is not; the box `(10, 0)` to `(20, 10)` overlaps it along an edge,
`(10, 10)` to `(20, 20)` at a corner, and `(11, 0)` to `(20, 10)` does not.

`closest_point(p)` clamps each coordinate of `p` to `[min, max]`. The result is a box point
with no rounding; it is `p` itself when `p` is in the box, and `None` only for an inverted
box. From `(MIN, MAX)` the nearest point of that box is `(0, 10)`.

## Circles

`contains_point(p)` compares the exact squared distance with the exact squared radius.
Each coordinate difference has magnitude at most `2^64 - 1`, so the squared distance is
below `2^129`, which fits neither `i128` nor `u128`; it is held in 256 bits, as is `r^2`
(below `2^126`). For the radius 5 raw about the origin, `(3, 4)` is on the circle and
contained, `(3, 5)` is outside.

At the ends of the range: with centre `(MIN, 0)` and radius `MAX`, `(-1, 0)` is at distance
`2^63 - 1` and contained and `(0, 0)` is not. A distance squared of `2^128 + 581896769`
(`dx = 2^64 - 1`, `dy = 6074001000`) is outside every circle; a 128-bit sum would wrap it to
`581896769`, inside a circle of radius `2^40` raw.

`overlaps_circle(other)` compares the squared centre distance with the square of the exact
radius sum, at most `2^64 - 2`. Tangent circles overlap: centres `(0, 0)` and `(6, 8)` with
radii 4 and 6 touch, with 4 and 5 do not. Centres `(MIN, 0)` and `(MAX - 1, 0)`, both of
radius `MAX`, touch at distance `2^64 - 2`.

`overlaps_aabb(aabb)` is `contains_point` of `aabb.closest_point(center)`. Both steps are
exact, so the answer is the exact closed overlap of the disc and the box. A circle centred at
`(13, 14)` with radius 5 touches the corner `(10, 10)` of the box above; radius 4 misses.

## Segments

`contains_point(p)` reads the exact sign of `orientation2(start, end, p)`: the point is on
the segment when it is exactly `Collinear` and inside the box spanned by the endpoints.
Exact for every input; a point on the line past an end is not on the segment. On the
segment `(0, 0)` to `(4, 2)` raw, `(2, 1)` and both ends are on it, `(6, 3)` is on the line
past the end, and `(2, 2)` is one raw unit off. On the diagonal `(MIN, MIN)` to `(MAX, MAX)`,
`(-1, -1)` is on it and `(1, 0)` is not.

`intersects(other)` takes the four orientations of each segment's endpoints against the
other's line. The segments cross when both pairs are strictly opposite (one
`CounterClockwise`, one `Clockwise`). Otherwise they meet exactly when an endpoint of one is
`Collinear` with the other and inside its endpoints' box. This covers every touching case:

- an endpoint on the other's interior (a T junction) or a shared endpoint;
- collinear segments overlapping in a range or touching at one point;
- a zero-length segment, which meets a segment it lies on and nothing else.

Parallel segments on different lines never meet. Collinear segments one raw unit apart do
not meet. The diagonals of the raw range cross at `(-1/2, -1/2)`, which is on the segment
from `(MIN, MAX)` to `(0, -1)` and past the end of the one to `(-1, 0)`; the answers are
`true` and `false`. An `f64` orientation answers the second one wrongly (the `f64` mutation
in the [test plan](test-plan.md#mutation-checks)).

## Closest Point On A Segment

`closest_point(p)` returns the point of the segment nearest `p`. With `a = start`,
`b = end`, the exact parameter of the nearest point is `n / d`:

```text
n = (p - a) . (b - a)        d = |b - a|^2
```

- `d = 0` (zero length): the result is `a`.
- `n <= 0`: the result is `a`, exactly.
- `n >= d`: the result is `b`, exactly.
- otherwise each coordinate is the exact value `(a d + (b - a) n) / d`, rounded once:

```text
raw = round_half_even((a d + (b - a) n) / d)
```

The tie goes to the even raw coordinate, not to an even offset from `a`. The segment
`(1, 1)` to `(4, 2)` and `p = (3, 0)` give the exact point `(5/2, 3/2)`, which rounds to
`(2, 2)`; rounding the offset `(3/2, 1/2)` from `a` would give `(3, 1)`.

Bound: each coordinate is within half a raw unit of the exact nearest point (exactly half
only at a tie). The rounded point is in the box of the two endpoints, because the exact
coordinate lies between the endpoint coordinates and rounding to nearest cannot pass an
integer. It need not lie on the segment's line: the exact `(1/2, 1/2)` on the segment
`(0, 0)` to `(1, 1)` rounds to `(0, 0)`, the start, but the exact `(3/2, 1/2)` on
`(0, 0)` to `(3, 1)` rounds to `(2, 0)`, which is off the line.

Widths: `|n|` and `d` are below `2^129` and each numerator below `2^194`, held in 256 bits.
The coordinate always fits `i64`, so the `None` the signature allows is never returned for
representable input. The offset from `a` need not fit: on the segment `(MIN, 0)` to
`(MAX, 0)`, `p = (MAX - 1, 5)` gives `(MAX - 1, 0)`, an offset of `2^64 - 2` raw units.

On the diagonal `(MIN, MIN)` to `(MAX, MAX)`, the foot of `(MAX, MIN + k)` is `(k - 1) / 2`
on both axes: `k = 0` is the tie `-1/2` and gives `(0, 0)`, `k = 2` the tie `1/2` and gives
`(0, 0)`, `k = 4` the tie `3/2` and gives `(2, 2)`.

## Rays

A ray is every point `o + t d` with `t >= 0`, where `d` is the raw value of its
`FixedUnitVec2` direction. A `FixedUnitVec2` is unit only to within one raw unit of length,
and the queries take `d` as given: they do not renormalize it, and every hit is decided
exactly from those raw values, so the ray is exactly the half-line the stored numbers
describe. `t` is measured in direction lengths, which is a distance in units only up to that
tolerance. Each component of `d` is at most `2^32 + 1` raw, and `|d| <= 2^32 + 2` raw.

Each query has two forms. `intersects_aabb`, `intersects_circle` and `intersects_segment`
answer whether the ray and the closed shape share a point. `hit_aabb`, `hit_circle` and
`hit_segment` return a `RayHit2`:

| `RayHit2` | Meaning |
| --- | --- |
| `Miss` | no shared point; exactly when the `intersects_` form is `false` |
| `At(t)` | the least exact parameter `t >= 0` of a shared point, rounded once (see [below](#first-hit-parameter)) |
| `Beyond` | the ray meets the shape, but that parameter rounds above `i64::MAX` raw |

`RayHit2::is_hit` is `true` for `At` and `Beyond`. The parameter is zero when the origin is
in the shape.

### Ray Against A Box

`intersects_aabb` intersects exact slab intervals. On an axis where `d` is nonzero, the
parameters of the two bounds are the fractions `(bound - o) / d`, entry and exit by the sign
of `d`. On an axis where `d` is zero, the origin must be within that axis's bounds. The ray
hits when `max(0, entries) <= min(exits)`, compared by cross-multiplication (products below
`2^97`), with no rounding. An edge or corner the ray grazes is hit, and an origin on or in
the box gives zero. An inverted box is never hit.

The decision differs from comparing rounded parameters. From the origin with
`d = (2^32, 3)` raw, the ray passes `2^-32` raw units below the point `(1431655765, 1)` raw,
since `3 1431655765 = 2^32 - 1`. The box from `(1431655665, 1)` to `(1431655765, 5)`, whose
bottom-right corner that is, is missed, yet its rounded entry and exit are both
`1431655765`, which a rounded slab test reads as a hit. The box one raw unit wider is hit at
`At(1431655765)`.

### Ray Against A Circle

With `w = c - o`, `a = |d|^2` and `b = w . d` on raw values, `intersects_circle` is true when
the origin is in the disc (`FixedCircle2::contains_point`), or when `b > 0` and the
discriminant `a r^2 - (w x d)^2` is at least zero. Every value is exact in 256 bits: `|b|` is
below `2^98` and the discriminant below `2^196`. A tangent ray hits; a circle behind the
origin, or a negative radius, is missed.

### Ray Against A Segment

With `q = start - o` and `s = end - start`:

- `d x s != 0`: the lines cross at `t = (q x s) / (d x s)` along the ray and
  `u = (q x d) / (d x s)` along the segment, and the ray hits when `t >= 0` and
  `0 <= u <= 1`. A segment touching the ray at an end, and an origin on the segment, hit.
- `d x s = 0` and `q x d != 0`: the segment is parallel to the ray and off its line, or a
  single point off it. It is missed.
- `d x s = 0` and `q x d = 0`: the segment is on the ray's line (collinear), or a single
  point on it. Its ends are at `t = (end - o) . d / |d|^2`. It is hit when the farther end
  has `t >= 0`, first at `max(0, t of the nearer end)`.

Every product is below `2^130`. Reversing the segment's endpoints changes nothing.

### First Hit Parameter

`At(t)` holds `round_half_even(2^32 t)` raw, the exact least parameter rounded once to the
nearest raw unit, ties to even: within half a raw unit of the exact value. `Beyond` replaces
it exactly when that rounded value is above `i64::MAX`, about `2^31` direction lengths from
the origin. Parameters are never negative.

- Box and segment: `t` is a fraction, and `2^32 num / den` is rounded by the exact long
  division of the predicates' 256-bit integer (numerators below `2^162`).
- Circle: `t = (b - sqrt(D)) / a` from an origin outside, with `D` the discriminant. With
  `u = 2^32 b` and `V = 2^64 D` (below `2^255`), `s = floor(sqrt(V))` is computed exactly.
  When `V = s^2` the quotient `(u - s) / a` is rounded. Otherwise `sqrt(V)` is irrational and
  lies in `(s, s + 1/2)` or in `(s + 1/2, s + 1)`; `V <= s^2 + s` tells which. Either way
  `2 a 2^32 t` lies strictly between two consecutive integers, no rounding boundary falls
  between it and their midpoint, and that midpoint over `2 a` is rounded instead, which is
  never a tie.

Ties occur. On the x axis the crossing of the segment `(5, -1)` to `(6, 1)` is at `11/2` raw
and gives `At(6)`, and `(4, -1)` to `(5, 1)` gives `At(4)`. A circle root can sit just below
a rounding boundary: with `w_y = 3 10^9`, `k = w_y^2` and `r = k + 1`, the root from the
origin along the x axis to the centre `(k + 2, w_y)` is `3/2` minus about `4.2 10^-20` raw,
which rounds to 1, not to the tie's 2.

## Point In Polygon

`FixedPolygon2::new(vertices)` borrows a slice. Its edges are `v[i]` to `v[(i + 1) mod n]`,
the last closing the loop. Either winding is accepted, and nothing is validated or
allocated.

`classify_point(p)` returns a `PolygonLocation`:

- `Boundary` when `p` is on some edge, by `FixedSegment2::contains_point` (an exact
  `orientation2` sign and the endpoints' box). Every vertex is on the boundary.
- Otherwise the winding number is counted from exact `orientation2` signs. An edge rising
  across `p`'s height, from `y <= p.y` to `y > p.y`, with `p` on its left adds one. An edge
  falling across it, with `p` on its right, subtracts one. The half-open rule counts an edge
  that ends level with `p` once, so a horizontal half-line through a vertex is not counted
  twice.
- `Inside` when the winding number is nonzero, `Outside` when it is zero.

No value is rounded, for every representable input. `contains_point(p)` is
`classify_point(p) != Outside`. In the square `(0, 0)`, `(10, 0)`, `(10, 10)`, `(0, 10)`,
`(10, 5)` is on the boundary and `(11, 5)` outside. In the diamond `(5, 0)`, `(10, 5)`,
`(5, 10)`, `(0, 5)`, the points `(-1, 5)`, `(1, 5)` and `(11, 5)`, level with two vertices,
are outside, inside and outside. In the corner triangle `(MIN, MIN)`, `(MAX, MIN)`,
`(MIN, MAX)`, whose long edge is `x + y = -1`, `(-1, -1)` is inside, `(0, -1)` on it and
`(0, 0)` outside.

### Degenerate And Self-Intersecting Polygons

Every input follows the one rule above:

| Input | Result |
| --- | --- |
| no vertices | `Outside` everywhere: the empty set |
| one vertex | `Boundary` at it, `Outside` elsewhere: a point |
| two vertices | `Boundary` on the segment, `Outside` elsewhere; the two edges cancel |
| repeated consecutive vertices | a zero-length edge, which changes nothing |
| collinear runs | the edges cover the same points; nothing changes |
| zero area (every vertex on one line) | `Boundary` on its edges, `Outside` elsewhere; it has no inside |
| self-intersecting | not detected. `Inside` means a nonzero winding number (the nonzero fill rule) |

For self-intersecting input the result is the nonzero rule, not the even-odd rule, and not a
claim that the polygon is valid. The centre of the pentagram `(0, 30)`, `(18, -24)`,
`(-29, 9)`, `(29, 9)`, `(-18, -24)` has winding number 2 and is `Inside`; the even-odd rule
would put it outside. Both lobes of a bow tie are inside and its crossing point is on the
boundary. A square traced twice is inside with winding number 2.

## Order Independence

No answer depends on argument order. `overlaps`, `overlaps_circle` and `intersects` are
symmetric. Reversing a segment's endpoints changes neither `contains_point` nor
`intersects`, which read only signs and boxes. It does not change `closest_point` either: the
exact nearest point is the same and is rounded the same way. That is why no canonical
endpoint order is needed, unlike the plane clipping edge cut.

A ray query does not depend on the segment's endpoint order. A polygon's classification does
not depend on its starting vertex or its winding: reversing the vertices negates the winding
number, which keeps it nonzero or zero.

## Relation To The 3D Queries

The types parallel `FixedSegment3`, `FixedRay3`, `FixedAabb3` and `FixedSphere3`, with the
same constructors and errors. The queries are a different contract from the 3D ones in
`bunny-query` and `bunny-broadphase`:

- `bunny_query::aabb_intersects_sphere` computes the squared distance with rounded Q32.32
  products. `overlaps_aabb` here is exact on raw values.
- `bunny_query::closest_point_aabb` and `bunny_broadphase::aabbs_overlap` read an inverted
  box's bounds as given. Here an inverted box is empty.
- `bunny_query::closest_points_segments` works in rounded Q32.32 steps. `closest_point` here
  rounds once, from an exact value.
- `bunny_query::ray_intersects_aabb` can decide a hit from rounded slab distances, and
  `ray_intersects_sphere` works in rounded Q32.32 steps. The ray queries here decide every
  hit exactly and round only the returned parameter.
