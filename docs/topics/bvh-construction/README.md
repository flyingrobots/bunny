# BVH Construction

This chapter defines Bunny's current contract for building bounding volume
hierarchies in `bunny-broadphase`: the flat node format both builders write,
the quartile builder `build_bvh`, the binned surface-area-heuristic (SAH)
builder `build_bvh_sah`, and `refit_bvh`, which gives a built tree new
primitive boxes without changing its shape. The traversals that read these trees
(`intersect_aabb`, `intersect_ray`, `closest_hit`, `any_hit`) are described in
the [ray-tracing chapter](../ray-tracing/README.md#closest-hit-traversal).

## Status

This is a living topic chapter, not a proposal.

Both builders are implemented in `crates/bunny-broadphase/src/bvh/` and
exported from `bunny_broadphase` as `build_bvh` and `build_bvh_sah`, with
`BvhNode`. The SAH builder's parameters are public constants in
`bunny_broadphase::bvh`: `SAH_BIN_COUNT`, `SAH_MAX_LEAF_SIZE`,
`SAH_MAX_DEPTH`, `SAH_TRAVERSAL_COST` and `SAH_INTERSECTION_COST`.
Refit is implemented in `crates/bunny-broadphase/src/refit.rs` and exported as
`refit_bvh`, with its error type `RefitError`.

Evidence is listed in the [test plan](test-plan.md). Timings of both builders
and of traversal through their trees are in the
[performance ledger](../performance/README.md#9-binned-sah-bvh-builder-new-api).

All arithmetic is integer arithmetic on raw Q32.32 values. There is no
floating point, no `unsafe` code and no target-specific path, so every
supported target builds the same bytes from the same input.

## Buffers And Node Format

Both builders take the same arguments and write the same format:

```rust,ignore
fn build(nodes: &mut [BvhNode], prim_indices: &mut [u32], primitives: &[FixedAabb3])
    -> Option<usize>;
```

- `nodes` must hold at least `2 * n - 1` nodes and `prim_indices` at least `n`
  entries, where `n = primitives.len()`; otherwise the result is `None`. `n`
  and `2 * n - 1` must fit in `u32`. Larger buffers are accepted; entries past
  the ones written are left alone.
- An empty input returns `Some(0)` and writes nothing.
- The result is the number of nodes written. The root is node 0.
- A node with `prim_count == 0` is interior: its children are nodes
  `first_child_or_prim_idx` and `first_child_or_prim_idx + 1`. Otherwise it is
  a leaf listing `prim_indices[first_child_or_prim_idx..][..prim_count]`.
- Every node's `bounds` is exactly the union of its primitives' boxes, so it
  contains its children's bounds and every primitive below it.
- Every primitive appears in exactly one leaf. Children are allocated in
  depth-first order: a node's two children are reserved together, then the left
  subtree is built completely before the right one.
- Nothing is allocated. The output depends only on `primitives` (the previous
  contents of the buffers are overwritten).

## The Quartile Builder

`build_bvh` is the original builder, and its output is fixed: no change may
alter a bit of it (the Fidelity Rule in the
[performance chapter](../performance/README.md#the-fidelity-rule)). As it
stands:

- It recurses once per tree level, with no depth cap.
- At each node it tries three planes per axis, at the quartiles of the span of
  primitive centroids on that axis (`min + k * (max - min) / 4` in Q32.32 for
  `k` = 1, 2, 3), making a full pass over the node's primitives for each.
  A primitive goes left when its centroid is strictly below the plane.
- A plane's cost is `area(left) * left_count + area(right) * right_count`,
  with `surface_area` and Q32.32 operators, which round products and saturate
  at the ends of the range. The first cheapest plane wins (x before y before
  z, lower quartile first).
- A node of more than four primitives always takes the best plane; a node of
  two to four takes it only when its cost is strictly below
  `area(node) * count`. A node with no plane that puts primitives on both sides
  (for example, all centroids equal) becomes a leaf of any size.
- The partition swaps entries in place and is not stable, so a leaf's entries
  are not in any particular order.

### Depth

`build_bvh` has no depth cap, and adding one would change its output for the
inputs it caps, so it has none. Its trees can be deeper than the 63 levels
below the root that every traversal holds
([ray-tracing chapter](../ray-tracing/README.md#depth-and-the-traversal-stack)),
including for inputs entirely inside the trace domain (every coordinate below
`2^40` raw in magnitude). On such a tree a traversal returns its complete
answer or `TraversalError::StackOverflow`, never a different answer. For ray
tracing, use `build_bvh_sah`, whose trees are at most 60 deep.

What is known about inputs inside the trace domain, from a targeted search
(BVH-TP-024):

- **Collinear points split off one per level.** Point boxes on one axis at
  `T - d_k` raw, with `d_0 = 0` and `d_(k+1) = ceil(4 (d_k + 1) / 3)`, have
  zero area everywhere, so every plane costs the same and the first quartile
  plane wins; the gap below the farthest point is more than a quarter of the
  span, so that plane leaves the farthest point alone. 94 such points
  (`T = 2^40 - 1 - 2^32`) give a tree 90 deep, the deepest found.
- **Thin triangles come close.** Triangles in the planes `x` of those points,
  one raw unit wide in y and z, give a tree 70 deep. The same construction
  continued on a second axis gave 83 (201 points), and on all three 84.
- **Ratios near 4/3 are the boundary.** At 13/10 the farthest point is no
  longer alone and the same construction gave 52; ratios 3/2, 2 and 4 gave 63,
  37 and 17; a progression rising from the bottom by 2 gave 18. Earlier
  constructions in the independent review peaked at 60.

## The Binned SAH Builder

`build_bvh_sah` builds a tree for the same traversals with a binned surface
area heuristic. Its output differs from `build_bvh`'s.

### Costs

At each node, the primitives' centroids are counted into `SAH_BIN_COUNT` (16)
equal-width bins per axis, and every plane between two adjacent bins is
costed. With `T = SAH_TRAVERSAL_COST` (1), `I = SAH_INTERSECTION_COST` (1)
and `A` the half surface area (`dx * dy + dy * dz + dz * dx`):

```text
split cost = T * A(node) + I * (A(left) * left_count + A(right) * right_count)
leaf cost  = I * A(node) * count
```

Both are scaled by `A(node)` (the usual SAH divides by it), so the comparison
needs no division.

Binning uses doubled centroids, `min + max` per axis as `i128`, so no rounding
happens. On an axis whose doubled centroids span `origin..=origin + width`
with `width > 0`, a primitive with doubled centroid `c` falls in bin
`(c - origin) * 16 / (width + 1)`, which is always below 16. Plane `p` (1 to
15) sends bins below `p` left. An axis whose centroids are all equal has no
planes; a plane with an empty side is not a candidate.

### Arithmetic And Overflow

Areas are `u128` values in raw units squared. A box near the ends of the
Q32.32 range has extents near `2^64` raw units, whose products overflow even
`u128`, so each node fixes one right shift for every extent it costs: zero
when the node's widest extent is below `2^45` raw units (8 192 units), and
otherwise the smallest shift that brings it below `2^45`. Children lie inside
their node, so every extent costed at that node is below `2^45` after the
shift. Then an area is below `3 * 2^90`, an area times a count (below
`2^32`) below `2^124`, and a split cost below `2^128` for weights up to 4.
Areas are exact for nodes narrower than 8 192 units; wider nodes lose the low
bits of their extents, which can only turn nearly equal costs into ties. The
operations are checked anyway: an overflow would fail the build (`None`)
rather than choose a split, and the tests reach the widest boxes and the
largest counts without one.

### Decisions And Ties

The tree is a deterministic function of `primitives`:

1. A node of one primitive is a leaf. A node at depth `SAH_MAX_DEPTH` (60;
   the root is depth 0) is a leaf whatever its size.
2. The cheapest plane wins. Equal costs go to the lower axis (x, then y, then
   z), and on one axis to the lower plane.
3. A node of at most `SAH_MAX_LEAF_SIZE` (4) primitives takes the plane only
   when the split cost is strictly below the leaf cost; equal costs make a
   leaf. A larger node always takes the cheapest plane.
4. A larger node with no plane (every centroid equal on every axis) is split
   in half by primitive order: the first `count / 2` entries go left.
5. The partition is stable. The root's entries start as `0..n`, and after each
   partition both sides are in ascending primitive order again, so every leaf
   lists its primitives in ascending index order. (The implementation swaps
   left-going entries forward and sorts each side in place; entries are
   distinct, so the result does not depend on the sort algorithm.)

### Depth And Stack

The build does not recurse. Pending nodes wait in a fixed array of
`SAH_MAX_DEPTH + 1` entries on the call stack: a node at depth `d` leaves at
most one pending sibling per level above it. The depth cap also keeps every
tree within the traversals' fixed 64-entry stacks: a tree of depth 60 needs at
most 61 entries
([ray-tracing chapter](../ray-tracing/README.md#depth-and-the-traversal-stack)).

A node at depth 60 is a leaf whatever its size: a **capped leaf**. It is the
only leaf that can hold more than `SAH_MAX_LEAF_SIZE` primitives, because a
larger node elsewhere always splits. A capped leaf is still a valid leaf: every
traversal tests each of its primitives.

The cap is a guarantee, not a tuning knob: with 16 equal-width bins every
split narrows the chosen axis, so ordinary inputs stay far shallower (18
levels on the 27 650-triangle benchmark scene). Inputs inside the trace domain
reach it. 70 collinear point boxes at `2^40 - 1 - d_k` raw, with `d_0 = 0` and
`d_(k+1) = ceil(16 (d_k + 1) / 15)`, put each node's lowest point alone in the
lowest bin; with zero area everywhere the lowest plane wins, so each level
splits off one point, and the tree is 60 deep with a capped leaf of 10
(BVH-TP-022). The independent review's witness, point boxes at raw `17^k` for
`k` from 0 to 14 on each axis in turn plus `2^18` identical boxes at the
origin (262 189 boxes), gives a tree 60 deep whose 32 768 leaves at depth 60
hold 8 primitives each (BVH-TP-023). The build's work stack fills only on a
left spine, where every level leaves its right child waiting: the review's
chains extended to `k` = 15, plus `2^14` identical boxes, need all 61 entries
(BVH-TP-026).

### Cost

Each node makes one pass to find its bounds and centroid spans, one to bin, a
sweep over 16 bins per axis, and a partition with two in-place sorts. Build
time and traversal time on the benchmark scene are in the
[performance ledger](../performance/README.md#9-binned-sah-bvh-builder-new-api).

## Choosing A Builder

Both builders' trees give the same `closest_hit` and `any_hit` answers for
the same primitives, because those answers do not depend on the tree's shape
([ray-tracing chapter](../ray-tracing/README.md#correctness-the-answer-does-not-depend-on-the-tree)).
The visitor traversals `intersect_ray` and `intersect_aabb` report every
primitive in each leaf they reach, so which primitives they report, and in
what order, depends on the tree. A tree from one builder is not a substitute
for golden data recorded with the other.

## Refit

`refit_bvh` updates a built tree for primitives that have moved: the same
primitives, in the same order, with new boxes. It keeps the tree's topology
and primitive order and replaces only the bounds.

```rust,ignore
fn refit_bvh(nodes: &mut [BvhNode], prim_indices: &[u32], primitives: &[FixedAabb3])
    -> Result<(), RefitError>;
```

### Refit Buffers

- `nodes` is the used node slice a builder filled (the first `used` nodes;
  the root is node 0), taken mutably. Only the `bounds` field of each node is
  written.
- `prim_indices` is the primitive-index buffer the builder filled, taken
  immutably: refit cannot change the primitive order.
- `primitives` holds one new box per primitive, indexed as the input to the
  build was. Its length must equal the number of slots the leaves list, which
  for a built tree is the number of primitives it was built over.
- There is no scratch buffer, no allocation and no recursion.

### Post-Condition

After `Ok(())`, for every node of the slice:

- a leaf's bounds are the union of the new boxes of the primitives it lists;
- an interior node's bounds are the union of its two children's bounds.

The union of boxes is, per axis, the least `min` and the greatest `max` of
their raw values, so nothing is rounded and every bound is one of the input
values. An inverted box (a `min` above its `max` on some axis) is accepted and
unioned the same way, because both builders accept it and union it the same
way: refit accepts exactly the boxes the builders accept. The result is a
function of the topology, `prim_indices` and `primitives` only, so refitting
a tree to the boxes it was built over gives back the same bits, and
refitting to other boxes and then back restores it bit for bit.

### Order And The Single Pass

Both builders reserve a node's two children at the end of the nodes written
so far ([Buffers And Node Format](#buffers-and-node-format)), so every
interior node's first child has a larger index than the node itself, and a
tree has one more leaf than interior nodes. `refit_bvh` relies on the first
fact: it walks the nodes once from the last to the first, so both children of
a node are updated before the node. A test checks both facts on the output of
both builders (BVH-TP-040).

### Refusals And All-Or-Nothing

Refit checks the whole buffer before it writes anything, so a refused refit
leaves the buffer exactly as it was, with no scratch copy: a validating pass
over every node, then the writing pass. The errors are `RefitError` values,
and the first fault found in this order is reported:

1. each node from first to last: an interior node whose children are not both
   inside the slice (`InvalidNodeIndex`), or whose first child is at or before
   the node itself (`ChildNotAfterParent`; every cycle has such a node, and a
   node slice with unused stale nodes after the tree usually fails here); a
   leaf whose slots are not all inside `prim_indices`
   (`InvalidPrimitiveRange`);
2. a slice without exactly one more leaf than interior nodes (`NotATree`);
3. a box count other than the number of slots the leaves list
   (`PrimitiveCountMismatch`);
4. each leaf from first to last: a listed primitive with no box
   (`InvalidPrimitiveIndex`).

The node checks and the count make every index refit uses valid and rule out
cycles. They are not a proof that the nodes form a tree, and refit does not
check that `prim_indices` is a permutation: a shared child pair with
unreachable leaves can pass the count, and a primitive listed twice is not
detected. Refit still writes the post-condition for every node of such a
buffer; it does not make a malformed tree into a good one.

### What Refit Preserves

**Ordering.** Refit changes no order: the topology, the leaf ranges and
`prim_indices` are untouched. The searches that break ties by lowest
primitive index (`closest_hit`, `any_hit` and their compact forms) answer
independently of the tree
([ray-tracing chapter](../ray-tracing/README.md#correctness-the-answer-does-not-depend-on-the-tree)),
so refit cannot change which of several equally near primitives wins.

**Correctness, not quality.** A refitted tree satisfies the same bound rule a
built tree does (every node's box contains every primitive below it), so the
searches on it return what they return on a tree rebuilt over the moved
boxes, and what a test of every primitive returns. For `intersect_aabb`,
which reports whole leaves, the primitives whose own box overlaps the query
are the same set. This holds however badly the motion fits the old tree:
primitives swapping places, collapsing to one point or spreading out. A
refitted tree's boxes can overlap far more than a rebuilt tree's, so its
searches can do more work. That degradation is expected and is not measured
here: no timing or quality measure is part of this contract, and when to
rebuild is the caller's decision.

**The compact layout.** Refit applies to the standard layout only. A compact
tree from `compact_bvh` is not refitted in place; convert the refitted tree
again with `compact_bvh`.

### Not Covered

Refit keeps the topology. Inserting or removing primitives, changing a
tree's shape after a motion, and deciding when to rebuild are not provided.
