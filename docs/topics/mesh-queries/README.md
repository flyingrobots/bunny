# Mesh Queries

This chapter defines Bunny's contract for closest-point queries against triangles and
triangle meshes, and for the borrowed mesh view that connects a `bunny-mesh` vertex and
index buffer to a BVH and to the ray and closest-point searches:

- the exact closest point of one triangle to a query point, in `bunny-query`;
- the box bound that prunes a closest-point search, in `bunny-query`;
- the nearest-first closest-point BVH traversal, in `bunny-broadphase`, with the argument
  that its answer does not depend on the tree;
- `MeshView` and `MeshBvh`, in `bunny-mesh`.

The ray kernels and the closest-hit traversal that `MeshBvh::closest_hit` runs keep their
own contract in [Ray Tracing](../ray-tracing/README.md); the node format and the builders
are in [BVH Construction](../bvh-construction/README.md).

## Status

This is a living topic chapter, not a proposal. It answers flyingrobots/bunny#246.

- `crates/bunny-query/src/point_query.rs`, with `point_query/feature.rs` (the closest
  feature of a triangle) and `point_query/wide.rs` (256-bit integers), exports
  `PointQuery` (`new`, `point`, `closest_on_triangle`, `box_distance_bound`),
  `TrianglePoint` (`point`, `distance_squared`, `cmp_distance`, `cmp_bound`) and
  `POINT_BOX_GAP_LIMIT_RAW`.
- `crates/bunny-broadphase/src/closest_point.rs` exports `closest_point` and
  `ClosestPoint` (`primitive`, `point`, `distance_squared`).
- `crates/bunny-mesh/src/mesh_bvh.rs` exports `MeshView` (`new`, `positions`, `indices`,
  `triangle_count`, `triangle`, `write_bounds`), `MeshBvh` (`new`, `mesh`, `bvh`,
  `closest_hit`, `closest_point`) and `MeshError`. `bunny-mesh` depends on
  `bunny-query` and `bunny-broadphase` for them.

Evidence is listed in the [test plan](test-plan.md). All arithmetic is integer arithmetic
on raw Q32.32 values. There is no floating point, no `unsafe` code, no allocation and no
target-specific path, so every supported target returns the same bits.

## Domain

The query point and every triangle vertex are inside the trace domain of
[Ray Tracing](../ray-tracing/README.md#trace-domain): every raw coordinate has
`abs(raw) < 2^40` (`TRACE_COORD_LIMIT_RAW`, below 256.0).

- `PointQuery::new` returns `None` for a point with a coordinate outside it; `2^40 - 1`
  and `-(2^40 - 1)` are accepted on every axis, `2^40` and `-2^40` are not.
- Triangles are `TraceTriangle` values, which `TraceTriangle::new` refuses outside the
  domain in the same way.
- `MeshView::new` refuses a vertex buffer with any vertex outside it (see
  [Mesh View](#mesh-view)).

Every squared distance below is in raw units squared unless it is stated as a Q32.32 value.

## Closest Point On A Triangle

`query.closest_on_triangle(&triangle)` returns a `TrianglePoint`: the closest point of the
closed triangle to the query point and its squared distance, both held as exact rationals.

**The point set.** The triangle is two-sided, and its edges and corners belong to it. A
triangle whose vertices are collinear, or whose vertices coincide, is not refused: it is
the segment or the point its vertices span (their convex hull), and its closest point is
that set's.

**The rule.** Let `n = (b - a) x (c - a)`.

1. If `n` is not zero, the weights of the projection of `p` onto the plane of the triangle
   are `n . ((b - p) x (c - p))`, `n . ((c - p) x (a - p))` and `n . ((a - p) x (b - p))`
   over `|n|^2` (each cross product is unchanged by moving `p` along `n`). When all three
   numerators are at least zero, the projection is on the closed triangle and is the
   closest point: `p - n (n . (p - a)) / |n|^2`, at squared distance
   `(n . (p - a))^2 / |n|^2`.
2. Otherwise the closest point is on the boundary, the union of the closed edges `a b`,
   `b c` and `c a`. The closest point of an edge `u w` is `u + (w - u) t / L` with
   `t = (w - u) . (p - u)` clamped to `0..=L`, `L = |w - u|^2`, at squared distance
   `(|p - u|^2 L - t^2) / L` inside the edge; at a clamped end it is the vertex, at its
   integer squared distance. An edge of length zero is its one vertex. The nearest of the
   three is taken, the earlier on an exact tie; tied edges share the same point, because
   the closest point of a convex set is unique.

A triangle with `n = 0` skips step 1, so step 2 gives the closest point of its segment or
point. Every decision above (the signs of the three weights, the clamp of `t`, the choice
between edges) is made on exact integers, so the feature chosen is the exact one.

**What is exact and what is rounded.**

| Value | Exact or rounded |
| --- | --- |
| Which feature of the triangle is nearest, and the closest point as a rational | Exact. |
| `TrianglePoint::cmp_distance(&other)` | Exact: the two rational squared distances are compared by cross multiplication. |
| `TrianglePoint::cmp_bound(bound)` | Exact: the rational squared distance against an integer bound in raw units squared. |
| `TrianglePoint::point()` | Each coordinate of the exact point rounded once to the nearest raw unit, ties to even: within half a raw unit per axis, so within `sqrt(3) / 2` raw of the exact point. |
| `TrianglePoint::distance_squared()` | The squared distance to the exact point (not to the rounded one) as a Q32.32 value, rounded once to the nearest raw unit, ties to even: within half a raw unit, `2^-33` square units, of the exact value. |

Both roundings divide the exact numerator by the exact denominator with restoring long
division and round on the remainder, so a value whose exact numerator and denominator
differ between two representations (the same point reached as a face and as an edge point)
rounds to the same bits. The rounding is symmetric about zero. The squared distance is
below `3 * 2^82` raw units squared, so its Q32.32 value is below `3 * 2^50` raw.

Equal rounded values do not mean an exact tie: two triangles whose squared distances
differ by less than a raw unit can report the same `distance_squared` and still compare
unequal under `cmp_distance`, and searches use the exact comparison.

### Widths

With coordinates below `2^40` in magnitude, every difference of two points is below
`2^41`. Then, per component or in total:

| Value | Bound | Held in |
| --- | --- | --- |
| `n`, and the cross products `(b - p) x (c - p)` and the like | `2 * 2^41 * 2^41 = 2^83` | `i128` |
| Each weight numerator `n . (x x y)` | `3 * 2^166 < 2^168` | 256 bits |
| `|n|^2` | `3 * 2^166 < 2^168` | 256 bits |
| `n . (p - a)` | `3 * 2^83 * 2^41 < 2^126` | `i128` |
| Face squared-distance numerator `(n . (p - a))^2` | `2^252` | 256 bits |
| Face point numerators `p_i |n|^2 - n_i (n . (p - a))` | `2^208 + 2^209 < 2^210` | 256 bits |
| Edge `t` and `L` | `3 * 2^82 < 2^84` | `i128` |
| Edge point numerators `u_i L + e_i t` | `3 * 2^122 + 3 * 2^123 < 2^126` | `i128` |
| Edge squared-distance numerator `|p - u|^2 L - t^2` (it is `|(p - u) x e|^2`, at least zero) | `2^168` | 256 bits |
| Vertex squared distance | `3 * 2^82` | `i128` |
| Cross-multiplied comparison of two squared distances | `2^252 * 2^168 = 2^420` | 512 bits |
| Rounding of a point coordinate: denominator `* 2^41` (the quotient is below `2^40`, since the point lies between vertex coordinates) | `2^209` | 256 bits |
| Rounding of the squared distance: denominator `* 2^32 * 2^52` (the quotient is below `3 * 2^50`) | `2^252` | 256 bits |

The 256-bit values are two's complement and wrap modulo `2^256`; every true value above is
below `2^255` in magnitude, so no wrapped value differs from the true one.

## Box Bound

`query.box_distance_bound(&aabb)` returns a lower bound, in raw units squared, on the
squared distance from the query point to any point of the box. Per axis the gap is
`max(min - p, p - max, 0)`, computed exactly in `i128` from any `FixedAabb3`, capped at
`POINT_BOX_GAP_LIMIT_RAW = 2^42`; the bound is the sum of the three squares, below
`3 * 2^84`.

- Below the cap the bound is exactly the squared distance to the box, so it is never above
  the squared distance to any point in it; a point inside the box or on its boundary gets
  zero, and one raw unit outside a face gets one.
- A capped gap counts as `2^42`, whose square `2^84` is already above every squared
  distance from a trace-domain point to a trace-domain triangle (below `3 * 2^82`), so a
  subtree pruned by a capped bound would be pruned by the exact one too. The cap lets boxes
  outside the trace domain (any `i64` corners) be measured without overflow, and lowers the
  bound, so it stays conservative.
- The corners are not checked for order: an axis whose minimum is above its maximum
  contributes the larger of its two signed gaps. Such a box contains no point, so the
  conservative-bound argument below does not apply to it; trees from the builders never
  contain one.

`TrianglePoint::cmp_bound(bound)` compares an exact squared distance with such a bound;
a bound of `2^127` or more is compared as `2^127 - 1`, which changes no answer.

## Closest-Point Traversal

`closest_point(bvh, query, nearest)` returns the primitive whose closest point to the query
point is nearest, as a `ClosestPoint { primitive, point, distance_squared }` with the
rounded values of that primitive's `TrianglePoint`, or `None` when the closure leaves out
every primitive or the tree is empty. `bvh` is a `BvhView`, as for `closest_hit`.
`nearest(primitive)` returns that primitive's `TrianglePoint`, or `None` to leave it out.

**Tie rule.** Distances are compared exactly. Of several primitives at the same exact
squared distance, the lowest primitive index wins. Ties are common, not a corner case:
every query point nearest to a shared edge or vertex of a mesh is exactly equidistant from
every triangle that shares it. Two tied triangles need not have the same closest point;
the lowest index's point is reported.

### Search Order And Pruning

1. The root is pushed with its box bound.
2. At an interior node both children are pushed with their box bounds, the farther first,
   so the nearer is searched first; on equal bounds the left child is searched first.
3. A node popped from the stack is skipped when the best point so far is strictly nearer
   than its bound (`cmp_bound` is `Less`). A node whose bound equals the best distance is
   searched: it may hold an equally near primitive with a lower index.
4. At a leaf, each primitive is tested in the order the primitive-index buffer lists it,
   and a point replaces the best one when it is strictly nearer, or exactly as near with a
   lower primitive index.

Nothing is allocated; the stack is a fixed array of 64 entries on the call stack.

**Errors.** They are the `TraversalError` values of `closest_hit`, under the same
conditions: `InvalidNodeIndex` for a child index outside the node slice (including
`u32::MAX`, whose sibling would overflow) and `InvalidPrimitiveRange` for a leaf range past
the primitive-index slice, each reported when the search reaches the node;
`InvalidNodeIndex` once the search would make more than `nodes.len() / 2` interior visits,
which only a buffer that is not a tree causes
([Malformed Buffers](../ray-tracing/README.md#malformed-buffers)); and `StackOverflow` at
the first push that would need a 65th entry. Every waiting entry is the other child of a
node on the current path, so a tree of depth at most 63 is always searched completely; a
comb of depth 64 whose far leaves all wait overflows. `build_bvh_sah` builds no tree deeper
than 60. `StackOverflow` ends the search with no partial result.

### Correctness: The Answer Does Not Depend On The Tree

The result is the primitive a test of every primitive would choose: of all primitives the
closure returns a point for, the one with the smallest exact squared distance, and among
those the lowest index. It holds for every tree over the same primitives whose depth the
stack holds, whatever its shape and leaf order, when the node bounds contain the boxes of
the primitives below them (as both builders guarantee) and every point the closure returns
lies in its primitive's box (true of `closest_on_triangle` and `TraceTriangle::bounds`,
since the closest point is on the triangle). The argument:

1. **The bound is a lower bound.** For a node holding primitive `k`, the closest point of
   `k` lies in `k`'s box and so in the node's box; the box bound is at most the squared
   distance to any point of the box, so it is at most `k`'s exact squared distance.
2. **The winner's ancestors are never skipped.** Let the winner have exact squared distance
   `d`. Every point found is at least `d` away, so the best so far is never strictly nearer
   than `d`; every ancestor of the winner has bound at most `d`, and a node is skipped only
   when the best is strictly nearer than its bound. So every ancestor is searched and the
   winner is tested.
3. **The winner is kept.** When it is tested, no kept point is strictly nearer, and an
   equally near kept point has a higher index (the winner has the lowest index among those
   at `d`), so it replaces it; nothing found later is strictly nearer or equally near with
   a lower index.
4. **Nothing is invented.** Every kept point came from the closure.

The closure must be a function of the primitive index alone; `closest_on_triangle` is.

## Mesh View

The mesh input layout is a borrowed Rust view, not a serialized or versioned format: the
caller keeps its vertex buffer (`&[FixedVec3]`), its index buffer (`IndexBufferLayout`,
16-bit or 32-bit `Triangle16`/`Triangle32` faces), its node buffer and its
primitive-index buffer, and the view refers to them.

**`MeshView::new(positions, indices)`** checks the mesh once and returns `MeshError`, in
this order:

| Error | When |
| --- | --- |
| `EmptyMesh` | The index buffer has no triangles. |
| `TooManyTriangles` | It has more than `u32::MAX` triangles, more than primitive indices can name. |
| `VertexOutsideTraceDomain { vertex }` | The first vertex, used or not, with a coordinate outside the trace domain. |
| `IndexOutOfRange { triangle }` | The first triangle that names a vertex past the end of the vertex buffer. |

After it, every triangle can be prepared, so `MeshView::triangle(i)` returns the
`TraceTriangle` of triangle `i` of the index buffer, built from the vertex buffer on each
call, and `None` only past the last triangle. Triangle `i` of the index buffer is BVH
primitive `i`. Degenerate triangles are kept: the ray test misses them
([Exact Triangle Test](../ray-tracing/README.md#exact-triangle-test)) and the
closest-point test treats them as their segment or point.

**Building.** `mesh.write_bounds(&mut boxes)` writes the bounds of triangle `i` to
`boxes[i]` and returns the triangle count, or `BoundsBufferTooSmall` (writing nothing)
when `boxes` is shorter. The boxes go to `build_bvh_sah` (or `build_bvh`) with node and
primitive-index buffers the caller owns, as for any BVH.

**`MeshBvh::new(mesh, bvh)`** joins the mesh to the built tree and returns
`PrimitiveOutOfRange { entry }` for the first primitive-index entry that names no
triangle. It does not inspect the nodes; the searches check them as `closest_hit` does.

- `MeshBvh::closest_hit(ray, t_max)` is `closest_hit` with `TraceRay::hit_triangle` over
  `0..=limit`: the nearest triangle hit, the lowest index of equally near ones, with the
  contract of [Closest-Hit Traversal](../ray-tracing/README.md#closest-hit-traversal).
- `MeshBvh::closest_point(query)` is `closest_point` with `closest_on_triangle`.

### Example

The rustdoc of `MeshBvh` builds a two-triangle square through this layout, traces a ray
onto the shared diagonal and queries a point above a shared corner; both resolve to the
lower triangle index. `crates/bunny-codec/tests/mesh_bvh_stanford_tests.rs` does the same
for the committed Stanford bunny fixture (1 889 vertices, 3 851 triangles), read with
`parse_ascii_ply`.
