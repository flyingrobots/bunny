# Plane Classification, Segment And Polygon Clipping Test Plan

This document defines how Bunny verifies the fixed-point plane, segment and
convex polygon clipping contract.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design.

## Scope

This plan covers `FixedPlane3`, `PlaneSide`, `PlaneSpan`, `FixedSegment3`,
`SegmentClip` and `PolygonClip` in `bunny-geom`:

- signed distance and three-way point classification;
- classification of a segment, a triangle and a box against a plane;
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
| PC-REQ-014 | A segment, triangle or box is classified as a PlaneSpan: Coplanar when every vertex is On, Front when every vertex is Front or On and one is Front, Behind when every vertex is Behind or On and one is Behind, Straddling when one is Front and one Behind; a shape touching the plane from one side takes that side. | `README.md#shape-classification` |
| PC-REQ-015 | Segment and triangle classification apply the existing point rule to the endpoints or vertices; a degenerate triangle is classified, not refused; overflow at any vertex returns None. | `README.md#segments-and-triangles` |
| PC-REQ-016 | Box classification equals the classification of its eight corners under the existing point rule, None included, bit for bit, and is computed from two corners. | `README.md#boxes` |
| PC-REQ-017 | An inverted box is classified as the box with the same eight corners; a zero-extent box gets the point rule's answer for its one point. | `README.md#inverted-and-zero-extent-boxes` |
| PC-REQ-018 | A zero-normal plane classifies every shape by the sign of -offset, and returns None only when -offset overflows. | `README.md#zero-normal-plane` |
| PC-REQ-019 | Triangle and segment classification agree with clipping: Front and Coplanar clip to the input unchanged, Behind clips to Outside (a segment touching from behind to its touching point), and None means the clip overflows too. | `README.md#agreement-with-clipping` |

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

[[requirement]]
id = "PC-REQ-014"
summary = "A segment, triangle or box is classified as a PlaneSpan: Coplanar when every vertex is On, Front when every vertex is Front or On and one is Front, Behind when every vertex is Behind or On and one is Behind, Straddling when one is Front and one Behind; a shape touching the plane from one side takes that side."
status = "active"

[[requirement]]
id = "PC-REQ-015"
summary = "Segment and triangle classification apply the existing point rule to the endpoints or vertices; a degenerate triangle is classified, not refused; overflow at any vertex returns None."
status = "active"

[[requirement]]
id = "PC-REQ-016"
summary = "Box classification equals the classification of its eight corners under the existing point rule, None included, bit for bit, and is computed from two corners."
status = "active"

[[requirement]]
id = "PC-REQ-017"
summary = "An inverted box is classified as the box with the same eight corners; a zero-extent box gets the point rule's answer for its one point."
status = "active"

[[requirement]]
id = "PC-REQ-018"
summary = "A zero-normal plane classifies every shape by the sign of -offset, and returns None only when -offset overflows."
status = "active"

[[requirement]]
id = "PC-REQ-019"
summary = "Triangle and segment classification agree with clipping: Front and Coplanar clip to the input unchanged, Behind clips to Outside (a segment touching from behind to its touching point), and None means the clip overflows too."
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
| PC-TP-025 | Triangle, constructed | PC-REQ-014, PC-REQ-015 | Plane z = 1, vertices on it and one raw unit either side: crossing, wholly front, wholly behind, coplanar, touching at a vertex and at an edge from each side; all six vertex orders agree. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_025_triangle_crossing_coplanar_touching_and_outside` |
| PC-TP-026 | Segment, constructed | PC-REQ-014, PC-REQ-015 | Plane z = 1: crossing by one raw unit, coplanar, touching from each side, wholly front and behind, both endpoint orders. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_026_segment_crossing_coplanar_touching_and_outside` |
| PC-TP-027 | Box, constructed | PC-REQ-014, PC-REQ-016 | Plane z = 1: boxes crossing by one raw unit, flat in the plane, touching a face from each side and one raw unit clear; the eight-corner classification agrees. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_027_box_crossing_coplanar_touching_and_outside` |
| PC-TP-028 | Box, oblique and negative normals | PC-REQ-014, PC-REQ-016 | x + y + z = 3 touching a corner from each side and one raw unit past it; -x + y = 0 touching an edge from each side, where the corner most along the normal has the smaller x. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_028_box_against_oblique_and_negative_normals` |
| PC-TP-029 | Degenerate triangles | PC-REQ-015 | Collinear vertices crossing and in the plane, three coincident vertices one raw unit either side and on (the point rule), two coincident on-plane vertices with the third behind. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_029_degenerate_triangles_are_classified_by_their_vertices` |
| PC-TP-030 | Inverted box | PC-REQ-017 | Boxes with min > max on one or more axes classify as the box with min and max swapped and as their eight corners, for an axis and a negative normal. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_030_inverted_box_is_the_box_with_the_same_corners` |
| PC-TP-031 | Zero-extent box | PC-REQ-017 | A box and a triangle at one point get classify's answer (On maps to Coplanar) for points on and one raw unit either side, under two planes. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_031_zero_extent_box_follows_the_point_rule` |
| PC-TP-032 | Zero-normal plane | PC-REQ-018 | Offsets -1, 0 and 1 raw give Front, Coplanar and Behind for a full-range box, triangle and segment; offset i64::MIN gives None. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_032_zero_normal_plane_uses_the_sign_of_the_offset` |
| PC-TP-033 | Overflow | PC-REQ-015, PC-REQ-016 | A product overflowing at some corners, and a partial sum x + y overflowing at one corner, give None as the eight-corner definition does; one raw unit narrower gives Front. | `crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_033_overflow_returns_none_as_the_point_rule_does` |
| PC-TP-034 | Seeded boxes | PC-REQ-014, PC-REQ-016, PC-REQ-017 | 20 000 boxes (seed 0xA076_1D64_78BD_642F, wide random and unit-grid modes, inverted axes included) equal the eight-corner classification through classify and an independent i128 model, and four representable interior points per box agree with its class. | `crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_034_seeded_boxes_match_the_eight_corner_definition` |
| PC-TP-035 | Boxes at the ends of the range | PC-REQ-016, PC-REQ-017 | 50 000 boxes and planes (seed 0xE703_7ED1_A0B4_28DB) whose every coordinate, normal component and offset is within two raw units of i64::MIN, -2^62, -2^32, -1, 0, 1, 2^32, 2^62 or i64::MAX equal the eight-corner classification and the model, None included. | `crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_035_boxes_at_the_ends_of_the_range_match_the_eight_corner_definition` |
| PC-TP-036 | Seeded triangles and segments | PC-REQ-014, PC-REQ-015, PC-REQ-019 | 30 000 triangles and segments (seed 0x8EBC_6AF0_9C88_C6E3, three modes) equal their vertex classification through classify and the model, and agree with clip_polygon and clip_segment. | `crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_036_seeded_triangles_and_segments_match_their_vertices` |
| PC-TP-037 | Mutation | PC-REQ-014, PC-REQ-015, PC-REQ-016, PC-REQ-017 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

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

[[case]]
id = "PC-TP-025"
requirements = ["PC-REQ-014", "PC-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_025_triangle_crossing_coplanar_touching_and_outside"
oracle = "Plane z = 1, vertices on it and one raw unit either side: crossing, wholly front, wholly behind, coplanar, touching at a vertex and at an edge from each side; all six vertex orders agree."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-026"
requirements = ["PC-REQ-014", "PC-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_026_segment_crossing_coplanar_touching_and_outside"
oracle = "Plane z = 1: crossing by one raw unit, coplanar, touching from each side, wholly front and behind, both endpoint orders."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-027"
requirements = ["PC-REQ-014", "PC-REQ-016"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_027_box_crossing_coplanar_touching_and_outside"
oracle = "Plane z = 1: boxes crossing by one raw unit, flat in the plane, touching a face from each side and one raw unit clear; the eight-corner classification agrees."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-028"
requirements = ["PC-REQ-014", "PC-REQ-016"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_028_box_against_oblique_and_negative_normals"
oracle = "x + y + z = 3 touching a corner from each side and one raw unit past it; -x + y = 0 touching an edge from each side, where the corner most along the normal has the smaller x."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-029"
requirements = ["PC-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_029_degenerate_triangles_are_classified_by_their_vertices"
oracle = "Collinear vertices crossing and in the plane, three coincident vertices one raw unit either side and on (the point rule), two coincident on-plane vertices with the third behind."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-030"
requirements = ["PC-REQ-017"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_030_inverted_box_is_the_box_with_the_same_corners"
oracle = "Boxes with min > max on one or more axes classify as the box with min and max swapped and as their eight corners, for an axis and a negative normal."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-031"
requirements = ["PC-REQ-017"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_031_zero_extent_box_follows_the_point_rule"
oracle = "A box and a triangle at one point get classify's answer (On maps to Coplanar) for points on and one raw unit either side, under two planes."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-032"
requirements = ["PC-REQ-018"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_032_zero_normal_plane_uses_the_sign_of_the_offset"
oracle = "Offsets -1, 0 and 1 raw give Front, Coplanar and Behind for a full-range box, triangle and segment; offset i64::MIN gives None."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-033"
requirements = ["PC-REQ-015", "PC-REQ-016"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_tests.rs::pc_tp_033_overflow_returns_none_as_the_point_rule_does"
oracle = "A product overflowing at some corners, and a partial sum x + y overflowing at one corner, give None as the eight-corner definition does; one raw unit narrower gives Front."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-034"
requirements = ["PC-REQ-014", "PC-REQ-016", "PC-REQ-017"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_034_seeded_boxes_match_the_eight_corner_definition"
oracle = "20 000 boxes (seed 0xA076_1D64_78BD_642F, wide random and unit-grid modes, inverted axes included) equal the eight-corner classification through classify and an independent i128 model, and four representable interior points per box agree with its class."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-035"
requirements = ["PC-REQ-016", "PC-REQ-017"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_035_boxes_at_the_ends_of_the_range_match_the_eight_corner_definition"
oracle = "50 000 boxes and planes (seed 0xE703_7ED1_A0B4_28DB) whose every coordinate, normal component and offset is within two raw units of i64::MIN, -2^62, -2^32, -1, 0, 1, 2^32, 2^62 or i64::MAX equal the eight-corner classification and the model, None included."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-036"
requirements = ["PC-REQ-014", "PC-REQ-015", "PC-REQ-019"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_span_sweep_tests.rs::pc_tp_036_seeded_triangles_and_segments_match_their_vertices"
oracle = "30 000 triangles and segments (seed 0x8EBC_6AF0_9C88_C6E3, three modes) equal their vertex classification through classify and the model, and agree with clip_polygon and clip_segment."
tier = "fast"
status = "planned"

[[case]]
id = "PC-TP-037"
requirements = ["PC-REQ-014", "PC-REQ-015", "PC-REQ-016", "PC-REQ-017"]
evidence = "manual-audit"
artifact = "docs/topics/plane-clipping/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose fails at least one case above."
tier = "manual"
status = "planned"
```

## Mutation Checks

Planned: each rule of the shape classification broken on purpose in
`crates/bunny-geom/src/plane/span.rs`, the plane tests run, and the kernel
restored; each mutation must fail a named case.

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
