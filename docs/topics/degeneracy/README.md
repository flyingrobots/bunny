# Degeneracy

This chapter is the inventory of what Bunny's public shape constructors and
queries in `bunny-geom`, `bunny-query`, `bunny-broadphase` and `bunny-mesh` do
today with degenerate input: inverted and zero-size boxes, negative and zero
radii, zero-length segments, zero directions, collinear and repeated triangle
vertices, empty inputs, coincident and zero-normal planes, and values at the
ends of the raw range (`i64::MIN` and `i64::MAX` raw). It answers Rule 7 of
`CODE_STANDARDS.md` ("Geometry Degeneracy Is Not an Edge Case") for the
functions that exist on the default branch.

## Status

This is a living reference of CURRENT behaviour, not a policy.

Every cell below is pinned by a characterization test (the
`degen_tp_*_characterization_*` tests listed in the [test plan](test-plan.md)).
A characterization test asserts what the code does now, whatever that is. It
is not an endorsed contract: several cells disagree with each other, and the
[Inconsistencies](#inconsistencies) section lists them. A uniform policy, and
which of these cells it would change, is an open decision for the repository
owner, tracked in flyingrobots/bunny#116. Until that decision lands, a change
to any cell here is a behaviour change to a public function: change the test,
this chapter and the function's documentation together.

No function panics on any input pinned here. Every refusal is a typed error
or `None`.

## Reading The Cells

| Word | Meaning |
| --- | --- |
| refused | A typed error or `None` before any arithmetic on the shape. |
| as given | The fields are used in the formula unchanged, with no check. |
| swapped | An inverted axis is read as the box with the same eight corners: the same answer as the box with that axis's `min` and `max` exchanged. |
| empty | The shape is treated as containing no point. |
| magnitude | A negative radius gives the answer of its absolute value. |
| saturated | An intermediate is clamped to the raw range by the saturating `+`, `-` and `*` of `FixedQ32_32`, so the answer can be wrong without any signal. |

## Inverted Boxes

A box with `min > max` on some axis. `FixedAabb3::new` and the public fields
keep one as given; `FixedAabb3::try_new` refuses it with `InvalidAabbBounds`.

| Function | Today | Case |
| --- | --- | --- |
| `FixedAabb3::merge`, `expand`, `expand_axes`, `bounding_sphere` | refused, `BoundsError::InvertedBox` | DEGEN-TP-006 |
| `FixedAabb3::checked_transform` | refused, `TransformError::InvertedBox` | DEGEN-TP-013 |
| `FixedPlane3::classify_aabb`, `FixedFrustum::classify_aabb` | swapped | DEGEN-TP-010 |
| `ray_intersects_aabb`, `ray_intersects_aabb_prepared` | swapped on an axis whose direction component is nonzero; a miss on an axis whose component is zero | DEGEN-TP-020 |
| `TraceRay::box_entry` | a miss on every axis | DEGEN-TP-021 |
| `TraceBox::new` | refused, `MinimumAboveMaximum` | DEGEN-TP-028 |
| `closest_point_aabb` | as given: per axis a coordinate below `min` gives `min` and any other gives `max`, so on `[10, 0]` the point 5 gives 10 and 20 gives 0 | DEGEN-TP-024 |
| `aabb_intersects_sphere` | as given, through `closest_point_aabb` | DEGEN-TP-025 |
| `utils::union_aabb`, `refit_bvh` | as given: per axis least `min` and greatest `max`, flyingrobots/bunny#477 | BOUNDS-TP-012 |
| `utils::aabbs_overlap`, `intersect_aabb` | as given: an inverted box overlaps a box around it but not itself | DEGEN-TP-040, DEGEN-TP-042 |
| `utils::surface_area` | an inverted axis counts as zero extent | DEGEN-TP-040 |
| `sweep_and_prune`, `build_bvh`, `build_bvh_sah` | as given | DEGEN-TP-042 |
| `uniform_grid_pairs`, `grid_key_count` | refused, `UniformGridError::InvertedBox` with the lowest index | DEGEN-TP-042 |
| `quantize_scalar`, `quantize_vertex` | every value is code 0 | DEGEN-TP-061 |
| `dequantize_scalar`, `dequantize_vertex` | every code is `min` | DEGEN-TP-061 |
| `compute_mesh_hash` | hashed as given; differs from the swapped box | DEGEN-TP-063 |

## Zero-Size Boxes

A box with `min == max` on one or more axes (a flat box, a segment or a
point). Accepted by every constructor, `TraceBox::new` included.

| Function | Today | Case |
| --- | --- | --- |
| `FixedAabb3::try_new`, `TraceBox::new` | accepted | DEGEN-TP-001, DEGEN-TP-028 |
| `FixedAabb3::checked_transform` | the point transform, as a point box | DEGEN-TP-013 |
| `FixedFrustum::classify_aabb` | a flat box on a zero-thickness slab is `Inside` | DEGEN-TP-008 |
| `ray_intersects_aabb` | a point box on the ray is a hit with equal entry and exit | DEGEN-TP-020 |
| `TraceRay::box_entry` | a point box on the ray is entered | DEGEN-TP-021 |
| `closest_point_aabb` | the box's point | DEGEN-TP-024 |
| `utils::aabbs_overlap`, `sweep_and_prune`, `uniform_grid_pairs` | two equal point boxes overlap and are a pair | DEGEN-TP-040, DEGEN-TP-043 |
| `quantize_scalar`, `dequantize_scalar` | every value is code 0; every code is `min` | DEGEN-TP-061 |

## Negative And Zero Radii

`FixedSphere3::new` and `FixedCapsule3::new` keep a negative radius;
`try_new` refuses it and accepts zero.

| Function | Negative | Zero | Case |
| --- | --- | --- | --- |
| `FixedSphere3::try_new`, `FixedCapsule3::try_new` | refused | accepted | DEGEN-TP-002 |
| `FixedAabb3::from_sphere` | refused, `NegativeRadius` | the point box | DEGEN-TP-006 |
| `FixedFrustum::classify_sphere` | refused, `NegativeRadius` | the point rule | DEGEN-TP-012 |
| `ray_intersects_sphere` | magnitude | never hit, even by a ray through the centre | DEGEN-TP-022 |
| `aabb_intersects_sphere` | magnitude | point containment | DEGEN-TP-025 |
| `TraceSphere::new`, `TraceDisc::new`, `TraceCylinder::new` | refused, `RadiusNotPositive` | refused, `RadiusNotPositive` | DEGEN-TP-028 |

A cylinder of zero height (`bottom == top`) is refused with `CapsNotOrdered`
(DEGEN-TP-028).

## Zero-Length Segments

`FixedSegment3::new` and the public fields accept `start == end`.

| Function | Today | Case |
| --- | --- | --- |
| `FixedCapsule3::try_new` | accepted: the sphere about the point | DEGEN-TP-002 |
| `FixedPlane3::clip_segment` | `Kept` unchanged or `Outside`, by the point's side | DEGEN-TP-009 |
| `FixedPlane3::classify_segment` | the point's side; `On` is `Coplanar` | DEGEN-TP-009 |
| `closest_points_segments` | the point itself, or its clamped projection on the other segment | DEGEN-TP-027 |

## Zero Directions

`FixedRay3::new` and `FixedLine3::new` take a `FixedUnitVec3`, which cannot be
zero. Every constructor that normalizes refuses a zero direction, and also a
nonzero one that does not normalize: `(1, 1, 0)` raw, or any component at
`i64::MIN` raw, whose length exceeds `i64::MAX`.

| Function | Today | Case |
| --- | --- | --- |
| `FixedUnitVec3::new` | `None` | DEGEN-TP-003 |
| `FixedRay3::try_new` | refused, `InvalidRayDirection` | DEGEN-TP-003 |
| `FixedLine3::try_new` | refused, `InvalidLineDirection` | DEGEN-TP-003 |
| `TraceRay::new` | `None`; one raw unit is accepted | DEGEN-TP-028 |
| `FixedRay3::checked_transform` | a direction that maps to zero is refused, `DegenerateDirection` | DEGEN-TP-013 |

## Degenerate Triangles

Collinear vertices, a repeated vertex, or one point. `FixedTriangle3::new`
accepts them; `FixedTriangle3::try_new` refuses them with
`DegenerateTriangle`, decided exactly at every magnitude.

| Function | Today | Case |
| --- | --- | --- |
| `FixedTriangle3::is_degenerate` | exact, including at the ends of the raw range | DEGEN-TP-004 |
| `FixedAabb3::from_triangle`, `mesh_bounds` | accepted | DEGEN-TP-006, DEGEN-TP-060 |
| `orientation2`, `orientation3` | `Collinear`, `Coplanar` | DEGEN-TP-014 |
| `barycentric2`, `barycentric3_coplanar` | `None` | DEGEN-TP-014 |
| `ray_intersects_triangle` | never hit | DEGEN-TP-023 |
| `TraceTriangle::new` | accepted; `TraceRay::hit_triangle` never hits it | DEGEN-TP-023 |
| `closest_point_triangle` | collinear distinct vertices: the closest point of the segment; a repeated vertex: not always the closest point | DEGEN-TP-026 |
| `FixedPlane3::clip_polygon` | clipped like any polygon, repeated vertices kept | DEGEN-TP-011 |
| `validate_topology` | findings `IndexDegenerateTriangle` and `GeometricallyDegenerateTriangle`, never an error | DEGEN-TP-062 |

## Empty Inputs

| Function | Today | Case |
| --- | --- | --- |
| `FixedAabb3::from_points` | refused, `NoPoints` | DEGEN-TP-006 |
| `mesh_bounds` | refused, `EmptyMesh` | DEGEN-TP-060 |
| `FixedPlane3::clip_polygon` | refused, `TooFewVertices` (also for one or two vertices) | DEGEN-TP-011 |
| `build_bvh`, `build_bvh_sah`, `sweep_and_prune` | `Some(0)` | DEGEN-TP-041 |
| `uniform_grid_pairs` | an empty, complete report | DEGEN-TP-041 |
| `compact_bvh` | `Ok(None)` | DEGEN-TP-041 |
| `refit_bvh` | `Ok(())` | DEGEN-TP-041 |
| `intersect_aabb`, `closest_hit` | no visit; `Ok(())`, `Ok(None)` | DEGEN-TP-041 |
| `validate_topology` | an empty, complete report | DEGEN-TP-062 |
| `compute_mesh_hash` | a hash | DEGEN-TP-063 |

## Coincident And Zero-Normal Planes

`FixedPlane3::new` accepts a zero normal, and `FixedFrustum::new` holds its six
planes as given.

| Input | Today | Case |
| --- | --- | --- |
| A zero normal | every point has signed distance `-offset`: all `On`, all `Front` or all `Behind` | DEGEN-TP-007 |
| Two coincident planes facing each other | a point on them is `On` for both; a frustum of such pairs is the closed zero-thickness slab | DEGEN-TP-008 |
| `FixedFrustum::from_clip_matrix` of the zero matrix | six zero-normal planes; every point is inside | DEGEN-TP-012 |

## Values At The Ends Of The Raw Range

| Function | Today | Case |
| --- | --- | --- |
| Constructors (`try_new` of boxes, spheres, triangles) | accept the full range | DEGEN-TP-001, DEGEN-TP-002 |
| `orientation2`, `orientation3`, `is_degenerate` | exact | DEGEN-TP-004, DEGEN-TP-014 |
| `FixedPlane3::signed_distance` and the queries on it | `None` when the distance leaves the raw range | DEGEN-TP-009 |
| `FixedPlane3::clip_segment` | `None` for a touching segment whose `end - start` overflows, flyingrobots/bunny#472 | DEGEN-TP-009 |
| `FixedAabb3::expand`, `bounding_sphere` | refused, `OutOfRange` | DEGEN-TP-006 |
| `closest_point_aabb` | exact | DEGEN-TP-024 |
| `aabb_intersects_sphere` | saturated: a false overlap | DEGEN-TP-025 |
| `closest_point_triangle`, `closest_points_segments` | saturated operators; within one raw unit on the cases pinned | DEGEN-TP-026, DEGEN-TP-027 |
| `utils::surface_area`, `utils::get_centroid` | saturated: the centroid of a point box at `i64::MAX` is `i64::MAX / 2 + 1` | DEGEN-TP-040 |
| `quantize_scalar`, `dequantize_scalar` | exact | DEGEN-TP-061 |
| Trace-domain constructors | refused at `2^40` raw | DEGEN-TP-028 |

## Matrices

The `bunny-geom` transforms map through `FixedAffine3`. `FixedMat3::try_inverse`
and `FixedAffine3::try_inverse` test the ROUNDED determinant for zero, so an
exactly singular matrix whose determinant rounds to a nonzero raw value is
inverted (flyingrobots/bunny#483, DEGEN-TP-080).

## Inconsistencies

These are facts about the current code, each pinned above. Which of them to
resolve, and how, is the open decision in flyingrobots/bunny#116.

1. An inverted box is refused by the bounds helpers, `checked_transform`,
   `TraceBox::new` and `uniform_grid_pairs`; swapped by plane and frustum
   classification; swapped or a miss by `ray_intersects_aabb` depending on
   the ray; a miss by `TraceRay::box_entry`; and as given by `union_aabb`,
   `closest_point_aabb`, `aabbs_overlap`, `sweep_and_prune` and the BVH
   builders.
2. `uniform_grid_pairs` documents the pairs of `sweep_and_prune`, but refuses
   an inverted box that `sweep_and_prune` pairs.
3. `ray_intersects_aabb` and `TraceRay::box_entry` disagree on a box inverted
   on an axis the ray moves along.
4. A zero radius is accepted by `FixedSphere3::try_new` and refused by
   `TraceSphere::new`; a zero-radius sphere is a point to the frustum and to
   `aabb_intersects_sphere`, and never hit by `ray_intersects_sphere`.
5. A negative radius is refused by `from_sphere` and `classify_sphere` and read
   as its magnitude by `ray_intersects_sphere` and `aabb_intersects_sphere`.
6. `closest_point_triangle` can return a point that is not the closest when a
   vertex is repeated.
7. `aabb_intersects_sphere`, `closest_point_triangle`,
   `closest_points_segments`, `surface_area` and `get_centroid` use saturating
   operators and can give a wrong answer with no signal, where the plane,
   frustum, ray and bounds queries return `None` or `OutOfRange`.
8. `clip_segment` returns `None` for a representable touching answer
   (flyingrobots/bunny#472), and `try_inverse` inverts some exactly singular
   matrices (flyingrobots/bunny#483).
