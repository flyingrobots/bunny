# Plane Classification, Segment And Polygon Clipping

This document defines Bunny's fixed-point plane, shape classification, segment
clipping and convex polygon clipping contract.

The surface lives in `bunny-geom`: `FixedPlane3`, `PlaneSide`, `PlaneSpan`,
`FixedSegment3`, `SegmentClip`, `PolygonClip` and `PolygonClipError`. Clipping a
concave polygon is out of scope.

## Status

This is a living topic chapter, not a proposal. The verification design is
[`test-plan.md`](test-plan.md).

## Plane

`FixedPlane3 { normal, offset }` is the set `normal . p = offset`. The signed
distance of `p` is `normal . p - offset`. The normal need not be unit length;
distances scale by its length. A zero normal is accepted and gives every point
the distance `-offset`.

`signed_distance` multiplies each coordinate with `checked_mul`, adds with
`checked_add` and subtracts the offset with `checked_sub`. It returns `None` on
overflow.

## Classification

`classify` returns `Some(PlaneSide::Front)` for positive signed distance,
`Behind` for negative and `On` for exactly zero (raw value 0), or `None` on
overflow. Classification is exact in the rounded distance: two points with the
same computed distance get the same side.

## On-Plane Rule

Clipping keeps the closed front half-space. A point whose signed distance is
exactly zero belongs to the front: `On` and `Front` are both kept, only
`Behind` is clipped. Classification and clipping share one predicate, so they
cannot disagree.

## Clipping

`clip_segment(FixedSegment3) -> Option<SegmentClip>`:

| Endpoint sides | Result |
| --- | --- |
| both kept (Front or On, including a coplanar segment) | `Kept` with the segment unchanged |
| both Behind | `Outside` |
| one kept, one Behind | `Kept` with the front part, original endpoint order preserved |

A segment touching the plane at one end and otherwise behind returns a
degenerate segment whose two endpoints are the touching point. `None` means
overflow, never "outside".

## Rounding Of The Clipped Endpoint

For a crossing, with signed distances `da` and `db`:

1. `t = da / (da - db)` is one `checked_div`, rounded ties-to-even in Q32.32.
2. Each coordinate is `a + (b - a) * t`, where `(b - a) * t` is a `checked_mul`
   rounded ties-to-even.

The kept original endpoint is returned bit for bit. When `da` is zero or `db`
is zero the intersection is that exact endpoint, but it is still computed by the
interpolation, so the call returns `None` when `b - a` overflows on some axis:
for the plane `z = 0`, the segment from `(i64::MAX, 0, 0)` (on the plane) to
`(-2, 0, -2^32)` in raw units returns `None`, where `-1` instead of `-2` returns
the degenerate segment at the touching point.

Bound: per coordinate the result differs from the exact intersection by at
most `|b - a| / 2^33 + 1` raw units (the half unit in `t` times the span, plus
the half unit of the product), plus the half unit of rounding when the exact
value is itself rounded. The sweep test checks `|b - a| / 2^33 + 2`.

Consequence: the clipped endpoint can lie behind the plane by rounding error.
For the plane `z = 1` and `z` running 0 to 3, `t` is raw 1431655765 and the
endpoint is raw 4294967295, one raw unit behind the plane. Clipping does not
repair it.

## Overflow

Any overflow in a distance, in `da - db`, in `t`, or in the interpolation
returns `None`. Nothing saturates and nothing panics.

## Polygon Clipping

`clip_polygon(&[FixedVec3], &mut [FixedVec3]) -> Result<PolygonClip, PolygonClipError>`
clips a convex polygon to the closed front half-space with Sutherland-Hodgman
against one plane. Bunny's core crates do not allocate: the result is written
to the start of a caller-provided buffer and `PolygonClip::Kept(len)` reports
how many slots hold it. Slots from `len` on are left unchanged on success.

### Vertex order

The winding is preserved. Output starts at the first input vertex that is kept
(Front or On) and then follows input order. Where an edge leaves the front, its
cut point follows the kept vertex; where it enters the front, its cut point
precedes the kept vertex. If every vertex is kept the input is returned
unchanged (it starts at vertex 0).

### Vertices on the plane

A vertex exactly on the plane is kept, as for segments. An edge with an
on-plane end gets no cut, since the cut would repeat that vertex. Input
vertices are never merged, so repeated input vertices stay repeated. Rounding
of a cut can in rare cases make it equal a neighbor; it is not removed.

### Degenerate results

| Case | Result |
| --- | --- |
| fewer than three input vertices | `Err(TooFewVertices)` |
| no vertex kept | `Ok(Outside)` |
| fewer than three vertices remain (a corner or an edge touches the plane) | `Ok(Outside)` |
| every vertex on the plane, or in front | `Kept` with the input unchanged |

`Outside` therefore means that no polygon of three or more vertices survives.

### Buffer size

A convex polygon with `n` vertices yields at most `n + 1` vertices (one plane
crosses its boundary twice), so a buffer of `n + 1` always suffices. When the
result does not fit, the call returns `Err(OutputTooSmall)` and the buffer
contents are unspecified. When the polygon is `Outside` nothing is written and
an empty buffer is accepted. Concave input is not rejected; its result is
unspecified but memory-safe and may report `OutputTooSmall`.

### Cut points and shared edges

`crossing` is not symmetric in its endpoints: `a + (b - a) * t` with
`t = da / (da - db)` and `b + (a - b) * t'` can round differently, and
`clip_segment(a, b)` and `clip_segment(b, a)` can return different cut points.
The difference needs a rounding tie: for the plane `z = 0`, `a = (0, 0, -1)`
and `b = (1, 0, 1)` in raw units give `t = 1/2` and an x offset of half a raw
unit, so ties-to-even returns x = 0 from `a` and x = 1 from `b`.
Two adjacent polygons traverse a shared edge in opposite directions, so a cut
computed in traversal direction would open a crack between them. The polygon
path therefore computes every cut in the canonical direction of the edge:
endpoints ordered ascending by raw `(x, y, z)` tuple. The cut is then
bit-identical to `clip_segment` applied to the canonically ordered segment, and
the same for both polygons whichever way they traverse the edge. It is the
`clip_segment` result for `(a, b)` only when `a` is the smaller endpoint.
`clip_segment` itself is unchanged.

The per-coordinate bound of the rounding section applies to every cut, as does
the caveat that a cut can lie slightly behind the plane.

### Overflow

Overflow in any distance, `t` or interpolation returns `Err(Overflow)`;
nothing saturates or panics. Errors are reported in the order the walk meets
them.

## Shape Classification

`classify_segment(FixedSegment3)`, `classify_triangle([FixedVec3; 3])` and
`classify_aabb(FixedAabb3)` return `Option<PlaneSpan>`. Each is defined by the
point rule, `classify`, at the shape's vertices: the two endpoints, the three
vertices, or the eight corners of the box. There is no second on-plane rule.

| Vertex sides | `PlaneSpan` |
| --- | --- |
| every vertex `On` | `Coplanar` |
| every vertex `Front` or `On`, at least one `Front` | `Front` |
| every vertex `Behind` or `On`, at least one `Behind` | `Behind` |
| at least one `Front` and one `Behind` | `Straddling` |
| any vertex's signed distance overflows | `None` |

A shape that touches the plane without crossing it, at a vertex, an edge or a
face, takes the side of its other vertices. This follows the closed front
half-space of clipping: a `Front` shape is kept whole, and a `Behind` shape keeps
at most the touching part, which `clip_polygon` reports as `Outside` and
`clip_segment` as the degenerate segment at the touching point. `PlaneSpan` is a
new type: `PlaneSide` keeps its three point outcomes.

Classification is exact in the rounded signed distance, as for points. It is not
the classification of the real-valued shape against the real-valued plane: a
vertex whose exact distance is a fraction of a raw unit can round to `On`.

### Segments and triangles

Endpoints and vertices are read in order, and the first overflow returns
`None`. A degenerate triangle (collinear or coincident vertices) or a
zero-length segment is classified like any other, not refused; with all
vertices at one point the answer is the point rule's, with `On` as `Coplanar`.

### Boxes

`classify_aabb` returns exactly what the eight corners give, `None` included,
from two corners. Per axis it takes the end with the smallest rounded product
with the normal component and the end with the largest: for a component of zero
or more, the smaller and the larger coordinate; for a negative component, the
other way round. A zero component gives a zero product at both ends.

Why two corners are enough. The signed distance is
`((x + y) + z) - offset`, where `x`, `y` and `z` are per-axis products rounded to
nearest, ties to even, by `checked_mul`. That rounding is monotone in the
coordinate, so each corner's product on each axis lies between the two chosen
ones. Each partial sum and the distance are therefore between those of the two
chosen corners, and any corner whose partial sum leaves the range has a chosen
corner that leaves it on the same side. The least corner's distance is the
minimum over the eight corners and the other's is the maximum, so the sides of
the two corners decide the table above.

Every representable point inside the box, not only its corners, lies between the
same two distances, so a `Front` box has no representable point `Behind`, and
the reverse.

### Inverted and zero-extent boxes

`classify_aabb` accepts any `FixedAabb3`, as `box_entry` does in
[`../ray-tracing/README.md`](../ray-tracing/README.md). An axis with
`min > max` is read as the same two coordinates in order, so an inverted box is
classified as the box with the same eight corners: on the plane `z = 1`, a box
with z running from 2 to 0 is `Straddling`, like 0 to 2. `FixedAabb3::try_new`
remains the way to refuse inverted bounds. A box with `min == max` is one point
and gets the point rule's answer.

### Zero-normal plane

Every product is exactly zero, so every point has the distance `0 - offset`. A
segment, triangle or box is `Front` for a negative offset, `Behind` for a
positive one and `Coplanar` for zero, however large its coordinates, and
`Straddling` never occurs. The one overflow is the offset raw `i64::MIN`, whose
negation is not representable: every shape returns `None`.

### Agreement with clipping

For a triangle, `Front` and `Coplanar` mean `clip_polygon` returns it unchanged,
`Behind` means `Outside`, `Straddling` means a kept polygon of three or four
vertices (or an overflow in a cut), and `None` means `Err(Overflow)`. For a
segment, `Front` and `Coplanar` mean `clip_segment` returns it unchanged and
`Behind` means `Outside`, or the touching point as a degenerate segment, subject
to the interpolation overflow noted under the rounding section.

### Intermediate widths

Nothing here is wider than `classify`. Each product is formed exactly in `i128`
(at most `2^126` in magnitude for `i64::MIN * i64::MIN`), rounded by a 32-bit
shift to at most about `2^94`, and refused when it does not fit `i64`. Each
partial sum is two `i64` values added in `i128` (below `2^64`) and refused
outside `i64`. The exact dot product, up to about `3 * 2^126`, is never held:
`classify` rounds per product and refuses out-of-range partial sums, and it
never wraps. Its domain is therefore order dependent: the point `(i64::MAX, 1,
0)` raw under the normal `(1, 1, 0)` returns `None` because `x + y` overflows,
even where a negative `z` term would bring the sum back. The shape
classifications inherit this rule unchanged.

## Required Tests

The repository keeps tests for classification, exact crossings in both
directions, fully front and fully behind segments, touching ends, coplanar
segments, golden raw outputs, tie rounding, overflow, and a seeded sweep against
an exact `i128` reference. Polygon clipping adds exact squares and triangles,
on-plane vertices and edges, degenerate and too-small cases, shared-edge
symmetry, golden raw outputs, overflow, and a seeded sweep of convex polygons
against an independent reference. Shape classification adds constructed
crossing, coplanar, touching and outside cases one raw unit either side of the
plane, degenerate triangles, inverted and zero-extent boxes, the zero normal,
overflow, and seeded sweeps against the eight-corner definition and an
independent `i128` model. They run natively and under WASM through
`wasm_bindgen_test`.
