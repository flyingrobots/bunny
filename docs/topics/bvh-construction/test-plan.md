# BVH Construction Test Plan

This document defines how Bunny verifies the BVH construction contract: the
shared buffer and node format, the quartile builder `build_bvh` and the binned
SAH builder `build_bvh_sah`.

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
- tree quality (SAH cost) independent of timing.

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
| BVH-TP-021 | Allocation | BVH-REQ-001 | A counting allocator sees zero allocations during a build of 240 boxes, 40 of them identical. | `crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs::bvh_tp_021_build_bvh_sah_allocates_zero_times` |
| BVH-TP-020 | Mutations | BVH-REQ-005, BVH-REQ-006 | The partition and the cost comparison, broken on purpose, make the cases above fail. | [Mutation checks](#mutation-checks) |
| BVH-TP-022 | Depth cap reached | BVH-REQ-010, BVH-REQ-008 | 70 collinear point boxes inside the trace domain, at `2^40 - 1 - d_k` raw with `d_0 = 0` and `d_(k+1) = ceil(16 (d_k + 1) / 15)`: every node's lowest point is alone in the lowest bin and every cost is zero, so each level splits off one point and the tree is exactly 60 deep (a literal), with primitives 0 to 9 in one capped leaf at depth 60. The tree is well formed, needs at most 61 stack entries, and closest_hit, any_hit, intersect_aabb and the compact searches equal a test of every primitive. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_022_seventy_collinear_points_reach_the_depth_cap` |
| BVH-TP-023 | The review's witness | BVH-REQ-010 | Point boxes at raw `17^k` (k = 0..14) on each axis in turn plus 2^18 identical boxes at the origin (262 189 primitives) build a valid tree of depth 60 whose 32 768 leaves at depth 60 hold 8 primitives each. Measured once; its 14 s debug build is too slow for the fast tier, and BVH-TP-022 kills the same mutations. | [Depth cap evidence](#depth-cap-evidence) |
| BVH-TP-024 | Quartile depth inside the domain | BVH-REQ-011, BVH-REQ-004 | 94 collinear point boxes inside the trace domain at `2^40 - 1 - 2^32 - d_k` raw with `d_0 = 0` and `d_(k+1) = ceil(4 (d_k + 1) / 3)`: every cost is zero and the first quartile plane leaves only the farthest point on its left, so build_bvh splits off one point per level until four remain, a tree exactly 90 deep. 94 thin triangles in the planes `x` of those points (bounds one raw unit in y and z) give a tree 70 deep (recorded from the frozen builder). On both, intersect_aabb, any_hit and closest_hit along `-x` return StackOverflow, and closest_hit along `+x` equals a test of every primitive. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_024_build_bvh_exceeds_the_traversal_depth_inside_the_trace_domain` |
| BVH-TP-025 | Depth mutations | BVH-REQ-010 | `SAH_MAX_DEPTH = 64` fails BVH-TP-022 and a build stack of `SAH_MAX_DEPTH` entries fails BVH-TP-026. | [Mutation checks](#mutation-checks) |
| BVH-TP-026 | Build stack at the cap | BVH-REQ-010 | The review's chains extended to `17^k` raw for `k` from 0 to 15 on each axis in turn (outside the trace domain), plus 2^14 identical boxes at the origin (16 432 boxes): every chain point is split off to the right and every halving keeps the second half waiting, so 59 right children wait above the deepest interior node and the build needs all 61 entries of its work stack. The tree is well formed and 60 deep (literals), and a box query reaches every primitive. About 0.9 s in a debug build. | `crates/bunny-broadphase/tests/bvh_depth_tests.rs::bvh_tp_026_a_left_spine_to_the_cap_fills_the_sah_build_stack` |

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
oracle = "dhat counts zero heap blocks during the build."
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
```

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
