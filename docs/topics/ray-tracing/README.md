# Ray Tracing

This chapter defines Bunny's current contract for ray tracing: the kernels in
`bunny-query` (a prepared ray, a prepared triangle, a conservative ray-box test
and an exact ray-triangle test) and the closest-hit and any-hit BVH traversals
in `bunny-broadphase` that run them.

The kernels are the per-node and per-primitive tests a closest-hit traversal
runs millions of times, so they avoid division wherever the answer allows it.
The general-purpose solvers in `crates/bunny-query/src/ray.rs`
(`ray_intersects_aabb`, `ray_intersects_triangle`) and the all-hits traversal
`bunny_broadphase::intersect_ray` keep their own contracts; the overflow
behaviour of `ray_intersects_aabb`, which `intersect_ray` uses for every node,
is described in
[General-Purpose Ray-AABB Solver](#general-purpose-ray-aabb-solver).

`PreparedRay3` with `ray_intersects_aabb_prepared` (`bunny-query`) is the exact
counterpart: the same `(enter, exit)` bits as `ray_intersects_aabb` on every input, with a
prepared divisor per axis. It is for callers that need those exact distances for many boxes
per ray. For traversal, `TraceRay::box_entry` is the faster test; the measured comparison is
ledger entry 5 in [Performance](../performance/README.md).

## Status

This is a living topic chapter, not a proposal.

The kernels are implemented in `crates/bunny-query/src/trace.rs` and exported
as:

- `TraceRay` (`new`, `from_ray`, `origin`, `direction`, `box_entry`,
  `hit_triangle`);
- `TraceTriangle` (`new`, `vertices`, `bounds`);
- `TriangleHit` (`distance`, `front_face`, `barycentrics`,
  `barycentric_numerators`);
- `BarycentricNumerators` (`new`, `u`, `v`, `det`), in
  `crates/bunny-query/src/barycentric.rs`;
- `TRACE_COORD_LIMIT_RAW`, `TRACE_DIRECTION_LIMIT_RAW` and
  `TRACE_UNIT_DIRECTION_LIMIT_RAW`.

The traversals are implemented in `crates/bunny-broadphase/src/closest_hit.rs`
and exported as:

- `closest_hit` and `any_hit`;
- `BvhView` (`new`, `nodes`, `prim_indices`), the borrowed BVH buffers they
  search;
- `ClosestHit` (`primitive`, `distance`).

A second node layout for the same searches is implemented in
`crates/bunny-broadphase/src/compact.rs` and `compact_hit.rs` and exported as:

- `compact_bvh`, which converts a built tree into it, and `CompactError`;
- `CompactNode` (`bounds`, `children`), `CompactChild` (`first`, `prim_count`,
  `is_leaf`) and `CompactRoot` (`bounds`, `child`);
- `compact_closest_hit`, `compact_any_hit` and `CompactView` (`root`, `nodes`,
  `prim_indices`).

See [Compact Layout](#compact-layout).

Evidence is listed in the [test plan](test-plan.md). Benchmarks of the kernels
and the traversals are not part of this chapter yet.

All arithmetic is integer arithmetic on raw Q32.32 values (`i64`, widened to
`u128` or `i128`). There is no floating point, no `unsafe` code and no
target-specific path, so every supported target returns the same bits.

## Trace Domain

The kernels accept only values inside a fixed trace domain:

| Value | Bound | Constant |
| --- | --- | --- |
| Ray origin and triangle vertex coordinates | `abs(raw) < 2^40` (below 256.0) | `TRACE_COORD_LIMIT_RAW` |
| Ray direction components, `TraceRay::new` | `abs(raw) <= 2^32` (at most 1.0) | `TRACE_DIRECTION_LIMIT_RAW` |
| Ray direction components, `TraceRay::from_ray` | `abs(raw) <= 2^32 + 1` | `TRACE_UNIT_DIRECTION_LIMIT_RAW` |

`TraceRay::new` returns `None` when an origin coordinate or a direction
component is outside the domain, or when the direction is zero.
`TraceTriangle::new` returns `None` when any vertex coordinate is outside the
domain. `TraceRay::from_ray` accepts a `FixedRay3` and returns `None` only for
an origin outside the domain: `FixedUnitVec3::try_from_unit` allows the length
to be off by one raw unit, so a unit direction can have one component of
magnitude `2^32 + 1` (the other two then have squares summing to at most
`2^33 + 2`, since the sum of all three must be less than `(2^32 + 2)^2`), and `from_ray` accepts it although `new`
refuses it. `FixedUnitVec3::new` (normalization) never produces a component
above `2^32`: the floored root is at least the largest component, so the
quotient is at most 1.0 (TRACE-TP-409 counts this over generated inputs).

**Widths with direction components up to `2^32 + 1`.** Each line of the argument
was redone for this bound; none is tight:

- The reciprocal `floor(2^63 / abs(d))` is `2^31 - 1` for `abs(d) = 2^32 + 1`
  (`(2^32 + 1)(2^31 - 1) = 2^63 - 2^31 - 1`, remainder `2^31 + 1 < 2^32 + 1`).
  The box test's products are `near * r` and `far * (r + 1)`, with `r` below
  `2^63` and `near`, `far` below `2^64`: below `2^127` whatever `d` is.
- Edge and origin-offset components are below `2^41` (at most `2^41 - 2`).
  Each component of `p = d x e2` is at most `2 (2^32 + 1)(2^41 - 2)`, below
  `2^75` (below `2^74` when `abs(d) <= 2^32`).
- `det = e1 . p = d . (e2 x e1)`; each component of `e2 x e1` is below `2^83`, so
  `abs(det)` is below `3 (2^32 + 1) 2^83 = 3 * 2^115 + 3 * 2^83`, below `2^117`.
- `u = s . p` is below `3 * 2^41 * 2^75`, and `v = d . q` is below
  `3 (2^32 + 1) 2^83`, with `q = s x e1` below `2^83` per component: both below
  `2^117` before the comparisons with `det`.
- `t = e2 . q` does not involve `d`: below `3 * 2^41 * 2^83 = 3 * 2^124`, below `2^126`.
- `u + v` is below `2^118`; all of these fit `i128` (below `2^127`) with at least
  nine bits to spare.

The bounds exist so that every intermediate of the triangle test fits `i128`
exactly: edges and origin offsets are below `2^41`, cross products below
`2^84`, and the largest numerator below `2^126`. Box coordinates are not
restricted; `box_entry` accepts any `FixedAabb3`.

The direction need not be a unit vector. Every distance in this chapter is a
multiple of the direction: the point at distance `t` is
`origin + direction * t`, and distances are raw Q32.32 values of `t`.

## Conservative Box Test

`TraceRay::box_entry(aabb, t_max)` returns a lower bound on the distance at
which the ray enters `aabb`, or `None` when the ray certainly does not reach the
box within distances `0..=t_max`. A negative `t_max` always returns `None`.

`TraceRay::new` stores one reciprocal per direction component,
`r = floor(2^63 / abs(d))` (zero for a zero component). For each axis with a
nonzero component, with `near` and `far` the raw coordinate differences from
the origin to the slab planes in the direction of travel:

- the slab lower bound is `floor(near * r / 2^31)`, with a negative `near`
  (origin already past the near plane) taken as zero;
- the slab upper bound is `floor(far * (r + 1) / 2^31) + 1`, saturating;
- a negative `far` (the slab is behind the origin) rejects the box.

An axis with a zero component accepts the whole distance range when the origin
coordinate lies within `min..=max` on that axis (inclusive) and rejects the box
otherwise. The entry bound is the largest slab lower bound; the box is rejected
when it exceeds the smallest slab upper bound or `t_max`.

The exact slab distances are `near * 2^32 / abs(d)` and `far * 2^32 / abs(d)`.
Because `r <= 2^63 / abs(d) < r + 1`, each lower bound is at or below its exact
value and each upper bound is above its exact value. Therefore:

- **It never rejects a box the ray reaches.** If the ray reaches the box at an
  exact entry `T <= t_max`, every computed lower bound is at most `T`, every
  computed upper bound is above the exact exit, and the test passes.
  Touching a face, edge or corner counts as reaching the box.
- **It may accept a box the ray narrowly misses**, by at most the rounding
  slack below. Callers must treat an accepted box as "may contain a hit", which
  is what a traversal needs.
- **The bound is within `2^11 + 2` raw units plus `T / 2^30` below the exact
  entry `T`.** That is the stated contract. The per-axis derivation gives a
  tighter figure: the reciprocal loses less than one unit of `2^63 / abs(d)`,
  so a slab lower bound falls short by less than `near / 2^31 + 1` raw units,
  and `near <= T` because `abs(d) <= 2^32`. For a `from_ray` direction
  component of `2^32 + 1`, `near <= T + T / 2^32`, so the shortfall is below
  `T / 2^31 + 1 + T / 2^63`, which is below `T / 2^31 + 2` for `T < 2^63`. The
  test checks both the contract and the tighter `T / 2^31 + 2`.

The box test needs no division at query time: one multiplication and one shift
per slab plane.

How the implementation computes this (flyingrobots/bunny#258; the results are
bit for bit those of the earlier `i128` version, which the tests keep as a
reference oracle):

- **Differences in `i64`.** For a negative direction component, bitwise not
  (`!x = -x - 1`) reverses the order of `i64` and keeps `!a - !b = b - a`, so
  the slab becomes one crossed forwards: the box test applies `!` to the
  origin and the slab's planes on such axes. A signed comparison then decides
  whether a plane is ahead of the origin, and the difference of two `i64`
  values `a >= b` lies in `0..2^64`, so the wrapping `i64` subtraction read as
  `u64` is exact for any box coordinates, including the ends of the `i64`
  range.
- **Parallel axes in the same path.** A zero component is treated as a forward
  crossing whose reciprocal is zero: its far-plane check is `origin <= max`, a
  near plane ahead of the origin (`min > origin`) rejects, its lower bound is
  0 and its upper bound saturates to `u64::MAX`.
- **Straight-line code.** All three axes, both bounds and the comparison with
  `t_max` are computed without early returns and combined into one validity
  flag, so the test has no data-dependent branch of its own; arithmetic on a
  rejected axis cannot overflow and its result is discarded.

## Exact Triangle Test

`TraceRay::hit_triangle(triangle, t_min, t_max)` decides exactly whether the
ray hits the triangle at a distance in `t_min..=t_max`, and returns a
`TriangleHit` when it does.

The test follows the Moller-Trumbore construction in exact integers. With
`e1 = v1 - v0`, `e2 = v2 - v0`, `d` the direction and `s = origin - v0`, all
products in `i128`:

```text
p   = d x e2         det = e1 . p
q   = s x e1         u   = s . p      v = d . q      t = e2 . q
```

After the sign of `det` is folded into `det`, `u`, `v` and `t` (so `det > 0`),
the ray's line crosses the triangle when `0 <= u`, `0 <= v` and `u + v <= det`.
These comparisons are exact: no rounding happens before the hit is decided.

- **Edge rule.** Edges and corners belong to the triangle (all three
  comparisons are inclusive). Two triangles that share an edge both report a
  ray through that edge, so a ray through a shared edge of a closed mesh never
  passes between them. A closest-hit caller must break the resulting tie.
- **Parallel and degenerate cases.** `det == 0` is a miss. That covers rays
  parallel to the triangle's plane, including rays lying in that plane, and
  zero-area triangles (duplicate or collinear vertices).
- **Behind the origin.** A crossing with `t < 0` is a miss whatever `t_min` is.

Only after a crossing is established does the test divide, once:

- **Distance rounding.** `TriangleHit::distance` is
  `floor(t * 2^32 / det)`, the floor of the true distance in raw units. A
  crossing whose distance does not fit `i64` is a miss; this can only happen
  with very short directions.
- **Distance limits** are inclusive and apply to the rounded distance:
  the hit is reported when `t_min <= distance <= t_max`.
- **Barycentrics** are computed on demand by `TriangleHit::barycentrics`:
  `(floor(u * 2^32 / det), floor(v * 2^32 / det))`, the weights of `v1` and
  `v2`. The hit point is `v0 + (v1 - v0) * u + (v2 - v0) * v`. Each weight lies
  in `0.0..=1.0`, and their sum is at most 1.0.
- **Exact numerators.** `TriangleHit::barycentric_numerators` returns the
  `u`, `v` and `det` the test compared, unrounded, as a
  `BarycentricNumerators`. Each is below `2^117` for a ray and triangle in the
  trace domain: direction components are at most `2^32 + 1` and edge components
  below `2^41`, so each component of `p = d x e2` is below `2^75` and
  `det = e1 . p` is below `3 (2^32 + 1) 2^83`, below `2^117`; `u` and `v` are at
  most `det`.
  `BarycentricNumerators::new(u, v, det)` builds the same type from any values
  that describe a point on a triangle (`det > 0`, `u >= 0`, `v >= 0`,
  `u + v <= det`) and returns `None` otherwise, so a caller can state weights
  without a ray. Neither changes `barycentrics`.

The division is a long division of the `i128` numerator by the `i128`
denominator that brings the 32 fractional bits down in steps the denominator's
width allows, so it is exact for every value in the trace domain.

## Facing

The triangle is two-sided: both faces report hits.
`TriangleHit::front_face` is `true` when the ray arrives on the side from
which `v0, v1, v2` appear counter-clockwise, that is when
`d . (e1 x e2) < 0`, so the ray travels against the right-handed normal
`e1 x e2`. This matches the winding convention of
[the coordinate law](../coordinate-law/README.md).

## Accessors

`TraceRay::origin` and `TraceRay::direction` return the values given to `new`;
the direction is not normalized. `TraceTriangle::vertices` returns the three
vertices given to `new`, and `TraceTriangle::bounds` returns the smallest
`FixedAabb3` that contains them.

## General-Purpose Ray-AABB Solver

`ray_intersects_aabb(ray, aabb)` in `crates/bunny-query/src/ray.rs` returns
`Some((t_enter, t_exit))` or `None`. It is not a trace kernel: it divides, its
ray is a `FixedRay3` with a unit direction, and its coordinates are not
restricted. `bunny_broadphase::intersect_ray` visits a node exactly when this
function returns `Some` for the node's bounds.

**Bounds.** For each axis with a non-zero direction component the two slab
distances are `(bound - origin) / direction`, rounded to the nearest raw unit
with ties to even, exactly as `FixedQ32_32::checked_div` rounds. `t_enter` is
the largest near-slab distance, `t_exit` the smallest far-slab distance. An
axis with a zero component does not bound the interval and rejects the box when
the origin lies outside `min..=max` on that axis. A side that no axis bounds is
`i64::MIN` or `i64::MAX` raw.

**Overflow.** A slab distance that does not fit Q32.32 saturates to `i64::MIN`
or `i64::MAX` raw by its sign. The coordinate difference `bound - origin` is
taken in `i128`, so a difference past `i64` is never saturated before the
division. Such distances arise from a direction component of a few raw units
(for a slab of half-width `k` centred on the origin, any component of at most `2k`
raw units), or from a bound about `2^31` units or more from the origin.

**Hit decision.** When every slab distance fits, the hit is decided from the
rounded distances (`t_enter <= t_exit` and `t_exit >= 0`), so a ray that misses
by less than half a raw unit can report a hit whose bounds are equal or whose
exit is zero. When some slab distance does not fit, the function decides the hit
exactly: every slab distance is an `i128` ratio, compared by
cross-multiplication (all products stay below `2^127`), and only then are the
two bounds rounded and saturated. A saturated bound therefore never turns a miss
into a hit; two entries that both saturate to the maximum are still compared
exactly.

**History.** Before flyingrobots/bunny#211 the function returned `None` whenever
a coordinate difference or slab distance overflowed, so rays through a box were
reported as misses. The change touches only those inputs: every hit reported
before is reported with the same bits, which a differential test against the
retained old implementation checks. The analysis is in
[`docs/design/0006-ray-aabb-slab-overflow.md`](../../design/0006-ray-aabb-slab-overflow.md).

## Closest-Hit Traversal

`closest_hit(bvh, ray, t_max, hit)` returns the primitive the ray hits first
within distances `0..=t_max`, as a `ClosestHit { primitive, distance }`, or
`None` when nothing is hit. `any_hit(bvh, ray, t_max, hit)` returns whether
anything is hit in that range.

`bvh` is a `BvhView`: the node buffer (truncated to the nodes `build_bvh`
used; the root is node 0) and the primitive-index buffer. Both stay owned by
the caller, and the traversal allocates nothing; its stack is a fixed array of
64 entries on the call stack.

The primitive test is a caller closure, so the traversal works for any
primitive type:

- `closest_hit` calls `hit(primitive, limit)`. It returns the hit distance, or
  `None` for a miss. `limit` is the best distance found so far (at first
  `t_max`); a triangle closure passes it to `TraceRay::hit_triangle` as
  `t_max` with `t_min` zero. A reported distance outside `0..=limit` is
  ignored.
- `any_hit` calls `hit(primitive)`, which returns whether the primitive is hit
  within `0..=t_max`. After the first `true` the search ends and the closure is
  not called again.

### Search Order And Pruning

The search is depth first and front to back:

1. The root is pushed with its `box_entry` bound if the ray can reach it.
2. At an interior node, both children are tested with
   `TraceRay::box_entry(child, limit)`. A child the ray cannot reach within
   the current limit is dropped. The farther child is pushed first and the
   nearer one on top, so the nearer one is searched first; when both bounds
   are equal the left child is searched first.
3. A node popped from the stack is skipped when its entry bound is strictly
   greater than the current limit, which may have shrunk since it was pushed.
   A node whose bound equals the limit is still searched.
4. At a leaf, each primitive is tested in the order the primitive-index buffer
   lists it. A hit replaces the best one when its distance is smaller, or equal
   with a lower primitive index. The limit becomes the best distance.

Malformed buffers are reported as `TraversalError::InvalidNodeIndex` or
`TraversalError::InvalidPrimitiveRange` when the search reaches the malformed
node; a malformed node the search prunes is not inspected. A tree deeper than
the 64-entry stack allows returns `TraversalError::StackOverflow`. An empty
node buffer is a miss. A buffer that is not a tree is bounded as described
next.

### Malformed Buffers

`BvhNode` buffers are public data, so a caller can hand any of the four plain
traversals (`intersect_aabb`, `intersect_ray`, `closest_hit`, `any_hit`) a
buffer whose child index names an ancestor (a cycle) or in which two interior
nodes name the same child pair (a shared subtree). Every index can be in range
and the stack need never fill, so without a bound a cycle is searched forever.

Each traversal therefore allows at most `nodes.len() / 2` interior visits and
returns `TraversalError::InvalidNodeIndex` when a visit would exceed them. The
count is a countdown in a local of the search loop. The box queries spend it
after an interior visit has pushed its children, so a full stack still reports
`StackOverflow` first; `closest_hit` and `any_hit` spend it when they pop an
interior node.

**Why a tree never reaches the bound.** In a tree every node has at most one
parent and the root has none, so a search reaches each node at most once, and
`i` interior visits whose children are all inside the slice name `2i + 1`
distinct nodes: `2i + 1 <= n`, so `i <= n / 2`. A child outside the slice ends
the search with the error it always had. The box queries pop the right child
next, which is out of range whenever either child is (the children are `first`
and `first + 1`), with no callback in between. In `closest_hit` and `any_hit`
the count can run out only at an interior node whose children are not both in
range (the visits before it already name every node), and that visit would
return the same `InvalidNodeIndex` before calling anything. So on every buffer in
which no node is reached twice, including ones whose only faults are
out-of-range indices, bad leaf ranges or a depth past 64, the results, errors
and callback calls are exactly those of the functions before
flyingrobots/bunny#269, which a differential test against frozen copies checks.

**What a buffer that is not a tree returns.** A cycle is `InvalidNodeIndex`
after at most `n / 2` interior visits, with the leaf callbacks those visits
reached. A cycle the stack used to overflow on first now returns
`InvalidNodeIndex` instead of `StackOverflow`. A shared subtree is searched as
the tree it unfolds into while the unfolding needs at most `n / 2` interior
visits (a leaf shared by two parents is then reported twice) and is
`InvalidNodeIndex` beyond that; with a query that reaches every node this is
exactly the rule by which `compact_bvh` returns `NotATree`. The bound limits
work; it is not a check that the buffer is a tree, and it does not inspect
nodes the search does not reach. `InvalidNodeIndex` does not say whether the
index was out of range or the bound ran out.

### Correctness: The Answer Does Not Depend On The Tree

The result is the hit a test of every primitive would choose: of all
primitives whose test reports a distance in `0..=t_max`, the one with the
smallest distance, and among those the lowest primitive index. That holds for
every tree over the same primitives, whatever its shape and whatever order its
leaves list primitives in. The argument, for a tree whose node bounds contain
the bounds of every primitive below them (as `build_bvh` guarantees) and a
closure that reports `floor(T)` for a hit at true distance `T` (as
`hit_triangle` does):

1. **The box bound is a lower bound.** `box_entry` returns an integer raw
   value at or below the exact entry distance of the box
   ([Conservative Box Test](#conservative-box-test)). Its computed slab upper
   bounds are above the exact exits, so a box the ray reaches is rejected only
   when that lower bound exceeds the distance limit.
2. **A primitive inside a node is no nearer than the node.** The hit point at
   true distance `T` lies in the primitive's box, so in every ancestor's box;
   the ray is inside that box at `T`, so the exact entry `E` is at most `T`.
   The computed bound `b` therefore satisfies `b <= E <= T`.
3. **The floor keeps the order.** The reported distance is `floor(T)`, and `b`
   is an integer at most `T`, so `b <= floor(T)`: the floor of a value that is
   at least an integer bound is itself at least that bound.
4. **Ties are kept.** Let the winner be the primitive the full test chooses,
   with reported distance `d`. Until it is found, the limit is at least `d`
   (the limit only ever becomes the distance of a hit that was accepted, and no
   accepted hit beats the winner). Every ancestor of the winner has bound
   `b <= d <= limit`, and a node is dropped or skipped only when its bound is
   strictly greater than the limit, so every ancestor is searched and the
   winner is tested. Its test reports `d <= limit`, and no hit already kept can
   beat `(d, index)`, so it is kept; nothing found later beats it either.
5. **Nothing better is invented.** Every kept hit was reported by the closure
   within `0..=t_max`, so the result is never a primitive the full test would
   not consider.

`any_hit` runs the same search with the limit held at `t_max` until the first
hit, which ends it. Every ancestor of a primitive hit at true distance `T` with
`floor(T) <= t_max` has bound `b <= floor(T) <= t_max` (steps 1 to 3), so it is
searched: `any_hit` returns `true` exactly when testing every primitive would
find a hit.

**What the caller must guarantee.** The claim needs three things of the
closure and the tree:

- every reported distance is at least the floor of some distance at which the
  ray is inside the primitive's box (for `any_hit`: a primitive is reported
  hit only if the ray is inside its box at a distance whose floor is at most
  `t_max`), and the tree's node bounds contain the box of every primitive
  below them;
- the closure answers the same way for a primitive whatever `limit` it
  receives, except that it may return `None` (or a distance above the limit)
  for hits beyond the limit, as `hit_triangle` does;
- it has no other dependence on call order (it is a function of its inputs).

A closure that reports distances larger than the floor of the true distance
(a conservative overestimate) still satisfies the first condition. A closure
that reports a distance below the box bound of a primitive's subtree breaks
step 4: that primitive is found only if the search happens to reach its leaf
before the limit drops below the subtree's bound, so the answer can depend on
the tree's shape and differ from the full test. It is still a primitive the
closure reported within `0..=t_max`, and the traversal remains deterministic
for a fixed tree.

### Compact Layout

`compact_bvh(nodes, out)` converts a tree that `build_bvh` filled into a second
layout in which each interior node holds both of its children's boxes, so a
search step reads one node and tests two boxes from one place:

- **`CompactNode`** is `bounds: [FixedAabb3; 2]` and
  `children: [CompactChild; 2]`, left then right: 112 bytes, 8-byte aligned.
- **`CompactChild`** is `first` and `prim_count`. A leaf (`prim_count > 0`)
  stores its primitive range in the parent, so leaves have no node of their
  own and a leaf visit reads no node. An interior child's `first` is the index
  of its `CompactNode`.
- **`CompactRoot`** is the root's box and the root itself as a `CompactChild`:
  compact node 0 for an interior root, or a leaf for a one-leaf tree, which
  then has no compact node at all. An empty tree converts to `None`.

A tree of `n` nodes has `(n - 1) / 2` interior nodes and needs that many
compact nodes; the scene's 55 277 nodes of 56 bytes (3.10 MB) become 27 638
compact nodes of 112 bytes (3.10 MB), because each box is still stored once
(in its parent) and the leaves' ranges move into their parents. The nodes are
written to the front of `out` in depth-first order, each node followed by its
left subtree and then its right subtree. The conversion is integer-only,
copies bounds and ranges without arithmetic, and writes the same bytes on
every target. It returns the root and the number of nodes written, or a
`CompactError`: `InvalidNodeIndex` when a child index is outside the node
slice, `OutputTooSmall` when `out` is shorter than the tree needs, `NotATree`
when more interior nodes are reached than a tree of that size has (a node
reached twice), and `TooDeep` when more than 64 right subtrees wait at once,
which needs a tree deeper than 64 levels. Leaf primitive ranges are copied
unchecked and are checked by the search when it reaches them, as
`closest_hit` checks them.

`compact_closest_hit` and `compact_any_hit` take a `CompactView` (the root,
the compact nodes and the source tree's primitive-index buffer) and run the
[search above](#search-order-and-pruning) step for step: the same box tests
in the same order, the nearer child first with ties to the left, the same
pop-time skip, the same leaf order and the same acceptance rule. A stack
entry carries the child's reference with its entry bound. On the tree
`compact_bvh` made from a source tree, both therefore call the primitive test
with the same primitives and limits in the same order as `closest_hit` and
`any_hit` on the source tree, and return the same result; the correctness
argument above applies unchanged, because every node box is the source box.
Errors are the same `TraversalError` values: `InvalidNodeIndex` for a
reference to a missing compact node, `InvalidPrimitiveRange` for a leaf range
past the buffer, `StackOverflow` past 64 entries. Because `CompactView`'s
fields are public, a hand-built compact tree can contain a reference cycle
(a child that names an ancestor); a search that pushes only one child per
visit would then loop forever without filling the stack. A tree visits each
node at most once, so the search counts interior visits and returns
`InvalidNodeIndex` once they exceed the number of compact nodes, the
counterpart of the [bound on the plain traversals](#malformed-buffers).

The layout is a second representation to keep in step with the first: a
change to the search order or pruning rule must be made in both
`closest_hit.rs` and `compact_hit.rs`. Its speed is recorded in the
performance chapter's ledger.

### Example

The rustdoc of `closest_hit` builds a BVH over two triangles with `build_bvh`
and traces a ray with a closure over `TraceRay::hit_triangle`.
