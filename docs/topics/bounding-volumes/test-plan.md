# Bounding Volume Test Plan

This document defines how Bunny proves [`bunny-bounds/1`](README.md): the box helpers on
`FixedAabb3` in `bunny-geom` (`from_points`, `from_triangle`, `merge`, `expand`, `expand_axes`,
`from_sphere`, `bounding_sphere`) and `mesh_bounds` in `bunny-mesh`, and dedicated tests of the
existing `union_aabb`, `TraceSphere::bounds` and `TraceTriangle::bounds`. It answers
flyingrobots/bunny#151.

## Scope

- The seven `FixedAabb3` helpers and `BoundsError`, with every refusal, the exact results, and
  the one rounding (the bounding sphere's centre and radius).
- `bunny_mesh::mesh_bounds` and `MeshBoundsError`.
- The existing `union_aabb`, `TraceSphere::bounds` and `TraceTriangle::bounds`, as they behave
  today.

Out of scope: oriented boxes and frusta (named in the issue's context as future shapes), the
float `Aabb3` DTO, and the per-triangle boxes a BVH builder needs.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| BOUNDS-REQ-001 | `FixedAabb3::from_points(&[FixedVec3])` is the exact per-axis minimum and maximum of the points: one point is the degenerate box at that point, the result is always ordered, nothing allocates, and no points is `Err(BoundsError::NoPoints)`. | `README.md#points-and-triangles` |
| BOUNDS-REQ-002 | `FixedAabb3::from_triangle([FixedVec3; 3])` is the exact per-axis minimum and maximum of the three vertices, for every input (degenerate and collinear triangles included); it never fails. | `README.md#points-and-triangles` |
| BOUNDS-REQ-003 | `merge(&other)` refuses an input with a minimum above its maximum on any axis (`InvertedBox`), as `FixedAabb3::try_new` does; otherwise it is exactly the smallest box containing both inputs. | `README.md#merge` |
| BOUNDS-REQ-004 | `expand(m)` and `expand_axes(m)` move each minimum down and each maximum up by the margin, exactly in `i128`: an inverted input is `InvertedBox`; a result minimum above its maximum is `ShrinkInvertsBox` (checked before the range); a coordinate outside `i64` is `OutOfRange`; a zero margin is the identity and a shrink to a point is allowed. | `README.md#expand` |
| BOUNDS-REQ-005 | `FixedAabb3::from_sphere(&FixedSphere3)` is exactly `centre - r` to `centre + r` per axis: a negative radius is `NegativeRadius` (as `FixedSphere3::try_new`), radius zero is a point, a coordinate outside `i64` is `OutOfRange`. | `README.md#sphere-to-box` |
| BOUNDS-REQ-006 | `bounding_sphere()` refuses an inverted box; its centre is each axis's midpoint rounded once to nearest, ties to even; its radius is the smallest raw radius whose square reaches the farthest corner from that centre, so the sphere contains all eight corners exactly; a radius above `i64::MAX` is `OutOfRange`. | `README.md#box-to-sphere` |
| BOUNDS-REQ-007 | `bunny_mesh::mesh_bounds(&[FixedVec3], IndexBufferLayout)` is the exact box of the vertices the triangles use (unused vertices are not covered): no triangles is `EmptyMesh`, and the first triangle with an index past the vertex buffer is `IndexOutOfRange { triangle }`. | `README.md#mesh-bounds` |
| BOUNDS-REQ-008 | The existing `bunny_broadphase::utils::union_aabb`, `TraceSphere::bounds` and `TraceTriangle::bounds` are pinned as they behave today, including `union_aabb`'s unchecked result on inverted inputs. | `README.md#existing-helpers` |

```toml
[[requirement]]
id = "BOUNDS-REQ-001"
summary = "from_points is the exact per-axis min and max of a slice, a single point is a degenerate box, no points is NoPoints; no allocation."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-002"
summary = "from_triangle is the exact per-axis min and max of three vertices and never fails."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-003"
summary = "merge refuses an inverted input with InvertedBox; otherwise exactly the smallest box containing both."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-004"
summary = "expand is min - m and max + m exactly; InvertedBox, then ShrinkInvertsBox, then OutOfRange; zero margin is the identity."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-005"
summary = "from_sphere is centre minus and plus r exactly; negative radius refused; OutOfRange outside i64."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-006"
summary = "bounding_sphere: midpoint ties to even, radius the ceiling of the exact root to the farthest corner, contains every corner, OutOfRange above i64::MAX."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-007"
summary = "mesh_bounds is the exact box of the indexed vertices; EmptyMesh; first bad triangle is IndexOutOfRange."
status = "active"

[[requirement]]
id = "BOUNDS-REQ-008"
summary = "union_aabb, TraceSphere::bounds and TraceTriangle::bounds pinned as they behave today."
status = "active"
```

## Oracles

Each test file writes its own reference and shares no code with the kernel. Boxes are
compared on raw `i64` values. Minima and maxima come from the standard library's `min` and
`max` over a gathered list; margins are applied in `i128` and the range is decided by
`i64::try_from`; the refusal order is decided by the same three questions in the same order
(inverted input, inverted result, out of range). The bounding-sphere oracle rounds each
midpoint by choosing the even one of the two neighbours of an exact half, finds the radius as
the smallest `r` with `r * r` at least the farthest corner's squared distance by bisection on
squares (no square root), and checks every one of the eight corners with exact `u128` squared
distances. The kernel uses `div_euclid`, a parity correction and `FixedQ32_32::sqrt_u128`.

## Fixtures

None. Seeds and counts are stated in each test and pinned: BOUNDS-TP-008 and BOUNDS-TP-009
draw 10 000 cases each from seed `0xB015_1000_0000_0001` (the second XORed with 9),
BOUNDS-TP-011 2 000 meshes from `0xB015_1000_0000_0002`, BOUNDS-TP-015 5 000 spheres and
triangles from `0xB015_1000_0000_0003`, BOUNDS-TP-013 5 000 pairs from
`0xB015_1000_0000_0004`. Raw values are a uniform 64-bit pattern shifted right by 0 to 63 bits,
so every magnitude occurs; trace-domain values are shifted by 24 to 63 bits. Every failure
message names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| BOUNDS-TP-001 | Hand-derived, empty, single point | BOUNDS-REQ-001, BOUNDS-REQ-002 | No points is `NoPoints`; one point is the degenerate box; four points (one repeated); the full raw range `MIN` to `MAX`; a triangle, a triangle of one repeated vertex, and the widest triangle. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_001_points_and_triangles_hand_derived` |
| BOUNDS-TP-002 | Hand-derived, inverted bounds | BOUNDS-REQ-003 | Merge of two boxes, symmetry, identity, a contained box, the full range; an inverted box on either side; two inverted boxes whose plain min/max would be ordered; `min == max` accepted and `min == max + 1` refused. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_002_merge_hand_derived_and_inverted_refused` |
| BOUNDS-TP-003 | Hand-derived, inverted bounds | BOUNDS-REQ-004 | Zero margin is the identity; grow and shrink by one; shrink to a point allowed and one raw unit more refused; an odd extent cannot reach a point; per-axis margins; an inverted input refused even by a margin that would order it; the margin `i64::MIN` raw on the full range. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_003_expand_margins_hand_derived` |
| BOUNDS-TP-004 | Boundary, overflow | BOUNDS-REQ-004 | On and one raw unit either side of the range: `MAX - 1` grown by 0, 1 and 2; `MIN + 1` grown by 1 and 2; a point at 0, 1 and -1 grown by `i64::MAX` raw; `[MAX, MAX]` shrunk by 1 is `ShrinkInvertsBox`, never `OutOfRange`. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_004_expand_overflow_one_raw_unit_either_side` |
| BOUNDS-TP-005 | Hand-derived, boundary, overflow | BOUNDS-REQ-005 | A sphere's box; radius zero; radius -1 raw and `i64::MIN` raw refused; centres `MAX - r` and `MAX - r + 1`, `MIN + r` and `MIN + r - 1` for `r = 1000` raw; the radius `i64::MAX` raw at the origin. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_005_sphere_to_box_hand_derived_and_boundaries` |
| BOUNDS-TP-006 | Hand-derived, rounding | BOUNDS-REQ-006 | The square `[0, 2]^2 x [0, 0]` has radius 2 where nearest rounding of `sqrt(2)` raw would give 1 and miss a corner; midpoint ties to even on both signs; a 3-4-5 box is exact; a point has radius 0; an inverted box is refused; each checked against the oracle's eight corners. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_006_box_to_sphere_rounds_the_radius_outward` |
| BOUNDS-TP-007 | Boundary, overflow | BOUNDS-REQ-006 | Radius exactly `i64::MAX` raw (`[MIN + 1, MAX]` and `[MIN + 2, MAX]` on one axis) accepted; `[MIN, MAX]` (radius `2^63`) refused; squared distance `MAX^2` accepted and `MAX^2 + 1` refused; the full-range box refused. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_007_box_to_sphere_radius_range_one_raw_unit_either_side` |
| BOUNDS-TP-008 | Seeded oracle | BOUNDS-REQ-001, BOUNDS-REQ-002, BOUNDS-REQ-003, BOUNDS-REQ-004 | 10 000 seeded cases: `from_points` (0 to 8 points) and `from_triangle` equal the oracle's per-axis min and max; `merge` equals the oracle and contains both inputs; `expand_axes` and `expand` equal the oracle's result or refusal, and an accepted box is the input moved by exactly the margin; the outcome counts are pinned. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_008_seeded_points_merge_and_expand_match_the_oracle` |
| BOUNDS-TP-009 | Seeded oracle | BOUNDS-REQ-005, BOUNDS-REQ-006 | 10 000 seeded boxes: `bounding_sphere` equals the oracle's centre and radius, reaches all eight corners, and `r - 1` misses the farthest; the box of the sphere contains the box; `from_sphere` of a seeded sphere equals the oracle; the counts are pinned. | `crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_009_seeded_spheres_contain_every_corner` |
| BOUNDS-TP-010 | Hand-derived, empty, out of range | BOUNDS-REQ-007 | No triangles (16- and 32-bit, with and without vertices) is `EmptyMesh`; an unused far vertex is not covered; a degenerate triangle; index one past the end and `u32::MAX` name the first bad triangle; no vertices at all; the full raw range. | `crates/bunny-mesh/tests/mesh_bounds_tests.rs::bounds_tp_010_mesh_bounds_hand_derived` |
| BOUNDS-TP-011 | Seeded oracle | BOUNDS-REQ-007 | 2 000 seeded meshes of 0 to 11 vertices and 0 to 5 triangles with indices up to two past the end, in both index widths, equal the oracle; the outcome counts are pinned. | `crates/bunny-mesh/tests/mesh_bounds_tests.rs::bounds_tp_011_seeded_mesh_bounds_match_the_oracle` |
| BOUNDS-TP-012 | Existing helper, inverted bounds | BOUNDS-REQ-008 | `union_aabb` today: exact on ordered boxes; an inverted input gives a box that is neither set union nor corner hull; two inverted boxes give an ordered box; `merge` refuses both. | `crates/bunny-broadphase/tests/union_aabb_tests.rs::bounds_tp_012_union_aabb_as_it_behaves_today` |
| BOUNDS-TP-013 | Seeded oracle | BOUNDS-REQ-003, BOUNDS-REQ-008 | 5 000 seeded ordered pairs: `union_aabb` equals the per-axis min and max and equals `merge`. | `crates/bunny-broadphase/tests/union_aabb_tests.rs::bounds_tp_013_seeded_union_aabb_equals_merge_on_ordered_boxes` |
| BOUNDS-TP-014 | Existing helper | BOUNDS-REQ-008 | `TraceSphere::bounds` and `TraceTriangle::bounds` on hand-derived spheres and triangles, a degenerate and a collinear triangle, and the trace-domain extremes. | `crates/bunny-query/tests/trace_bounds_tests.rs::bounds_tp_014_trace_bounds_hand_derived` |
| BOUNDS-TP-015 | Seeded oracle | BOUNDS-REQ-002, BOUNDS-REQ-005, BOUNDS-REQ-008 | 5 000 seeded trace-domain spheres and triangles: the trace bounds equal `centre -/+ r` and the per-axis min and max, and equal `FixedAabb3::from_sphere` and `from_triangle`. | `crates/bunny-query/tests/trace_bounds_tests.rs::bounds_tp_015_seeded_trace_bounds_match_the_oracle_and_the_geometry_helpers` |
| BOUNDS-TP-016 | Mutation | BOUNDS-REQ-001, BOUNDS-REQ-003, BOUNDS-REQ-004, BOUNDS-REQ-005, BOUNDS-REQ-006, BOUNDS-REQ-007 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "BOUNDS-TP-001"
requirements = ["BOUNDS-REQ-001", "BOUNDS-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_001_points_and_triangles_hand_derived"
oracle = "Hand-derived per-axis minima and maxima."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-002"
requirements = ["BOUNDS-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_002_merge_hand_derived_and_inverted_refused"
oracle = "Hand-derived boxes; refusal decided by min above max."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-003"
requirements = ["BOUNDS-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_003_expand_margins_hand_derived"
oracle = "Hand-derived margins and refusals."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-004"
requirements = ["BOUNDS-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_004_expand_overflow_one_raw_unit_either_side"
oracle = "Hand-derived thresholds in raw units."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-005"
requirements = ["BOUNDS-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_005_sphere_to_box_hand_derived_and_boundaries"
oracle = "Hand-derived boxes and thresholds."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-006"
requirements = ["BOUNDS-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_006_box_to_sphere_rounds_the_radius_outward"
oracle = "Hand-derived centres and radii; exact squared corner distances."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-007"
requirements = ["BOUNDS-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_007_box_to_sphere_radius_range_one_raw_unit_either_side"
oracle = "Hand-derived thresholds; squares compared exactly in u128."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-008"
requirements = ["BOUNDS-REQ-001", "BOUNDS-REQ-002", "BOUNDS-REQ-003", "BOUNDS-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_008_seeded_points_merge_and_expand_match_the_oracle"
oracle = "Independent i128 oracle over full-range seeded inputs."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-009"
requirements = ["BOUNDS-REQ-005", "BOUNDS-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/bounds_tests.rs::bounds_tp_009_seeded_spheres_contain_every_corner"
oracle = "Ties-to-even midpoint by neighbours, ceiling root by bisection on squares, exact u128 corner distances."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-010"
requirements = ["BOUNDS-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/mesh_bounds_tests.rs::bounds_tp_010_mesh_bounds_hand_derived"
oracle = "Hand-derived boxes and refusals."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-011"
requirements = ["BOUNDS-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/mesh_bounds_tests.rs::bounds_tp_011_seeded_mesh_bounds_match_the_oracle"
oracle = "Oracle gathers indexed vertices and takes per-axis min and max."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-012"
requirements = ["BOUNDS-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/union_aabb_tests.rs::bounds_tp_012_union_aabb_as_it_behaves_today"
oracle = "Hand-derived results of today's code."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-013"
requirements = ["BOUNDS-REQ-003", "BOUNDS-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/union_aabb_tests.rs::bounds_tp_013_seeded_union_aabb_equals_merge_on_ordered_boxes"
oracle = "Per-axis min and max."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-014"
requirements = ["BOUNDS-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_bounds_tests.rs::bounds_tp_014_trace_bounds_hand_derived"
oracle = "Hand-derived boxes."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-015"
requirements = ["BOUNDS-REQ-002", "BOUNDS-REQ-005", "BOUNDS-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_bounds_tests.rs::bounds_tp_015_seeded_trace_bounds_match_the_oracle_and_the_geometry_helpers"
oracle = "Per-axis arithmetic in the test."
tier = "fast"
status = "planned"

[[case]]
id = "BOUNDS-TP-016"
requirements = ["BOUNDS-REQ-001", "BOUNDS-REQ-003", "BOUNDS-REQ-004", "BOUNDS-REQ-005", "BOUNDS-REQ-006", "BOUNDS-REQ-007"]
evidence = "manual-audit"
artifact = "docs/topics/bounding-volumes/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "planned"
```

## Mutations

Planned: each rule of the kernel broken on purpose, the tests run, and the kernel restored.

## Determinism Obligations And Evidence

The helpers use only integer comparisons and `i128`/`u128` arithmetic; the tests use fixed
seeds and no clock. They run natively and in the WebAssembly gate.
