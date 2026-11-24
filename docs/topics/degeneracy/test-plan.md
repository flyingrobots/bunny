# Degeneracy Test Plan

This document records the characterization tests behind the
[degeneracy inventory](README.md): what each public constructor and query in
`bunny-geom`, `bunny-query`, `bunny-broadphase` and `bunny-mesh` (and the
`FixedMat3` and `FixedAffine3` inverses they rely on) does today with
degenerate input.

## Scope

- inverted and zero-size boxes, negative and zero radii, zero-length segments,
  zero directions, degenerate triangles, empty inputs, coincident and
  zero-normal planes, and values at `i64::MIN` and `i64::MAX` raw;
- the functions on the default branch only. Shapes in open pull requests are
  inventoried in flyingrobots/bunny#116, not here.

## Test Goals

- Pin each cell of the inventory with a test that asserts the current answer,
  whatever it is. These are characterization tests: they are named
  `degen_tp_*_characterization_*` and their module documentation says they are
  not an endorsed contract. A test pinning a known defect names its issue.
- Choose each input so the answer is derived by hand from the function's
  formula (small whole-unit coordinates, or the ends of the raw range), never
  by running the function under test.
- State the inconsistencies the pinned cells reveal in the chapter, and leave
  the choice of a uniform policy to the repository owner
  (flyingrobots/bunny#116).

## Requirements

Each requirement states that a family of cells is pinned as it behaves today.
It is not a requirement that the behaviour stay as it is: a decided policy
replaces the cell, its test and its row together.

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| DEGEN-REQ-001 | Inverted boxes: each listed function's current answer for a box with min above max on some axis (refused, swapped, a miss, or as given) is pinned. | `README.md#inverted-boxes` |
| DEGEN-REQ-002 | Zero-size boxes: each listed function's current answer for a box with min equal to max on some axis is pinned. | `README.md#zero-size-boxes` |
| DEGEN-REQ-003 | Negative and zero radii: each listed function's current answer (refused, magnitude, point rule, never hit) is pinned. | `README.md#negative-and-zero-radii` |
| DEGEN-REQ-004 | Zero-length segments: each listed function's current answer for start equal to end is pinned. | `README.md#zero-length-segments` |
| DEGEN-REQ-005 | Zero directions: every normalizing ray and line constructor, TraceRay::new and FixedRay3::checked_transform refuse a zero direction, and the non-normalizable nonzero directions stated are refused. | `README.md#zero-directions` |
| DEGEN-REQ-006 | Degenerate triangles: each listed function's current answer for collinear, repeated or single-point vertices is pinned. | `README.md#degenerate-triangles` |
| DEGEN-REQ-007 | Empty inputs: each listed function's current answer for no points, no triangles, no boxes or no nodes is pinned. | `README.md#empty-inputs` |
| DEGEN-REQ-008 | Coincident and zero-normal planes: a zero normal gives every point distance -offset, coincident opposed planes keep the closed plane, and the zero clip matrix gives a frustum containing every point. | `README.md#coincident-and-zero-normal-planes` |
| DEGEN-REQ-009 | Ends of the raw range: each listed function's current answer at i64::MIN and i64::MAX raw (exact, refused, None or saturated) is pinned. | `README.md#values-at-the-ends-of-the-raw-range` |
| DEGEN-REQ-010 | Matrices: FixedMat3::try_inverse and FixedAffine3::try_inverse return an inverse for the exactly singular matrix of flyingrobots/bunny#483. | `README.md#matrices` |

```toml
[[requirement]]
id = "DEGEN-REQ-001"
summary = "Inverted boxes: each listed function's current answer for a box with min above max on some axis (refused, swapped, a miss, or as given) is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-002"
summary = "Zero-size boxes: each listed function's current answer for a box with min equal to max on some axis is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-003"
summary = "Negative and zero radii: each listed function's current answer (refused, magnitude, point rule, never hit) is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-004"
summary = "Zero-length segments: each listed function's current answer for start equal to end is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-005"
summary = "Zero directions: every normalizing ray and line constructor, TraceRay::new and FixedRay3::checked_transform refuse a zero direction, and the non-normalizable nonzero directions stated are refused."
status = "active"

[[requirement]]
id = "DEGEN-REQ-006"
summary = "Degenerate triangles: each listed function's current answer for collinear, repeated or single-point vertices is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-007"
summary = "Empty inputs: each listed function's current answer for no points, no triangles, no boxes or no nodes is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-008"
summary = "Coincident and zero-normal planes: a zero normal gives every point distance -offset, coincident opposed planes keep the closed plane, and the zero clip matrix gives a frustum containing every point."
status = "active"

[[requirement]]
id = "DEGEN-REQ-009"
summary = "Ends of the raw range: each listed function's current answer at i64::MIN and i64::MAX raw (exact, refused, None or saturated) is pinned."
status = "active"

[[requirement]]
id = "DEGEN-REQ-010"
summary = "Matrices: FixedMat3::try_inverse and FixedAffine3::try_inverse return an inverse for the exactly singular matrix of flyingrobots/bunny#483."
status = "active"
```

## Fixtures

No external fixtures. Every input is constructed in the test files from whole
units (`n << 32` raw) or raw values at the ends of the range.

## Test Cases

| ID | Category | Requirements | Case | Oracle | Test |
| --- | --- | --- | --- | --- | --- |
| DEGEN-TP-001 | Box constructors | DEGEN-REQ-001, DEGEN-REQ-002, DEGEN-REQ-009 | new keeps an inverted box; try_new refuses it with InvalidAabbBounds; a zero-size box and the full raw range are accepted. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_001_characterization_box_constructors` |
| DEGEN-TP-002 | Radius constructors | DEGEN-REQ-003, DEGEN-REQ-004, DEGEN-REQ-009 | Sphere and capsule new keep -1 raw; try_new refuses it and accepts 0 and i64::MAX; a point capsule is accepted. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_002_characterization_radius_constructors` |
| DEGEN-TP-003 | Zero direction | DEGEN-REQ-005, DEGEN-REQ-009 | FixedUnitVec3::new, FixedRay3::try_new and FixedLine3::try_new refuse zero and (1, 1, 0) raw and (i64::MIN, 0, 0); (1, 0, 0) raw and (i64::MAX, 0, 0) are accepted. | Hand-derived lengths. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_003_characterization_zero_direction` |
| DEGEN-TP-004 | Triangle constructors | DEGEN-REQ-006, DEGEN-REQ-009 | Collinear, repeated and single-point triangles are degenerate and refused by try_new; on x = y = z at both ends of the raw range degenerate, one raw unit off not. | Exact collinearity by hand. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_004_characterization_triangle_constructors` |
| DEGEN-TP-005 | Oriented box | DEGEN-REQ-002, DEGEN-REQ-003 | Zero half extents accepted, a negative one refused naming the axis, a zero axis refused as out of length tolerance. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_005_characterization_obb_constructor` |
| DEGEN-TP-006 | Bounds helpers | DEGEN-REQ-001, DEGEN-REQ-003, DEGEN-REQ-006, DEGEN-REQ-007, DEGEN-REQ-009 | from_points of nothing is NoPoints; from_triangle of a collinear triangle; merge, expand and bounding_sphere refuse an inverted box; from_sphere refuses -1 and gives the point box for 0; the full box cannot grow or be bounded by a sphere. | Hand-derived boxes. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_006_characterization_bounds_helpers` |
| DEGEN-TP-007 | Zero-normal plane | DEGEN-REQ-008, DEGEN-REQ-009 | Signed distance -offset at the origin and both ends of the range; On and Behind everywhere; a segment is kept whole or outside. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_007_characterization_zero_normal_plane` |
| DEGEN-TP-008 | Coincident planes | DEGEN-REQ-002, DEGEN-REQ-008 | z >= 0 and -z >= 0: a point on z = 0 is On for both; a frustum of such pairs contains the plane only; a flat box on it is Inside. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_008_characterization_coincident_planes` |
| DEGEN-TP-009 | Segment clip and classify | DEGEN-REQ-004, DEGEN-REQ-009 | Zero-length segments in front, behind and on the plane; the flyingrobots/bunny#472 example is None and one raw unit inside the threshold Kept; a distance of i64::MAX is returned and one past it is None. | Hand-derived; the #472 example. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_009_characterization_segment_clip_and_classify` |
| DEGEN-TP-010 | Inverted box against planes | DEGEN-REQ-001 | An inverted box classifies as its swapped box against a straddled and a clear plane and against a frustum. | The swapped box. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_010_characterization_inverted_box_against_planes` |
| DEGEN-TP-011 | Polygon clip | DEGEN-REQ-006, DEGEN-REQ-007 | The empty and two-vertex polygons are TooFewVertices; collinear, repeated and on-plane single-point polygons are Kept(3) unchanged. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_011_characterization_polygon_clip` |
| DEGEN-TP-012 | Frustum degenerates | DEGEN-REQ-003, DEGEN-REQ-008, DEGEN-REQ-009 | classify_sphere refuses -1 raw; zero radius on and behind a plane is Inside and Outside; the zero clip matrix contains the origin and both ends of the range. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_012_characterization_frustum_degenerates` |
| DEGEN-TP-013 | Transforms | DEGEN-REQ-001, DEGEN-REQ-002, DEGEN-REQ-005 | checked_transform refuses an inverted box, maps a point box to itself under the identity, and refuses a ray under the zero linear part as DegenerateDirection. | Hand-derived. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_013_characterization_transforms` |
| DEGEN-TP-014 | Predicates | DEGEN-REQ-006, DEGEN-REQ-009 | orientation2 and orientation3 of collinear, coplanar and repeated points at both ends of the range; barycentric2 and barycentric3_coplanar of degenerate triangles are None. | Exact collinearity by hand. | `crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_014_characterization_predicates` |
| DEGEN-TP-020 | Ray and inverted box | DEGEN-REQ-001, DEGEN-REQ-002 | ray_intersects_aabb and the prepared form: a box inverted on x (direction nonzero) is the swapped hit (10, 11); inverted on y (direction zero) is None; a point box gives (10, 10). | Hand-derived slab distances. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_020_characterization_ray_aabb_inverted_and_zero_size` |
| DEGEN-TP-021 | Box entry and inverted box | DEGEN-REQ-001, DEGEN-REQ-002 | TraceRay::box_entry: the swapped box is entered at 10; inverted on x or y is None; a point box is entered at 10. | Hand-derived. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_021_characterization_box_entry_inverted_and_zero_size` |
| DEGEN-TP-022 | Ray and sphere radius | DEGEN-REQ-003 | Radius 1 and -1 give the same hit at (-1, 0, 0); radius 0 on the ray is None. | Hand-derived. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_022_characterization_ray_sphere_radius` |
| DEGEN-TP-023 | Ray and degenerate triangle | DEGEN-REQ-006 | ray_intersects_triangle and TraceRay::hit_triangle miss a collinear and a single-point triangle; TraceTriangle::new accepts both. | Zero exact determinant by hand. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_023_characterization_ray_degenerate_triangle` |
| DEGEN-TP-024 | Closest point on a box | DEGEN-REQ-001, DEGEN-REQ-002, DEGEN-REQ-009 | On the inverted axis [10, 0]: 5 and -5 give 10, 20 gives 0; a point box gives its point; the full box returns a point at both ends unchanged. | Hand-derived clamp. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_024_characterization_closest_point_aabb` |
| DEGEN-TP-025 | Box and sphere | DEGEN-REQ-001, DEGEN-REQ-003, DEGEN-REQ-009 | Radius 1 and -1 meet the segment box at (2, 0, 0); radius 0 meets it only on it; an inverted box misses; radius 65 536 at (MAX, MAX, MAX) is reported as meeting the point box at (MIN, MIN, MIN), a saturated false overlap. | Hand-derived distances; the true distance is about 3.7e9 units. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_025_characterization_aabb_sphere` |
| DEGEN-TP-026 | Closest point on a degenerate triangle | DEGEN-REQ-006, DEGEN-REQ-009 | Collinear distinct vertices give the closest point of their segment for three points; a repeated vertex gives (0, 0, 0) for (1, 1, 0), not the closest (1, 0, 0); a single point gives itself; a raw-range-wide triangle is within one raw unit. | Hand-derived closest points on the segment hull. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_026_characterization_closest_point_degenerate_triangle` |
| DEGEN-TP-027 | Closest points of degenerate segments | DEGEN-REQ-004, DEGEN-REQ-009 | Two points give themselves; a point against a segment gives its projection 2 raw units off (3, 0, 0); parallel segments give one valid pair; raw-range-wide crossing segments are within one raw unit. | Hand-derived. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_027_characterization_closest_points_degenerate_segments` |
| DEGEN-TP-028 | Trace constructors | DEGEN-REQ-002, DEGEN-REQ-003, DEGEN-REQ-005, DEGEN-REQ-009 | TraceRay::new refuses zero and 2^40 raw; TraceBox accepts a point box and refuses inverted and out-of-domain corners; TraceSphere, TraceDisc and TraceCylinder refuse 0 and -1 radius; a cylinder with bottom equal to or above top is CapsNotOrdered. | Hand-derived. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_028_characterization_trace_constructors` |
| DEGEN-TP-029 | Shape frame | DEGEN-REQ-005 | Repeated axes are AxesNotOrthogonal, swapped axes AxesLeftHanded, the translation frame is accepted. | Hand-derived. | `crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_029_characterization_shape_frame` |
| DEGEN-TP-040 | Broadphase box helpers | DEGEN-REQ-001, DEGEN-REQ-002, DEGEN-REQ-009 | aabbs_overlap of an inverted box with a box around it and with itself, and of a point box with itself; surface_area of an inverted axis and of the full box; get_centroid of point boxes at both ends. | Hand-derived; MAX / 2 + 1 and MIN / 2 from saturated sums. | `crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_040_characterization_box_helpers` |
| DEGEN-TP-041 | Broadphase empty input | DEGEN-REQ-007 | Both builders and sweep_and_prune return Some(0); uniform_grid_pairs an empty complete report; compact_bvh Ok(None); refit_bvh Ok; intersect_aabb and closest_hit visit nothing. | Hand-derived. | `crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_041_characterization_empty_input` |
| DEGEN-TP-042 | Broadphase inverted boxes | DEGEN-REQ-001 | sweep_and_prune pairs an inverted box with a box around it; uniform_grid_pairs refuses the same input with InvertedBox { index: 1 }; both builders make one leaf whose bounds are the as-given union; intersect_aabb reports both primitives of the leaf. | Hand-derived. | `crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_042_characterization_inverted_boxes_in_broadphase` |
| DEGEN-TP-043 | Broadphase zero-size boxes and cells | DEGEN-REQ-002 | Two equal point boxes are one pair for sweep_and_prune and uniform_grid_pairs; cell sizes 0 and -1 raw are refused. | Hand-derived. | `crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_043_characterization_zero_size_boxes_and_cells` |
| DEGEN-TP-060 | Mesh bounds | DEGEN-REQ-006, DEGEN-REQ-007 | No triangles is EmptyMesh; a collinear and a repeated-index triangle are bounded. | Hand-derived. | `crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_060_characterization_mesh_bounds` |
| DEGEN-TP-061 | Quantization ranges | DEGEN-REQ-001, DEGEN-REQ-002, DEGEN-REQ-009 | Zero-size and inverted ranges map five values to code 0 and u16::MAX to min; the full raw range quantizes its ends to 0 and u16::MAX and dequantizes u16::MAX to i64::MAX; an inverted box quantizes a vertex to (0, 0, 0). | Hand-derived. | `crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_061_characterization_quantization_ranges` |
| DEGEN-TP-062 | Topology degenerates | DEGEN-REQ-006, DEGEN-REQ-007 | An empty mesh gives an empty complete report; a repeated-index and a collinear triangle give one finding of each degenerate kind. | Hand-derived. | `crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_062_characterization_topology_degenerates` |
| DEGEN-TP-063 | Mesh hash inputs | DEGEN-REQ-001, DEGEN-REQ-007 | An empty mesh is hashed with ordered and with inverted bounds, and the two hashes differ. | Inequality of the hashes. | `crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_063_characterization_mesh_hash_inputs` |
| DEGEN-TP-080 | Singular matrix inverse | DEGEN-REQ-010 | The flyingrobots/bunny#483 matrix is exactly singular (checked in i128), its determinant is raw 1, and FixedMat3::try_inverse and FixedAffine3::try_inverse return Some. | Exact row combination in i128. | `crates/bunny-linalg/tests/degeneracy_characterization_tests.rs::degen_tp_080_characterization_singular_matrix_is_inverted` |

```toml
[[case]]
id = "DEGEN-TP-001"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_001_characterization_box_constructors"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-002"
requirements = ["DEGEN-REQ-003", "DEGEN-REQ-004", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_002_characterization_radius_constructors"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-003"
requirements = ["DEGEN-REQ-005", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_003_characterization_zero_direction"
oracle = "Hand-derived lengths."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-004"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_004_characterization_triangle_constructors"
oracle = "Exact collinearity by hand."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-005"
requirements = ["DEGEN-REQ-002", "DEGEN-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_005_characterization_obb_constructor"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-006"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-003", "DEGEN-REQ-006", "DEGEN-REQ-007", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_006_characterization_bounds_helpers"
oracle = "Hand-derived boxes."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-007"
requirements = ["DEGEN-REQ-008", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_007_characterization_zero_normal_plane"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-008"
requirements = ["DEGEN-REQ-002", "DEGEN-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_008_characterization_coincident_planes"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-009"
requirements = ["DEGEN-REQ-004", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_009_characterization_segment_clip_and_classify"
oracle = "Hand-derived; the #472 example."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-010"
requirements = ["DEGEN-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_010_characterization_inverted_box_against_planes"
oracle = "The swapped box."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-011"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-007"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_011_characterization_polygon_clip"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-012"
requirements = ["DEGEN-REQ-003", "DEGEN-REQ-008", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_012_characterization_frustum_degenerates"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-013"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002", "DEGEN-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_013_characterization_transforms"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-014"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/degeneracy_characterization_tests.rs::degen_tp_014_characterization_predicates"
oracle = "Exact collinearity by hand."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-020"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_020_characterization_ray_aabb_inverted_and_zero_size"
oracle = "Hand-derived slab distances."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-021"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_021_characterization_box_entry_inverted_and_zero_size"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-022"
requirements = ["DEGEN-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_022_characterization_ray_sphere_radius"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-023"
requirements = ["DEGEN-REQ-006"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_023_characterization_ray_degenerate_triangle"
oracle = "Zero exact determinant by hand."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-024"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_024_characterization_closest_point_aabb"
oracle = "Hand-derived clamp."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-025"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-003", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_025_characterization_aabb_sphere"
oracle = "Hand-derived distances; the true distance is about 3.7e9 units."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-026"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_026_characterization_closest_point_degenerate_triangle"
oracle = "Hand-derived closest points on the segment hull."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-027"
requirements = ["DEGEN-REQ-004", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_027_characterization_closest_points_degenerate_segments"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-028"
requirements = ["DEGEN-REQ-002", "DEGEN-REQ-003", "DEGEN-REQ-005", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_028_characterization_trace_constructors"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-029"
requirements = ["DEGEN-REQ-005"]
evidence = "test"
test = "crates/bunny-query/tests/degeneracy_characterization_tests.rs::degen_tp_029_characterization_shape_frame"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-040"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_040_characterization_box_helpers"
oracle = "Hand-derived; MAX / 2 + 1 and MIN / 2 from saturated sums."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-041"
requirements = ["DEGEN-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_041_characterization_empty_input"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-042"
requirements = ["DEGEN-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_042_characterization_inverted_boxes_in_broadphase"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-043"
requirements = ["DEGEN-REQ-002"]
evidence = "test"
test = "crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs::degen_tp_043_characterization_zero_size_boxes_and_cells"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-060"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_060_characterization_mesh_bounds"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-061"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-002", "DEGEN-REQ-009"]
evidence = "test"
test = "crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_061_characterization_quantization_ranges"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-062"
requirements = ["DEGEN-REQ-006", "DEGEN-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_062_characterization_topology_degenerates"
oracle = "Hand-derived."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-063"
requirements = ["DEGEN-REQ-001", "DEGEN-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/degeneracy_characterization_tests.rs::degen_tp_063_characterization_mesh_hash_inputs"
oracle = "Inequality of the hashes."
tier = "fast"
status = "implemented"

[[case]]
id = "DEGEN-TP-080"
requirements = ["DEGEN-REQ-010"]
evidence = "test"
test = "crates/bunny-linalg/tests/degeneracy_characterization_tests.rs::degen_tp_080_characterization_singular_matrix_is_inverted"
oracle = "Exact row combination in i128."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

Every case is a constructed input with a hand-derived answer and no
randomness. The tests run in `cargo test --workspace` on Linux, macOS and
Windows in the `determinism-matrix` CI job and under WebAssembly in
`wasm-headless`, where they must give the same answers.

## Gaps

- No mutation checks are recorded: these tests pin existing behaviour and add
  no kernel, so there is no new rule to break on purpose.
- Coincident planes apply only to `FixedPlane3` and `FixedFrustum`; no
  `bunny-query`, `bunny-broadphase` or `bunny-mesh` function takes a plane.
- `TraceBox::new`, `TraceSphere::new` and the other trace constructors refuse
  coordinates at `2^40` raw and beyond, so their queries are not exercised at
  the ends of the raw range.
- `closest_point_triangle` and `closest_points_segments` use saturating
  operators; the cases pinned at the ends of the range stay within one raw
  unit, and no input is pinned where saturation changes their answer.
- `compact_closest_hit`, `compact_any_hit`, `any_hit` and the counted searches
  share their searches with `closest_hit` and are not pinned separately on
  empty input.
