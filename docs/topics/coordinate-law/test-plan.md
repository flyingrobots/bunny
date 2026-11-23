# Coordinate Law Test Plan

This document defines how Bunny proves the coordinate-law contract.

The current coordinate-law chapter is
[`README.md`](README.md). This test plan is a living verification design, not a
historical proposal.

## Scope

This plan covers Bunny's coordinate-space conventions:

- right-handed 3D basis orientation
- canonical `XY` plane and winding
- unit policy
- coordinate-space naming conventions
- transform, rotation, bounds, and projection reservations
- external-format boundary expectations

Only conventions backed by implemented APIs can have executable tests today.
Reserved future APIs are tracked as open gaps until those APIs exist.

## Test Goals

- Prove that existing vector operations implement the right-handed basis
  required by the coordinate law.
- Prove that `XY` counter-clockwise winding produces a `+Z` normal.
- Keep the tests native and WASM-compatible.
- Assert public behavior through `bunny-linalg` public types.
- Avoid private helpers, stdout, stderr, logs, and documentation prose as test
  oracles.

## Non-Goals

- Matrix, transform, projection, viewport, quaternion, and angle tests are not
  executable until those APIs exist. The pinhole camera's frame and image
  convention are verified in the optics test plan (`OPTICS-TP-001`,
  `OPTICS-TP-007`).
- This plan does not test renderer-specific coordinate conventions.
- This plan does not test external file-format conversion until boundary
  adapters explicitly expose those conversions.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| CL-REQ-001 | Bunny's canonical 3D frame is right-handed. | `README.md#canonical-3d-frame` |
| CL-REQ-002 | `+X cross +Y` is `+Z`, with cyclic basis products following the right-hand rule. | `README.md#canonical-3d-frame` |
| CL-REQ-003 | Reversing cross-product operands negates the basis result. | `README.md#canonical-3d-frame` |
| CL-REQ-004 | Bunny's default 2D plane is the `XY` plane. | `README.md#canonical-2d-plane` |
| CL-REQ-005 | Counter-clockwise `XY` winding viewed from `+Z` produces a `+Z` normal. | `README.md#canonical-2d-plane` |
| CL-REQ-006 | Bunny coordinates are unitless fixed-point Bunny units. | `README.md#units` |
| CL-REQ-007 | Future transforms are named `target_from_source`. | `README.md#coordinate-spaces` |
| CL-REQ-008 | Future matrix transforms use column-vector semantics and right-to-left composition. | `README.md#transform-convention` |
| CL-REQ-009 | Positive rotation follows the right-hand rule. | `README.md#rotation-and-angle-direction` |
| CL-REQ-010 | Bounds require `min <= max`, inclusive boundary contact, and explicit emptiness. | `README.md#bounds-and-boundary-contact` |
| CL-REQ-011 | Canonical NDC uses `x/y` in `[-1, 1]`, `z` in `[0, 1]`, and positive `y` up. | `README.md#projection-and-ndc` |
| CL-REQ-012 | External boundary adapters document source convention and exact conversion into Bunny convention. | `README.md#external-format-boundaries` |
| CL-REQ-013 | View space looks down `-Z`; projection builders map the near plane to NDC `z = 0` and the far plane to `z = 1`. | `README.md#projection-and-ndc` |
| CL-REQ-014 | Viewport coordinates have the origin at the top-left corner of the viewport, pixel y runs down, and a coordinate is continuous with pixel corners on integers (pixel `i` spans `[i, i + 1)`). | `README.md#viewport-conversion` |
| CL-REQ-015 | NDC to viewport maps `x_px = (x_ndc + 1) * width / 2` and `y_px = (1 - y_ndc) * height / 2`, each rounded by flooring the exact raw value; the inverse floors `2 * px / size` the same way. | `README.md#viewport-conversion` |
| CL-REQ-016 | Viewport depth equals NDC depth; the mapping neither clips nor clamps. | `README.md#viewport-conversion` |
| CL-REQ-017 | A viewport with a zero width or height, and any result outside the Q32.32 range, is a typed error. | `README.md#viewport-conversion` |
| CL-REQ-018 | The round trip through the viewport is within one raw unit per axis and exact unless the size and the shifted NDC raw value are both odd. | `README.md#viewport-conversion` |

| CL-REQ-019 | `Viewport::try_unproject` (`bunny-unproject/1`) takes a continuous viewport coordinate in the closed viewport `[0, width] x [0, height]` (integers on pixel corners), a `clip_from_view` of either builder's shape and an affine `view_from_world`, and returns the ray from the world point at NDC depth `0` to the point at depth `1`: origin the depth-0 point, direction their difference, not normalized. | `README.md#unprojection` |
| CL-REQ-020 | The viewport map, the projection (perspective divide included) and the view are inverted exactly; each origin and direction component is the exact value rounded once to nearest, ties to even (within half a raw unit), equal to the general inverse of `clip_from_world` before that rounding. | `README.md#unprojection` |
| CL-REQ-021 | Unprojection refuses, in order and each with a typed `UnprojectError`: a coordinate outside the viewport, an unsupported or singular projection, a perspective point at infinity or behind the camera, a non-affine or singular view, a rounded component outside Q32.32, and a rounded zero direction. | `README.md#unprojection` |
| CL-REQ-022 | For the same sample, the unprojected ray through `try_look_at`, `try_perspective` or `try_orthographic` and the matching `Viewport` lies along `PinholeCamera::ray` or `OrthographicCamera::ray` within a measured tolerance of `2^6` raw. | `README.md#unprojection` |
| CL-REQ-023 | A world point projected to the viewport (NDC rounded once, viewport floored) lies on the unprojected ray within a bound derived from those roundings and the kernel's half raw unit. | `README.md#unprojection` |
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Bunny's canonical 3D frame is right-handed."
status = "active"

[[requirement]]
id = "CL-REQ-002"
summary = "+X cross +Y is +Z, with cyclic basis products following the right-hand rule."
status = "active"

[[requirement]]
id = "CL-REQ-003"
summary = "Reversing cross-product operands negates the basis result."
status = "active"

[[requirement]]
id = "CL-REQ-004"
summary = "Bunny's default 2D plane is the XY plane."
status = "active"

[[requirement]]
id = "CL-REQ-005"
summary = "Counter-clockwise XY winding viewed from +Z produces a +Z normal."
status = "active"

[[requirement]]
id = "CL-REQ-006"
summary = "Bunny coordinates are unitless fixed-point Bunny units."
status = "active"

[[requirement]]
id = "CL-REQ-007"
summary = "Future transforms are named target_from_source."
status = "reserved"

[[requirement]]
id = "CL-REQ-008"
summary = "Future matrix transforms use column-vector semantics and right-to-left composition."
status = "reserved"

[[requirement]]
id = "CL-REQ-009"
summary = "Positive rotation follows the right-hand rule."
status = "reserved"

[[requirement]]
id = "CL-REQ-010"
summary = "Bounds require min <= max, inclusive boundary contact, and explicit emptiness."
status = "reserved"

[[requirement]]
id = "CL-REQ-011"
summary = "Canonical NDC uses x/y in [-1, 1], z in [0, 1], and positive y up."
status = "active"

[[requirement]]
id = "CL-REQ-012"
summary = "External boundary adapters document source convention and exact conversion into Bunny convention."
status = "reserved"

[[requirement]]
id = "CL-REQ-013"
summary = "View space looks down -Z; projection builders map the near plane to NDC z = 0 and the far plane to z = 1."
status = "active"

[[requirement]]
id = "CL-REQ-014"
summary = "Viewport coordinates have the origin at the top-left corner of the viewport, pixel y runs down, and a coordinate is continuous with pixel corners on integers (pixel i spans [i, i + 1))."
status = "active"

[[requirement]]
id = "CL-REQ-015"
summary = "NDC to viewport maps x_px = (x_ndc + 1) * width / 2 and y_px = (1 - y_ndc) * height / 2, each rounded by flooring the exact raw value; the inverse floors 2 * px / size the same way."
status = "active"

[[requirement]]
id = "CL-REQ-016"
summary = "Viewport depth equals NDC depth; the mapping neither clips nor clamps."
status = "active"

[[requirement]]
id = "CL-REQ-017"
summary = "A viewport with a zero width or height, and any result outside the Q32.32 range, is a typed error."
status = "active"

[[requirement]]
id = "CL-REQ-018"
summary = "The round trip through the viewport is within one raw unit per axis and exact unless the size and the shifted NDC raw value are both odd."
status = "active"

[[requirement]]
id = "CL-REQ-019"
summary = "Viewport::try_unproject (bunny-unproject/1) maps a closed-viewport coordinate, a builder-shaped clip_from_view and an affine view_from_world to the ray from the NDC depth-0 world point to the depth-1 point."
status = "active"

[[requirement]]
id = "CL-REQ-020"
summary = "Every step is inverted exactly; each output component is the exact value rounded once to nearest, ties to even, equal to the general inverse of clip_from_world before that rounding."
status = "active"

[[requirement]]
id = "CL-REQ-021"
summary = "Unprojection refuses, in order, with typed errors: outside viewport, unsupported or singular projection, point at infinity or behind the camera, non-affine or singular view, overflow, zero direction."
status = "active"

[[requirement]]
id = "CL-REQ-022"
summary = "The unprojected ray agrees with PinholeCamera::ray and OrthographicCamera::ray for the same sample within a measured tolerance of 2^6 raw."
status = "active"

[[requirement]]
id = "CL-REQ-023"
summary = "A world point projected to the viewport lies on its unprojected ray within a bound derived from the NDC rounding, the viewport floor and the kernel rounding."
status = "active"
```

Reserved requirements are part of the coordinate-law vocabulary, but they do
not become active verification obligations until Bunny exposes the API surface
that can satisfy or reject them.

## Fixtures

No external fixtures are required for the current executable tests.

The basis vectors are constructed directly from public fixed-point values:

- `FixedQ32_32::from_raw(ONE_RAW)`
- `FixedVec3::new`
- public `FixedVec3::cross`

Future projection, transform, mesh, and external-format tests may add checked-in
fixtures. Each fixture must document:

- source
- generation command or hand-computation proof
- expected canonical convention
- regeneration policy
- whether bytes or structured values are the oracle

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| CL-TP-001 | Golden path, determinism | CL-REQ-001, CL-REQ-002, CL-REQ-003 | Exact `FixedVec3` equality for public cross-product outputs. | `crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_001_canonical_basis_is_right_handed` |
| CL-TP-002 | Golden path, winding | CL-REQ-004, CL-REQ-005 | Exact `FixedVec3` equality for `(b - a).cross(c - a)` and reversed winding. | `crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_002_xy_counter_clockwise_winding_points_toward_positive_z` |
| CL-TP-003 | Golden path, unit policy | CL-REQ-006 | Exact raw Q32.32 values for whole Bunny-unit coordinates. | `crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_003_bunny_units_are_unitless_fixed_raw_values` |
| CL-TP-004 | Golden path, NDC depth | CL-REQ-011, CL-REQ-013 | Exact raw NDC `z` of near-plane and far-plane points for both projection builders. | `crates/bunny-linalg/tests/projection_tests.rs::cl_tp_004_near_plane_maps_to_zero_and_far_plane_to_one` |
| CL-TP-005 | Golden path, camera convention | CL-REQ-011, CL-REQ-013 | Exact NDC `x/y` signs for `+X`/`+Y` view points, and `w_clip < 0` behind the camera. | `crates/bunny-linalg/tests/projection_tests.rs::cl_tp_005_camera_looks_down_negative_z_with_positive_y_up` |
| CL-TP-006 | Golden path, viewport | CL-REQ-014, CL-REQ-015 | Hand-computed exact raw pixels for a 4 x 2 viewport: corners, centre, quarter points, top-left origin and y down. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_006_four_by_two_viewport_has_exact_golden_vectors` |
| CL-TP-007 | Golden path, viewport sizes | CL-REQ-014, CL-REQ-015 | Exact raw pixels for 1 x 1, 1920 x 1080 and odd 3 x 5 viewports at the corners and the centre. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_007_one_by_one_and_hd_viewports_have_exact_golden_vectors` |
| CL-TP-008 | Boundary, rounding | CL-REQ-015 | Exact raw results one raw unit either side of the half-unit rounding edges in a 1 x 1 viewport, including negative values (floor, not truncation). | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_008_rounding_is_floor_of_exact_raw_value_at_the_edges` |
| CL-TP-009 | Boundary, depth and outside the cube | CL-REQ-016 | Depth equals NDC depth for sampled values; points outside the NDC cube map linearly with hand-computed raw results. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_009_depth_passes_through_and_points_outside_the_cube_map_linearly` |
| CL-TP-010 | Negative, typed errors | CL-REQ-017 | Exact error variants for zero extents and for Q32.32 overflow of each axis in both directions; stable messages. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_010_empty_extent_and_overflow_are_typed_errors` |
| CL-TP-011 | Golden path, inverse | CL-REQ-015 | Hand-computed exact raw NDC for the inverse mapping, including floor behaviour for 1 x 1 and width 3. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_011_inverse_mapping_has_exact_golden_vectors` |
| CL-TP-012 | Property, round trip | CL-REQ-015, CL-REQ-018 | Seeded random sizes and NDC values checked against exact i128 arithmetic: floor bound on the forward map and the exact lost-bit round-trip rule. | `crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_012_seeded_round_trip_matches_exact_integer_arithmetic` |
| CL-TP-013 | Golden path, unprojection | CL-REQ-019, CL-REQ-020 | Hand-derived exact rays of a 4 x 2 viewport for `try_perspective(1, 2, 1, 2)` with the identity view and a look-at view from `(1, 2, 3)` down `+X`: the centre, the four corners, the top-left and bottom-right pixel centres and two edge midpoints. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_013_perspective_golden_rays_at_centre_corners_and_boundary_pixels` |
| CL-TP-014 | Golden path, unprojection | CL-REQ-019, CL-REQ-020 | Hand-derived exact rays for `try_orthographic(2, 1, 1, 3)` at the same samples and views. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_014_orthographic_golden_rays_at_centre_corners_and_boundary_pixels` |
| CL-TP-015 | Boundary, viewport | CL-REQ-019, CL-REQ-021 | `OutsideViewport` one raw unit outside each edge; every edge and corner accepted. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_015_coordinates_outside_the_closed_viewport_are_refused` |
| CL-TP-016 | Negative, projection | CL-REQ-021 | `UnsupportedProjection` one raw unit off each entry outside the shapes; `SingularProjection` at zero and acceptance one raw unit either side for `m00`, `m11`, `m23` and orthographic `m22`; `PointAtInfinity` at `m22 = -1.0` and `0`, `PointBehindCamera` one raw unit past each. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_016_projection_shape_singularity_and_homogeneous_w_thresholds` |
| CL-TP-017 | Negative, view and range | CL-REQ-020, CL-REQ-021 | `UnsupportedView` off the last row; `SingularView` at a rank-two linear part and acceptance one raw unit either side; origin and direction at `i64::MAX` and `i64::MIN` and one and two raw units past; a direction of exactly half a raw unit ties to the even zero (`DegenerateRay`) with one raw unit of view scale either side. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_017_view_overflow_and_degenerate_thresholds` |
| CL-TP-018 | Property, oracle | CL-REQ-020 | 1 500 seeded cases (seed `0x0150_0E7A_0000_0001`): builder and raw projections of both shapes, look-at and raw affine views with entries up to 63 bits, sizes 1 to 2^20; every accepted ray equals the general inverse by Cramer's rule on `clip_from_world`, rounded once, bit for bit; every `Overflow` and `DegenerateRay` agrees with it. The run matched 1 147, overflowed 290 and was degenerate in 63. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_018_seeded_rays_match_the_general_inverse_oracle_bit_for_bit` |
| CL-TP-019 | Property, round trip | CL-REQ-023 | 2 759 seeded world points (seed `0x0150_0E7A_0000_0012`) in front of look-at cameras with builder projections, projected exactly, rounded to raw NDC and floored by `try_ndc_to_viewport`: each lies within its derived bound of the unprojected ray, compared as squared distances in exact rationals. | `crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_019_world_points_round_trip_onto_their_ray_within_the_derived_bound` |
| CL-TP-020 | Differential, pinhole camera | CL-REQ-022 | 2 400 seeded samples (seed `0x0150_A61E_E000_0000`): the unprojected ray against `PinholeCamera::ray`, as exact integer cross products: same orientation, camera direction within `2^6` raw of the unprojected line, unprojected origin within `2^6` raw of the camera ray, eye within `2^6` raw of the unprojected line. Measured worst: `2^4`, `2^3`, `2^2`. | `crates/bunny-optics/tests/unproject_agreement_tests.rs::cl_tp_020_unprojected_rays_agree_with_the_pinhole_camera` |
| CL-TP-021 | Differential, orthographic camera | CL-REQ-022 | 2 400 seeded samples (seed `0x0150_A61E_E000_0003`) against `OrthographicCamera::ray` with near `0`, the same three measures. Measured worst: `2^0`, `2^4`, `2^4`. | `crates/bunny-optics/tests/unproject_agreement_tests.rs::cl_tp_021_unprojected_rays_agree_with_the_orthographic_camera` |
| CL-TP-022 | Mutation | CL-REQ-019, CL-REQ-020, CL-REQ-021 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001", "CL-REQ-002", "CL-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_001_canonical_basis_is_right_handed"
oracle = "Exact FixedVec3 equality for public cross-product outputs."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-002"
requirements = ["CL-REQ-004", "CL-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_002_xy_counter_clockwise_winding_points_toward_positive_z"
oracle = "Exact FixedVec3 equality for public winding-derived normal outputs."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-003"
requirements = ["CL-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/coordinate_law_tests.rs::cl_tp_003_bunny_units_are_unitless_fixed_raw_values"
oracle = "Exact raw Q32.32 values for whole Bunny-unit coordinates."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-004"
requirements = ["CL-REQ-011", "CL-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/projection_tests.rs::cl_tp_004_near_plane_maps_to_zero_and_far_plane_to_one"
oracle = "Exact raw NDC z of near-plane and far-plane points for both projection builders."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-005"
requirements = ["CL-REQ-011", "CL-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/projection_tests.rs::cl_tp_005_camera_looks_down_negative_z_with_positive_y_up"
oracle = "Exact NDC x/y signs for +X/+Y view points, and w_clip < 0 behind the camera."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-006"
requirements = ["CL-REQ-014", "CL-REQ-015"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_006_four_by_two_viewport_has_exact_golden_vectors"
oracle = "Hand-computed exact raw pixels for a 4 x 2 viewport."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-007"
requirements = ["CL-REQ-014", "CL-REQ-015"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_007_one_by_one_and_hd_viewports_have_exact_golden_vectors"
oracle = "Exact raw pixels for 1 x 1, 1920 x 1080 and 3 x 5 viewports."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-008"
requirements = ["CL-REQ-015"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_008_rounding_is_floor_of_exact_raw_value_at_the_edges"
oracle = "Exact raw results at half-unit rounding edges in a 1 x 1 viewport."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-009"
requirements = ["CL-REQ-016"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_009_depth_passes_through_and_points_outside_the_cube_map_linearly"
oracle = "Depth passthrough and hand-computed raw results outside the cube."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-010"
requirements = ["CL-REQ-017"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_010_empty_extent_and_overflow_are_typed_errors"
oracle = "Exact error variants and stable messages."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-011"
requirements = ["CL-REQ-015"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_011_inverse_mapping_has_exact_golden_vectors"
oracle = "Hand-computed exact raw NDC for the inverse mapping."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-012"
requirements = ["CL-REQ-015", "CL-REQ-018"]
evidence = "test"
test = "crates/bunny-linalg/tests/viewport_tests.rs::cl_tp_012_seeded_round_trip_matches_exact_integer_arithmetic"
oracle = "Exact i128 arithmetic for the forward floor bound and the round-trip rule."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-013"
requirements = ["CL-REQ-019", "CL-REQ-020"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_013_perspective_golden_rays_at_centre_corners_and_boundary_pixels"
oracle = "Hand-derived exact rays for an axis-aligned perspective camera at the centre, corners and boundary pixels."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-014"
requirements = ["CL-REQ-019", "CL-REQ-020"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_014_orthographic_golden_rays_at_centre_corners_and_boundary_pixels"
oracle = "Hand-derived exact rays for an axis-aligned orthographic camera at the centre, corners and boundary pixels."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-015"
requirements = ["CL-REQ-019", "CL-REQ-021"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_015_coordinates_outside_the_closed_viewport_are_refused"
oracle = "Exact error variant one raw unit outside each edge; edges accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-016"
requirements = ["CL-REQ-021"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_016_projection_shape_singularity_and_homogeneous_w_thresholds"
oracle = "Exact error variants on and one raw unit either side of each projection threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-017"
requirements = ["CL-REQ-020", "CL-REQ-021"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_017_view_overflow_and_degenerate_thresholds"
oracle = "Exact error variants and raw results on and either side of the view, range and tie thresholds."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-018"
requirements = ["CL-REQ-020"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_018_seeded_rays_match_the_general_inverse_oracle_bit_for_bit"
oracle = "Bit-for-bit equality with an independent general-inverse oracle in unbounded integers."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-019"
requirements = ["CL-REQ-023"]
evidence = "test"
test = "crates/bunny-linalg/tests/unproject_tests.rs::cl_tp_019_world_points_round_trip_onto_their_ray_within_the_derived_bound"
oracle = "Per-case bound derived from the stated roundings, in exact rationals."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-020"
requirements = ["CL-REQ-022"]
evidence = "test"
test = "crates/bunny-optics/tests/unproject_agreement_tests.rs::cl_tp_020_unprojected_rays_agree_with_the_pinhole_camera"
oracle = "Measured tolerance against the camera's own rays, exact integer comparison."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-021"
requirements = ["CL-REQ-022"]
evidence = "test"
test = "crates/bunny-optics/tests/unproject_agreement_tests.rs::cl_tp_021_unprojected_rays_agree_with_the_orthographic_camera"
oracle = "Measured tolerance against the camera's own rays, exact integer comparison."
tier = "fast"
status = "implemented"

[[case]]
id = "CL-TP-022"
requirements = ["CL-REQ-019", "CL-REQ-020", "CL-REQ-021"]
evidence = "manual-audit"
artifact = "docs/topics/coordinate-law/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the unprojection kernel, `unproject_tests` and
`unproject_agreement_tests` run, and the kernel restored; each fails the named
cases.

| Mutation | Fails |
| --- | --- |
| NDC `x` sign flipped (`ax = W ONE - 2 X`) | CL-TP-013, -014, -017, -018, -019, -020, -021 |
| `y` not flipped (`ay = 2 Y - H ONE`) | CL-TP-013, -014, -018, -019, -020, -021 |
| Viewport made half-open (`x = width` refused) | CL-TP-013, -014, -015, -018 |
| Ties rounded away from zero instead of to even | CL-TP-017 |
| Perspective direction negated (`+ONE` scale) | CL-TP-013, -015, -018, -020 |
| Orthographic direction negated | CL-TP-014, -017, -018, -021 |
| View translation added instead of subtracted | CL-TP-013, -014, -017, -018, -019, -020, -021 |
| Cofactor matrix used in place of the adjugate | CL-TP-013, -014, -018, -019, -020, -021 |
| `PointAtInfinity` check at depth 1 removed | CL-TP-016 |
| `PointBehindCamera` check removed | CL-TP-016 |
| `DegenerateRay` check removed | CL-TP-017 |
| Orthographic `m22 = 0` accepted | CL-TP-016 |

## Determinism Obligations And Evidence

The current tests use only public deterministic fixed-point operations:

- integer raw construction
- vector subtraction
- vector cross product
- exact equality on raw fixed-point values through `FixedVec3`

There is no randomness, time, filesystem access, locale-sensitive formatting,
parallel scheduling, map iteration, stdout, stderr, or logging in the test
oracle.

The tests run in native Rust test mode and are annotated with
`wasm_bindgen_test`, so the same assertions participate in the headless WASM
gate.

## Known Failures

The current executable surface has no known failing coordinate-law cases.

Future implementation work must add explicit negative tests for:

- invalid transform composition
- point/vector misuse where type APIs can reject it
- inverted bounds where a bounds API owns the rejection
- external-format adapters that omit required convention metadata

## Edge Cases And Unusual Inputs

Current basis tests intentionally use exact unit basis vectors because they
prove convention, not numeric range handling. Numeric range and overflow edges
belong to the numeric constitution and the saturating-arithmetic audit.

Future coordinate APIs must add boundary cases at:

- `min == max` bounds
- zero-length directions
- zero-area triangles
- identity transforms
- inverse transforms
- 180-degree rotations
- near-plane and far-plane NDC limits
- viewport edges and origin flips

## Stress And Fuzz

No fuzz target exists for coordinate law alone today.

Future transform and projection APIs should add seeded property or fuzz tests
for:

- transform inverse round trips
- point versus vector transform differences
- stable projection and unprojection examples
- external-format conversion round trips where the source format is bounded

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| `target_from_source` composition examples. | Matrix and transform primitives. |
| Point versus vector translation behavior. | Typed point/vector/transform APIs. |
| Positive rotation direction beyond cross-product orientation. | Angle, quaternion, or rotation APIs. |
| External source convention conversion tests. | Format-specific adapter APIs that expose convention conversion. |
