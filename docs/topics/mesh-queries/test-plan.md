# Mesh Queries Test Plan

This document defines how Bunny proves the [mesh queries chapter](README.md): the exact
closest point of a triangle and its box bound in `bunny-query`, the `closest_point` BVH
traversal in `bunny-broadphase`, and `MeshView` and `MeshBvh` in `bunny-mesh`. It answers
flyingrobots/bunny#246.

## Scope

- `PointQuery`, `TrianglePoint` and `POINT_BOX_GAP_LIMIT_RAW`: the domain, the exact
  feature rule, degenerate triangles, the two roundings, the exact comparisons and the
  widths at the edges of the domain; the box bound and its cap.
- `closest_point` and `ClosestPoint`: agreement with a test of every primitive, the tie
  rule, independence from the tree's shape and leaf order, each pruning rule, malformed and
  deep trees, and allocation.
- `MeshView`, `MeshBvh` and `MeshError`: refusals and their order, the triangle order, both
  searches through the layout, and the committed Stanford bunny fixture.

Out of scope: the ray kernels and `closest_hit` themselves (the ray-tracing test plan) and
timing (no performance claim is made).

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| MQ-REQ-001 | `PointQuery::new` accepts a point whose raw coordinates all have `abs(raw) < 2^40` and returns `None` otherwise; `MeshView::new` refuses a vertex buffer with any vertex outside that domain. | `README.md#domain` |
| MQ-REQ-002 | `closest_on_triangle` finds the exact closest point of the closed, two-sided triangle (edges and corners included): the plane projection when its three weights are at least zero, else the nearest point of the three closed edges; a triangle with collinear or coincident vertices is the segment or point they span. | `README.md#closest-point-on-a-triangle` |
| MQ-REQ-003 | `TrianglePoint::point` is each coordinate of the exact closest point rounded once to nearest, ties to even (within half a raw unit per axis); `distance_squared` is the exact squared distance to the exact point over `2^32`, rounded once to nearest, ties to even. | `README.md#closest-point-on-a-triangle` |
| MQ-REQ-004 | `cmp_distance` and `cmp_bound` compare exact squared distances, with no rounding; every intermediate fits its stated width for inputs in the trace domain, so no input wraps or panics. | `README.md#widths` |
| MQ-REQ-005 | `box_distance_bound` is the exact squared distance from the point to the box while every gap is below `2^42` raw, caps larger gaps at `2^42`, and is never above the squared distance to any point of the box. | `README.md#box-bound` |
| MQ-REQ-006 | `closest_point` returns what a test of every primitive returns, bit for bit: the exactly nearest primitive, the lowest index of exactly equidistant ones, with its rounded point and squared distance, whatever the tree's shape and leaf order. | `README.md#correctness-the-answer-does-not-depend-on-the-tree` |
| MQ-REQ-007 | `closest_point` skips a subtree only when the best point is strictly nearer than its box bound, keeps one whose bound equals the best distance, searches the nearer child first and allocates nothing. | `README.md#search-order-and-pruning` |
| MQ-REQ-008 | `closest_point` reports malformed and too-deep trees with the `TraversalError` values and conditions of `closest_hit`, returns `None` for an empty tree, and ignores primitives the closure leaves out. | `README.md#search-order-and-pruning` |
| MQ-REQ-009 | `MeshView::new` refuses an empty index buffer, more than `u32::MAX` triangles, a vertex outside the trace domain and an index past the vertex buffer, in that order and naming the first offender; `write_bounds` refuses a short buffer and writes nothing; `MeshBvh::new` refuses a primitive index that names no triangle. | `README.md#mesh-view` |
| MQ-REQ-010 | Triangle `i` of the index buffer, at either width, is BVH primitive `i`; `MeshBvh::closest_hit` and `MeshBvh::closest_point` equal a test of every triangle, on constructed meshes and on the committed Stanford bunny fixture. | `README.md#mesh-view` |

```toml
[[requirement]]
id = "MQ-REQ-001"
summary = "PointQuery::new accepts abs(raw) < 2^40 per coordinate and refuses the rest; MeshView::new refuses any vertex outside the trace domain."
status = "active"

[[requirement]]
id = "MQ-REQ-002"
summary = "closest_on_triangle finds the exact closest point of the closed triangle; collinear or coincident vertices give the segment or point they span."
status = "active"

[[requirement]]
id = "MQ-REQ-003"
summary = "point() and distance_squared() are the exact values rounded once to nearest, ties to even: within half a raw unit."
status = "active"

[[requirement]]
id = "MQ-REQ-004"
summary = "cmp_distance and cmp_bound are exact; every intermediate fits its stated width in the trace domain; nothing wraps or panics."
status = "active"

[[requirement]]
id = "MQ-REQ-005"
summary = "box_distance_bound is exact below the 2^42 gap cap, capped above it, and never above the squared distance to a point of the box."
status = "active"

[[requirement]]
id = "MQ-REQ-006"
summary = "closest_point equals a test of every primitive bit for bit, lowest index on exact ties, for every tree shape and leaf order."
status = "active"

[[requirement]]
id = "MQ-REQ-007"
summary = "closest_point skips a subtree only when the best is strictly nearer than its bound, searches nearer first, allocates nothing."
status = "active"

[[requirement]]
id = "MQ-REQ-008"
summary = "closest_point reports malformed and too-deep trees as closest_hit does, finds nothing in an empty tree, ignores primitives left out."
status = "active"

[[requirement]]
id = "MQ-REQ-009"
summary = "MeshView, write_bounds and MeshBvh refuse bad input with typed MeshError values in a stated order, naming the first offender."
status = "active"

[[requirement]]
id = "MQ-REQ-010"
summary = "Triangle i of the index buffer is primitive i at both widths; the mesh searches equal a test of every triangle, the Stanford fixture included."
status = "active"
```

## Oracles

`crates/bunny-query/tests/support/point_triangle_oracle.rs` is written apart from the kernel
and shares no helper with it. It uses arbitrary-size sign-magnitude integers and unreduced
rationals; it solves the face weights from the 2x2 Gram system
`[e1.e1 e1.e2; e1.e2 e2.e2] [v; w] = [e1.s; e2.s]` by Cramer's rule (the kernel uses the
signs of triple products); it takes the minimum over all seven candidates (three vertices,
the feet on three edges and the face point), each squared distance summed from the
rational point (the kernel uses closed forms over one denominator); and it checks rounding
by comparing twice the error with one (the kernel divides). The traversal cases use a test
of every primitive with the exact comparison and the lowest index, which shares the
per-triangle kernel and none of the tree logic; the kernel itself is proven by the oracle.

## Fixtures

- Seeds: `0xB002_0246_0000_0001` (MQ-TP-006, 3 000 cases; MQ-TP-007 uses it XOR `0xB0C5`,
  2 000 boxes), `0xB002_0246_0000_0002` (MQ-TP-010 to MQ-TP-014; 13 and 14 XOR their
  number), `0xB002_0246_0000_0003` (MQ-TP-022, 300 cases) and `0xB002_0246_0000_0004`
  (MQ-TP-030, 128 rays and 64 points), each drawn with a 64-bit LCG. Every failure message
  names the seed and the case.
- `crates/bunny-codec/tests/fixtures/stanford-bunny-ascii/bun_zipper_res3.ply`, the
  committed Stanford bunny (see its README). It is excluded from the published
  `bunny-codec` package, so `mesh_bvh_stanford_tests.rs` is excluded with it.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| MQ-TP-001 | Boundary | MQ-REQ-001 | On every axis `2^40 - 1` and `-(2^40 - 1)` raw are accepted, `2^40`, `-2^40` and `i64::MIN` refused; the accepted point reads back unchanged. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_001_query_points_outside_the_trace_domain_are_refused` |
| MQ-TP-002 | Hand-derived | MQ-REQ-002, MQ-REQ-003 | A right triangle of 4 units: a point above the face, on the face, on an edge, on a vertex, in each vertex region and in two edge regions give hand-computed points and squared distances, and agree with the oracle. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_002_each_region_gives_its_hand_derived_point` |
| MQ-TP-003 | Ties | MQ-REQ-003 | Exact points at `1/2`, `3/2` and `-3/2` raw round to even (0, 2, -2); thirds round to nearest; squared distances of exactly `1/2` and `3/2` raw (as Q32.32) round to 0 and 2, one raw unit squared above `1/2` to 1, and `46 340^2` and `46 341^2` raw squared to 0 and 1. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_003_coordinates_and_squared_distances_round_to_nearest_ties_to_even` |
| MQ-TP-004 | Degenerate | MQ-REQ-002 | Collinear triangles with the third vertex between, beyond and before the other two, two coincident vertices, three coincident vertices and a sliver with a normal of one raw unit squared: the oracle's point and squared distance, and two hand-computed values. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_004_degenerate_triangles_are_the_segment_or_point_they_span` |
| MQ-TP-005 | Overflow | MQ-REQ-004 | Query points at the eight corners of the domain (`+-(2^40 - 1)`) against 40 triangles spanning it, including the largest squared distance `3 (2^41 - 2)^2` raw squared, agree with the oracle with no wrap or panic. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_005_the_widest_inputs_in_the_trace_domain_are_exact` |
| MQ-TP-006 | Seeded oracle | MQ-REQ-002, MQ-REQ-003, MQ-REQ-004 | 3 000 seeded cases (seed `0xB002_0246_0000_0001`) from five families (the whole domain, a few raw units, collinear, unit scale, extremes): point and squared distance are the oracle's exact values rounded; `cmp_distance` agrees with the oracle against the previous case and against the edge `t0 t1` alone (1 272 exact ties); `cmp_bound` agrees with the oracle at five integer bounds per case. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_006_seeded_points_match_the_oracle_and_compare_exactly` |
| MQ-TP-007 | Boundary and seeded | MQ-REQ-005 | Inside, on a face, on a corner and one raw unit outside give 0, 0, 0 and 1; gaps of `2^42 - 1`, `2^42` and `2^42 + 1` give `(2^42 - 1)^2`, `2^84`, `2^84`; an inverted axis gives its larger signed gap; 2 000 seeded boxes give the exact distance and never exceed the distance to a seeded point in the box. | `crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_007_the_box_bound_is_the_exact_distance_and_never_above_it` |
| MQ-TP-010 | Brute force | MQ-REQ-006 | Seven seeded triangle soups (40 to 190 triangles, degenerate ones included; one spanning the domain) and 60 query points each: ten trees per mesh (both builders, both builders over shuffled orders, random splits with leaves of 1 to 8 primitives in shuffled order) all equal the brute force bit for bit. | `crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_010_every_tree_shape_returns_what_a_test_of_every_triangle_returns` |
| MQ-TP-011 | Ties | MQ-REQ-006 | Three height-field grids of 72 triangles (shared edges and vertices) at three scales, queried at 160 grid vertices, edge points and points above them (539 exact non-winning ties): every tree shape equals the brute force, so the lowest index wins on every tree. | `crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_011_shared_edges_and_vertices_resolve_to_the_lowest_index_on_every_tree` |
| MQ-TP-012 | Constructed | MQ-REQ-005, MQ-REQ-007 | A subtree whose bound (1) is below the best distance (2) is searched, which a bound one gap unit too large would prune; a subtree whose bound equals the best distance and holds an equal twin of lower index is searched, in two leaves and in one leaf listed in descending order; a subtree strictly farther than the best is not tested. | `crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_012_pruning_keeps_every_subtree_that_can_hold_the_answer` |
| MQ-TP-013 | Malformed | MQ-REQ-008 | An empty tree is `None`; a dangling child, a child index of `u32::MAX` and a cycle are `InvalidNodeIndex`; a short leaf is `InvalidPrimitiveRange`; a comb of depth 63 equals the brute force and one of depth 64 is `StackOverflow`. | `crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_013_malformed_trees_are_the_traversal_errors_and_empty_trees_find_nothing` |
| MQ-TP-014 | Closure | MQ-REQ-008 | With every primitive left out the answer is `None`; with only odd primitives it is the brute force over them, with their own indices, on 40 seeded query points. | `crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_014_primitives_the_closure_leaves_out_are_ignored` |
| MQ-TP-015 | Allocation | MQ-REQ-007 | A search over a 64-triangle SAH tree, kernel included, makes zero heap allocations and equals the brute force. Native only (the counter needs a global allocator). | `crates/bunny-broadphase/tests/closest_point_allocation_tests.rs::mq_tp_015_closest_point_allocates_nothing` |
| MQ-TP-020 | Refusals | MQ-REQ-001, MQ-REQ-009 | Empty index buffers at both widths; a vertex at `+-(2^40 - 1)` accepted and at `+-2^40` refused on every axis, the first offender named; the first bad index at both widths, after the vertex check; a short bounds buffer left untouched; a primitive index equal to the count refused. | `crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_020_meshes_are_refused_with_typed_errors_in_order` |
| MQ-TP-021 | Layout | MQ-REQ-010 | For an octahedron plus a degenerate face at 16-bit and 32-bit widths, `triangle(i)` and `write_bounds` equal `TraceTriangle::new` on the faces' vertices; past the count is `None` and untouched. | `crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_021_triangle_i_of_the_index_buffer_is_primitive_i_at_both_widths` |
| MQ-TP-022 | Brute force | MQ-REQ-010 | Through the documented layout, `MeshBvh::closest_point` and `closest_hit` equal a test of every triangle for 300 seeded points and rays (seed `0xB002_0246_0000_0003`), and a shared vertex and the centre resolve to the lowest index. | `crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_022_mesh_searches_equal_a_test_of_every_triangle` |
| MQ-TP-023 | Doctest | MQ-REQ-010 | The `MeshBvh` rustdoc builds a two-triangle square through the layout; a ray onto the shared corner and a point above it resolve to triangle 0 with hand-computed values. | `crates/bunny-mesh/src/mesh_bvh.rs::MeshBvh` |
| MQ-TP-030 | Real asset | MQ-REQ-010 | The committed Stanford bunny fixture (1 889 vertices, 3 851 triangles) through `MeshView`, `write_bounds`, `build_bvh_sah` and `MeshBvh`: 128 seeded rays and 64 seeded query points (seed `0xB002_0246_0000_0004`) equal a test of every triangle, and 20 vertices return distance zero, the vertex itself, and the lowest index of the faces that use it. | `crates/bunny-codec/tests/mesh_bvh_stanford_tests.rs::mq_tp_030_the_stanford_bunny_is_traced_and_queried_through_the_mesh_layout` |
| MQ-TP-040 | Mutation | MQ-REQ-001, MQ-REQ-002, MQ-REQ-003, MQ-REQ-004, MQ-REQ-005, MQ-REQ-006, MQ-REQ-007, MQ-REQ-008, MQ-REQ-009, MQ-REQ-010 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "MQ-TP-001"
requirements = ["MQ-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_001_query_points_outside_the_trace_domain_are_refused"
oracle = "Hand-derived domain boundary, one raw unit either side."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-002"
requirements = ["MQ-REQ-002", "MQ-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_002_each_region_gives_its_hand_derived_point"
oracle = "Hand-computed points and squared distances in whole units; the rational oracle."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-003"
requirements = ["MQ-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_003_coordinates_and_squared_distances_round_to_nearest_ties_to_even"
oracle = "Hand-derived exact halves and the rational oracle."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-004"
requirements = ["MQ-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_004_degenerate_triangles_are_the_segment_or_point_they_span"
oracle = "The rational oracle, which takes the hull of the vertices; hand-computed segment and point distances."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-005"
requirements = ["MQ-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_005_the_widest_inputs_in_the_trace_domain_are_exact"
oracle = "The rational oracle; the largest squared distance rounded by hand in i128."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-006"
requirements = ["MQ-REQ-002", "MQ-REQ-003", "MQ-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_006_seeded_points_match_the_oracle_and_compare_exactly"
oracle = "Exact rational oracle sharing no helper with the kernel."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-007"
requirements = ["MQ-REQ-005"]
evidence = "test"
test = "crates/bunny-query/tests/point_triangle_tests.rs::mq_tp_007_the_box_bound_is_the_exact_distance_and_never_above_it"
oracle = "Squared distance to the clamped point, computed independently in i128."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-010"
requirements = ["MQ-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_010_every_tree_shape_returns_what_a_test_of_every_triangle_returns"
oracle = "A test of every triangle with the exact comparison and the lowest index; it shares the kernel and none of the tree logic."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-011"
requirements = ["MQ-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_011_shared_edges_and_vertices_resolve_to_the_lowest_index_on_every_tree"
oracle = "The brute force; exact ties counted with cmp_distance."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-012"
requirements = ["MQ-REQ-005", "MQ-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_012_pruning_keeps_every_subtree_that_can_hold_the_answer"
oracle = "Hand-derived squared distances and the list of tested primitives."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-013"
requirements = ["MQ-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_013_malformed_trees_are_the_traversal_errors_and_empty_trees_find_nothing"
oracle = "The TraversalError contract of closest_hit; the brute force."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-014"
requirements = ["MQ-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_tests.rs::mq_tp_014_primitives_the_closure_leaves_out_are_ignored"
oracle = "The brute force over the subset."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-015"
requirements = ["MQ-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_point_allocation_tests.rs::mq_tp_015_closest_point_allocates_nothing"
oracle = "allocation_counter's count of heap blocks on the calling thread."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-020"
requirements = ["MQ-REQ-001", "MQ-REQ-009"]
evidence = "test"
test = "crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_020_meshes_are_refused_with_typed_errors_in_order"
oracle = "Hand-built meshes with one fault each."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-021"
requirements = ["MQ-REQ-010"]
evidence = "test"
test = "crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_021_triangle_i_of_the_index_buffer_is_primitive_i_at_both_widths"
oracle = "TraceTriangle::new and bounds on the index buffer read directly."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-022"
requirements = ["MQ-REQ-010"]
evidence = "test"
test = "crates/bunny-mesh/tests/mesh_bvh_tests.rs::mq_tp_022_mesh_searches_equal_a_test_of_every_triangle"
oracle = "Brute force over every triangle; hand-derived ties."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-023"
requirements = ["MQ-REQ-010"]
evidence = "doctest"
artifact = "crates/bunny-mesh/src/mesh_bvh.rs::MeshBvh"
oracle = "Hand-computed distances in whole units."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-030"
requirements = ["MQ-REQ-010"]
evidence = "test"
test = "crates/bunny-codec/tests/mesh_bvh_stanford_tests.rs::mq_tp_030_the_stanford_bunny_is_traced_and_queried_through_the_mesh_layout"
oracle = "Brute force over all 3 851 triangles; the faces that name each vertex, read from the index buffer."
tier = "fast"
status = "planned"

[[case]]
id = "MQ-TP-040"
requirements = ["MQ-REQ-001", "MQ-REQ-002", "MQ-REQ-003", "MQ-REQ-004", "MQ-REQ-005", "MQ-REQ-006", "MQ-REQ-007", "MQ-REQ-008", "MQ-REQ-009", "MQ-REQ-010"]
evidence = "manual-audit"
artifact = "docs/topics/mesh-queries/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "planned"
```

## Mutations

Each mutation was applied to the source, `point_triangle_tests`, `closest_point_tests` and
`mesh_bvh_tests` run, and the source restored; each fails the named cases.

| Mutation | Fails |
| --- | --- |
| Face: accept the projection when a weight is negative | MQ-TP-002, 004, 005, 006, 010, 011, 014, 022 |
| Face: never take the projection (edges only) | MQ-TP-002, 005, 006 |
| Edge: no clamp at the far end until `t > 2L` | MQ-TP-002, 004, 006, 010, 011, 022 |
| Closest: ignore the third edge | MQ-TP-005, 006 |
| Compare: cross-multiply by the wrong denominators | MQ-TP-002, 003, 004, 005, 006, 010, 011, 014, 022 |
| Rounding: ties away from zero | MQ-TP-003, 006 |
| Rounding: truncate | MQ-TP-003, 005, 006 |
| Squared distance scaled by `2^31` | MQ-TP-002, 003, 004, 005, 006 |
| Box bound: every gap one raw unit too large | MQ-TP-007, 011, 012, 022 |
| Box bound: cap at `2^20` | MQ-TP-007 |
| `cmp_bound` against `bound + 1` | MQ-TP-006, 011, 012, 022 |
| Search: skip a subtree whose bound equals the best | MQ-TP-011, 012, 022 |
| Search: a later equal point replaces the best | MQ-TP-011, 022 |
| Search: an equal point never replaces the best | MQ-TP-011, 012, 022 |
| Search: farther child first | MQ-TP-012, 013 |
| Search: no interior-visit bound | MQ-TP-013 |
| Mesh: accept a coordinate of `2^40` | MQ-TP-020 |
| Mesh: accept a primitive index equal to the count | MQ-TP-020 |
| Query: accept a coordinate of `2^40` | MQ-TP-001 |

## Determinism Obligations And Evidence

The kernel and the search use only integer arithmetic; the tests use fixed seeds and no
clock. Every case except MQ-TP-015 is a `wasm_bindgen_test` that also runs natively.

