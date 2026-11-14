# BVH Construction

This chapter defines Bunny's current contract for building bounding volume
hierarchies in `bunny-broadphase`: the flat node format both builders write,
the quartile builder `build_bvh`, and the binned surface-area-heuristic (SAH)
builder `build_bvh_sah`. The traversals that read these trees
(`intersect_aabb`, `intersect_ray`, `closest_hit`, `any_hit`) are described in
the [ray-tracing chapter](../ray-tracing/README.md#closest-hit-traversal).

## Status

This is a living topic chapter, not a proposal.

Both builders are implemented in `crates/bunny-broadphase/src/bvh/` and
exported from `bunny_broadphase` as `build_bvh` and `build_bvh_sah`, with
`BvhNode`. The SAH builder's parameters are public constants in
`bunny_broadphase::bvh`: `SAH_BIN_COUNT`, `SAH_MAX_LEAF_SIZE`,
`SAH_MAX_DEPTH`, `SAH_TRAVERSAL_COST` and `SAH_INTERSECTION_COST`.

Evidence is listed in the [test plan](test-plan.md). Timings of both builders
and of traversal through their trees are in the
[performance ledger](../performance/README.md#6-binned-sah-bvh-builder-new-api).

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
tree within the traversals' fixed 64-entry stacks: a leaf at depth 60 needs at
most 61 entries.

The cap is a guarantee, not a tuning knob: with 16 equal-width bins every
split narrows the chosen axis, so ordinary inputs stay far shallower (18
levels on the 27 650-triangle benchmark scene). A unit test lowers the cap to
reach it.

### Cost

Each node makes one pass to find its bounds and centroid spans, one to bin, a
sweep over 16 bins per axis, and a partition with two in-place sorts. Build
time and traversal time on the benchmark scene are in the
[performance ledger](../performance/README.md#6-binned-sah-bvh-builder-new-api).

## Choosing A Builder

Both builders' trees give the same `closest_hit` and `any_hit` answers for
the same primitives, because those answers do not depend on the tree's shape
([ray-tracing chapter](../ray-tracing/README.md#correctness-the-answer-does-not-depend-on-the-tree)).
The visitor traversals `intersect_ray` and `intersect_aabb` report every
primitive in each leaf they reach, so which primitives they report, and in
what order, depends on the tree. A tree from one builder is not a substitute
for golden data recorded with the other.
