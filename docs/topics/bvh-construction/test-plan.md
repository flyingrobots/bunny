# BVH Construction Test Plan

This document defines how Bunny verifies the BVH construction contract: the
shared buffer and node format, the quartile builder `build_bvh`, the binned
SAH builder `build_bvh_sah`, and `refit_bvh`.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design: it records requirements, executable evidence and
known gaps.

## Scope

- the buffer contract both builders share (sizes, refusals, empty input);
- the structure of a built tree (reachability, child and leaf ranges, exact
  bounds, every primitive in exactly one leaf, depth);
- determinism of both builders, locked by golden fingerprints that run on
  every CI platform;
- the SAH builder's tie rules, leaf rule, depth cap, degenerate inputs and
  overflow freedom;
- agreement of `closest_hit` and `any_hit` on SAH trees with a test of every
  primitive and with the quartile tree;
- tree quality (SAH cost) independent of timing;
- `refit_bvh`: the builders' node order it relies on, its exact post-condition,
  its refusals and all-or-nothing rule, and equal query answers on refitted
  and rebuilt trees (refit quality is not measured).

Timing is covered by the [performance chapter](../performance/README.md); the
traversals' own contract by the [ray-tracing test plan](../ray-tracing/test-plan.md).

## Test Goals

- Compare built trees against oracles that do not share the builder's code: a
  recursive structural walk that recomputes every node's bounds from its
  primitives, hand-derived layouts for small inputs, and brute-force ray
  tests.
- Assert exact raw values and exact layouts, never approximate ones.
- Lock the full output bytes of both builders with golden fingerprints, so a
  change to either is a visible contract change.
- Break each tie rule and the partition on purpose and record which case fails.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| BVH-REQ-001 | Both builders refuse buffers smaller than 2n - 1 nodes or n indices, return Some(0) for no primitives, accept larger buffers, and allocate nothing. | `README.md#buffers-and-node-format` |
| BVH-REQ-002 | A built tree is well formed: every node reachable once, valid child and leaf ranges, every primitive in exactly one leaf, node bounds exactly the union of their primitives, at most 2n - 1 nodes. | `README.md#buffers-and-node-format` |
| BVH-REQ-003 | Each builder's output is a deterministic function of its input, identical on every target. | `README.md#buffers-and-node-format` |
| BVH-REQ-004 | build_bvh's output does not change. | `README.md#the-quartile-builder` |
| BVH-REQ-005 | build_bvh_sah breaks ties toward the lower axis and then the lower plane, and makes a leaf when a small node's split costs no less than the leaf. | `README.md#decisions-and-ties` |
| BVH-REQ-006 | build_bvh_sah splits nodes above SAH_MAX_LEAF_SIZE (by primitive order when all centroids are equal), keeps leaves in ascending index order, and never builds deeper than SAH_MAX_DEPTH. | `README.md#decisions-and-ties` |
| BVH-REQ-007 | build_bvh_sah never overflows or panics, including boxes at the ends of the Q32.32 range. | `README.md#arithmetic-and-overflow` |
| BVH-REQ-008 | closest_hit and any_hit on a build_bvh_sah tree equal a test of every primitive and the answers on the build_bvh tree. | `README.md#choosing-a-builder` |
| BVH-REQ-009 | On the seeded quality scenes, the SAH tree's SAH cost is below the quartile tree's. | `README.md#costs` |
| BVH-REQ-010 | build_bvh_sah builds no tree deeper than 60 (the root at depth 0), so its trees need at most 61 of the traversals' 64 stack entries: a node at depth 60 is a leaf of whatever size (a capped leaf, which can hold more than SAH_MAX_LEAF_SIZE primitives), and an input inside the trace domain reaches it. | `README.md#depth-and-stack` |
| BVH-REQ-011 | build_bvh has no depth cap: inputs inside the trace domain build trees deeper than 63, on which a traversal returns its complete answer or StackOverflow, never another answer. | `README.md#depth` |
| BVH-REQ-020 | refit_bvh takes the used node slice mutably, the primitive-index buffer immutably and one box per primitive, needs no scratch buffer, does not recurse and allocates nothing, including when it refuses. | `README.md#refit-buffers` |
| BVH-REQ-021 | After a refit every leaf's bounds are exactly the raw per-axis union of its listed primitives' new boxes and every interior node's bounds exactly the union of its children's; inverted and extreme boxes the builders accept are accepted and unioned the same way. | `README.md#post-condition` |
| BVH-REQ-022 | Refit changes only bounds: topology, leaf ranges and prim_indices are unchanged, refit to the build input is the identity bit for bit, and refit to other boxes and back restores the tree bit for bit. | `README.md#post-condition` |
| BVH-REQ-023 | Both builders store every interior node's first child at a larger index than the node, and write one more leaf than interior nodes, so one pass from the last node to the first refits a tree. | `README.md#order-and-the-single-pass` |
| BVH-REQ-024 | Refit refuses, with a typed RefitError in the stated order, children outside the slice, a child at or before its parent, a leaf range outside prim_indices, a leaf count other than interior plus one, a box count other than the listed slots, and a listed primitive with no box. | `README.md#refusals-and-all-or-nothing` |
| BVH-REQ-025 | A refused refit writes nothing: the buffer is checked whole before the writing pass. | `README.md#refusals-and-all-or-nothing` |
| BVH-REQ-026 | On seeded corpora and motions, including ones that degrade the tree, closest_hit, any_hit, the compact searches on a re-converted tree, intersect_aabb filtered by primitive and the overlapping pairs give the same answers on refitted trees as on trees rebuilt by each builder, equal to a test of every primitive and to sweep_and_prune. | `README.md#what-refit-preserves` |

```toml
[[requirement]]
id = "BVH-REQ-001"
summary = "Both builders refuse buffers smaller than 2n - 1 nodes or n indices, return Some(0) for no primitives, accept larger buffers, and allocate nothing."
status = "active"

[[requirement]]
id = "BVH-REQ-002"
summary = "A built tree is well formed: every node reachable once, valid child and leaf ranges, every primitive in exactly one leaf, node bounds exactly the union of their primitives, at most 2n - 1 nodes."
status = "active"

[[requirement]]
id = "BVH-REQ-003"
summary = "Each builder's output is a deterministic function of its input, identical on every target."
status = "active"

[[requirement]]
id = "BVH-REQ-004"
summary = "build_bvh's output does not change."
status = "active"

[[requirement]]
id = "BVH-REQ-005"
summary = "build_bvh_sah breaks ties toward the lower axis and then the lower plane, and makes a leaf when a small node's split costs no less than the leaf."
status = "active"

[[requirement]]
id = "BVH-REQ-006"
summary = "build_bvh_sah splits nodes above SAH_MAX_LEAF_SIZE (by primitive order when all centroids are equal), keeps leaves in ascending index order, and never builds deeper than SAH_MAX_DEPTH."
status = "active"

[[requirement]]
id = "BVH-REQ-007"
summary = "build_bvh_sah never overflows or panics, including boxes at the ends of the Q32.32 range."
status = "active"

[[requirement]]
id = "BVH-REQ-008"
summary = "closest_hit and any_hit on a build_bvh_sah tree equal a test of every primitive and the answers on the build_bvh tree."
status = "active"

[[requirement]]
id = "BVH-REQ-009"
summary = "On the seeded quality scenes, the SAH tree's SAH cost is below the quartile tree's."
status = "active"

[[requirement]]
id = "BVH-REQ-010"
summary = "build_bvh_sah builds no tree deeper than 60 (the root at depth 0), so its trees need at most 61 of the traversals' 64 stack entries: a node at depth 60 is a leaf of whatever size (a capped leaf, which can hold more than SAH_MAX_LEAF_SIZE primitives), and an input inside the trace domain reaches it."
status = "active"

[[requirement]]
id = "BVH-REQ-011"
summary = "build_bvh has no depth cap: inputs inside the trace domain build trees deeper than 63, on which a traversal returns its complete answer or StackOverflow, never another answer."
status = "active"

[[requirement]]
id = "BVH-REQ-020"
summary = "refit_bvh takes the used node slice mutably, the primitive-index buffer immutably and one box per primitive, needs no scratch buffer, does not recurse and allocates nothing, including when it refuses."
status = "active"

[[requirement]]
id = "BVH-REQ-021"
summary = "After a refit every leaf's bounds are exactly the raw per-axis union of its listed primitives' new boxes and every interior node's bounds exactly the union of its children's; inverted and extreme boxes the builders accept are accepted and unioned the same way."
status = "active"

[[requirement]]
id = "BVH-REQ-022"
summary = "Refit changes only bounds: topology, leaf ranges and prim_indices are unchanged, refit to the build input is the identity bit for bit, and refit to other boxes and back restores the tree bit for bit."
status = "active"

[[requirement]]
id = "BVH-REQ-023"
summary = "Both builders store every interior node's first child at a larger index than the node, and write one more leaf than interior nodes, so one pass from the last node to the first refits a tree."
status = "active"

[[requirement]]
id = "BVH-REQ-024"
summary = "Refit refuses, with a typed RefitError in the stated order, children outside the slice, a child at or before its parent, a leaf range outside prim_indices, a leaf count other than interior plus one, a box count other than the listed slots, and a listed primitive with no box."
status = "active"

[[requirement]]
id = "BVH-REQ-025"
summary = "A refused refit writes nothing: the buffer is checked whole before the writing pass."
status = "active"

[[requirement]]
id = "BVH-REQ-026"
summary = "On seeded corpora and motions, including ones that degrade the tree, closest_hit, any_hit, the compact searches on a re-converted tree, intersect_aabb filtered by primitive and the overlapping pairs give the same answers on refitted trees as on trees rebuilt by each builder, equal to a test of every primitive and to sweep_and_prune."
status = "active"
```

## Fixtures

No external fixtures. Inputs are constructed boxes and seeded scenes built in
the tests from a 64-bit linear congruential generator; every seed is a named
constant. The structural checker, fingerprint and SAH cost live in
`crates/bunny-broadphase/tests/common/mod.rs`.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| BVH-TP-001 | Buffers | BVH-REQ-001 | No primitives give Some(0). | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_001_empty_input_uses_no_nodes` |
| BVH-TP-002 | Buffers | BVH-REQ-001, BVH-REQ-002 | One primitive gives one leaf with its exact box. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_002_one_primitive_is_one_leaf` |
| BVH-TP-003 | Buffers | BVH-REQ-001 | 2n - 2 nodes, n - 1 indices, and no nodes for one primitive are refused; exactly 2n - 1 and larger buffers build. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_003_buffers_too_small_are_refused` |
| BVH-TP-004 | Structure | BVH-REQ-002, BVH-REQ-006 | The structural walk accepts SAH trees over scattered, clustered and grid-on-ground scenes of 2 to 600 boxes, with no leaf above the leaf size. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_004_seeded_trees_are_well_formed` |
| BVH-TP-005 | Determinism | BVH-REQ-003 | Two builds, from a copy of the input and into buffers holding stale values, give identical words. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_005_building_twice_gives_the_same_tree` |
| BVH-TP-006 | Degenerate | BVH-REQ-006 | 1 000 identical boxes split in halves by index to depth 8 with leaves of at most four; ten give the hand-derived layout 5 / 5, then 2 / 3. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_006_identical_boxes_split_in_half_by_index` |
| BVH-TP-007 | Degenerate | BVH-REQ-002, BVH-REQ-006 | Nested boxes with one centroid, flat boxes with zero z extent, points, and points on a line all give well-formed trees. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_007_equal_centroids_and_zero_extent_axes_build` |
| BVH-TP-008 | Degenerate | BVH-REQ-002, BVH-REQ-009 | Two 40-unit ground boxes among 2 000 small ones give a well-formed tree with a lower SAH cost than the quartile tree. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_008_one_huge_primitive_among_small_ones` |
| BVH-TP-009 | Overflow | BVH-REQ-007 | Boxes spanning the whole Q32.32 range and touching both ends build a well-formed tree, and an everywhere query visits every primitive. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_009_boxes_at_the_ends_of_the_range_build_without_overflow` |
| BVH-TP-010 | Depth | BVH-REQ-006, BVH-REQ-010 | Points at 2^k raw units on each axis stay within depth 60 (a literal, not `SAH_MAX_DEPTH`) and the 64-entry traversal stack. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_010_geometric_spacing_stays_within_the_depth_cap` |
| BVH-TP-011 | Ties | BVH-REQ-005 | Two segments with zero area everywhere (split cost equals leaf cost) make one leaf; two cubes, 13 against 14, split. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_011_equal_split_and_leaf_costs_make_a_leaf` |
| BVH-TP-012 | Ties | BVH-REQ-005 | Three boxes on x whose planes all cost 52: the lowest plane wins and box 0 is split off first (hand-derived layout and bounds). | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_012_equal_planes_on_one_axis_choose_the_lowest_plane` |
| BVH-TP-013 | Ties | BVH-REQ-005 | Four cubes on a square: x wins the tie with y (indices 0, 2, 1, 3); stretched along y, y wins (0, 1, 2, 3). | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_013_equal_axes_choose_x_before_y` |
| BVH-TP-014 | Rays | BVH-REQ-008 | Over 5 000 seeded rays and scenes of 40 and 600 triangles with two ground triangles and duplicates, closest_hit and any_hit on both trees equal a test of every triangle. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_014_closest_and_any_hit_on_the_sah_tree_equal_every_primitive_and_the_old_tree` |
| BVH-TP-015 | Quality | BVH-REQ-009 | The SAH cost (the builder's own weights, exact areas) of the SAH tree is below the quartile tree's on three seeded scenes; the values are printed. | `crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_015_sah_cost_is_lower_than_the_quartile_builder_on_seeded_scenes` |
| BVH-TP-016 | Depth | BVH-REQ-006 | With the cap lowered to 3, every leaf is at depth 3 or less and some leaf is above the leaf size; uncapped, none is. | `crates/bunny-broadphase/src/bvh/sah/tests/mod.rs::bvh_tp_016_nodes_at_the_depth_cap_become_leaves_whatever_their_size` |
| BVH-TP-017 | Overflow | BVH-REQ-007 | The widest box's scaled area is exactly 3 * (2^45 - 1)^2; its weighted area at u32::MAX primitives, split cost and leaf cost do not overflow; below 2^45 areas are exact. | `crates/bunny-broadphase/src/bvh/sah/tests/mod.rs::bvh_tp_017_costs_of_the_widest_boxes_and_largest_counts_do_not_overflow` |
| BVH-TP-018 | Golden | BVH-REQ-003 | Node count and FNV-1a fingerprint of the SAH tree over 520 seeded boxes. | `crates/bunny-broadphase/tests/bvh_sah_golden.rs::bvh_tp_018_sah_tree_bytes_golden_vector` |
| BVH-TP-019 | Golden | BVH-REQ-003, BVH-REQ-004 | Node count and FNV-1a fingerprint of the quartile tree over the same boxes. | `crates/bunny-broadphase/tests/bvh_sah_golden.rs::bvh_tp_019_quartile_tree_bytes_golden_vector` |
| BVH-TP-021 | Allocation | BVH-REQ-001 | A counting allocator sees zero allocations by the building thread during a build of 240 boxes, 40 of them identical ([measurement rule](#allocation-measurement-rule)). | `crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs::bvh_tp_021_build_bvh_sah_allocates_zero_times` |
| BVH-TP-020 | Mutations | BVH-REQ-005, BVH-REQ-006 | The partition and the cost comparison, broken on purpose, make the cases above fail. | [Mutation checks](#mutation-checks) |
| BVH-TP-022 | Depth cap reached | BVH-REQ-010, BVH-REQ-008 | 70 collinear point boxes inside the trace domain, at `2^40 - 1 - d_k` raw with `d_0 = 0` and `d_(k+1) = ceil(16 (d_k + 1) / 15)`: every node's lowest point is alone in the lowest bin and every cost is zero, so each level splits off one point and the tree is exactly 60 deep (a literal), with primitives 0 to 9 in one capped leaf at depth 60. The tree is well formed, needs at most 61 stack entries, and closest_hit, any_hit, intersect_aabb and the compact searches equal a test of every primitive. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_022_seventy_collinear_points_reach_the_depth_cap` |
| BVH-TP-023 | The review's witness | BVH-REQ-010 | Point boxes at raw `17^k` (k = 0..14) on each axis in turn plus 2^18 identical boxes at the origin (262 189 primitives) build a valid tree of depth 60 whose 32 768 leaves at depth 60 hold 8 primitives each. Measured once; its 14 s debug build is too slow for the fast tier, and BVH-TP-022 kills the same mutations. | [Depth cap evidence](#depth-cap-evidence) |
| BVH-TP-024 | Quartile depth inside the domain | BVH-REQ-011, BVH-REQ-004 | 94 collinear point boxes inside the trace domain at `2^40 - 1 - 2^32 - d_k` raw with `d_0 = 0` and `d_(k+1) = ceil(4 (d_k + 1) / 3)`: every cost is zero and the first quartile plane leaves only the farthest point on its left, so build_bvh splits off one point per level until four remain, a tree exactly 90 deep. 94 thin triangles in the planes `x` of those points (bounds one raw unit in y and z) give a tree 70 deep (recorded from the frozen builder). On both, intersect_aabb, any_hit and closest_hit along `-x` return StackOverflow, and closest_hit along `+x` equals a test of every primitive. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_024_build_bvh_exceeds_the_traversal_depth_inside_the_trace_domain` |
| BVH-TP-027 | Measurement rule | BVH-REQ-001 | A second thread allocates in a loop (at least 1 000 blocks, guaranteed to fall inside the window) while a scope is measured: the scope counts 0. | `crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs::tscope_tp_001_foreign_thread_allocations_are_not_counted` |
| BVH-TP-028 | Measurement rule | BVH-REQ-001 | The measuring thread's own allocation is counted: one `Vec` with capacity is one block. | `crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs::tscope_tp_002_own_thread_allocations_are_counted` |
| BVH-TP-025 | Depth mutations | BVH-REQ-010 | `SAH_MAX_DEPTH = 64` fails BVH-TP-022 and a build stack of `SAH_MAX_DEPTH` entries fails BVH-TP-026. | [Mutation checks](#mutation-checks) |
| BVH-TP-026 | Build stack at the cap | BVH-REQ-010 | The review's chains extended to `17^k` raw for `k` from 0 to 15 on each axis in turn (outside the trace domain), plus 2^14 identical boxes at the origin (16 432 boxes): every chain point is split off to the right and every halving keeps the second half waiting, so 59 right children wait above the deepest interior node and the build needs all 61 entries of its work stack. The tree is well formed and 60 deep (literals), and a box query reaches every primitive. About 0.9 s in a debug build. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_026_a_left_spine_to_the_cap_fills_the_sah_build_stack` |
| BVH-TP-040 | Refit order | BVH-REQ-023 | The node indices of each built tree, read directly. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_040_both_builders_store_children_after_their_parent` |
| BVH-TP-041 | Refit post-condition | BVH-REQ-021, BVH-REQ-022 | A recursive walk from the root recomputing every node's box with raw i64 min and max written in the test, compared exactly, reaching every node once. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_041_refit_gives_the_exact_union_and_keeps_topology_and_order` |
| BVH-TP-042 | Refit identity | BVH-REQ-021, BVH-REQ-022 | The builders' own output, word for word. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_042_refit_to_the_build_input_is_the_identity_bit_for_bit` |
| BVH-TP-043 | Refit round trip | BVH-REQ-022 | The built tree, word for word. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_043_refit_and_refit_back_restores_the_tree_bit_for_bit` |
| BVH-TP-044 | Refit empty and hand-built | BVH-REQ-020, BVH-REQ-021, BVH-REQ-024 | The validator, and the expected error value. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_044_an_empty_tree_refits_only_to_no_boxes` |
| BVH-TP-045 | Refit box count | BVH-REQ-024, BVH-REQ-025 | Hand-derived counts. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_045_a_box_count_other_than_the_listed_slots_is_refused` |
| BVH-TP-046 | Refit child range | BVH-REQ-024, BVH-REQ-025 | Hand-derived boundary. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_046_children_outside_the_slice_are_refused` |
| BVH-TP-047 | Refit child order | BVH-REQ-024, BVH-REQ-025 | Hand-derived boundary, and the builder's returned node count. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_047_a_child_at_or_before_its_parent_is_refused` |
| BVH-TP-048 | Refit leaf range | BVH-REQ-024, BVH-REQ-025 | Hand-derived boundary. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_048_a_leaf_range_outside_the_index_buffer_is_refused` |
| BVH-TP-049 | Refit primitive index | BVH-REQ-024, BVH-REQ-025 | Hand-derived boundary and the validator. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_049_a_primitive_index_outside_the_boxes_is_refused` |
| BVH-TP-050 | Refit leaf count | BVH-REQ-024, BVH-REQ-025 | Hand-derived counts. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_050_a_leaf_count_other_than_interior_plus_one_is_refused` |
| BVH-TP-051 | Refit error order | BVH-REQ-024 | The stated order. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_051_the_lowest_faulty_node_and_structure_before_counts_are_reported` |
| BVH-TP-052 | Refit all-or-nothing | BVH-REQ-025 | The tree's words before the call. | `crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_052_a_refused_refit_of_a_built_tree_writes_nothing` |
| BVH-TP-053 | Refit rays | BVH-REQ-026, BVH-REQ-022 | A test of every triangle with hit_triangle, nearest then lowest index. | `crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_053_ray_searches_on_refitted_and_rebuilt_trees_equal_every_primitive` |
| BVH-TP-054 | Refit box queries | BVH-REQ-026 | Closed-interval overlap on raw values, written in the test, over every primitive. | `crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_054_box_queries_on_refitted_and_rebuilt_trees_equal_every_primitive` |
| BVH-TP-055 | Refit pairs | BVH-REQ-026 | Every pair tested with the raw overlap written in the test. | `crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_055_overlapping_pairs_from_refitted_and_rebuilt_trees_equal_sweep_and_prune` |
| BVH-TP-056 | Refit allocation | BVH-REQ-020 | allocation_counter::measure on the calling thread. | `crates/bunny-broadphase/tests/refit_allocation_tests.rs::bvh_tp_056_refit_allocates_zero_times` |
| BVH-TP-057 | Refit mutations | BVH-REQ-021, BVH-REQ-024, BVH-REQ-025 | Eleven deliberate mutations of `refit.rs`, each failing at least one case. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "BVH-TP-001"
requirements = ["BVH-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_001_empty_input_uses_no_nodes"
oracle = "No primitives give Some(0)."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-002"
requirements = ["BVH-REQ-001", "BVH-REQ-002"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_002_one_primitive_is_one_leaf"
oracle = "One primitive gives one leaf with its exact box."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-003"
requirements = ["BVH-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_003_buffers_too_small_are_refused"
oracle = "Short node or index buffers are refused; exact and larger buffers build."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-004"
requirements = ["BVH-REQ-002", "BVH-REQ-006"]
evidence = "property"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_004_seeded_trees_are_well_formed"
oracle = "A recursive walk that recomputes bounds accepts every seeded SAH tree."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-005"
requirements = ["BVH-REQ-003"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_005_building_twice_gives_the_same_tree"
oracle = "Two builds give identical words."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-006"
requirements = ["BVH-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_006_identical_boxes_split_in_half_by_index"
oracle = "Hand-derived halving layout and depth."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-007"
requirements = ["BVH-REQ-002", "BVH-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_007_equal_centroids_and_zero_extent_axes_build"
oracle = "The structural walk accepts trees over equal centroids, flat boxes and points."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-008"
requirements = ["BVH-REQ-002", "BVH-REQ-009"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_008_one_huge_primitive_among_small_ones"
oracle = "The structural walk and the exact SAH cost."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-009"
requirements = ["BVH-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_009_boxes_at_the_ends_of_the_range_build_without_overflow"
oracle = "The build succeeds, the walk accepts it, and a query reaches every primitive."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-010"
requirements = ["BVH-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_010_geometric_spacing_stays_within_the_depth_cap"
oracle = "Depth at most 60, written as a literal (not the constant under test), and a traversal that does not overflow its stack."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-011"
requirements = ["BVH-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_011_equal_split_and_leaf_costs_make_a_leaf"
oracle = "Hand-derived layouts: one leaf on a tie, a split at 13 against 14."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-012"
requirements = ["BVH-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_012_equal_planes_on_one_axis_choose_the_lowest_plane"
oracle = "Hand-derived layout and bounds for equal plane costs."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-013"
requirements = ["BVH-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_013_equal_axes_choose_x_before_y"
oracle = "Hand-derived layout and index order for equal axis costs."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-014"
requirements = ["BVH-REQ-008"]
evidence = "property"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_014_closest_and_any_hit_on_the_sah_tree_equal_every_primitive_and_the_old_tree"
oracle = "A test of every triangle (nearest, then lowest index) over seeded rays."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-015"
requirements = ["BVH-REQ-009"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_tests.rs::bvh_tp_015_sah_cost_is_lower_than_the_quartile_builder_on_seeded_scenes"
oracle = "The exact SAH cost of both trees over the same primitives."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-016"
requirements = ["BVH-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/src/bvh/sah/tests/mod.rs::bvh_tp_016_nodes_at_the_depth_cap_become_leaves_whatever_their_size"
oracle = "Leaf depths and sizes under a lowered cap."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-017"
requirements = ["BVH-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/src/bvh/sah/tests/mod.rs::bvh_tp_017_costs_of_the_widest_boxes_and_largest_counts_do_not_overflow"
oracle = "Exact scaled areas computed by hand; checked arithmetic returns values."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-018"
requirements = ["BVH-REQ-003"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_golden.rs::bvh_tp_018_sah_tree_bytes_golden_vector"
oracle = "Golden node count and fingerprint."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-019"
requirements = ["BVH-REQ-003", "BVH-REQ-004"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_golden.rs::bvh_tp_019_quartile_tree_bytes_golden_vector"
oracle = "Golden node count and fingerprint."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-021"
requirements = ["BVH-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs::bvh_tp_021_build_bvh_sah_allocates_zero_times"
oracle = "The calling thread allocates zero heap blocks during the build; other threads are not counted."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-022"
requirements = ["BVH-REQ-010", "BVH-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_022_seventy_collinear_points_reach_the_depth_cap"
oracle = "Hand derived from the binning rule: each offset is at least 16/15 of the previous one plus one, so a node's lowest point is alone in bin 0, every half area is zero and the lowest plane wins, peeling one point per level; 70 - 60 = 10 points remain at depth 60. Brute force for the searches."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-023"
requirements = ["BVH-REQ-010"]
evidence = "manual-audit"
artifact = "docs/topics/bvh-construction/test-plan.md#depth-cap-evidence"
oracle = "The structural walk and a count of leaves at depth 60, measured once in the container."
tier = "manual"
status = "implemented"

[[case]]
id = "BVH-TP-024"
requirements = ["BVH-REQ-011", "BVH-REQ-004"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_024_build_bvh_exceeds_the_traversal_depth_inside_the_trace_domain"
oracle = "Hand derived for the points from the quartile rule (each offset is at least 4/3 of the previous one plus one, so the first quartile plane leaves only the farthest point on its left); the triangles' depth 70 is a recorded value of the frozen build_bvh. Brute force over every primitive for the answer that completes."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-025"
requirements = ["BVH-REQ-010"]
evidence = "manual-audit"
artifact = "docs/topics/bvh-construction/test-plan.md#mutation-checks"
oracle = "Each mutant's tests under timeout 120 with every source touched before the build."
tier = "manual"
status = "implemented"

[[case]]
id = "BVH-TP-026"
requirements = ["BVH-REQ-010"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_026_a_left_spine_to_the_cap_fills_the_sah_build_stack"
oracle = "The left-turn count of the built tree (59 waiting right children plus the two pushed children is 61 entries, the stack's documented size) and the structural walk; depth 60 and 59 are recorded values of the input checked against the stack size."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-020"
requirements = ["BVH-REQ-005", "BVH-REQ-006"]
evidence = "manual-audit"
artifact = "docs/topics/bvh-construction/test-plan.md#mutation-checks"
oracle = "Each deliberate mutation makes at least one case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "BVH-TP-027"
requirements = ["BVH-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs::tscope_tp_001_foreign_thread_allocations_are_not_counted"
oracle = "Another thread's allocations, forced into the measurement window, change the count by nothing: it stays 0."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-028"
requirements = ["BVH-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs::tscope_tp_002_own_thread_allocations_are_counted"
oracle = "A deliberate allocation inside the scope on the measuring thread is counted exactly once."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-040"
requirements = ["BVH-REQ-023"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_040_both_builders_store_children_after_their_parent"
oracle = "The node indices of each built tree, read directly."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-041"
requirements = ["BVH-REQ-021", "BVH-REQ-022"]
evidence = "property"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_041_refit_gives_the_exact_union_and_keeps_topology_and_order"
oracle = "A recursive walk from the root recomputing every node's box with raw i64 min and max written in the test, compared exactly, reaching every node once."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-042"
requirements = ["BVH-REQ-021", "BVH-REQ-022"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_042_refit_to_the_build_input_is_the_identity_bit_for_bit"
oracle = "The builders' own output, word for word."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-043"
requirements = ["BVH-REQ-022"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_043_refit_and_refit_back_restores_the_tree_bit_for_bit"
oracle = "The built tree, word for word."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-044"
requirements = ["BVH-REQ-020", "BVH-REQ-021", "BVH-REQ-024"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_044_an_empty_tree_refits_only_to_no_boxes"
oracle = "The validator, and the expected error value."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-045"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_045_a_box_count_other_than_the_listed_slots_is_refused"
oracle = "Hand-derived counts."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-046"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_046_children_outside_the_slice_are_refused"
oracle = "Hand-derived boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-047"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_047_a_child_at_or_before_its_parent_is_refused"
oracle = "Hand-derived boundary, and the builder's returned node count."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-048"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_048_a_leaf_range_outside_the_index_buffer_is_refused"
oracle = "Hand-derived boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-049"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_049_a_primitive_index_outside_the_boxes_is_refused"
oracle = "Hand-derived boundary and the validator."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-050"
requirements = ["BVH-REQ-024", "BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_050_a_leaf_count_other_than_interior_plus_one_is_refused"
oracle = "Hand-derived counts."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-051"
requirements = ["BVH-REQ-024"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_051_the_lowest_faulty_node_and_structure_before_counts_are_reported"
oracle = "The stated order."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-052"
requirements = ["BVH-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_tests.rs::bvh_tp_052_a_refused_refit_of_a_built_tree_writes_nothing"
oracle = "The tree's words before the call."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-053"
requirements = ["BVH-REQ-026", "BVH-REQ-022"]
evidence = "property"
test = "crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_053_ray_searches_on_refitted_and_rebuilt_trees_equal_every_primitive"
oracle = "A test of every triangle with hit_triangle, nearest then lowest index."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-054"
requirements = ["BVH-REQ-026"]
evidence = "property"
test = "crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_054_box_queries_on_refitted_and_rebuilt_trees_equal_every_primitive"
oracle = "Closed-interval overlap on raw values, written in the test, over every primitive."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-055"
requirements = ["BVH-REQ-026"]
evidence = "property"
test = "crates/bunny-broadphase/tests/refit_query_tests.rs::bvh_tp_055_overlapping_pairs_from_refitted_and_rebuilt_trees_equal_sweep_and_prune"
oracle = "Every pair tested with the raw overlap written in the test."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-056"
requirements = ["BVH-REQ-020"]
evidence = "test"
test = "crates/bunny-broadphase/tests/refit_allocation_tests.rs::bvh_tp_056_refit_allocates_zero_times"
oracle = "allocation_counter::measure on the calling thread."
tier = "fast"
status = "implemented"

[[case]]
id = "BVH-TP-057"
requirements = ["BVH-REQ-021", "BVH-REQ-024", "BVH-REQ-025"]
evidence = "manual-audit"
artifact = "docs/topics/bvh-construction/test-plan.md#mutation-checks"
oracle = "Each deliberate mutation of refit.rs makes at least one refit case fail."
tier = "manual"
status = "implemented"
```

## Allocation Measurement Rule

BVH-TP-021 and `bvh_allocation_tests.rs` count heap blocks with
`allocation_counter::measure`, which counts only the allocations made by the
thread that runs the measured closure. A process-wide count would also see the
test harness and any other thread, and failed once in CI with a kernel that
allocated nothing. The same rule holds for the codec's allocation tests
(`ply_allocation_tests.rs`, `ply_ascii_allocation_tests.rs`). The instrument
itself is checked in `thread_scoped_allocation_tests.rs`: besides BVH-TP-027
and BVH-TP-028, a reallocation is counted on the measuring thread
(`tscope_tp_003`), a block freed on another thread changes nothing
(`tscope_tp_004`), and two threads measuring at once see only their own blocks
(`tscope_tp_005`). Inside a scope the tests use only atomics to wait, because
parking a thread allocates.

## Mutation Checks

Each mutation below was applied inside the container (the source in the
repository was not changed), `cargo test -p bunny-broadphase` was run, and the
cases that failed were recorded.

| Mutation | Cases that fail |
| --- | --- |
| Partition: a primitive in the plane's own bin goes left (`<` to `<=` in `Plane::sends_left`) | The builder's count check (the partition must send exactly the sweep's left count left) fails the build: BVH-TP-004, 005, 007, 008, 009, 010, 014, 015, 018 and 021. |
| The same, with the count check also disabled | BVH-TP-004, 005, 007, 008, 014, 015 and the golden BVH-TP-018. |
| Split against leaf: an equal cost splits (`<` to `<=`) | BVH-TP-011. |
| Plane tie: the higher of two equal planes wins | BVH-TP-012. |
| Axis tie: the later of two equal axes wins | BVH-TP-013 and BVH-TP-018. |
| Partition not stable: the two in-place sorts removed | BVH-TP-004, 007, 008, 010, 014 (leaf order) and BVH-TP-018. |
| `SAH_MAX_DEPTH = 64` | BVH-TP-022 (the tree is 64 deep). The independent review found that no earlier case failed. |
| The SAH build stack of `SAH_MAX_DEPTH` entries instead of `SAH_MAX_DEPTH + 1` | BVH-TP-026 (the build returns `None`). The independent review found that no earlier case failed. |
| Leaf rule: nodes up to twice the leaf size may stay leaves | BVH-TP-006 and BVH-TP-007. |

The refit mutations (BVH-TP-057) were applied to `crates/bunny-broadphase/src/refit.rs` on
this machine, `cargo test -p bunny-broadphase --test refit_tests --test refit_query_tests
--test refit_allocation_tests` was run for each, and the source was restored:

| Mutation | Cases that fail |
| --- | --- |
| The writing pass skips the root (`1..` instead of `0..`) | BVH-TP-041, 044, 049, 054, 055. |
| The writing pass runs first to last (parents before children) | BVH-TP-041, 043, 044, 049, 053, 054, 055. |
| A node's new bounds are unioned with its old ones (conservative, but not the post-condition) | BVH-TP-041, 043, 044, 049. |
| No `ChildNotAfterParent` check | BVH-TP-047, 051. |
| The validating pass's result ignored (writing proceeds) | BVH-TP-044, 045, 046, 047, 049, 050, 051, 052. |
| No leaf-count (`NotATree`) check | BVH-TP-050. |
| No box-count check | BVH-TP-044, 045, 051, 052. |
| The primitive-index pass checks against `usize::MAX` | BVH-TP-049, 052. |
| A leaf's bounds from its first primitive only | BVH-TP-041, 042, 043, 044, 053, 054, 055. |
| Child range check `right > len` instead of `right >= len` | BVH-TP-046. |
| Leaf range one slot short | Every refit case. |

## Depth Cap Evidence

BVH-TP-023 was measured once in the rust-dev container, in a debug test build:
point boxes at raw `17^k` (`k` from 0 to 14) on x, then y, then z, plus
`2^18` identical point boxes at the origin, 262 189 boxes in all. The build
took 14.2 s and gave a well-formed tree 60 deep with 32 768 leaves at depth
60, each of 8 primitives (capped leaves: twice the leaf size). It is not a
fast case. BVH-TP-022 and BVH-TP-026 reach the cap in milliseconds and kill
the same two mutations.

The search for small inputs that reach the cap (also measured once) found:
with the `17^k` chains on all three axes the depth grew by one per doubling
of the identical boxes (for `k < 15`: 57 with 2^14, 58 with 2^15, 60 with
2^17; for `k < 16`: 59 with 2^13, 60 with 2^14); adding chains
at `-17^k` lowered the depth (44 with 90 chain points), because a middle
plane then halves the node; and 800 consecutive integer points on one axis
reached 60 with a capped leaf of 11.

## Determinism Obligations And Evidence

The builders and their tests use only integer arithmetic (`i64`, `i128`,
`u64`, `u128`, `usize`) with checked or defined results, in-place slice
sorts of distinct integers, and no floating point, host randomness, time,
filesystem access, maps or parallel scheduling. The golden fingerprints
(BVH-TP-018, BVH-TP-019) run in `cargo test --workspace` on Linux, macOS and
Windows in the `determinism-matrix` CI job and under WebAssembly in
`wasm-headless`, so a target that builds different bytes fails.

The fingerprint of `build_bvh` was recorded on this revision. `build_bvh`'s
source changed only in the visibility of four helpers it shares with the SAH
builder, and its earlier tests (`bvh_builder_tests.rs`,
`bvh_allocation_tests.rs`, `determinism.rs`, `bvh_tests.rs`,
`closest_hit_tests.rs`) pass unmodified.

## Gaps

- Tree quality is measured on synthetic scenes and the benchmark scene only.
