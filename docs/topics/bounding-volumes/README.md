# Bounding Volumes

`FixedAabb3` in `bunny-geom` carries the box helpers callers need to build and combine bounds:
from points, a triangle or a sphere, merged, expanded, and converted to a bounding sphere. The
contract is named `bunny-bounds/1` (`BOUNDS_CONTRACT_ID` in `bunny-geom`). `mesh_bounds` in
`bunny-mesh` bounds a mesh's triangles. Every refusal is a typed error: `BoundsError` in
`bunny-geom` and `MeshBoundsError` in `bunny-mesh`. Nothing wraps, saturates or panics.

The helpers return the existing `FixedAabb3` and `FixedSphere3`; there is no new box type. The
test plan is [`test-plan.md`](test-plan.md).

## Exactness

Every coordinate is a raw `i64` (Q32.32). Minima, maxima and comparisons are exact. Margins
and radii are added in `i128`, where no sum of two raw values can overflow, and a result outside
the `i64` raw range is refused with `BoundsError::OutOfRange`; it is never clamped, as the
[numeric constitution](../../NUMERIC_CONSTITUTION.md#overflow-policy) asks of geometry whose
overflow would invalidate the result. The only rounding is the bounding sphere's
([below](#box-to-sphere)).

## Inverted Boxes

A box whose minimum is above its maximum on some axis is **inverted**. `FixedAabb3::new`
accepts one, as it always has; `FixedAabb3::try_new` refuses one with
`GeomError::InvalidAabbBounds`. The helpers follow `try_new`: every helper that takes a box
refuses an inverted one with `BoundsError::InvertedBox`, before anything else, and every box a
helper returns is ordered. A flat box (minimum equal to maximum on an axis) is ordered and
accepted. `TraceBox::new` refuses inverted boxes the same way
([ray tracing](../ray-tracing/README.md)).

## Points And Triangles

`FixedAabb3::from_points(&[FixedVec3])` is the smallest box containing the points: each axis's
exact minimum and maximum. One point gives the degenerate box at that point. An empty slice is
`Err(BoundsError::NoPoints)`: an empty set has no bounds, and no box of zeros or inverted
sentinel stands in for it. The slice is read once and nothing allocates.

`FixedAabb3::from_triangle([v0, v1, v2])` is the same for three vertices. It accepts every
input, collinear and coincident vertices included, and cannot fail.

## Merge

`a.merge(&b)` is the smallest box containing both: each axis's minimum of the minima and
maximum of the maxima, exact. It is symmetric, and `a.merge(&a)` is `a`. Either box inverted is
`InvertedBox`.

## Expand

`b.expand(margin)` moves every face out by `margin`; `b.expand_axes(margin)` moves each axis's
two faces by that axis's component. The result is `min - m` and `max + m` on each axis, exact.
A zero margin returns the box unchanged. A negative margin shrinks the box, down to a flat box
but not past it. The checks run in this order:

1. the input is inverted: `InvertedBox` (even when the margin would order it);
2. a result minimum is above its maximum: `ShrinkInvertsBox`;
3. a result coordinate is outside `i64`: `OutOfRange`.

An axis that shrinks without inverting stays inside the input, so only a growing axis can leave
the range. For example, `[0, MAX - 1]` grows by 1 raw to `[-1, MAX]` and by 2 raw is
`OutOfRange`; `[0, 4]` shrinks by 2 raw to `[2, 2]` and by 3 raw is `ShrinkInvertsBox`;
`[MAX, MAX]` shrunk by 1 raw is `ShrinkInvertsBox`, not `OutOfRange`.

## Sphere To Box

`FixedAabb3::from_sphere(&sphere)` is `centre - r` to `centre + r` on each axis, exact. A
radius of zero gives the degenerate box at the centre. A negative radius, which
`FixedSphere3::new` accepts and `FixedSphere3::try_new` refuses, is `NegativeRadius`; a
coordinate outside `i64` is `OutOfRange` (a centre of `MAX - r` reaches `MAX`, one raw unit
more is refused).

## Box To Sphere

`b.bounding_sphere()` returns a `FixedSphere3` that contains every point of the box.

- **Centre.** Each axis's midpoint `(min + max) / 2`, rounded once to nearest, ties to even,
  so within 1/2 raw of the true midpoint. The sum is formed in `i128`; the midpoint lies in
  `[min, max]`, so it fits.
- **Radius.** The smallest raw value `r` whose square is at least the exact squared distance
  from that centre to the farthest corner. Per axis the farthest face is `max(c - min, max -
  c)`, at most `2^63` raw, so the squared distance is below `3 * 2^126` and is exact in `u128`.
  `r` is the floor root from `FixedQ32_32::sqrt_u128`, plus one unless that root is exact: the
  ceiling of the exact root. It is **rounded outward, never to nearest**, so every corner is
  inside: `r^2` reaches each of the eight corners exactly, and `r - 1` would miss the
  farthest one. A radius rounded to nearest could leave a corner out: the box `[0, 2] x [0, 2] x
  [0, 0]` raw has its corners `sqrt(2)` raw from its centre, which rounds to 1 but needs 2.
- **Refusals.** An inverted box is `InvertedBox`. A radius above `i64::MAX` raw is
  `OutOfRange`: on one axis, `[MIN + 1, MAX]` has radius exactly `i64::MAX` and `[MIN, MAX]`
  (radius `2^63`) is refused; a squared distance of `MAX^2 + 1` is refused.

The sphere is the smallest with a whole raw radius around the rounded centre, not always the
smallest sphere containing the box. By the triangle inequality, moving the centre at most 1/2
raw per axis moves each corner distance by at most `sqrt(3) / 2` raw, and the ceiling adds less
than one, so `r` is below the true half-diagonal plus `1 + sqrt(3) / 2` raw. That bound is an
argument, not a tested property; the tests check containment and minimality around the rounded
centre exactly.

## Mesh Bounds

`bunny_mesh::mesh_bounds(&[FixedVec3], IndexBufferLayout)` is the smallest box containing
every triangle: each axis's exact minimum and maximum over the vertices the triangles use, in
either index width. A vertex that no triangle uses is **not** covered: the bounds are those of
the surface. `FixedAabb3::from_points(vertices)` bounds the whole vertex buffer instead.

- No triangles is `MeshBoundsError::EmptyMesh`, whatever the vertex buffer holds.
- A triangle with an index at or past `vertices.len()` is `MeshBoundsError::IndexOutOfRange {
  triangle }`, naming the first such triangle in buffer order; this is the condition
  `IndexBufferLayout::is_valid` checks.

It lives in `bunny-mesh` because that crate has both the index layouts and `FixedAabb3` in
reach; `bunny-geom` does not depend on `bunny-mesh`, and no dependency is added.

## Existing Helpers

Three helpers older than this contract are pinned by dedicated tests as they behave today;
none is changed.

- `bunny_broadphase::utils::union_aabb(a, b)` takes each axis's minimum of the minima and
  maximum of the maxima and **does not check order**. On ordered boxes it equals `merge`. On an
  inverted input its result is neither the other box nor the hull of both boxes' corners:
  `[0, 2]` with the inverted `[-10, -20]` on one axis gives `[-10, 2]`, and two inverted boxes
  `[2, 0]` and `[-5, -7]` give the ordered `[-5, 0]`. `merge` refuses both.
- `TraceSphere::bounds` (`bunny-query`) is `centre - r` to `centre + r`, and equals
  `FixedAabb3::from_sphere` of the same sphere; in the trace domain it cannot overflow.
- `TraceTriangle::bounds` (`bunny-query`) is each axis's minimum and maximum of the three
  vertices, and equals `FixedAabb3::from_triangle`.
