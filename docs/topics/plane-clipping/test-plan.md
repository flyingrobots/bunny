# Plane Classification, Segment And Polygon Clipping Test Plan

This document defines how Bunny verifies the fixed-point plane, segment and
convex polygon clipping contract.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design.

## Scope

This plan covers `FixedPlane3`, `PlaneSide`, `FixedSegment3`, and `SegmentClip`
in `bunny-geom`:

- signed distance and three-way point classification;
- the rule for a point exactly on the plane;
- clipping one segment to the closed front half-space;
- the rounding of the clipped endpoint;
- checked overflow behavior;
- convex polygon clipping into a caller buffer: vertex order, degenerate
  results, buffer size, canonical cut direction on shared edges.

Clipping concave polygons is out of scope.

## Test Goals

- Prove classification and clipping agree on which side a point on the plane
  belongs to.
- Assert exact raw Q32.32 outputs where the intersection is representable.
- Lock the clipped endpoint's rounding with golden raw vectors.
- Prove overflow returns `None` instead of panicking or saturating.
- Compare a seeded random sweep against an exact `i128` reference.
- Stay native and WASM compatible through `wasm_bindgen_test`.

## Requirements

The table is the human-readable view. The fenced TOML block after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| PC-REQ-001 | Classification is Front for positive signed distance, Behind for negative, On for exactly zero. | `README.md#classification` |
| PC-REQ-002 | Clipping keeps the closed front half-space: a point with signed distance zero is kept. | `README.md#on-plane-rule` |
| PC-REQ-003 | A crossing segment returns the front part, original endpoint order preserved, with one checked division. | `README.md#clipping` |
| PC-REQ-004 | Fully behind segments return Outside; fully front or coplanar segments return unchanged. | `README.md#clipping` |
| PC-REQ-005 | The clipped endpoint rounds the parameter with ties-to-even and each coordinate offset with ties-to-even. | `README.md#rounding-of-the-clipped-endpoint` |
| PC-REQ-006 | Overflow in distance, parameter or interpolation returns None. | `README.md#overflow` |
| PC-REQ-007 | The clipped endpoint stays within a stated raw bound of the exact intersection. | `README.md#rounding-of-the-clipped-endpoint` |
| PC-REQ-008 | Polygon clipping keeps the closed front half-space, preserves winding and starts output at the first kept input vertex. | `README.md#vertex-order` |
| PC-REQ-009 | Fewer than three input vertices is an error; a result with fewer than three vertices is Outside; all-on-plane and all-front polygons are unchanged. | `README.md#degenerate-results` |
| PC-REQ-010 | A cut is skipped on an edge with an on-plane end, and input vertices are never merged. | `README.md#vertices-on-the-plane` |
| PC-REQ-011 | The output buffer holds a convex polygon's result when it has n + 1 slots; a smaller buffer reports OutputTooSmall. | `README.md#buffer-size` |
| PC-REQ-012 | Each cut is computed in the canonical edge direction, so adjacent polygons agree bit for bit and the cut equals clip_segment of the canonical segment, within the stated bound. | `README.md#cut-points-and-shared-edges` |
| PC-REQ-013 | Polygon overflow returns Overflow. | `README.md#overflow` |

```toml
[[requirement]]
id = "PC-REQ-001"
summary = "Classification is Front for positive signed distance, Behind for negative, On for exactly zero."
status = "active"

[[requirement]]
id = "PC-REQ-002"
summary = "Clipping keeps the closed front half-space: a point with signed distance zero is kept."
status = "active"

[[requirement]]
id = "PC-REQ-003"
summary = "A crossing segment returns the front part, original endpoint order preserved, with one checked division."
status = "active"

[[requirement]]
id = "PC-REQ-004"
summary = "Fully behind segments return Outside; fully front or coplanar segments return unchanged."
status = "active"

[[requirement]]
id = "PC-REQ-005"
summary = "The clipped endpoint rounds the parameter with ties-to-even and each coordinate offset with ties-to-even."
status = "active"

[[requirement]]
id = "PC-REQ-006"
summary = "Overflow in distance, parameter or interpolation returns None."
status = "active"

[[requirement]]
id = "PC-REQ-007"
summary = "The clipped endpoint stays within a stated raw bound of the exact intersection."
status = "active"

[[requirement]]
id = "PC-REQ-008"
summary = "Polygon clipping keeps the closed front half-space, preserves winding and starts output at the first kept input vertex."
status = "active"

[[requirement]]
id = "PC-REQ-009"
summary = "Fewer than three input vertices is an error; a result with fewer than three vertices is Outside; all-on-plane and all-front polygons are unchanged."
status = "active"

[[requirement]]
id = "PC-REQ-010"
summary = "A cut is skipped on an edge with an on-plane end, and input vertices are never merged."
status = "active"

[[requirement]]
id = "PC-REQ-011"
summary = "The output buffer holds a convex polygon's result when it has n + 1 slots; a smaller buffer reports OutputTooSmall."
status = "active"

[[requirement]]
id = "PC-REQ-012"
summary = "Each cut is computed in the canonical edge direction, so adjacent polygons agree bit for bit and the cut equals clip_segment of the canonical segment, within the stated bound."
status = "active"

[[requirement]]
id = "PC-REQ-013"
summary = "Polygon overflow returns Overflow."
status = "active"
```

## Fixtures

No external fixtures. Every case builds values from `FixedQ32_32::from_raw`.
Exact cases use the plane `z = 1` and endpoints on a power-of-two grid so the
intersection is representable.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| PC-TP-001 | Classification | PC-REQ-001 | Front, Behind and On for axis and non-unit normals. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_001_classification_has_three_outcomes` |
| PC-TP-002 | Crossing, exact | PC-REQ-003 | Plane `z = 1`, intersection at t = 1/4, exact raw points, both directions. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_002_crossing_segment_keeps_front_part_exactly` |
| PC-TP-003 | Both front, both behind | PC-REQ-004 | Unchanged / Outside. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_003_fully_front_is_kept_and_fully_behind_is_outside` |
| PC-TP-004 | Touching at one end | PC-REQ-002 | On-plane end is kept: degenerate segment or whole segment. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_004_touching_end_belongs_to_front` |
| PC-TP-005 | Coplanar | PC-REQ-002, PC-REQ-004 | Segment inside the plane is returned unchanged. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_005_coplanar_segment_is_kept` |
| PC-TP-006 | Golden, inexact | PC-REQ-005 | t = 1/3 gives raw 1431655765 and endpoint raw 4294967295. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_006_golden_third_locks_raw_outputs` |
| PC-TP-007 | Ties-to-even | PC-REQ-005 | Constructed ties at 1.5 and 0.5 raw round to 2 and 0. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_007_parameter_ties_round_to_even` |
| PC-TP-008 | Overflow | PC-REQ-006 | Distance and interpolation overflow return None. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_008_overflow_returns_none` |
| PC-TP-009 | Seeded sweep | PC-REQ-001..004, PC-REQ-007 | Exact i128 rational intersection, classification consistency, retained endpoints on the front. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_009_seeded_sweep_matches_exact_reference` |
| PC-TP-010 | Square crossing | PC-REQ-008 | Exact raw square clipped by x = 1; output starts at the first kept vertex; area sign positive. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_010_square_crossing_starts_at_first_kept_vertex` |
| PC-TP-011 | One vertex on plane | PC-REQ-002, PC-REQ-010 | Touching apex is Outside; apex with front base unchanged; on-plane vertex gets no duplicate cut. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_011_one_vertex_on_the_plane` |
| PC-TP-012 | All in, all out | PC-REQ-008, PC-REQ-009 | Unchanged / Outside, including one raw unit behind. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_012_all_in_and_all_out` |
| PC-TP-013 | Edge in plane | PC-REQ-009 | Edge on plane with rest behind is Outside; rest in front is unchanged. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_013_edge_in_the_plane` |
| PC-TP-014 | Coplanar polygon | PC-REQ-009 | Polygon inside the plane is returned unchanged. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_014_coplanar_polygon_is_unchanged` |
| PC-TP-015 | Degenerate inputs | PC-REQ-009, PC-REQ-010 | Fewer than three vertices is an error; repeated input vertices are preserved; two on-plane vertices are Outside. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_015_degenerate_inputs_and_repeated_vertices` |
| PC-TP-016 | Buffer bounds | PC-REQ-011 | A corner cut gives n + 1 vertices; n + 1 slots fit, n slots give OutputTooSmall; Outside needs none; tail untouched. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_016_output_buffer_bounds` |
| PC-TP-017 | Shared edge, both directions | PC-REQ-012 | Two triangles traversing an edge in opposite directions contain the same canonical cut for 400 edges. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_017_shared_edge_cut_ignores_traversal_direction` |
| PC-TP-018 | Canonical equals segment clip | PC-REQ-012 | The polygon cut equals clip_segment of the canonical segment for three vertex rotations. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_018_cut_equals_segment_clip_in_canonical_direction` |
| PC-TP-019 | Golden, inexact | PC-REQ-005, PC-REQ-012 | t = 1/3 gives cut raw 4294967295 in a triangle with exact golden output list. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_019_golden_inexact_cut` |
| PC-TP-020 | Winding, clockwise | PC-REQ-008 | Clockwise square stays clockwise with exact output. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_020_winding_survives_both_orientations` |
| PC-TP-021 | Overflow | PC-REQ-013 | Distance and interpolation overflow return Overflow. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_021_overflow_is_an_error` |
| PC-TP-022 | Seeded sweep | PC-REQ-008, PC-REQ-009, PC-REQ-011, PC-REQ-012 | 2000 convex polygons against an independent Sutherland-Hodgman and exact i128 cut bound; at most n + 1 vertices. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_022_seeded_sweep_matches_reference` |
| PC-TP-023 | Sweep, reversed polygon | PC-REQ-012 | A polygon and its reverse clip to the same point set. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_023_sweep_reversed_polygon_gives_the_same_points` |
| PC-TP-024 | Rounding tie asymmetry | PC-REQ-012 | Constructed tie: clip_segment differs by direction, polygons both emit the canonical cut. | `crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_024_rounding_tie_makes_clip_segment_asymmetric_but_polygons_agree` |

```toml
[[case]]
id = "PC-TP-001"
requirements = ["PC-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_001_classification_has_three_outcomes"
oracle = "Front, Behind and On for axis and non-unit normals."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-002"
requirements = ["PC-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_002_crossing_segment_keeps_front_part_exactly"
oracle = "Plane z = 1, intersection at t = 1/4, exact raw points, both directions."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-003"
requirements = ["PC-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_003_fully_front_is_kept_and_fully_behind_is_outside"
oracle = "Unchanged for fully front, Outside for fully behind."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-004"
requirements = ["PC-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_004_touching_end_belongs_to_front"
oracle = "On-plane end is kept: degenerate segment or whole segment."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-005"
requirements = ["PC-REQ-002", "PC-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_005_coplanar_segment_is_kept"
oracle = "Segment inside the plane is returned unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-006"
requirements = ["PC-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_006_golden_third_locks_raw_outputs"
oracle = "t = 1/3 gives raw 1431655765 and endpoint raw 4294967295."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-007"
requirements = ["PC-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_007_parameter_ties_round_to_even"
oracle = "Constructed ties at 1.5 and 0.5 raw round to 2 and 0."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-008"
requirements = ["PC-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_008_overflow_returns_none"
oracle = "Distance and interpolation overflow return None."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-009"
requirements = ["PC-REQ-001", "PC-REQ-002", "PC-REQ-003", "PC-REQ-004", "PC-REQ-007"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_009_seeded_sweep_matches_exact_reference"
oracle = "Exact i128 rational intersection, classification consistency, retained endpoints on the front."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-010"
requirements = ["PC-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_010_square_crossing_starts_at_first_kept_vertex"
oracle = "Exact raw square clipped by x = 1; output starts at the first kept vertex; area sign positive."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-011"
requirements = ["PC-REQ-002", "PC-REQ-010"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_011_one_vertex_on_the_plane"
oracle = "Touching apex is Outside; apex with front base unchanged; on-plane vertex gets no duplicate cut."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-012"
requirements = ["PC-REQ-008", "PC-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_012_all_in_and_all_out"
oracle = "Unchanged / Outside, including one raw unit behind."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-013"
requirements = ["PC-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_013_edge_in_the_plane"
oracle = "Edge on plane with rest behind is Outside; rest in front is unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-014"
requirements = ["PC-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_014_coplanar_polygon_is_unchanged"
oracle = "Polygon inside the plane is returned unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-015"
requirements = ["PC-REQ-009", "PC-REQ-010"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_015_degenerate_inputs_and_repeated_vertices"
oracle = "Fewer than three vertices is an error; repeated input vertices are preserved; two on-plane vertices are Outside."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-016"
requirements = ["PC-REQ-011"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_016_output_buffer_bounds"
oracle = "A corner cut gives n + 1 vertices; n + 1 slots fit, n slots give OutputTooSmall; Outside needs none; tail untouched."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-017"
requirements = ["PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_017_shared_edge_cut_ignores_traversal_direction"
oracle = "Two triangles traversing an edge in opposite directions contain the same canonical cut for 400 edges."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-018"
requirements = ["PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_018_cut_equals_segment_clip_in_canonical_direction"
oracle = "The polygon cut equals clip_segment of the canonical segment for three vertex rotations."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-019"
requirements = ["PC-REQ-005", "PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_019_golden_inexact_cut"
oracle = "t = 1/3 gives cut raw 4294967295 in a triangle with exact golden output list."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-020"
requirements = ["PC-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_020_winding_survives_both_orientations"
oracle = "Clockwise square stays clockwise with exact output."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-021"
requirements = ["PC-REQ-013"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_021_overflow_is_an_error"
oracle = "Distance and interpolation overflow return Overflow."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-022"
requirements = ["PC-REQ-008", "PC-REQ-009", "PC-REQ-011", "PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_022_seeded_sweep_matches_reference"
oracle = "2000 convex polygons against an independent Sutherland-Hodgman and exact i128 cut bound; at most n + 1 vertices."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-023"
requirements = ["PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_023_sweep_reversed_polygon_gives_the_same_points"
oracle = "A polygon and its reverse clip to the same point set."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-024"
requirements = ["PC-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/polygon_clip_tests.rs::pc_tp_024_rounding_tie_makes_clip_segment_asymmetric_but_polygons_agree"
oracle = "Constructed tie: clip_segment differs by direction, polygons both emit the canonical cut."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

Tests use only integer raw construction, checked Q32.32 operations and exact
equality. The sweep uses a fixed-seed xorshift generator; there is no time,
filesystem, locale or logging in any oracle.

## Known Failures

- The clipped endpoint can lie behind the plane by rounding error (PC-TP-006
  shows one raw unit). Clipping does not repair it; the stated bound is the
  contract.

## Edge Cases And Unusual Inputs

- A zero normal gives the constant signed distance `-offset` for every point.
- A segment touching the plane at one end returns a degenerate segment.
- A polygon touching the plane at a vertex or an edge is Outside.
- Minimum and maximum raw coordinates overflow into `None`.

## Stress And Fuzz

PC-TP-009 runs 2000 seeded segments and PC-TP-022 and PC-TP-023 run 2000 and
1000 seeded convex polygons against exact references. Any failure
must be minimized into a named regression case.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Concave polygon clipping. | A concrete consumer need. |
| Normalized or validated plane constructors. | A concrete consumer need. |
