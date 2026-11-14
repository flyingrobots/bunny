# Plane Classification, Segment And Polygon Clipping

This document defines Bunny's fixed-point plane, segment clipping and convex
polygon clipping contract.

The surface lives in `bunny-geom`: `FixedPlane3`, `PlaneSide`, `FixedSegment3`,
`SegmentClip`, `PolygonClip` and `PolygonClipError`. Clipping a concave polygon
is out of scope.

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
is zero the intersection is that exact endpoint.

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
`t = da / (da - db)` rounds differently from `b + (a - b) * t'`, and
`clip_segment(a, b)` and `clip_segment(b, a)` can return different cut points.
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

## Required Tests

The repository keeps tests for classification, exact crossings in both
directions, fully front and fully behind segments, touching ends, coplanar
segments, golden raw outputs, tie rounding, overflow, and a seeded sweep against
an exact `i128` reference. Polygon clipping adds exact squares and triangles,
on-plane vertices and edges, degenerate and too-small cases, shared-edge
symmetry, golden raw outputs, overflow, and a seeded sweep of convex polygons
against an independent reference. They run natively and under WASM through
`wasm_bindgen_test`.
