# Exact Separating-Axis Tests (`bunny-sat/1`)

This chapter states Bunny's separating-axis tests for convex shapes: two oriented boxes
(`sat_obb_obb`) and a triangle against an axis-aligned box (`sat_triangle_aabb`), both in
`bunny-geom`. Each projects the two shapes onto a fixed, ordered list of candidate axes and
compares the projections exactly. Nothing is rounded, no axis is normalised, and no input
overflows, so the answer is a property of the exact shapes, not of an arithmetic. The identity
string is `bunny-sat/1` (`bunny_geom::SAT_CONTRACT_ID`). The [test plan](test-plan.md) holds
the oracles and mutation checks. It is the first slice of flyingrobots/bunny#147.

| Function | Shapes | Axes | Refused |
| --- | --- | --- | --- |
| `sat_obb_obb(first, second)` | two `FixedObb3` | 15 (`ObbAxis`) | never |
| `sat_triangle_aabb(triangle, aabb)` | a `FixedTriangle3` and a `FixedAabb3` | 13 (`TriangleAabbAxis`) | never |

Both are `no_std` and allocate nothing; neither needs `alloc`.

## Classification

Every shape is a closed set: a box contains its faces, a triangle its edges. Both functions
return a `SatClass<A>`, where `A` names a candidate axis:

| Variant | Meaning |
| --- | --- |
| `Separated { axis }` | The shapes are disjoint. On `axis` their projections are disjoint intervals. |
| `Touching { axis }` | The shapes meet, but only within a plane normal to `axis`: on it their projections share exactly one end point, and an arbitrarily small translation along it separates them. |
| `Overlapping` | On every candidate the projections share more than one point: every small enough translation of either shape keeps them meeting. |

Precisely, with `M = first - second` the Minkowski difference (the closed set of all
differences of a point of each shape), the shapes are `Separated` when the origin is not in
`M`, `Touching` when it is on the boundary of `M`, and `Overlapping` when it is in the interior
of `M`. For two solid shapes, `Overlapping` is the same as "their interiors meet". For flat
shapes it is not: two flat boxes crossing through each other's relative interiors are
`Overlapping` (no small translation pulls them apart), while two flat boxes in one plane are
`Touching`.

`SatClass::intersects()` is "not `Separated`", the closed test of
`bunny_broadphase::aabbs_overlap`, where touching counts as overlap. `SatClass::axis()` is the
reported axis, `None` for `Overlapping`.

## The Gap and the Reporting Rule

On a candidate axis the two shapes project to closed intervals `[p_lo, p_hi]` and
`[q_lo, q_hi]`. The gap is `max(q_lo - p_hi, p_lo - q_hi)`: positive when the intervals are
disjoint, zero when they share exactly one end point, negative otherwise. Only its sign is
needed, and it is read by comparing end points (`q_lo` with `p_hi`, `p_lo` with `q_hi`), so
the difference is never formed.

The candidates are tested in their documented order:

1. the first axis with a positive gap is reported as `Separated`, and no later axis is tested;
2. otherwise, the first axis with a zero gap is reported as `Touching`;
3. otherwise the result is `Overlapping`.

An axis that is exactly the zero vector is skipped: both shapes project to the single point
zero on it, which says nothing, and reporting it would make every pair touch. A nonzero axis
is never skipped. So the reported axis is a deterministic function of the inputs and their
argument order.

## Oriented Boxes

`sat_obb_obb(first, second)` takes each box as the exact set `center + axes * l`,
`abs(l_i) <= half_extents_i` (`FixedObb3`; no corner is rounded). With `a_i` the columns of
the first box's axes and `b_j` those of the second, as raw integer vectors, the candidates
(`ObbAxis`) in order are:

| Position | Axis | Vector |
| --- | --- | --- |
| 0 to 2 | `FaceOfFirst(k)` | `a_(k+1) x a_(k+2)` (indices mod 3): `a_1 x a_2`, `a_2 x a_0`, `a_0 x a_1` |
| 3 to 5 | `FaceOfSecond(k)` | the same for `b` |
| 6 to 14 | `EdgePair { first: i, second: j }` | `a_i x b_j`, row-major: `(0, 0)`, `(0, 1)`, `(0, 2)`, `(1, 0)`, ..., `(2, 2)` |

`FaceOfFirst(k)` is the normal of the pair of faces the box's axis `k` crosses. It is the cross
product of the other two columns, not the column `a_k`: a `FixedObb3`'s columns are orthonormal
only within the rotation contracts' tolerance (length within 11 raw units of one, dot products
within 12 of zero), so `a_k` is not exactly normal to those faces, and testing it would answer
for a slightly different box. The face normals bound the box exactly. An `EdgePair` is zero
exactly when the two columns are parallel (for example both boxes on the identity axes, where
`a_i x b_i = 0`), and is then skipped; a face normal is never zero, since the columns are
nearly orthonormal.

### Projection

On an axis `L`, with every value raw, a box projects on the grid of `2^-64` units to

```text
c . L 2^32  -+  sum_i h_i |a_i . L|
```

with `c` the centre and `h_i` the half extents: the centre in raw units is `c 2^32` on this
grid, and a column times a half extent is already on it. Both boxes use the same `L`, so the
common positive scale `|L|` cancels from every comparison and no normalisation is needed.

### Widths

For every box the constructor accepts, at every centre:

- each column entry has magnitude at most `2^32 + 11`, so each component of an axis (a cross
  product of two columns) is below `2^66`;
- `|c . L| 2^32 < 3 2^63 2^66 2^32 < 2^163`;
- each `|a_i . L| < 3 2^33 2^66 < 2^101`, each `h_i |a_i . L| < 2^63 2^101 = 2^164`, and their
  sum is below `2^166`;
- so both interval ends are below `2^167` in magnitude.

The kernel forms these in a private 256-bit two's complement integer
(`crates/bunny-geom/src/sat/wide.rs`) whose wrapping operations are exact below `2^255`, so no
input overflows and there is no refusal. The workspace has other private wide integers;
flyingrobots/bunny#424 tracks sharing one.

### Argument Order

The class does not depend on the argument order. The reported axis is the first in the order
for the arguments as given, so swapping them can report a different axis that is equally
valid: when the first box's face 0 and the second box's face 0 both separate, `(a, b)` reports
`FaceOfFirst(0)` (a's face) and `(b, a)` reports `FaceOfFirst(0)` (b's face), and
`EdgePair { first: i, second: j }` becomes `EdgePair { first: j, second: i }`.

## Triangles Against Axis-Aligned Boxes

`sat_triangle_aabb(triangle, aabb)` takes the triangle as the closed set of convex
combinations of its vertices, degenerate ones included (a repeated vertex or three collinear
points make a segment or a point), and the box as the convex hull of its eight corners. An
inverted box (`min > max` on an axis) is the box with the same eight corners, the rule of plane
classification (`PlaneSpan`) and frustum culling; a flat box (`min == max` on some axes) is a
rectangle, a segment or a point. With the edges `e_0 = v1 - v0`, `e_1 = v2 - v1`,
`e_2 = v0 - v2` and the unit axes `u_0 = x`, `u_1 = y`, `u_2 = z`, the candidates
(`TriangleAabbAxis`) in order are:

| Position | Axis | Vector | Skipped when |
| --- | --- | --- | --- |
| 0 to 2 | `BoxFace(k)` | `u_k` | never |
| 3 | `TriangleFace` | `(v1 - v0) x (v2 - v0)` | the triangle is degenerate (`FixedTriangle3::is_degenerate`) |
| 4 to 12 | `EdgeCross { edge, axis }` | `e_edge x u_axis`, row-major by edge | the edge is parallel to `u_axis` or has zero length |

The projections are the exact predicates of [`bunny-predicates/1`](../predicates/README.md), so
they are exact for every representable input, `i64::MIN` and `i64::MAX` coordinates included:

- on `BoxFace(k)`, the raw coordinate `k`: the triangle spans its vertices' least and greatest,
  the box `[min_k, max_k]` after reordering;
- on `TriangleFace`, `sextupled_signed_volume3(v0, v1, v2, x)`, which is
  `(x - v0) . ((v1 - v0) x (v2 - v0))`: zero for the whole triangle, and for the box the least
  and greatest value over its eight corners (the value is affine in `x`, so a box's extremes
  are at corners);
- on `EdgeCross { edge, axis }`, `doubled_signed_area2(p, q, x')` in the plane that drops
  coordinate `axis` (the other two coordinates in increasing order), with `p`, `q` the edge's
  projected ends: it is `-(e x u_axis) . (x - p)` up to sign, zero on the edge, so the triangle
  spans from zero to its third vertex's value, and the box spans its four projected corners.

Each value is below `2^194` in magnitude (`2 (2^64 - 1)^3` for a volume) and the comparisons
are of end points, so nothing overflows.

## Why the Axes Are Complete

Two compact convex polytopes are disjoint exactly when the origin is outside their Minkowski
difference `M`, and then a facet plane of `M` separates them. Every facet normal of `M` is the
normal of a face of one shape or the cross product of an edge direction of each; for a box
`M` is a zonotope whose facet normals are the cross products of pairs of its generators (the
columns, or the box axes and triangle edges). The candidate lists contain every such vector,
including those built from an axis with a zero half extent and those that turn out zero. For a
flat or degenerate shape, `M` may have no interior; widening every zero extent by a small
`epsilon` gives a solid `M_epsilon` whose facet normals are still in the lists, and a strict
separation of the origin from `M` survives a small enough widening. So `Separated` is reported
exactly when the shapes are disjoint, `Touching` exactly when the origin is on the boundary of
`M` (a boundary point lies on a facet plane, or, when `M` has no interior, on a plane that
contains `M` and whose normal is again a candidate), and `Overlapping` exactly when it is
interior.
The test plan checks this against a brute-force hull of `M` that knows nothing of axes.

## What This Slice Does Not Do

The issue also asks for an optional minimum translation axis: the axis on which the overlap,
measured in world units, is least. Comparing overlaps across axes of different lengths needs
`overlap_i |L_j|` against `overlap_j |L_i|`, so exact squared comparisons of 512-bit values,
and a stated tie order; it is left for a later slice, and this one does not report it. Other
shape pairs (OBB against triangle, sphere, capsule, convex hulls), contact points and
penetration depth are also out of scope.
