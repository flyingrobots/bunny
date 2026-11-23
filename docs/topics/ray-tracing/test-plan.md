# Ray Tracing Test Plan

This document defines how Bunny verifies the ray-tracing contract: the kernels
and the closest-hit and any-hit traversals built on them.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design. It records active requirements, executable
evidence, and open gaps for the benchmarks that will build on this work.

## Scope

This plan covers `TraceRay`, `TraceTriangle` and `TriangleHit` in
`bunny-query`:

- the trace domain and its refusals;
- the conservative `box_entry` test, its never-reject guarantee and its bound;
- the exact `hit_triangle` test, its edge rule and its facing;
- rounding of distances and barycentric weights;
- overflow freedom at the edges of the domain;
- the exact `hit_sphere` test of `TraceSphere` (contract `bunny-ray-sphere/1`), its
  rounding rule, its facing and normal, and its typed refusals;
- the exact `hit_cylinder` test of `TraceCylinder` (contract `bunny-ray-cylinder/1`),
  its rounding rule, its facing and normals, its tie rule at the rim, its degenerate
  rays and its typed refusals;
- the exact `hit_box` test of `TraceBox` (contract `bunny-ray-box/1`), its rounding
  rule, its face normal, its tie rule at edges and corners, its rule for a ray parallel
  to a slab, and its typed refusals;
- the exact `hit_disc` test of `TraceDisc` (contract `bunny-ray-disc/1`), its exact radius test,
  its rounding rule, its facing and its typed refusals;
- the overflow behaviour of the general-purpose `ray_intersects_aabb` and its
  effect on `bunny_broadphase::intersect_ray` (flyingrobots/bunny#211).

It also covers `closest_hit`, `any_hit`, `BvhView` and `ClosestHit` in
`bunny-broadphase`:

- agreement with a test of every primitive, including the tie rule;
- independence from the tree's shape and leaf order;
- the inclusive distance limit;
- the search order and each pruning rule;
- the early stop of `any_hit`;
- malformed and empty trees, and the interior-visit bound that `closest_hit`,
  `any_hit`, `intersect_ray` and `intersect_aabb` share (flyingrobots/bunny#269).

The other solvers in `crates/bunny-query/src/ray.rs` keep their own evidence
and are out of scope. Timing is out of scope until the benchmark cases exist.

## Test Goals

- Compare every result with an oracle that does not share the kernel's
  rounding: constructed exact cases, an exact rational slab test, and
  hand-computed golden vectors.
- Assert exact raw Q32.32 outputs, never approximate ones.
- Show that each rounding direction, each inclusive comparison and each
  pruning rule is load bearing by breaking it on purpose.
- Check traversal results against a test of every primitive, which shares the
  primitive test but none of the tree logic.
- Keep the evidence native and WASM-compatible: every case is a
  `wasm_bindgen_test` that also runs as a native test.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| TRACE-REQ-001 | `TraceRay::new` and `TraceTriangle::new` refuse coordinates outside the trace domain, direction components above 1.0 (`TraceRay::new`) and a zero direction. | `README.md#trace-domain` |
| TRACE-REQ-002 | box_entry never returns None for a box the ray reaches within 0..=t_max, including face, edge and corner contact. | `README.md#conservative-box-test` |
| TRACE-REQ-003 | box_entry returns a lower bound on the exact entry distance, short by at most 2^11 + 2 raw units plus entry / 2^30. | `README.md#conservative-box-test` |
| TRACE-REQ-004 | hit_triangle reports a hit exactly when the ray crosses the triangle at a non-negative distance within its limits, and reports the floor of the true distance. | `README.md#exact-triangle-test` |
| TRACE-REQ-005 | Triangle edges and corners are inclusive, so a ray through an edge shared by two triangles hits both. | `README.md#exact-triangle-test` |
| TRACE-REQ-006 | Distance limits of box_entry and hit_triangle are inclusive. | `README.md#exact-triangle-test` |
| TRACE-REQ-007 | Barycentric weights are the floors of the exact weights of v1 and v2. | `README.md#exact-triangle-test` |
| TRACE-REQ-008 | front_face is true when the ray arrives on the side from which the vertices appear counter-clockwise. | `README.md#facing` |
| TRACE-REQ-009 | No intermediate overflows for any input inside the trace domain. | `README.md#trace-domain` |
| TRACE-REQ-010 | Parallel rays (including rays in the triangle's plane), zero-area triangles and crossings whose distance does not fit i64 are misses. | `README.md#exact-triangle-test` |
| TRACE-REQ-011 | ray_intersects_aabb returns every hit it reported before flyingrobots/bunny#211 with the same bits, and every miss not caused by overflow stays a miss. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-012 | When a coordinate difference or slab distance overflows, ray_intersects_aabb reports a hit exactly when an exact rational slab test does. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-013 | Reported ray_intersects_aabb bounds are the exact slab bounds rounded to nearest (ties to even) and saturated to the Q32.32 range, with t_enter <= t_exit; coordinate differences are widened, not saturated. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-014 | bunny_broadphase::intersect_ray visits a leaf whose box a near-axis ray passes through. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-021 | On every tree whose depth the traversal stack holds (TRACE-REQ-033), closest_hit returns the hit a test of every primitive chooses: the smallest reported distance in 0..=t_max, then the lowest primitive index. | `README.md#correctness-the-answer-does-not-depend-on-the-tree` |
| TRACE-REQ-022 | The result of closest_hit and any_hit does not depend on the tree's shape or the order its leaves list primitives in. | `README.md#correctness-the-answer-does-not-depend-on-the-tree` |
| TRACE-REQ-023 | The distance limit of closest_hit and any_hit is inclusive. | `README.md#closest-hit-traversal` |
| TRACE-REQ-024 | The nearer child is searched first; a subtree whose entry bound is greater than the best distance is not searched, and one whose bound equals it is. | `README.md#search-order-and-pruning` |
| TRACE-REQ-025 | any_hit reports a hit exactly when a test of every primitive finds one, and does not call the primitive test after the first hit. | `README.md#correctness-the-answer-does-not-depend-on-the-tree` |
| TRACE-REQ-026 | Malformed BVH buffers that the search reaches are errors, an empty tree misses, and reported distances outside 0..=limit are ignored. | `README.md#search-order-and-pruning` |
| TRACE-REQ-027 | hit_triangle returns exactly what a frozen reference copy of its arithmetic returns (distance, facing and weights) for every ray, triangle and pair of limits, so an exact replacement of the kernel changes no bit. | `README.md#exact-triangle-test` |
| TRACE-REQ-028 | compact_bvh converts a built tree into the compact layout deterministically (the same bytes on every target) and reports malformed input as a typed error; compact_closest_hit and compact_any_hit on the result make the same primitive calls in the same order and return the same answers as closest_hit and any_hit on the source tree. | `README.md#compact-layout` |
| TRACE-REQ-029 | box_entry returns exactly what the retained pre-#258 box test returns (the whole Option<FixedQ32_32>) for every ray, every FixedAabb3, inverted boxes and boxes at the ends of the i64 range included, and every t_max, negative included. | `README.md#conservative-box-test` |
| TRACE-REQ-030 | TriangleHit::barycentric_numerators returns the exact u, v and det the triangle test decided the hit with (the weights of v1 and v2 are u / det and v / det, each below 2^117 in the trace domain), and BarycentricNumerators::new accepts exactly the values with det > 0, u >= 0, v >= 0 and u + v <= det. | `README.md#exact-triangle-test` |
| TRACE-REQ-031 | intersect_aabb, intersect_ray, closest_hit and any_hit make at most nodes.len() / 2 interior visits: a buffer whose search needs more (a reference cycle, or a subtree reached twice) is InvalidNodeIndex after a bounded number of steps, and on every buffer in which no node is reached twice the results, errors and callback calls are those of the retained pre-#269 functions. | `README.md#malformed-buffers` |
| TRACE-REQ-032 | `TraceRay::from_ray` returns `None` only for an origin outside the trace domain: it accepts every unit direction, including a component of `2^32 + 1` raw, which `FixedUnitVec3::try_from_unit` allows and `TraceRay::new` refuses; every width bound of the kernels holds for it, and `FixedUnitVec3::new` never produces a component above `2^32`. | `README.md#trace-domain` |
| TRACE-REQ-033 | intersect_aabb, intersect_ray, closest_hit, any_hit, compact_closest_hit and compact_any_hit hold at most d + 1 entries of their 64-entry stacks on a tree of depth d (the root at depth 0), so every tree of depth at most 63 is searched completely; on a deeper tree each returns its complete answer or StackOverflow, and a comb that keeps one pending sibling per level returns StackOverflow at depth 64. | `README.md#depth-and-the-traversal-stack` |
| TRACE-REQ-034 | compact_bvh queues at most d interior subtrees on a tree of depth d, so every tree of depth at most 64 converts unless another error applies; TooDeep means more than 64 interior subtrees waiting at once, which a tree of depth 65 can need. | `README.md#compact-layout` |
| TRACE-REQ-035 | `compact_closest_hit_counted` and compact_any_hit_counted return exactly the result or error of compact_closest_hit and compact_any_hit and make the same primitive calls in the same order on every CompactView, and on success their TraversalCounts hold, for that search, the box_entry calls, those that returned a bound, the popped entries skipped beyond the limit, the interior and leaf visits, the primitive tests and the deepest stack; TraversalCounts addition saturates and keeps the larger peak. | `README.md#counting-traversals` |
| TRACE-REQ-040 | TraceSphere::new accepts a centre and a positive radius inside the trace domain and refuses any other with a typed TraceSphereError (CenterOutsideTraceDomain, RadiusNotPositive, RadiusOutsideTraceDomain); no input to TraceSphere::new or TraceRay::hit_sphere wraps or panics. | `README.md#ray-sphere` |
| TRACE-REQ-041 | TraceRay::hit_sphere (contract bunny-ray-sphere/1, published as RAY_SPHERE_ID) returns, for every ray and sphere in the trace domain, the floor in raw units of the near root of the true quadratic when that rounded distance is in t_min..=t_max (front_face true), else that of the far root when it is (front_face false), else None. | `README.md#ray-sphere` |
| TRACE-REQ-042 | SphereHit::normal is a FixedUnitVec3: the outward normal at the exact (unrounded) root, normalized by Bunny's unit-vector rule (prescale to [2^61, 2^62), then FixedUnitVec3::new); it is never zero and each component is within 1/2 + 2^-24 raw units of 2^32 times the exact unit normal, for every admitted radius. | `README.md#ray-sphere` |
| TRACE-REQ-043 | A tangent ray hits at its one root with front_face true; an origin inside the sphere hits the far root from inside; a sphere whose roots are both below t_min is not hit. | `README.md#ray-sphere` |
| TRACE-REQ-065 | TraceCylinder::new accepts a positive radius and two cap heights bottom < top, all inside the trace domain, and refuses any other with a typed TraceCylinderError (RadiusNotPositive, RadiusOutsideTraceDomain, CapOutsideTraceDomain, CapsNotOrdered, checked in that order); no input to TraceCylinder::new or TraceRay::hit_cylinder wraps or panics. | `README.md#ray-cylinder` |
| TRACE-REQ-066 | TraceRay::hit_cylinder (contract bunny-ray-cylinder/1) returns, for every ray and cylinder about the +Y axis in the trace domain, the floor in raw units of the exact entry of the line into the closed solid when that rounded distance is in t_min..=t_max (front_face true), else that of the exact exit when it is (front_face false), else None; a cap distance is floor(n * 2^32 / q) and a side distance follows the rounding rule of bunny-ray-sphere/1. | `README.md#ray-cylinder` |
| TRACE-REQ-067 | CylinderHit::normal is a FixedUnitVec3: exactly (0, -1, 0) or (0, 1, 0) on the bottom or top cap, and on the side the unit normal at the exact root with y = 0, outward, each component within one raw unit of the exact unit normal; where a side root and a cap crossing are the same exact distance (the rim) the cap and its normal are reported, at the entry, at the exit and at a single touching point. | `README.md#ray-cylinder` |
| TRACE-REQ-069 | RAY_CYLINDER_ID is the published identity bunny-ray-cylinder/1 of TraceRay::hit_cylinder. | `README.md#ray-cylinder` |
| TRACE-REQ-068 | A line tangent to the side hits at its one contact point, front face; a ray parallel to the axis hits only a cap when inside or on the side and misses when outside; a ray in a cap's plane hits the side and one raw unit outside the plane misses; an origin inside hits the exit from inside; a cylinder whose rounded distances are both below t_min is not hit. | `README.md#ray-cylinder` |
| TRACE-REQ-044 | TraceBox::new accepts two corners inside the trace domain with no minimum above its maximum (equal bounds allowed) and refuses any other with a typed TraceBoxError (CornerOutsideTraceDomain, MinimumAboveMaximum); no input to TraceBox::new or TraceRay::hit_box wraps or panics. | `README.md#ray-box` |
| TRACE-REQ-045 | TraceRay::hit_box (contract bunny-ray-box/1) returns, for every ray and box in the trace domain, the floor in raw units of the true entry distance when that rounded distance is in t_min..=t_max (front_face true), else that of the true exit when it is (front_face false), else None; contact with a face, edge or corner is a hit. | `README.md#ray-box` |
| TRACE-REQ-046 | BoxHit::normal is the outward unit normal (raw 2^32 or -2^32 in one component, zero in the others) of the face crossed; when several faces are crossed at exactly the same true distance the lowest axis (x, y, z) is reported, for the entry and for the exit, and equal rounded distances are not a tie. | `README.md#ray-box` |
| TRACE-REQ-047 | An axis with a zero direction component never supplies a face: it rules the box out when the origin's coordinate is outside the closed slab and constrains nothing otherwise (a ray in a face's plane is inside it); an origin inside the box hits the exit from inside; a box whose entry and exit are both below t_min is not hit. | `README.md#ray-box` |
| TRACE-REQ-060 | TraceDisc::new accepts a height and a positive radius inside the trace domain and refuses any other with a typed TraceDiscError (HeightOutsideTraceDomain, RadiusNotPositive, RadiusOutsideTraceDomain); no input to TraceDisc::new or TraceRay::hit_disc wraps or panics. | `README.md#ray-disc` |
| TRACE-REQ-061 | TraceRay::hit_disc (contract bunny-ray-disc/1) returns, for every ray and disc in the trace domain with a nonzero direction z, the floor in raw units of the true distance to the plane z = height when that rounded distance is in t_min..=t_max and the exact point where the ray meets the plane is at most the radius from the axis (the rim is inside), else None; the radius test is decided on the exact meeting point, not the rounded one. | `README.md#ray-disc` |
| TRACE-REQ-062 | A ray whose direction has z = 0 misses every disc, including a ray that lies in the plane of the disc. | `README.md#ray-disc` |
| TRACE-REQ-063 | DiscHit::front_face is true exactly when the direction has a negative z; DiscHit::normal is (0, 0, 1) on both sides; a ray whose origin is on the disc has distance 0; a disc whose rounded distance is below t_min is not hit. | `README.md#ray-disc` |
| TRACE-REQ-070 | TraceRay::try_new returns Ok with the ray TraceRay::new builds, or the first failing of, in this order: OriginOutsideTraceDomain { origin } when an origin coordinate has abs(raw) >= 2^40, DirectionComponentTooLarge { direction } when a direction component has abs(raw) > 2^32, ZeroDirection when the direction is (0, 0, 0); each carries the values as given. | `README.md#typed-ray-and-triangle-refusals` |
| TRACE-REQ-071 | TraceTriangle::try_new returns Ok with the triangle TraceTriangle::new builds, or VertexOutsideTraceDomain { index, vertex } naming the first vertex in argument order with a coordinate of abs(raw) >= 2^40. | `README.md#typed-ray-and-triangle-refusals` |
| TRACE-REQ-072 | TraceRay::new and TraceTriangle::new, whose bodies are unchanged, return Some(x) exactly when their try_new forms return Ok(x), with equal values, for every input. | `README.md#typed-ray-and-triangle-refusals` |
| TRACE-REQ-073 | TraceRayError and TraceTriangleError (#[non_exhaustive]), TraversalError and CompactError implement Display, with a message that states the condition and the raw values it saw, and core::error::Error with no source, using only core. | `README.md#malformed-buffers` |

```toml
[[requirement]]
id = "TRACE-REQ-001"
summary = "TraceRay::new and TraceTriangle::new refuse coordinates outside the trace domain, direction components above 1.0 (TraceRay::new) and a zero direction."
status = "active"

[[requirement]]
id = "TRACE-REQ-002"
summary = "box_entry never returns None for a box the ray reaches within 0..=t_max, including face, edge and corner contact."
status = "active"

[[requirement]]
id = "TRACE-REQ-003"
summary = "box_entry returns a lower bound on the exact entry distance, short by at most 2^11 + 2 raw units plus entry / 2^30."
status = "active"

[[requirement]]
id = "TRACE-REQ-004"
summary = "hit_triangle reports a hit exactly when the ray crosses the triangle at a non-negative distance within its limits, and reports the floor of the true distance."
status = "active"

[[requirement]]
id = "TRACE-REQ-005"
summary = "Triangle edges and corners are inclusive, so a ray through an edge shared by two triangles hits both."
status = "active"

[[requirement]]
id = "TRACE-REQ-006"
summary = "Distance limits of box_entry and hit_triangle are inclusive."
status = "active"

[[requirement]]
id = "TRACE-REQ-007"
summary = "Barycentric weights are the floors of the exact weights of v1 and v2."
status = "active"

[[requirement]]
id = "TRACE-REQ-008"
summary = "front_face is true when the ray arrives on the side from which the vertices appear counter-clockwise."
status = "active"

[[requirement]]
id = "TRACE-REQ-009"
summary = "No intermediate overflows for any input inside the trace domain."
status = "active"

[[requirement]]
id = "TRACE-REQ-010"
summary = "Parallel rays (including rays in the triangle's plane), zero-area triangles and crossings whose distance does not fit i64 are misses."
status = "active"

[[requirement]]
id = "TRACE-REQ-011"
summary = "ray_intersects_aabb returns every hit it reported before flyingrobots/bunny#211 with the same bits, and every miss not caused by overflow stays a miss."
status = "active"

[[requirement]]
id = "TRACE-REQ-012"
summary = "When a coordinate difference or slab distance overflows, ray_intersects_aabb reports a hit exactly when an exact rational slab test does."
status = "active"

[[requirement]]
id = "TRACE-REQ-013"
summary = "Reported ray_intersects_aabb bounds are the exact slab bounds rounded to nearest (ties to even) and saturated to the Q32.32 range, with t_enter <= t_exit; coordinate differences are widened, not saturated."
status = "active"

[[requirement]]
id = "TRACE-REQ-014"
summary = "bunny_broadphase::intersect_ray visits a leaf whose box a near-axis ray passes through."
status = "active"

[[requirement]]
id = "TRACE-REQ-021"
summary = "On every tree whose depth the traversal stack holds (TRACE-REQ-033), closest_hit returns the hit a test of every primitive chooses: the smallest reported distance in 0..=t_max, then the lowest primitive index."
status = "active"

[[requirement]]
id = "TRACE-REQ-022"
summary = "The result of closest_hit and any_hit does not depend on the tree's shape or the order its leaves list primitives in."
status = "active"

[[requirement]]
id = "TRACE-REQ-023"
summary = "The distance limit of closest_hit and any_hit is inclusive."
status = "active"

[[requirement]]
id = "TRACE-REQ-024"
summary = "The nearer child is searched first; a subtree whose entry bound is greater than the best distance is not searched, and one whose bound equals it is."
status = "active"

[[requirement]]
id = "TRACE-REQ-025"
summary = "any_hit reports a hit exactly when a test of every primitive finds one, and does not call the primitive test after the first hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-026"
summary = "Malformed BVH buffers that the search reaches are errors, an empty tree misses, and reported distances outside 0..=limit are ignored."
status = "active"

[[requirement]]
id = "TRACE-REQ-027"
summary = "hit_triangle returns exactly what a frozen reference copy of its arithmetic returns (distance, facing and weights) for every ray, triangle and pair of limits, so an exact replacement of the kernel changes no bit."
status = "active"

[[requirement]]
id = "TRACE-REQ-028"
summary = "compact_bvh converts a built tree into the compact layout deterministically and reports malformed input as a typed error; compact_closest_hit and compact_any_hit on the result make the same primitive calls in the same order and return the same answers as closest_hit and any_hit on the source tree."
status = "active"

[[requirement]]
id = "TRACE-REQ-029"
summary = "box_entry returns exactly what the retained pre-#258 box test returns (the whole Option<FixedQ32_32>) for every ray, every FixedAabb3, inverted boxes and boxes at the ends of the i64 range included, and every t_max, negative included."
status = "active"

[[requirement]]
id = "TRACE-REQ-030"
summary = "TriangleHit::barycentric_numerators returns the exact u, v and det the triangle test decided the hit with (the weights of v1 and v2 are u / det and v / det, each below 2^117 in the trace domain), and BarycentricNumerators::new accepts exactly the values with det > 0, u >= 0, v >= 0 and u + v <= det."
status = "active"

[[requirement]]
id = "TRACE-REQ-031"
summary = "intersect_aabb, intersect_ray, closest_hit and any_hit make at most nodes.len() / 2 interior visits: a buffer whose search needs more (a reference cycle, or a subtree reached twice) is InvalidNodeIndex after a bounded number of steps, and on every buffer in which no node is reached twice the results, errors and callback calls are those of the retained pre-#269 functions."
status = "active"

[[requirement]]
id = "TRACE-REQ-032"
summary = "TraceRay::from_ray returns None only for an origin outside the trace domain: it accepts every unit direction, including a component of 2^32 + 1 raw, which FixedUnitVec3::try_from_unit allows and TraceRay::new refuses; every width bound of the kernels holds for it, and FixedUnitVec3::new never produces a component above 2^32."
status = "active"

[[requirement]]
id = "TRACE-REQ-033"
summary = "intersect_aabb, intersect_ray, closest_hit, any_hit, compact_closest_hit and compact_any_hit hold at most d + 1 entries of their 64-entry stacks on a tree of depth d (the root at depth 0), so every tree of depth at most 63 is searched completely; on a deeper tree each returns its complete answer or StackOverflow, and a comb that keeps one pending sibling per level returns StackOverflow at depth 64."
status = "active"

[[requirement]]
id = "TRACE-REQ-034"
summary = "compact_bvh queues at most d interior subtrees on a tree of depth d, so every tree of depth at most 64 converts unless another error applies; TooDeep means more than 64 interior subtrees waiting at once, which a tree of depth 65 can need."
status = "active"

[[requirement]]
id = "TRACE-REQ-035"
summary = "compact_closest_hit_counted and compact_any_hit_counted return exactly the result or error of compact_closest_hit and compact_any_hit and make the same primitive calls in the same order on every CompactView, and on success their TraversalCounts hold, for that search, the box_entry calls, those that returned a bound, the popped entries skipped beyond the limit, the interior and leaf visits, the primitive tests and the deepest stack; TraversalCounts addition saturates and keeps the larger peak."
status = "active"

[[requirement]]
id = "TRACE-REQ-040"
summary = "TraceSphere::new accepts a centre and a positive radius inside the trace domain and refuses any other with a typed TraceSphereError (CenterOutsideTraceDomain, RadiusNotPositive, RadiusOutsideTraceDomain); no input to TraceSphere::new or TraceRay::hit_sphere wraps or panics."
status = "active"

[[requirement]]
id = "TRACE-REQ-041"
summary = "TraceRay::hit_sphere (contract bunny-ray-sphere/1, published as RAY_SPHERE_ID) returns, for every ray and sphere in the trace domain, the floor in raw units of the near root of the true quadratic when that rounded distance is in t_min..=t_max (front_face true), else that of the far root when it is (front_face false), else None."
status = "active"

[[requirement]]
id = "TRACE-REQ-042"
summary = "SphereHit::normal is a FixedUnitVec3: the outward normal at the exact (unrounded) root, normalized by Bunny's unit-vector rule (prescale to [2^61, 2^62), then FixedUnitVec3::new); it is never zero and each component is within 1/2 + 2^-24 raw units of 2^32 times the exact unit normal, for every admitted radius."
status = "active"

[[requirement]]
id = "TRACE-REQ-043"
summary = "A tangent ray hits at its one root with front_face true; an origin inside the sphere hits the far root from inside; a sphere whose roots are both below t_min is not hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-065"
summary = "TraceCylinder::new accepts a positive radius and two cap heights bottom < top, all inside the trace domain, and refuses any other with a typed TraceCylinderError (RadiusNotPositive, RadiusOutsideTraceDomain, CapOutsideTraceDomain, CapsNotOrdered, checked in that order); no input to TraceCylinder::new or TraceRay::hit_cylinder wraps or panics."
status = "active"

[[requirement]]
id = "TRACE-REQ-066"
summary = "TraceRay::hit_cylinder (contract bunny-ray-cylinder/1) returns, for every ray and cylinder about the +Y axis in the trace domain, the floor in raw units of the exact entry of the line into the closed solid when that rounded distance is in t_min..=t_max (front_face true), else that of the exact exit when it is (front_face false), else None; a cap distance is floor(n * 2^32 / q) and a side distance follows the rounding rule of bunny-ray-sphere/1."
status = "active"

[[requirement]]
id = "TRACE-REQ-067"
summary = "CylinderHit::normal is a FixedUnitVec3: exactly (0, -1, 0) or (0, 1, 0) on the bottom or top cap, and on the side the unit normal at the exact root with y = 0, outward, each component within one raw unit of the exact unit normal; where a side root and a cap crossing are the same exact distance (the rim) the cap and its normal are reported, at the entry, at the exit and at a single touching point."
status = "active"

[[requirement]]
id = "TRACE-REQ-068"
summary = "A line tangent to the side hits at its one contact point, front face; a ray parallel to the axis hits only a cap when inside or on the side and misses when outside; a ray in a cap's plane hits the side and one raw unit outside the plane misses; an origin inside hits the exit from inside; a cylinder whose rounded distances are both below t_min is not hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-069"
summary = "RAY_CYLINDER_ID is the published identity bunny-ray-cylinder/1 of TraceRay::hit_cylinder."
status = "active"

[[requirement]]
id = "TRACE-REQ-044"
summary = "TraceBox::new accepts two corners inside the trace domain with no minimum above its maximum (equal bounds allowed) and refuses any other with a typed TraceBoxError (CornerOutsideTraceDomain, MinimumAboveMaximum); no input to TraceBox::new or TraceRay::hit_box wraps or panics."
status = "active"

[[requirement]]
id = "TRACE-REQ-045"
summary = "TraceRay::hit_box (contract bunny-ray-box/1) returns, for every ray and box in the trace domain, the floor in raw units of the true entry distance when that rounded distance is in t_min..=t_max (front_face true), else that of the true exit when it is (front_face false), else None; contact with a face, edge or corner is a hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-046"
summary = "BoxHit::normal is the outward unit normal (raw 2^32 or -2^32 in one component, zero in the others) of the face crossed; when several faces are crossed at exactly the same true distance the lowest axis (x, y, z) is reported, for the entry and for the exit, and equal rounded distances are not a tie."
status = "active"

[[requirement]]
id = "TRACE-REQ-047"
summary = "An axis with a zero direction component never supplies a face: it rules the box out when the origin's coordinate is outside the closed slab and constrains nothing otherwise (a ray in a face's plane is inside it); an origin inside the box hits the exit from inside; a box whose entry and exit are both below t_min is not hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-060"
summary = "TraceDisc::new accepts a height and a positive radius inside the trace domain and refuses any other with a typed TraceDiscError (HeightOutsideTraceDomain, RadiusNotPositive, RadiusOutsideTraceDomain); no input to TraceDisc::new or TraceRay::hit_disc wraps or panics."
status = "active"

[[requirement]]
id = "TRACE-REQ-061"
summary = "TraceRay::hit_disc (contract bunny-ray-disc/1) returns, for every ray and disc in the trace domain with a nonzero direction z, the floor in raw units of the true distance to the plane z = height when that rounded distance is in t_min..=t_max and the exact point where the ray meets the plane is at most the radius from the axis (the rim is inside), else None; the radius test is decided on the exact meeting point, not the rounded one."
status = "active"

[[requirement]]
id = "TRACE-REQ-062"
summary = "A ray whose direction has z = 0 misses every disc, including a ray that lies in the plane of the disc."
status = "active"

[[requirement]]
id = "TRACE-REQ-063"
summary = "DiscHit::front_face is true exactly when the direction has a negative z; DiscHit::normal is (0, 0, 1) on both sides; a ray whose origin is on the disc has distance 0; a disc whose rounded distance is below t_min is not hit."
status = "active"

[[requirement]]
id = "TRACE-REQ-070"
summary = "TraceRay::try_new returns Ok with the ray TraceRay::new builds, or the first failing of, in this order: OriginOutsideTraceDomain { origin } when an origin coordinate has abs(raw) >= 2^40, DirectionComponentTooLarge { direction } when a direction component has abs(raw) > 2^32, ZeroDirection when the direction is (0, 0, 0); each carries the values as given."
status = "active"

[[requirement]]
id = "TRACE-REQ-071"
summary = "TraceTriangle::try_new returns Ok with the triangle TraceTriangle::new builds, or VertexOutsideTraceDomain { index, vertex } naming the first vertex in argument order with a coordinate of abs(raw) >= 2^40."
status = "active"

[[requirement]]
id = "TRACE-REQ-072"
summary = "TraceRay::new and TraceTriangle::new, whose bodies are unchanged, return Some(x) exactly when their try_new forms return Ok(x), with equal values, for every input."
status = "active"

[[requirement]]
id = "TRACE-REQ-073"
summary = "TraceRayError and TraceTriangleError (#[non_exhaustive]), TraversalError and CompactError implement Display, with a message that states the condition and the raw values it saw, and core::error::Error with no source, using only core."
status = "active"
```

## Fixtures

No external fixtures are required. Every case builds rays, triangles and boxes
from raw Q32.32 values in the test file.

Seeded cases use a fixed 64-bit linear congruential generator with the seed
`TRACE_SEED` (`0xB002_7ACE_0000_0001`), varied per case by an exclusive-or with
the case number. Failure messages name the seed and the case index.

Constructed triangles have edges that are multiples of 64 raw-unit steps, so a
point with barycentric weights `(b, c) / 64` is a lattice point, and a ray is
started a whole number of direction lengths before it. The exact distance and
weights are therefore known without computing them.

Traversal cases use `CLOSEST_SEED` (`0xB002_C105_0000_0001`), varied the
same way, to scatter 40 to 600 small triangles (one in forty spans the scene)
through a cube 12 units wide, build them with `build_bvh`, and cast rays from
anywhere near the scene toward points inside it; one direction component in
five is zero or a few raw units. Constructed traversal cases stack flat
triangles under a ray falling with direction exactly -1.0, so every box bound
and hit distance is a whole number of units, and build the two-leaf trees by
hand so the search order is known.

`TRACE-TP-201` to `TRACE-TP-205` compare `hit_triangle` with a frozen reference copy of
its arithmetic in `crates/bunny-query/tests/support/trace_reference.rs`, which the
`bunny-perf` scene tests include as well. The copy must not be edited to follow the
kernel: it is the plain version that an optimized kernel is held to. The seeded corpus uses
`EQUIVALENCE_SEED` (`0xB002_7ACE_0000_0214`); magnitudes are drawn bit length first, so small
and large values are equally common.

`TRACE-TP-308` to `TRACE-TP-311` compare `box_entry` with the box test as it was before
flyingrobots/bunny#258, kept in `crates/bunny-query/tests/support/box_entry_reference.rs`
(which the `bunny-perf` tests include as well). The copy must not be edited to follow the
kernel: it is the definition the faster kernel is held to. Every comparison covers ten
distance limits: `i64::MIN`, -1, 0, 1, `i64::MAX`, two seeded values, and the reference's
own entry bound and its two neighbours, where the limit decides the answer. Seeds are
`0xB002_0258_0000_0001` (seeded cases) and `0xB002_0258_0000_0002` (extremes);
`TRACE-TP-308` replays the corpus of `TRACE-TP-007` with its own seed.

`TRACE-TP-315` checks `hit_triangle` against `crates/bunny-query/tests/support/volume_oracle.rs`,
which decides a crossing from the signs of the three volumes `det[a - o, b - o, d]`, one per
edge, and the distance from the plane equation, in a 256-bit integer with a bit-by-bit floor
division. It computes none of the kernel's Moller-Trumbore quantities and shares no code with
the kernel or the frozen copy; the kernel's `u`, `v` and `det` follow from the volumes by
expanding the triple products. `TRACE-TP-005` uses it too. The corpus (`EDGE_ORACLE_SEED`,
`0xB002_7ACE_0294_0001`, 40 000 cases, about 1.8 s in a debug build) solves one origin
coordinate so that a chosen edge numerator is exactly `-3` to `3`, or aims through a vertex, an
edge midpoint or a raw unit or two from one. With the fixed seed it reaches 15 565 hits, 1 564,
1 548 and 1 596 on the three edges, 7 001 on a vertex, 200, 204 and 210 misses one or two
units past each edge, 10 924 in the extreme families (8 947 of them on an edge or vertex), 642
grazing hits, 7 783 front and 7 782 back hits, 2 048 degenerate triangles and 914 parallel
rays. A separate exact Python model of the same generator and oracle gives the same counts.
The test asserts a floor of about 75 % of each.

The golden vectors in `TRACE-TP-009` and `TRACE-TP-010` are hand computed; the
comments beside each case give the exact value the floor is taken of.
Regenerating them is a contract change.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| TRACE-TP-001 | Constructed hits | TRACE-REQ-004, TRACE-REQ-007, TRACE-REQ-008 | Rays built through a known lattice point of a triangle at a whole-unit distance must return that exact distance, exact weights and the facing from the triple product. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_001_constructed_hits_report_exact_distance_and_weights` |
| TRACE-TP-002 | Constructed misses | TRACE-REQ-004 | Rays through points just outside each edge, and rays starting past the triangle, must miss. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_002_points_outside_the_triangle_or_behind_the_ray_miss` |
| TRACE-TP-003 | Inclusive distance limits | TRACE-REQ-006, TRACE-REQ-008 | A hit at exactly 5.0 is reported for limits 5.0..=5.0 and refused for limits one raw unit away. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_003_distance_limits_are_inclusive` |
| TRACE-TP-004 | Shared-edge watertightness | TRACE-REQ-005 | A ray through the diagonal of a split quad hits both halves; a ray elsewhere hits exactly one. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_004_a_shared_edge_never_leaks` |
| TRACE-TP-005 | Extreme domain values | TRACE-REQ-009 | Triangles and rays at the corners of the domain run under overflow-checked test arithmetic without panicking and equal the signed-volume oracle of TRACE-TP-315; exactly 3 of the 32 cases hit, and each hit's bounds are accepted. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_005_extreme_domain_values_do_not_overflow` |
| TRACE-TP-006 | Domain refusal | TRACE-REQ-001 | Values one raw unit past each bound return None; values at each bound are accepted; vertices round-trip. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_006_values_outside_the_trace_domain_are_refused` |
| TRACE-TP-007 | Box bound against an exact oracle | TRACE-REQ-002, TRACE-REQ-003 | An exact rational slab test over 40 000 seeded rays and boxes: every reached box is accepted, the bound is at or below the floored exact entry and within the stated slack, and many missed boxes are rejected. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_007_box_entry_never_rejects_a_box_the_ray_reaches` |
| TRACE-TP-008 | Box distance limit | TRACE-REQ-002, TRACE-REQ-006 | An entry at exactly 4.0 is accepted for t_max 4.0 and rejected one raw unit below; negative t_max, boxes behind the ray and an origin inside the box behave as stated. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_008_box_entry_respects_the_distance_limit` |
| TRACE-TP-009 | Golden triangle hits | TRACE-REQ-004, TRACE-REQ-005, TRACE-REQ-007, TRACE-REQ-008 | Hand-computed raw distance, weights and facing for four cases: an exact hit, a floored one-third distance, a back-face hit with floored one-third weights, and a corner hit. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_009_golden_triangle_hits_lock_raw_outputs` |
| TRACE-TP-010 | Golden box entries | TRACE-REQ-002, TRACE-REQ-003 | Hand-computed raw bounds for seven cases, including an inexact reciprocal, an edge graze, a corner graze whose exact exit equals its exact entry, a near miss and a face contact. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_010_golden_box_entries_lock_raw_outputs` |
| TRACE-TP-011 | Degenerate and far cases | TRACE-REQ-010 | Collinear and duplicate-vertex triangles, a ray in the plane, and a crossing 2^71 raw away all miss; control hits at the same places are reported. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_011_degenerate_triangles_parallel_rays_and_far_hits_miss` |
| TRACE-TP-012 | Deliberate mutations | TRACE-REQ-002, TRACE-REQ-003, TRACE-REQ-005 | Each rounding or edge rule, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |
| TRACE-TP-013 | Differential against the old function | TRACE-REQ-011, TRACE-REQ-013 | Over 200 000 seeded ordinary, axis-parallel, tiny-component, grazing and origin-inside cases, every hit of the retained pre-#211 implementation is returned with the same bits, every miss it reports without overflow stays a miss, and every bound equals the exact bound rounded and saturated. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::every_previous_hit_keeps_its_bits_on_the_ordinary_corpus` |
| TRACE-TP-014 | Exact rational oracle on the overflow region | TRACE-REQ-011, TRACE-REQ-012, TRACE-REQ-013 | Over 200 000 seeded cases with coordinates near and past 2^62, i64 extremes and components of 1 to 4 000 raw units, every case where the old function overflowed agrees in hit or miss with an i128 cross-multiplication slab test, and every bound equals the exact bound rounded and saturated. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::overflow_cases_agree_with_the_exact_rational_oracle` |
| TRACE-TP-015 | Oracle rounding pinned to checked_div | TRACE-REQ-013 | The test oracle's floor-based ties-to-even rounding equals FixedQ32_32::checked_div on 50 000 seeded representable quotients and on exact ties. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::rounding_oracle_matches_checked_div_on_representable_quotients` |
| TRACE-TP-016 | Golden one-raw-unit component | TRACE-REQ-012, TRACE-REQ-013 | Origin (-5, 0, 0), direction (1, 2^-32, 0), box [-1, 1]^3 returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::one_raw_unit_component_hits_the_box_it_passes_through` |
| TRACE-TP-017 | Golden two-raw-unit component | TRACE-REQ-012, TRACE-REQ-013 | The same ray with a component of 2 raw units returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::two_raw_unit_component_hits_the_box_it_passes_through` |
| TRACE-TP-018 | Golden negative tiny component | TRACE-REQ-012, TRACE-REQ-013 | The same ray with a component of -1 raw unit returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::negative_tiny_component_hits_too` |
| TRACE-TP-019 | Sweep across the old threshold | TRACE-REQ-011, TRACE-REQ-012 | For k = 1..=64 and components d = 1..=2k+3 raw units, box [-k, k]^3 from (-100, 0, 0) returns exactly (100 - k, 100 + k) on both sides of the old d <= 2k miss boundary. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::hits_across_the_old_overflow_threshold_of_twice_the_slab_half_width` |
| TRACE-TP-020 | Golden unrepresentable exit | TRACE-REQ-013 | From x = -1.5e9 into x in [5e8, 7e8] returns (2e9, i64::MAX raw). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::origin_far_from_box_saturates_the_unrepresentable_exit` |
| TRACE-TP-021 | Widened coordinate difference | TRACE-REQ-013 | A difference of 2^63 + 2^30 raw with direction 1 + 2^-32 returns the exactly computed raw exit 9 223 372 035 781 033 984, not the 9 223 372 034 707 292 160 that saturating the difference first gives. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::coordinate_difference_beyond_i64_is_widened_not_saturated` |
| TRACE-TP-022 | Saturated entries decided exactly | TRACE-REQ-012 | Two slabs whose entries both saturate to the maximum, and whose rounded bounds would tie, are a miss because the exact entry exceeds the exact exit. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_entries_that_tie_when_rounded_are_decided_exactly` |
| TRACE-TP-023 | Hit beyond the representable distance | TRACE-REQ-012, TRACE-REQ-013 | A box first reached more than 2^31 units from the origin is a hit with both bounds at i64::MAX raw. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::a_hit_beyond_the_representable_distance_saturates_both_bounds` |
| TRACE-TP-024 | No false hit from saturation | TRACE-REQ-012 | Near-axis rays with components of 0 to 4 raw units still miss a box beside their path. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_bounds_never_create_a_false_hit` |
| TRACE-TP-025 | Face contact at the edge of the range | TRACE-REQ-012, TRACE-REQ-013 | An origin at i64::MAX raw on the box's +x face returns (i64::MIN raw, 0). | `crates/bunny-query/tests/query_tests.rs::ray_aabb_saturates_an_overflowing_slab_distance_instead_of_missing` |
| TRACE-TP-026 | Broadphase leaf visit | TRACE-REQ-014 | A one-leaf BVH walked by intersect_ray with direction (1, 2^-32, 0) visits primitive 0. | `crates/bunny-broadphase/tests/bvh_tests.rs::issue_211_intersect_ray_visits_a_leaf_a_near_axis_ray_passes_through` |
| TRACE-TP-027 | Deliberate mutations of the overflow path | TRACE-REQ-012, TRACE-REQ-013 | Each rule of the overflow path, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |
| TRACE-TP-101 | Brute-force agreement | TRACE-REQ-021, TRACE-REQ-025 | Over about 3 000 seeded rays and 600 triangles, closest_hit equals a test of every triangle (minimum by distance, then index) and any_hit equals whether that test found a hit; the corpus has over 500 hits and 500 misses. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_101_closest_hit_equals_a_test_of_every_primitive` |
| TRACE-TP-102 | Inclusive limit | TRACE-REQ-023 | For each hit ray, t_max equal to the nearest distance returns that hit and any_hit is true; t_max one raw unit shorter returns no hit and any_hit is false. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_102_the_distance_limit_is_inclusive_and_agrees_with_every_primitive` |
| TRACE-TP-103 | Ties | TRACE-REQ-021 | Every triangle appears twice, so every hit is a tie; the result equals the full test and always names the first copy. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_103_equal_distances_choose_the_lowest_primitive_index` |
| TRACE-TP-104 | Malformed and empty trees | TRACE-REQ-026 | An empty tree misses for both searches; a child index past the buffer and a leaf range past the primitive buffer return the stated errors. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_104_malformed_trees_are_errors_and_empty_trees_miss` |
| TRACE-TP-105 | Out-of-range reports | TRACE-REQ-026 | A primitive test that reports a distance beyond the limit, or a negative one, for every primitive yields no hit. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_105_distances_outside_the_limit_from_the_primitive_test_are_ignored` |
| TRACE-TP-106 | Tree shape | TRACE-REQ-021, TRACE-REQ-022, TRACE-REQ-025 | The same triangles (a third duplicated, to make ties) built in reversed, interleaved and shuffled order, with leaf entries mapped back to the original indices, and under one leaf listing them in reverse, give identical closest_hit and any_hit answers to the original tree and the full test over 1 500 rays. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_106_differently_shaped_trees_give_the_same_answers` |
| TRACE-TP-107 | Front-to-back order | TRACE-REQ-024 | Two triangles 5.0 and 8.0 away, the far one in the left leaf: the near one is returned and it is the only primitive tested. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_107_the_nearer_child_is_searched_first_and_the_farther_one_is_skipped` |
| TRACE-TP-108 | Equal bound kept | TRACE-REQ-021, TRACE-REQ-024 | Two coincident triangles whose leaf bounds equal the hit distance 5.0 exactly; the leaf searched first holds index 1. Both are tested and index 0 is returned, and the same holds when one leaf lists them as 1, 0. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_108_a_subtree_that_starts_at_the_best_distance_is_still_searched` |
| TRACE-TP-109 | Early stop | TRACE-REQ-025 | Four stacked triangles that the ray hits: any_hit is true after exactly one primitive test; with t_max 4.0, below every box, it is false after none. | `crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_109_any_hit_stops_at_the_first_hit` |
| TRACE-TP-110 | Deliberate traversal mutations | TRACE-REQ-021, TRACE-REQ-024, TRACE-REQ-025 | Each pruning, ordering or stop rule, broken on purpose, makes at least one traversal case fail. | [Mutation checks](#mutation-checks) |
| TRACE-TP-201 | Seeded limits around the true distance | TRACE-REQ-027 | Over 30 000 seeded rays aimed at seeded triangles in five size classes (unit scale, det below 2^32, full domain budgets), hit_triangle equals the frozen reference at upper and lower limits equal to the true distance, one and two raw units below, one above, 0, -1, 1, i64::MIN, i64::MAX and i64::MAX - 1; each class has over 2 000 crossings. | `crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_201_seeded_limits_around_the_true_distance_agree` |
| TRACE-TP-202 | Whole distances with det a multiple of 2^32 | TRACE-REQ-027 | Rays falling straight onto a fixed triangle at whole raw distances, where det is a multiple of 2^32, equal the frozen reference at the same limits. | `crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_202_exact_distances_at_the_limit_agree` |
| TRACE-TP-203 | Extreme domain values against the reference | TRACE-REQ-027 | The domain-corner triangles and rays of TRACE-TP-005, where det and t use their full bit budgets, equal the frozen reference at the same limits; exactly 3 of the 32 cross, as TRACE-TP-005 counts. | `crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_203_extreme_domain_values_agree` |
| TRACE-TP-204 | det at and just past multiples of 2^32 | TRACE-REQ-027 | Raw-unit triangles hit at a corner with det exactly 2^32, 3 * 2^32, 2^32 + 1 (641 * 6 700 417) and 1 equal the frozen reference at the same limits. | `crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_204_determinants_at_multiples_of_2_pow_32_agree` |
| TRACE-TP-205 | The benchmark scene's triangle tests | TRACE-REQ-027 | Every triangle test that closest_hit makes on the benchmark scene's 1 024 camera rays equals the frozen reference at the limit the traversal passed and at the limits around the true distance. | `crates/bunny-perf/src/scene.rs::tests::hit_triangle_matches_the_reference_on_the_scene_traversal` |
| TRACE-TP-301 | Brute-force and source agreement | TRACE-REQ-028, TRACE-REQ-021, TRACE-REQ-025 | On seeded scenes of 2, 3, 5, 17, 64 and 600 triangles (3 500 rays in all), compact_closest_hit and compact_any_hit make the same primitive calls with the same limits in the same order, and return the same results, as closest_hit and any_hit on the source tree, which equal a test of every triangle; each scene has hits and misses. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_301_compact_searches_equal_the_source_tree_and_every_primitive` |
| TRACE-TP-302 | Ties, limits and shapes | TRACE-REQ-028, TRACE-REQ-022, TRACE-REQ-023 | With a third of 150 triangles duplicated (ties), on the built tree, two reshaped trees and a single leaf listing every triangle, the compact searches agree with the source searches and the full test at no limit, at the nearest distance and one raw unit below it. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_302_compact_searches_agree_on_ties_limits_and_other_tree_shapes` |
| TRACE-TP-303 | Empty, one-primitive and one-leaf trees | TRACE-REQ-028 | An empty tree converts to no root and both searches miss; a one-primitive tree converts to a leaf root with no compact node; a single leaf of three primitives converts to a leaf root; both agree with the source searches. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_303_empty_single_primitive_and_single_leaf_trees` |
| TRACE-TP-304 | Malformed conversion input | TRACE-REQ-028 | A child index past the slice (including u32::MAX) is InvalidNodeIndex, a too-small output is OutputTooSmall (and the exact size succeeds), a node reached twice is NotATree, and a comb tree needing more than 64 pending subtrees is TooDeep while 60 levels convert. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_304_malformed_input_is_a_typed_error` |
| TRACE-TP-305 | Malformed compact trees | TRACE-REQ-028, TRACE-REQ-026 | A root that names a missing compact node is InvalidNodeIndex, a leaf range past the primitive buffer is InvalidPrimitiveRange, and a chain that keeps a leaf on the stack per level is StackOverflow. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_305_malformed_compact_trees_are_traversal_errors` |
| TRACE-TP-306 | Order, equal-bound pruning and early stop | TRACE-REQ-028, TRACE-REQ-024, TRACE-REQ-025 | The constructed cases of TRACE-TP-107 to TRACE-TP-109 on the compact tree: the nearer leaf is the only one tested, a leaf starting at the best distance is still tested (calls 1 then 0, index 0 returned), and compact_any_hit stops after one test and makes none below every box. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_306_the_compact_search_keeps_the_order_pruning_and_early_stop` |
| TRACE-TP-307 | Conversion golden vectors | TRACE-REQ-028 | A hand-built nine-node tree converts to four exactly stated compact nodes in depth-first, left-first order; a seeded 300-triangle tree converts to bytes whose FNV-1a checksum is the stated constant, the same on every target that runs the tests. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_307_compact_conversion_golden_vectors` |
| TRACE-TP-318 | Shared nodes in the conversion | TRACE-REQ-028 | Constructed: a root whose two interior children name the same two leaves is NotATree in a five-node slice (three interior nodes reached, two allowed); with four unused nodes after them the same shape converts to three compact nodes, the shared leaves written under both parents. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_318_a_shared_node_converts_while_the_node_count_allows_it` |
| TRACE-TP-308 | Reference on the golden and oracle corpora | TRACE-REQ-029 | The inputs of TRACE-TP-010 (and an origin-inside case) and the 40 000 seeded cases of TRACE-TP-007, drawn with the same seed, return the same Option as the retained reference at every limit class; over 2 000 cases each reach and miss. | `crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_308_box_entry_matches_the_reference_on_the_golden_and_oracle_corpora` |
| TRACE-TP-309 | Reference on seeded trace-domain cases | TRACE-REQ-029 | 300 000 seeded rays (origins across the whole trace domain, within 4 units and within 2^20 raw; general, near-axis and axis-parallel directions with zero, tiny, unit and maximum components) against boxes around, ahead of, behind and far from the origin, with faces through the origin, inverted axes and full-range axes, return the same Option as the retained reference at every limit class; over a tenth of them each reach and miss. | `crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_309_box_entry_matches_the_reference_on_seeded_trace_domain_cases` |
| TRACE-TP-310 | Reference at the ends of i64 | TRACE-REQ-029 | Every (min, max) pair of 15 extreme coordinates (i64::MIN and MAX, their neighbours, the domain bound and +-2^62), inverted pairs included, on each axis, for 7 extreme origin coordinates and 9 extreme direction components, then 200 000 seeded boxes whose every coordinate is within a few units of an extreme, return the same Option as the retained reference at every limit class. | `crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_310_box_entry_matches_the_reference_at_the_ends_of_the_i64_range` |
| TRACE-TP-311 | Reference on the benchmark scene | TRACE-REQ-029 | Every box test the closest_hit and any_hit searches make on the benchmark scene's 1 024 camera rays returns the same Option as the retained reference at the recorded limit, at i64::MAX, 0, -1 and at the reference's entry bound and one below it. | `crates/bunny-perf/src/profile.rs::tests::every_recorded_box_test_matches_the_reference_box_test` |
| TRACE-TP-312 | Deliberate mutations of the faster box test | TRACE-REQ-029 | Each rule of the rewritten box test, broken on purpose, makes at least one of TRACE-TP-308 to TRACE-TP-311 fail. | [Mutation checks](#mutation-checks) |
| TRACE-TP-313 | Exact numerators of constructed hits | TRACE-REQ-030 | 4 000 seeded constructed hits (as in TRACE-TP-001, small triangles near the origin and large ones near the domain limit) through the point with weights `(b, c) / 64`: `64 u = b det` and `64 v = c det` exactly, `det < 2^117`, and `BarycentricNumerators::new` accepts the returned values unchanged. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_313_numerators_are_the_exact_weights_of_constructed_hits` |
| TRACE-TP-314 | Numerators off the triangle | TRACE-REQ-030 | `new` accepts the corners, an interior point and `det = i128::MAX`, and refuses `det <= 0`, a negative `u` or `v`, `u + v > det` and a `u + v` that overflows. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_314_numerators_off_the_triangle_are_refused` |
| TRACE-TP-315 | Independent near-edge oracle | TRACE-REQ-004, TRACE-REQ-005, TRACE-REQ-008, TRACE-REQ-009, TRACE-REQ-030 | 40 000 generated cases concentrated on and within three numerator units of each edge and vertex, in raw-unit, scaled (numerators near 2^107), domain-corner and long-direction (components near 2^32, grazing) families, with degenerate triangles: hit or miss, floor distance, facing and exact numerators equal a signed-volume oracle in 256-bit integers that shares no formula with the kernel. Minimum counts of hits, hits on each edge, vertex hits, misses one or two units past each edge, extreme-family hits, grazing hits, both facings, degenerate and parallel cases are asserted. | `crates/bunny-query/tests/trace_edge_oracle_tests.rs::trace_tp_315_hits_near_edges_and_vertices_match_a_signed_volume_oracle` |
| TRACE-TP-316 | Edge-oracle mutations | TRACE-REQ-004, TRACE-REQ-005, TRACE-REQ-008 | Each edge comparison relaxed or made strict, the distance rounded to nearest, and the facing inverted, each make TRACE-TP-315 fail. | [Edge-oracle mutation checks](#edge-oracle-mutation-checks) |
| TRACE-TP-317 | Inverted box on box_entry | TRACE-REQ-029 | Constructed: a ray along +x at 2^32 raw per unit; a box with x from 6 to 5 raw returns Some(6), as a box from 6 to 7 does; x from -5 to -6 (behind the origin) returns None; a box inverted on an axis the ray does not move along returns None. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_317_box_entry_on_an_inverted_box_is_defined_per_axis` |
| TRACE-TP-401 | Cycles: closest_hit | TRACE-REQ-031 | closest_hit on four cyclic buffers (node 0 its own left child beside a leaf the ray misses, the same beside a leaf covering everything, a three-node cycle through left children, and a node that is its own right child) returns InvalidNodeIndex with the stated number of primitive calls (none). | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_401_closest_hit_refuses_cyclic_buffers` |
| TRACE-TP-402 | Cycles: any_hit | TRACE-REQ-031 | any_hit on the four cyclic buffers of TRACE-TP-401 returns InvalidNodeIndex with no primitive call. | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_402_any_hit_refuses_cyclic_buffers` |
| TRACE-TP-403 | Cycles: intersect_ray | TRACE-REQ-031 | intersect_ray on the four cyclic buffers returns InvalidNodeIndex after exactly the stated leaf callbacks (one for the covering leaf, none otherwise). | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_403_intersect_ray_refuses_cyclic_buffers` |
| TRACE-TP-404 | Cycles: intersect_aabb | TRACE-REQ-031 | intersect_aabb on the four cyclic buffers returns InvalidNodeIndex after exactly the stated leaf callbacks (one for the covering leaf, none otherwise). | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_404_intersect_aabb_refuses_cyclic_buffers` |
| TRACE-TP-405 | Shared subtrees | TRACE-REQ-031 | With a query reaching every node, a shared leaf within the budget is searched as the tree it unfolds into (callbacks 0, 1, 0, as the retained functions give); a shared interior node and a 40-node ladder whose unfolding is exponential are InvalidNodeIndex; each is refused exactly when compact_bvh calls it NotATree. | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_405_shared_subtrees_are_searched_within_the_budget_and_refused_beyond_it` |
| TRACE-TP-406 | Differential against the retained functions | TRACE-REQ-031 | On seeded scenes of 1 to 300 triangles built by build_bvh and build_bvh_sah, the same trees with a seeded leaf range or child index made out of range, a 70-level comb (deeper than the stack) and a 64-level comb ending in out-of-range children reached with a full stack, all four functions return the same result and make the same callback calls (closest_hit: primitive and limit) in the same order as frozen copies of the pre-#269 functions, over up to 5 040 seeded queries (a ray with t_max unbounded or up to 15 units, and a box; over 2 500 required); failures name the seed and case. | `crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_406_valid_trees_keep_every_result_and_callback` |
| TRACE-TP-407 | Deliberate guard mutations | TRACE-REQ-031 | Removing the guard from each of the four functions makes its cycle case fail or not terminate under a wall-clock limit; a budget one smaller fires on a valid tree and fails an existing case. | [Mutation checks](#mutation-checks) |
| TRACE-TP-408 | `from_ray` and the unit vector of the issue | TRACE-REQ-032 | `(2^32 + 1, 0, 0)` and its negation on each axis are unit vectors (`try_from_unit`), `from_ray` returns `Some` with the direction unchanged, `new` returns `None` for the same direction and for `2^32 + 2` (not a unit vector), and the origin bound still refuses `2^40`. | `crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_408_from_ray_accepts_the_unit_vector_of_the_issue_on_every_axis` |
| TRACE-TP-409 | Normalization never exceeds 1.0 | TRACE-REQ-032 | Over 200 000 generated vectors (scales from 3 raw to `2^61`, axis-aligned, near-axis and tiny), at least 130 000 normalize and at least 50 000 are refused; no result has a component above `2^32`, at least 15 000 have one equal to it, the largest is exactly `2^32`, and `from_ray` accepts every result. | `crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_409_normalisation_never_yields_a_component_above_one` |
| TRACE-TP-410 | Kernels for long unit directions | TRACE-REQ-032 | 20 000 seeded rays with a component of `2^32 + 1` and 20 000 boxes: `hit_triangle` equals the signed-volume oracle (hit, floor distance, facing, exact numerators; at least 4 000 hits and 4 000 misses) and `box_entry` is never past the exact rational entry and short by at most `T / 2^31 + 2` (at least 1 000 reached and 1 000 certainly missed boxes). | `crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_410_kernels_agree_with_the_oracles_for_long_unit_directions` |
| TRACE-TP-411 | Known distance | TRACE-REQ-032 | A triangle in the plane `x = 2.0` and the ray along `+x` from 0 and along `-x` from 4.0, direction `2^32 + 1`: distance `floor(2^65 / (2^32 + 1)) = 8 589 934 590` raw, back face for `+x` and front face for `-x`. | `crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_411_a_long_direction_hit_has_the_exactly_floored_distance` |
| TRACE-TP-412 | Traversal depth boundary | TRACE-REQ-033, TRACE-REQ-021, TRACE-REQ-025, TRACE-REQ-028 | A comb of depth 63 and one of depth 64 whose right child at every level is the next level and enters the ray before its left leaf, so every search keeps the left leaf of each level pending: at depth 63 all six traversals (the compact two on the converted tree) return what a test of every primitive returns; at depth 64 each returns StackOverflow. | `crates/bunny-broadphase/tests/traversal_depth_tests.rs::trace_tp_412_a_comb_of_depth_63_is_searched_and_one_of_depth_64_overflows` |
| TRACE-TP-413 | Conversion depth boundary | TRACE-REQ-034 | A spine whose right child at every level is an interior node of two leaves, which compact_bvh queues while it writes the left subtree: depth 64 (64 subtrees waiting at the deepest level) converts and writes every interior node; depth 65 is TooDeep. A right comb of depth 101, which queues at most one subtree, converts. | `crates/bunny-broadphase/tests/traversal_depth_tests.rs::trace_tp_413_compact_conversion_queues_at_most_one_subtree_per_level` |
| TRACE-TP-414 | Deliberate depth mutations | TRACE-REQ-033, TRACE-REQ-034 | Each stack of the six traversals and of compact_bvh enlarged or shrunk by one makes TRACE-TP-412 or TRACE-TP-413 fail; the box queries' explicit capacity check moved by one is an equivalent mutant. | [Depth mutation checks](#depth-mutation-checks) |
| TRACE-TP-415 | Hand-counted trees | TRACE-REQ-035 | Hand-derived counts, written out in the test, on an empty tree (all zero), a one-leaf root of three primitives, a ray missing the root box, a root over two leaves where the nearer hit prunes the farther (one popped_skipped), where the nearer leaf is entered but missed (both leaves visited), where both leaves start at the hit distance (a bound equal to the limit is searched), where the ray enters the root but neither child, a three-level tree whose nearer subtree's hit skips the farther interior subtree at its pop, and compact_any_hit stopping at its first primitive test. | `crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_415_counts_on_constructed_trees_are_the_hand_derived_values` |
| TRACE-TP-416 | Counts against a recursive reference | TRACE-REQ-035 | On seeded scenes of 1 to 300 triangles, with a copy of every third one for ties, built by build_bvh and build_bvh_sah, and seeded rays (t_max unbounded and at the nearest hit), the counts and the primitive calls of both counted searches equal those of a recursive search written in the test that shares no code with the loop (it tracks the stack depth as pending siblings); failures name the seed. | `crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_416_counts_equal_a_recursive_reference_search` |
| TRACE-TP-417 | Counted and uncounted searches agree | TRACE-REQ-035 | On the trees of TRACE-TP-416 and on seeded corruptions of them (a child reference out of range or naming an ancestor, a leaf range past the buffer), and on a 70-level chain past the stack, both counted searches return the uncounted search's result or error and make the same primitive calls (closest: primitive and limit) in the same order; failures name the seed. | `crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_417_counted_and_uncounted_searches_agree_on_valid_and_malformed_trees` |
| TRACE-TP-418 | The benchmark scene's profile | TRACE-REQ-035 | On all 1 024 benchmark rays, both counted searches on the compact copy of the scene's tree give the profile replica's answer and as many box tests and primitive tests as the replica recorded for the same ray (the profile's counts are the counted searches', see PERF-TP-057). | `crates/bunny-perf/src/profile.rs::tests::the_counted_compact_search_matches_the_replica_on_every_ray` |
| TRACE-TP-419 | Adding counts | TRACE-REQ-035 | Adding two TraversalCounts sums each counter, saturating at u64::MAX, and keeps the larger peak_stack; += agrees with +. | `crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_419_adding_counts_sums_saturates_and_keeps_the_peak` |
| TRACE-TP-420 | Deliberate counting mutations | TRACE-REQ-035 | A box test not counted, a rejected box counted as entered, a skipped pop counted as a visit, a leaf counted twice, the last primitive test of a leaf not counted, the peak not tracked and the counted search pruning differently from the uncounted one (`<=` made `<` for the counting tally only) each make a case fail. | [Mutation checks](#counting-mutation-checks) |
| TRACE-TP-421 | Compact visit bound | TRACE-REQ-028 | A three-node compact cycle with a leaf beside each node: `compact_closest_hit` and `compact_any_hit` test primitives 0, 1 and 2 and refuse the fourth interior visit with `InvalidNodeIndex`, so a bound one looser (a fourth primitive test) or one tighter (no test of primitive 2) fails. | `crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_421_the_compact_visit_bound_allows_exactly_one_visit_per_node` |
| TRACE-TP-501 | Generated exact oracle | TRACE-REQ-041, TRACE-REQ-042 | Exact rational oracle in independent arbitrary-precision integers: discriminant as b^2 - a c, each rounded root found by bisection on the exact predicate k <= root, no square root; the normal checked without the kernel's formula, by exact sign decisions of p + q sqrt(disc): each component within 1/2 + 2^-24 raw units of the exact unit normal at the exact root, a positive dot product with the exact offset, and FixedUnitVec3::try_from_unit. 6 000 cases from seed 0x5EED_5F4E_0358_0001, one in three with its range moved onto its own exact roots (t_min on the near root or one past it, t_max on the near root or one before it, or both around the far root) by a second stream from seed 0x5EED_5F4E_0422_0001 (4 244 misses, 925 front hits, 831 back hits, 232 hits at a radius of at most 2^8 raw, 128 distances on t_min, 235 on t_max, 235 misses with a root one raw unit past t_max; floors 3 183, 693, 623, 174, 96, 176, 176), the seed and case printed on failure. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_501_hits_match_an_exact_rational_oracle_on_generated_rays` |
| TRACE-TP-502 | Tangent ray | TRACE-REQ-041, TRACE-REQ-043 | Constructed: the line y = 1 touches the unit sphere at distance 4.0 with normal (0, 1, 0), front face; one raw unit further out misses; one raw unit in is checked against the oracle and lies within the hand-computed half chord. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_502_a_tangent_ray_hits_once_and_a_ray_one_raw_unit_outside_misses` |
| TRACE-TP-503 | Origin inside | TRACE-REQ-041, TRACE-REQ-043 | Constructed: from the centre of a radius-2 sphere the hit is at 2.0, back face, normal (1, 0, 0); off-centre with a half-length direction the distance is 3.464.. (2 sqrt 3) and equals the oracle; t_min past the exit misses. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_503_an_origin_inside_the_sphere_hits_the_far_side_from_inside` |
| TRACE-TP-504 | Sphere behind the origin | TRACE-REQ-041, TRACE-REQ-043 | Constructed: roots at -11 and -9 miss with t_min = 0 and give the near root -11.0, front face, normal (-1, 0, 0) with t_min = -20; a ray pointing away misses. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_504_a_sphere_behind_the_origin_is_not_hit` |
| TRACE-TP-505 | Typed refusals | TRACE-REQ-040 | Constructed: each bound one raw unit past, and i64 extremes, give the stated error variant; the largest accepted sphere round-trips its centre and radius and its bounds are hand-computed. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_505_spheres_outside_the_trace_domain_are_refused_with_a_typed_error` |
| TRACE-TP-506 | Widest inputs | TRACE-REQ-040, TRACE-REQ-041 | The exact oracle of TRACE-TP-501 on every domain corner against the largest spheres, with the longest, shortest and a 2^32 + 1 unit direction, under overflow-checked test arithmetic. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_506_the_widest_inputs_match_the_oracle_without_overflow` |
| TRACE-TP-507 | 256-bit helper | TRACE-REQ-041 | Hand-computed: products of the largest values, carries across limbs, refused overflow, and isqrt of k^2 - 1, k^2 and k^2 + 1 up to k = 2^128 - 1. | `crates/bunny-query/src/trace/wide.rs::tests::isqrt_is_the_floor_root_around_squares` |
| TRACE-TP-508 | Deliberate mutations of the sphere test | TRACE-REQ-040, TRACE-REQ-041, TRACE-REQ-042 | Each rule broken on purpose makes a case above fail. | [Sphere mutation checks](#sphere-mutation-checks) |
| TRACE-TP-701 | Zero-normal reproducer | TRACE-REQ-042 | Constructed (independent review of flyingrobots/bunny#375): radius 1 raw, origin at the centre, direction (1, 1, 1); distance 0, back face, each component 2479700524 or 2479700525 raw (2^32 / sqrt 3 = 2479700524.87..) and the normal oracle of TRACE-TP-501 holds. The code at the merge of flyingrobots/bunny#375 returned (0, 0, 0). | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_701_a_one_raw_unit_sphere_hit_from_its_centre_has_a_unit_normal` |
| TRACE-TP-702 | Radius 0.01 reproducer | TRACE-REQ-042 | Constructed (same review): centre raw (12345, -777, 4242), radius raw 42949673, origin (-1, -1, -1), direction (1, 1, 1); distance raw 4270175561, front face, and the normal oracle holds. The code at the merge of flyingrobots/bunny#375 returned (-2480407998, -2479095798, -2479597698), 2^32 + 56.2 raw units long. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_702_a_sphere_of_radius_one_hundredth_has_a_unit_normal` |
| TRACE-TP-703 | The seed's seven zero normals | TRACE-REQ-042 | Cases 22, 129, 757, 988, 1275, 1628 and 2635 of the base corpus of TRACE-TP-501 (radius 1 raw each): distance and face equal the oracle and the normal oracle holds on all seven; a failure lists every one that breaks. All seven were (0, 0, 0) at the merge of flyingrobots/bunny#375. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_703_the_seeds_seven_old_zero_normals_are_unit_normals` |
| TRACE-TP-704 | Inclusive distance limits | TRACE-REQ-041 | Constructed: the unit sphere from (-4, 0, 0) along +x has exact roots 3.0 and 5.0; each root exactly on t_min and on t_max hits, t_min == t_max == 3.0 hits, one raw unit past t_max misses, and one raw unit above the near root moves the hit to the far root (back face, normal (1, 0, 0)). | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_704_the_distance_limits_are_inclusive_at_both_ends` |
| TRACE-TP-705 | Normal bound across radii | TRACE-REQ-042 | Constructed: a skew direction through an off-axis centre at radius 1 raw, 0.01, 1.0 and the largest admitted, from outside (front face) and from the centre (back face), each against the distance oracle and the normal oracle of TRACE-TP-501. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_705_the_normal_bound_holds_from_the_smallest_radius_to_the_largest` |
| TRACE-TP-706 | Public contract identity | TRACE-REQ-041 | RAY_SPHERE_ID equals bunny-ray-sphere/1. | `crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_706_the_contract_identity_is_public` |
| TRACE-TP-707 | 256-bit shifts | TRACE-REQ-041, TRACE-REQ-042 | Hand-computed: U256::checked_shl by 0, 128, 129, 200, 255 and 256 bits keeps every bit or refuses the overflow. | `crates/bunny-query/src/trace/wide.rs::tests::wide_shifts_keep_every_bit_or_refuse` |
| TRACE-TP-540 | Generated exact oracle | TRACE-REQ-066, TRACE-REQ-067, TRACE-REQ-068 | Exact rational oracle in independent arbitrary-precision integers that lists every point where the line meets a surface (a side root whose height is between the caps, a cap crossing inside its disc, each checked exactly) and takes the earliest and latest, a cap winning a tie; discriminant as b^2 - a c, each side root's floor by bisection on an exact predicate, no square root; each normal checked without the kernel's formula (a cap's is the exact axis; a side's has y = 0, each component within one raw unit of the exact unit normal a (o + d t) / (a r) built from the oracle's own root, and a positive dot product with the exact offset); 6 000 cases from seed 0x5EED_C71D_0360_0001, one in ten parallel to the axis, one in ten parallel to the caps (half of those in a cap's plane), one in ten through a rim point, one in five from inside (3 908 misses, 407 cap entries, 884 side entries, 441 cap exits, 360 side exits; floors 2 931, 305, 663, 330, 270), the seed and case printed on failure. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_540_hits_match_an_exact_rational_oracle_on_generated_rays` |
| TRACE-TP-541 | Grazing ray | TRACE-REQ-066, TRACE-REQ-068 | Constructed: the line z = 1 touches the unit cylinder's side at distance 4.0 with normal (0, 0, 1), front face; one raw unit further out misses; one raw unit in is checked against the oracle and lies within the hand-computed half chord; one raw unit above the top cap's plane misses. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_541_a_grazing_ray_touches_the_side_once_and_one_raw_unit_out_misses` |
| TRACE-TP-542 | Origin inside | TRACE-REQ-066, TRACE-REQ-068 | Constructed: from the centre the exits are at 1.0 through the side (1, 0, 0), 1.0 through the top cap and 2.0 through the bottom cap with a half-length direction, all back face; off-centre the distance is 1.732.. (sqrt 3) and is checked against the oracle; t_min past the exit misses. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_542_an_origin_inside_hits_the_exit_from_inside` |
| TRACE-TP-543 | Cylinder behind the origin | TRACE-REQ-066, TRACE-REQ-068 | Constructed: side crossings at -11 and -9 miss with t_min = 0 and give -11.0, front face, normal (-1, 0, 0) with t_min = -20; a ray below the bottom cap pointing down misses. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_543_a_cylinder_behind_the_origin_is_not_hit` |
| TRACE-TP-544 | Tie at the rim | TRACE-REQ-067 | Constructed: a line through the top rim at 1.0 and the bottom rim at 3.0 reports the top cap's normal at the entry and the bottom cap's at the exit; a line touching only the rim point (-1, 1, 0) reports the top cap, front face; one raw unit higher enters through the cap, one lower through the side (equal to the oracle). | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_544_at_the_rim_the_cap_wins_the_tie` |
| TRACE-TP-545 | Ray parallel to the axis | TRACE-REQ-066, TRACE-REQ-068 | Constructed: inside the radius the entry is the bottom cap at 4.0 and the exit the top cap at 6.0; on the side itself the bottom rim, with the cap's normal; one raw unit outside the radius misses. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_545_a_ray_parallel_to_the_axis_hits_only_a_cap` |
| TRACE-TP-546 | Ray in a cap's plane | TRACE-REQ-066, TRACE-REQ-068 | Constructed: along y = 1 and y = -1 the hit is the side at 3.0 with normal (-1, 0, 0), and the exit the side at 5.0; one raw unit below the bottom plane misses. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_546_a_ray_in_a_caps_plane_hits_the_side` |
| TRACE-TP-547 | Typed refusals | TRACE-REQ-065 | Constructed: each bound one raw unit past, i64 extremes, equal and reversed caps give the stated error variant, a bad radius before a bad cap; the largest and smallest accepted cylinders round-trip and their bounds are hand-computed. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_547_cylinders_outside_the_trace_domain_are_refused_with_a_typed_error` |
| TRACE-TP-548 | Widest inputs | TRACE-REQ-065, TRACE-REQ-066 | The exact oracle of TRACE-TP-540 on every domain corner against the largest, the thinnest and a one-raw-unit-tall cylinder, with the longest, shortest, axis-parallel, cap-parallel and a 2^32 + 1 unit direction, under overflow-checked test arithmetic. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_548_the_widest_inputs_match_the_oracle_without_overflow` |
| TRACE-TP-550 | Inclusive limits | TRACE-REQ-066 | Constructed: the side is entered at exactly 3.0 and left at exactly 5.0; the entry is hit with t_max = 3.0 (and t_min = t_max = 3.0) and missed with t_max one raw unit below; the exit is hit with t_max = 5.0 and with t_min = 5.0, and missed one raw unit past either limit. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_550_the_distance_limits_are_inclusive_at_both_ends` |
| TRACE-TP-551 | Contract identity | TRACE-REQ-069 | Constructed: RAY_CYLINDER_ID equals the string bunny-ray-cylinder/1 written in the test. | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_551_the_contract_identity_is_published` |
| TRACE-TP-552 | Smallest radius normals | TRACE-REQ-067 | Constructed: a cylinder of radius one raw unit hit by rays through its axis in five directions; each side normal passes the normal check of TRACE-TP-540 (within one raw unit of the exact unit normal, outward, y = 0). | `crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_552_side_normals_of_the_smallest_cylinder_are_unit_and_outward` |
| TRACE-TP-549 | Deliberate mutations of the cylinder test | TRACE-REQ-065, TRACE-REQ-066, TRACE-REQ-067, TRACE-REQ-068, TRACE-REQ-069 | Each rule broken on purpose makes a case above fail. | [Cylinder mutation checks](#cylinder-mutation-checks) |
| TRACE-TP-509 | Generated exact oracle | TRACE-REQ-045, TRACE-REQ-046, TRACE-REQ-047 | Exact rational oracle in its own reduced fractions: entry as the largest and exit as the smallest slab crossing, the face named by which face planes the exact hit point lies on; 8 000 cases from seed 0x5EED_B0C5_0359_0001 (4 851 misses, 2 415 front hits, 734 back hits, 791 hits with a zero direction component; floors 3 638, 1 811, 550, 593), aimed, inside, parallel, exact-feature and random rays, flat boxes, random ranges; the seed and case printed on failure. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_509_hits_match_an_exact_rational_oracle_on_generated_rays` |
| TRACE-TP-510 | Grazing rays | TRACE-REQ-045 | Constructed on the box [-1, 1]^3: a ray in the plane y = 1 hits at 4.0 and one raw unit outside misses; a diagonal that touches only the edge hits at 2.0, one raw unit higher misses and one raw unit lower clips through the x face; a ray along an edge line hits the end face and one raw unit off misses. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_510_a_grazing_ray_hits_and_one_raw_unit_outside_misses` |
| TRACE-TP-511 | Origin inside | TRACE-REQ-045, TRACE-REQ-047 | Constructed: from the centre the exit is at 1.0, back face, normal +x; a quarter-length direction gives 4.0; off-centre the nearer face wins; t_min past the exit misses; an origin on a face gives distance 0 front or back; a flat box in its own plane. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_511_an_origin_inside_the_box_hits_the_exit_from_inside` |
| TRACE-TP-512 | Box behind the origin | TRACE-REQ-045, TRACE-REQ-047 | Constructed: crossings at -6 and -4 miss with t_min = 0 and give -6.0, front face, normal -x, with t_min = -20; a ray pointing away misses; an entry below t_min reports the exit from inside; an entry above t_max misses. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_512_a_box_behind_the_origin_is_not_hit` |
| TRACE-TP-513 | Edge and corner ties | TRACE-REQ-046 | Constructed: edges xy, yz and xz and corners hit from both sides report the lowest axis, entering (-x, -y, -x, -x, +x) and leaving; two faces whose floors are both 1 but whose true crossings differ by 2^-32 report the later face, in both axis orders. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_513_edge_and_corner_hits_report_the_lowest_axis` |
| TRACE-TP-514 | Parallel to a slab | TRACE-REQ-047 | Constructed: an origin one raw unit outside a slab misses, an origin exactly on either bound plane hits through the other axes, a parallel axis never supplies the face, two parallel axes, and a zero-thickness box in and across its plane. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_514_a_ray_parallel_to_a_slab_is_decided_by_the_origin_in_it` |
| TRACE-TP-515 | Typed refusals | TRACE-REQ-044 | Constructed: each bound one raw unit past, i64 extremes, a reversed box and a corner both reversed and out of domain give the stated error variant; the largest box and a point box are accepted and round-trip. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_515_boxes_outside_the_trace_domain_are_refused_with_a_typed_error` |
| TRACE-TP-516 | Widest inputs | TRACE-REQ-044, TRACE-REQ-045 | The exact oracle on every domain corner against the largest box and unit boxes, with the longest, shortest and axis-only directions and a 2^32 + 1 unit direction, under overflow-checked test arithmetic. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_516_the_widest_inputs_match_the_oracle_without_overflow` |
| TRACE-TP-517 | The oracle's own faces | TRACE-REQ-046 | Hand-computed hits checked against the oracle itself, so a shared mistake cannot hide. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_517_the_oracle_names_the_face_it_expects` |
| TRACE-TP-518 | Deliberate mutations of the box test | TRACE-REQ-044, TRACE-REQ-045, TRACE-REQ-046, TRACE-REQ-047 | Each rule broken on purpose makes a case above fail. | [Box mutation checks](#box-mutation-checks) |
| TRACE-TP-519 | Distance limits | TRACE-REQ-045 | Constructed on [-1, 1]^3 from (-5, 0, 0) along +x: entry 4.0, exit 6.0. t_max = 4.0 hits the entry, 4.0 - 1 raw misses; t_min = 4.0 reports the entry, 4.0 + 1 raw the exit; the exit is on t_max = 6.0 and one raw unit short misses; a non-whole entry floor(4/3 * 2^32) = 5726623061 on t_min and t_max and one step past each. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_519_the_distance_limits_are_inclusive_exactly_and_one_step_past` |
| TRACE-TP-520 | Contract identity | TRACE-REQ-045 | RAY_BOX_ID equals the literal bunny-ray-box/1. | `crates/bunny-query/tests/trace_box_tests.rs::trace_tp_520_the_contract_identity_is_public` |
| TRACE-TP-560 | Generated exact oracle | TRACE-REQ-061, TRACE-REQ-062, TRACE-REQ-063 | Exact rational oracle in independent arbitrary-precision integers: the meeting point kept as the fraction tn / td, each hit coordinate as (o td + d tn) / td, the squared lengths compared exactly, the distance rounded by truncating division with a remainder correction; 6 000 cases from seed 0x5EED_D15C_0363_0001 of three kinds: random (half, aimed near the disc), rim-built after a whole number of direction lengths (a quarter), and rim-built with a non-integer distance (a quarter; d.x = d.z u, d.y = d.z v, any d.z down to 1 raw, either sign of distance), the rim kinds moved by -1, 0 or +1 raw unit and using the whole i64 distance range half the time (3 718 misses, 1 184 front hits, 1 098 back hits, 221 hits behind the origin, 172 cases where the exact radius test and a test on the rounded hit point disagree; floors 2 750, 880, 820, 165 and 129, all asserted), the seed and case printed on failure. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_560_hits_match_an_exact_rational_oracle_on_generated_rays` |
| TRACE-TP-561 | Rim | TRACE-REQ-061 | Constructed: a ray falling onto (3, 4, 0) of a radius-5 disc, and onto (-5, 0, 0), hits at 2.0, front face; one raw unit beyond the rim misses and one inside hits; a slanted ray with exact meeting point (3, 4, 0) hits at 4.0 and one raw unit off misses. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_561_the_rim_is_inside_and_one_raw_unit_beyond_it_is_outside` |
| TRACE-TP-562 | Grazing rays | TRACE-REQ-061 | Constructed: with d.z = -1 raw the meeting point 3.0 away is on the rim of radius 3.0 and outside radius 3.0 minus one raw unit; with d.z = -3 raw the exact meeting point 2863311530.67 raw is outside radius 2863311530 raw (whose rounded hit point is on the rim) and inside radius 2863311531; the shortest and barely tilted directions equal the oracle. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_562_grazing_rays_decide_on_the_exact_hit_not_the_rounded_one` |
| TRACE-TP-563 | Parallel and in-plane rays | TRACE-REQ-062 | Constructed: rays in the plane through the centre and the rim, from the centre, and rays one raw unit above and below the plane all miss, with any distance range; one raw unit of tilt makes the ray a hit at 1.0. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_563_a_ray_parallel_to_the_plane_misses_even_in_the_plane` |
| TRACE-TP-564 | Origin on the disc | TRACE-REQ-063 | Constructed: an origin on the disc, on its rim or above its centre hits at 0, 0 and 3.0 with the front face when the direction points to -z; one raw unit past the rim misses; from below the hit is a back face at 6.0 for a half-length direction. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_564_an_origin_on_or_over_the_disc_hits_it_from_the_right_side` |
| TRACE-TP-565 | Disc behind the origin | TRACE-REQ-061 | Constructed: a disc at z = -4 behind a ray toward +z misses with t_min = 0 and hits at -4.0, back face, with t_min = -8; a distance of -1 raw misses at t_min = 0 and hits with t_min = -1; -1/3 of a direction length is -1431655766 raw (floor, not truncation); a ray pointing away misses. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_565_a_disc_behind_the_origin_is_not_hit` |
| TRACE-TP-566 | Typed refusals | TRACE-REQ-060 | Constructed: each bound one raw unit past, and i64 extremes, give the stated error variant; the largest accepted disc round-trips its height and radius and its flat bounds are hand-computed. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_566_discs_outside_the_trace_domain_are_refused_with_a_typed_error` |
| TRACE-TP-567 | Widest inputs | TRACE-REQ-060, TRACE-REQ-061 | The exact oracle of TRACE-TP-560 on every domain corner against the largest discs at the extreme heights, with the longest, shortest and a 2^32 + 1 unit direction, under overflow-checked test arithmetic. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_567_the_widest_inputs_match_the_oracle_without_overflow` |
| TRACE-TP-569 | Distance limits | TRACE-REQ-061 | Constructed: a ray falling 2.0 onto the disc hits at exactly 2 * 2^32 raw; a range with t_min equal to it, t_max equal to it, both equal to it, hits; t_min one raw unit above or t_max one raw unit below misses. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_569_the_distance_limits_are_inclusive_at_both_ends` |
| TRACE-TP-570 | Contract identity | TRACE-REQ-061 | The public constant RAY_DISC_ID is the string bunny-ray-disc/1. | `crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_570_the_contract_identity_is_public` |
| TRACE-TP-568 | Deliberate mutations of the disc test | TRACE-REQ-060, TRACE-REQ-061, TRACE-REQ-062, TRACE-REQ-063 | Each rule broken on purpose makes a case above fail. | [Disc mutation checks](#disc-mutation-checks) |
| TRACE-TP-720 | Origin boundary | TRACE-REQ-070, TRACE-REQ-072 | Constructed, on each axis and sign: an origin coordinate of 2^40 - 1 raw is accepted and round-trips, 2^40 and 2^40 + 1 raw and the i64 extremes give OriginOutsideTraceDomain with the origin as given; the Option form is None exactly then. The limit is a literal, checked equal to TRACE_COORD_LIMIT_RAW. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_720_an_origin_outside_the_domain_is_named` |
| TRACE-TP-721 | Direction boundary | TRACE-REQ-070, TRACE-REQ-072 | Constructed, on each axis and sign: direction components of 2^32 - 1 and 2^32 raw are accepted (and all three at 2^32 at once), 2^32 + 1 raw and i64::MIN give DirectionComponentTooLarge with the direction as given; the Option form is None exactly then. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_721_a_direction_component_above_one_is_named` |
| TRACE-TP-722 | Zero direction | TRACE-REQ-070, TRACE-REQ-072 | Constructed: (0, 0, 0) gives ZeroDirection; one raw unit of either sign on each axis is accepted. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_722_a_zero_direction_is_named` |
| TRACE-TP-723 | Check order | TRACE-REQ-070 | Constructed: an origin outside the domain with a direction that is too large or zero gives OriginOutsideTraceDomain; a too-large direction gives DirectionComponentTooLarge. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_723_the_first_failing_ray_condition_is_reported` |
| TRACE-TP-724 | Vertex boundary | TRACE-REQ-071, TRACE-REQ-072 | Constructed, for each vertex, axis and sign: 2^40 - 1 raw is accepted and the vertices round-trip, 2^40 and 2^40 + 1 raw give VertexOutsideTraceDomain with that index and vertex; with several vertices outside, the first in argument order is named. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_724_a_triangle_vertex_outside_the_domain_is_named_by_index` |
| TRACE-TP-725 | Generated equivalence, rays | TRACE-REQ-070, TRACE-REQ-072 | 200 000 rays from seed 0xB002_0445_0000_0001 (134 691 accepted, 31 182 origin, 23 410 direction, 10 717 zero refusals; each at least 1 000), coordinates mostly in the domain and often on, or one raw unit either side of, each limit or at the i64 extremes: try_new equals the rule written in the test from the raw values, and new equals try_new(..).ok(). The seed and case are printed on failure. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_725_ray_option_and_result_forms_agree_on_generated_inputs` |
| TRACE-TP-726 | Generated equivalence, triangles | TRACE-REQ-071, TRACE-REQ-072 | 200 000 triangles from seed 0xB002_0445_0000_07A0 (155 152 accepted; 16 299, 14 909 and 13 640 naming vertex 0, 1 and 2): try_new equals the rule written in the test, and new equals try_new(..).ok(). | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_726_triangle_option_and_result_forms_agree_on_generated_inputs` |
| TRACE-TP-727 | Display and Error | TRACE-REQ-073 | The exact message of each TraceRayError and TraceTriangleError variant, written in the test with its raw values; each is a core::error::Error with no source. | `crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_727_trace_refusals_display_their_raw_values_and_are_errors` |
| TRACE-TP-728 | Display and Error | TRACE-REQ-073 | The exact message of each TraversalError variant, written in the test; each is a core::error::Error with no source. | `crates/bunny-broadphase/tests/traversal_error_display_tests.rs::trace_tp_728_traversal_errors_display_their_condition_and_are_errors` |
| TRACE-TP-729 | Display and Error | TRACE-REQ-073 | The exact message of each CompactError variant, written in the test; each is a core::error::Error with no source. | `crates/bunny-broadphase/tests/traversal_error_display_tests.rs::trace_tp_729_compact_errors_display_their_condition_and_are_errors` |
| TRACE-TP-730 | Refusal mutations | TRACE-REQ-070, TRACE-REQ-071, TRACE-REQ-072 | Each rule of the try_ forms broken on purpose, and the fast path's direction bound broken alone, makes a case above fail. | [Typed refusal mutation checks](#typed-refusal-mutation-checks) |

```toml
[[case]]
id = "TRACE-TP-001"
requirements = ["TRACE-REQ-004", "TRACE-REQ-007", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_001_constructed_hits_report_exact_distance_and_weights"
oracle = "Rays built through a known lattice point of a triangle at a whole-unit distance must return that exact distance, exact weights and the facing from the triple product."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-002"
requirements = ["TRACE-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_002_points_outside_the_triangle_or_behind_the_ray_miss"
oracle = "Rays through points just outside each edge, and rays starting past the triangle, must miss."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-003"
requirements = ["TRACE-REQ-006", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_003_distance_limits_are_inclusive"
oracle = "A hit at exactly 5.0 is reported for limits 5.0..=5.0 and refused for limits one raw unit away."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-004"
requirements = ["TRACE-REQ-005"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_004_a_shared_edge_never_leaks"
oracle = "A ray through the diagonal of a split quad hits both halves; a ray elsewhere hits exactly one."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-005"
requirements = ["TRACE-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_005_extreme_domain_values_do_not_overflow"
oracle = "Overflow-checked test arithmetic, the signed-volume oracle on all 32 corner cases, exactly 3 hits, and each hit's bounds accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-006"
requirements = ["TRACE-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_006_values_outside_the_trace_domain_are_refused"
oracle = "Values one raw unit past each bound return None; values at each bound are accepted; vertices round-trip."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-007"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003"]
evidence = "property"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_007_box_entry_never_rejects_a_box_the_ray_reaches"
oracle = "An exact rational slab test over 40 000 seeded rays and boxes: every reached box is accepted, the bound is at or below the floored exact entry and within the stated slack, and many missed boxes are rejected."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-008"
requirements = ["TRACE-REQ-002", "TRACE-REQ-006"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_008_box_entry_respects_the_distance_limit"
oracle = "An entry at exactly 4.0 is accepted for t_max 4.0 and rejected one raw unit below; negative t_max, boxes behind the ray and an origin inside the box behave as stated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-009"
requirements = ["TRACE-REQ-004", "TRACE-REQ-005", "TRACE-REQ-007", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_009_golden_triangle_hits_lock_raw_outputs"
oracle = "Hand-computed raw distance, weights and facing for four cases: an exact hit, a floored one-third distance, a back-face hit with floored one-third weights, and a corner hit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-010"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_010_golden_box_entries_lock_raw_outputs"
oracle = "Hand-computed raw bounds for seven cases, including an inexact reciprocal, an edge graze, a corner graze whose exact exit equals its exact entry, a near miss and a face contact."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-011"
requirements = ["TRACE-REQ-010"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_011_degenerate_triangles_parallel_rays_and_far_hits_miss"
oracle = "Collinear and duplicate-vertex triangles, a ray in the plane, and a crossing 2^71 raw away all miss; control hits at the same places are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-012"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003", "TRACE-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each rounding or edge rule, broken on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-013"
requirements = ["TRACE-REQ-011", "TRACE-REQ-013"]
evidence = "property"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::every_previous_hit_keeps_its_bits_on_the_ordinary_corpus"
oracle = "Over 200 000 seeded ordinary, axis-parallel, tiny-component, grazing and origin-inside cases, every hit of the retained pre-#211 implementation is returned with the same bits, every miss it reports without overflow stays a miss, and every bound equals the exact bound rounded and saturated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-014"
requirements = ["TRACE-REQ-011", "TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "property"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::overflow_cases_agree_with_the_exact_rational_oracle"
oracle = "Over 200 000 seeded cases with coordinates near and past 2^62, i64 extremes and components of 1 to 4 000 raw units, every case where the old function overflowed agrees in hit or miss with an i128 cross-multiplication slab test, and every bound equals the exact bound rounded and saturated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-015"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::rounding_oracle_matches_checked_div_on_representable_quotients"
oracle = "The test oracle's floor-based ties-to-even rounding equals FixedQ32_32::checked_div on 50 000 seeded representable quotients and on exact ties."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-016"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::one_raw_unit_component_hits_the_box_it_passes_through"
oracle = "Origin (-5, 0, 0), direction (1, 2^-32, 0), box [-1, 1]^3 returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-017"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::two_raw_unit_component_hits_the_box_it_passes_through"
oracle = "The same ray with a component of 2 raw units returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-018"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::negative_tiny_component_hits_too"
oracle = "The same ray with a component of -1 raw unit returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-019"
requirements = ["TRACE-REQ-011", "TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::hits_across_the_old_overflow_threshold_of_twice_the_slab_half_width"
oracle = "For k = 1..=64 and components d = 1..=2k+3 raw units, box [-k, k]^3 from (-100, 0, 0) returns exactly (100 - k, 100 + k) on both sides of the old d <= 2k miss boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-020"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::origin_far_from_box_saturates_the_unrepresentable_exit"
oracle = "From x = -1.5e9 into x in [5e8, 7e8] returns (2e9, i64::MAX raw)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-021"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::coordinate_difference_beyond_i64_is_widened_not_saturated"
oracle = "A difference of 2^63 + 2^30 raw with direction 1 + 2^-32 returns the exactly computed raw exit 9 223 372 035 781 033 984, not the 9 223 372 034 707 292 160 that saturating the difference first gives."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-022"
requirements = ["TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_entries_that_tie_when_rounded_are_decided_exactly"
oracle = "Two slabs whose entries both saturate to the maximum, and whose rounded bounds would tie, are a miss because the exact entry exceeds the exact exit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-023"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::a_hit_beyond_the_representable_distance_saturates_both_bounds"
oracle = "A box first reached more than 2^31 units from the origin is a hit with both bounds at i64::MAX raw."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-024"
requirements = ["TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_bounds_never_create_a_false_hit"
oracle = "Near-axis rays with components of 0 to 4 raw units still miss a box beside their path."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-025"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/query_tests.rs::ray_aabb_saturates_an_overflowing_slab_distance_instead_of_missing"
oracle = "An origin at i64::MAX raw on the box's +x face returns (i64::MIN raw, 0)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-026"
requirements = ["TRACE-REQ-014"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_tests.rs::issue_211_intersect_ray_visits_a_leaf_a_near_axis_ray_passes_through"
oracle = "A one-leaf BVH walked by intersect_ray with direction (1, 2^-32, 0) visits primitive 0."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-027"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each rule of the ray_intersects_aabb overflow path, broken on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-101"
requirements = ["TRACE-REQ-021", "TRACE-REQ-025"]
evidence = "property"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_101_closest_hit_equals_a_test_of_every_primitive"
oracle = "Over about 3 000 seeded rays and 600 triangles, closest_hit equals a test of every triangle (minimum by distance, then index) and any_hit equals whether that test found a hit; the corpus has over 500 hits and 500 misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-102"
requirements = ["TRACE-REQ-023"]
evidence = "property"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_102_the_distance_limit_is_inclusive_and_agrees_with_every_primitive"
oracle = "For each hit ray, t_max equal to the nearest distance returns that hit and any_hit is true; t_max one raw unit shorter returns no hit and any_hit is false."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-103"
requirements = ["TRACE-REQ-021"]
evidence = "property"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_103_equal_distances_choose_the_lowest_primitive_index"
oracle = "Every triangle appears twice, so every hit is a tie; the result equals the full test and always names the first copy."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-104"
requirements = ["TRACE-REQ-026"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_104_malformed_trees_are_errors_and_empty_trees_miss"
oracle = "An empty tree misses for both searches; a child index past the buffer and a leaf range past the primitive buffer return the stated errors."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-105"
requirements = ["TRACE-REQ-026"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_105_distances_outside_the_limit_from_the_primitive_test_are_ignored"
oracle = "A primitive test that reports a distance beyond the limit, or a negative one, for every primitive yields no hit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-106"
requirements = ["TRACE-REQ-021", "TRACE-REQ-022", "TRACE-REQ-025"]
evidence = "property"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_106_differently_shaped_trees_give_the_same_answers"
oracle = "The same triangles (a third duplicated, to make ties) built in reversed, interleaved and shuffled order, with leaf entries mapped back to the original indices, and under one leaf listing them in reverse, give identical closest_hit and any_hit answers to the original tree and the full test over 1 500 rays."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-107"
requirements = ["TRACE-REQ-024"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_107_the_nearer_child_is_searched_first_and_the_farther_one_is_skipped"
oracle = "Two triangles 5.0 and 8.0 away, the far one in the left leaf: the near one is returned and it is the only primitive tested."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-108"
requirements = ["TRACE-REQ-021", "TRACE-REQ-024"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_108_a_subtree_that_starts_at_the_best_distance_is_still_searched"
oracle = "Two coincident triangles whose leaf bounds equal the hit distance 5.0 exactly; the leaf searched first holds index 1. Both are tested and index 0 is returned, and the same holds when one leaf lists them as 1, 0."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-109"
requirements = ["TRACE-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/closest_hit_tests.rs::trace_tp_109_any_hit_stops_at_the_first_hit"
oracle = "Four stacked triangles that the ray hits: any_hit is true after exactly one primitive test; with t_max 4.0, below every box, it is false after none."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-110"
requirements = ["TRACE-REQ-021", "TRACE-REQ-024", "TRACE-REQ-025"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each pruning, ordering or stop rule, broken on purpose, makes at least one implemented traversal case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-201"
requirements = ["TRACE-REQ-027"]
evidence = "test"
test = "crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_201_seeded_limits_around_the_true_distance_agree"
oracle = "Over 30 000 seeded rays aimed at seeded triangles in five size classes (unit scale, det below 2^32, full domain budgets), hit_triangle equals the frozen reference at upper and lower limits equal to the true distance, one and two raw units below, one above, 0, -1, 1, i64::MIN, i64::MAX and i64::MAX - 1; each class has over 2 000 crossings."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-202"
requirements = ["TRACE-REQ-027"]
evidence = "test"
test = "crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_202_exact_distances_at_the_limit_agree"
oracle = "Rays falling straight onto a fixed triangle at whole raw distances, where det is a multiple of 2^32, equal the frozen reference at the same limits."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-203"
requirements = ["TRACE-REQ-027"]
evidence = "test"
test = "crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_203_extreme_domain_values_agree"
oracle = "The domain-corner cases of TRACE-TP-005 equal the frozen reference at the same limits; exactly 3 of the 32 cross."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-204"
requirements = ["TRACE-REQ-027"]
evidence = "test"
test = "crates/bunny-query/tests/trace_equivalence_tests.rs::trace_tp_204_determinants_at_multiples_of_2_pow_32_agree"
oracle = "Raw-unit triangles hit at a corner with det exactly 2^32, 3 * 2^32, 2^32 + 1 (641 * 6 700 417) and 1 equal the frozen reference at the same limits."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-205"
requirements = ["TRACE-REQ-027"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::hit_triangle_matches_the_reference_on_the_scene_traversal"
oracle = "Every triangle test that closest_hit makes on the benchmark scene's 1 024 camera rays equals the frozen reference at the limit the traversal passed and at the limits around the true distance."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-301"
requirements = ["TRACE-REQ-028", "TRACE-REQ-021", "TRACE-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_301_compact_searches_equal_the_source_tree_and_every_primitive"
oracle = "On seeded scenes of 2, 3, 5, 17, 64 and 600 triangles (3 500 rays in all), compact_closest_hit and compact_any_hit make the same primitive calls with the same limits in the same order, and return the same results, as closest_hit and any_hit on the source tree, which equal a test of every triangle; each scene has hits and misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-302"
requirements = ["TRACE-REQ-028", "TRACE-REQ-022", "TRACE-REQ-023"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_302_compact_searches_agree_on_ties_limits_and_other_tree_shapes"
oracle = "With a third of 150 triangles duplicated (ties), on the built tree, two reshaped trees and a single leaf listing every triangle, the compact searches agree with the source searches and the full test at no limit, at the nearest distance and one raw unit below it."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-303"
requirements = ["TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_303_empty_single_primitive_and_single_leaf_trees"
oracle = "An empty tree converts to no root and both searches miss; a one-primitive tree converts to a leaf root with no compact node; a single leaf of three primitives converts to a leaf root; both agree with the source searches."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-304"
requirements = ["TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_304_malformed_input_is_a_typed_error"
oracle = "A child index past the slice (including u32::MAX) is InvalidNodeIndex, a too-small output is OutputTooSmall (and the exact size succeeds), a node reached twice is NotATree, and a comb tree needing more than 64 pending subtrees is TooDeep while 60 levels convert."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-305"
requirements = ["TRACE-REQ-028", "TRACE-REQ-026"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_305_malformed_compact_trees_are_traversal_errors"
oracle = "A root that names a missing compact node is InvalidNodeIndex, a leaf range past the primitive buffer is InvalidPrimitiveRange, and a chain that keeps a leaf on the stack per level is StackOverflow."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-306"
requirements = ["TRACE-REQ-028", "TRACE-REQ-024", "TRACE-REQ-025"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_306_the_compact_search_keeps_the_order_pruning_and_early_stop"
oracle = "The constructed cases of TRACE-TP-107 to TRACE-TP-109 on the compact tree: the nearer leaf is the only one tested, a leaf starting at the best distance is still tested (calls 1 then 0, index 0 returned), and compact_any_hit stops after one test and makes none below every box."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-307"
requirements = ["TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_307_compact_conversion_golden_vectors"
oracle = "A hand-built nine-node tree converts to four exactly stated compact nodes in depth-first, left-first order; a seeded 300-triangle tree converts to bytes whose FNV-1a checksum is the stated constant, the same on every target that runs the tests."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-308"
requirements = ["TRACE-REQ-029"]
evidence = "test"
test = "crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_308_box_entry_matches_the_reference_on_the_golden_and_oracle_corpora"
oracle = "The inputs of TRACE-TP-010 (and an origin-inside case) and the 40 000 seeded cases of TRACE-TP-007, drawn with the same seed, return the same Option as the retained reference at every limit class; over 2 000 cases each reach and miss."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-309"
requirements = ["TRACE-REQ-029"]
evidence = "test"
test = "crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_309_box_entry_matches_the_reference_on_seeded_trace_domain_cases"
oracle = "300 000 seeded rays (origins across the whole trace domain, within 4 units and within 2^20 raw; general, near-axis and axis-parallel directions with zero, tiny, unit and maximum components) against boxes around, ahead of, behind and far from the origin, with faces through the origin, inverted axes and full-range axes, return the same Option as the retained reference at every limit class; over a tenth of them each reach and miss."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-310"
requirements = ["TRACE-REQ-029"]
evidence = "test"
test = "crates/bunny-query/tests/box_entry_equivalence_tests.rs::trace_tp_310_box_entry_matches_the_reference_at_the_ends_of_the_i64_range"
oracle = "Every (min, max) pair of 15 extreme coordinates (i64::MIN and MAX, their neighbours, the domain bound and +-2^62), inverted pairs included, on each axis, for 7 extreme origin coordinates and 9 extreme direction components, then 200 000 seeded boxes whose every coordinate is within a few units of an extreme, return the same Option as the retained reference at every limit class."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-311"
requirements = ["TRACE-REQ-029"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::every_recorded_box_test_matches_the_reference_box_test"
oracle = "Every box test the closest_hit and any_hit searches make on the benchmark scene's 1 024 camera rays returns the same Option as the retained reference at the recorded limit, at i64::MAX, 0, -1 and at the reference's entry bound and one below it."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-312"
requirements = ["TRACE-REQ-029"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each rule of the rewritten box test, broken on purpose, makes at least one of TRACE-TP-308 to TRACE-TP-311 fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-313"
requirements = ["TRACE-REQ-030"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_313_numerators_are_the_exact_weights_of_constructed_hits"
oracle = "Constructed exact weights: the hit point has weights (b, c) / 64 by construction."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-314"
requirements = ["TRACE-REQ-030"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_314_numerators_off_the_triangle_are_refused"
oracle = "Specified: the conditions of the exact triangle test."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-315"
requirements = ["TRACE-REQ-004", "TRACE-REQ-005", "TRACE-REQ-008", "TRACE-REQ-009", "TRACE-REQ-030"]
evidence = "test"
test = "crates/bunny-query/tests/trace_edge_oracle_tests.rs::trace_tp_315_hits_near_edges_and_vertices_match_a_signed_volume_oracle"
oracle = "Derived: an independent signed-volume formulation in 256-bit integers with its own floor division, over cases at and next to every edge and vertex."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-316"
requirements = ["TRACE-REQ-004", "TRACE-REQ-005", "TRACE-REQ-008"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#edge-oracle-mutation-checks"
oracle = "Each edge, distance-rounding and facing rule, broken on purpose, makes TRACE-TP-315 fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-317"
requirements = ["TRACE-REQ-029"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_317_box_entry_on_an_inverted_box_is_defined_per_axis"
oracle = "A box with x from 6 to 5 raw returns Some(6), as a box from 6 to 7 does; x from -5 to -6 returns None; a box inverted on an axis the ray does not move along returns None."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-318"
requirements = ["TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_318_a_shared_node_converts_while_the_node_count_allows_it"
oracle = "A root whose two interior children name the same two leaves is NotATree in a five-node slice; with four unused nodes after them it converts to three compact nodes."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-401"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_401_closest_hit_refuses_cyclic_buffers"
oracle = "closest_hit on four cyclic buffers (node 0 its own left child beside a leaf the ray misses, the same beside a leaf covering everything, a three-node cycle through left children, and a node that is its own right child) returns InvalidNodeIndex with the stated number of primitive calls (none)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-402"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_402_any_hit_refuses_cyclic_buffers"
oracle = "any_hit on the four cyclic buffers of TRACE-TP-401 returns InvalidNodeIndex with no primitive call."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-403"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_403_intersect_ray_refuses_cyclic_buffers"
oracle = "intersect_ray on the four cyclic buffers returns InvalidNodeIndex after exactly the stated leaf callbacks (one for the covering leaf, none otherwise)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-404"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_404_intersect_aabb_refuses_cyclic_buffers"
oracle = "intersect_aabb on the four cyclic buffers returns InvalidNodeIndex after exactly the stated leaf callbacks (one for the covering leaf, none otherwise)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-405"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_405_shared_subtrees_are_searched_within_the_budget_and_refused_beyond_it"
oracle = "With a query reaching every node, a shared leaf within the budget is searched as the tree it unfolds into (callbacks 0, 1, 0, as the retained functions give); a shared interior node and a 40-node ladder whose unfolding is exponential are InvalidNodeIndex; each is refused exactly when compact_bvh calls it NotATree."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-406"
requirements = ["TRACE-REQ-031"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_guard_tests.rs::trace_tp_406_valid_trees_keep_every_result_and_callback"
oracle = "On seeded scenes of 1 to 300 triangles built by build_bvh and build_bvh_sah, the same trees with a seeded leaf range or child index made out of range, a 70-level comb (deeper than the stack) and a 64-level comb ending in out-of-range children reached with a full stack, all four functions return the same result and make the same callback calls (closest_hit: primitive and limit) in the same order as frozen copies of the pre-#269 functions, over up to 5 040 seeded queries (a ray with t_max unbounded or up to 15 units, and a box; over 2 500 required); failures name the seed and case."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-407"
requirements = ["TRACE-REQ-031"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Removing the guard from each of the four functions makes its cycle case fail or not terminate under a wall-clock limit; a budget one smaller fires on a valid tree and fails an existing case."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-408"
requirements = ["TRACE-REQ-032"]
evidence = "test"
test = "crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_408_from_ray_accepts_the_unit_vector_of_the_issue_on_every_axis"
oracle = "`from_ray` accepts the unit vectors with a component of `2^32 + 1` on each axis and sign; `new` keeps its bound."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-409"
requirements = ["TRACE-REQ-032"]
evidence = "test"
test = "crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_409_normalisation_never_yields_a_component_above_one"
oracle = "`FixedUnitVec3::new` produces no component above `2^32` over 200 000 generated vectors with minimum counts."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-410"
requirements = ["TRACE-REQ-032"]
evidence = "test"
test = "crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_410_kernels_agree_with_the_oracles_for_long_unit_directions"
oracle = "Derived: signed-volume oracle for hit_triangle and exact rational entry for box_entry, with the contract slack."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-411"
requirements = ["TRACE-REQ-032"]
evidence = "test"
test = "crates/bunny-query/tests/trace_unit_direction_tests.rs::trace_tp_411_a_long_direction_hit_has_the_exactly_floored_distance"
oracle = "Constructed: the distance is floor(2^65 / (2^32 + 1)) raw."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-412"
requirements = ["TRACE-REQ-033", "TRACE-REQ-021", "TRACE-REQ-025", "TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_depth_tests.rs::trace_tp_412_a_comb_of_depth_63_is_searched_and_one_of_depth_64_overflows"
oracle = "Constructed combs: the stack holds the d pending left leaves and the two children of the deepest interior node, d + 1 entries, so depth 63 fits 64 entries and depth 64 does not; below the bound the answers equal a test of every primitive."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-413"
requirements = ["TRACE-REQ-034"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_depth_tests.rs::trace_tp_413_compact_conversion_queues_at_most_one_subtree_per_level"
oracle = "Constructed spines: the conversion queues one right subtree per level above the deepest interior node plus its two children, d entries for depth d, so 64 converts and 65 is TooDeep."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-414"
requirements = ["TRACE-REQ-033", "TRACE-REQ-034"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#depth-mutation-checks"
oracle = "Each stack capacity moved by one each way, each mutant's tests under timeout 120 with every source touched before the build; one equivalent mutant named."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-415"
requirements = ["TRACE-REQ-035"]
evidence = "test"
test = "crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_415_counts_on_constructed_trees_are_the_hand_derived_values"
oracle = "Constructed: counts derived by hand from the search order for each tree."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-416"
requirements = ["TRACE-REQ-035"]
evidence = "property"
test = "crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_416_counts_equal_a_recursive_reference_search"
oracle = "Retained reference: an independent recursive search with its own counters."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-417"
requirements = ["TRACE-REQ-035"]
evidence = "property"
test = "crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_417_counted_and_uncounted_searches_agree_on_valid_and_malformed_trees"
oracle = "Differential: the uncounted function, whose results the compact tests tie to closest_hit and a test of every primitive."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-418"
requirements = ["TRACE-REQ-035"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_counted_compact_search_matches_the_replica_on_every_ray"
oracle = "Retained reference: the profile replica's answers and recorded calls, itself checked against closest_hit and any_hit on every ray."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-419"
requirements = ["TRACE-REQ-035"]
evidence = "test"
test = "crates/bunny-broadphase/tests/counted_traversal_tests.rs::trace_tp_419_adding_counts_sums_saturates_and_keeps_the_peak"
oracle = "Constructed: exact sums and maxima."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-420"
requirements = ["TRACE-REQ-035"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#counting-mutation-checks"
oracle = "Each counting rule broken on purpose makes a case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-421"
requirements = ["TRACE-REQ-028"]
evidence = "test"
test = "crates/bunny-broadphase/tests/compact_tests.rs::trace_tp_421_the_compact_visit_bound_allows_exactly_one_visit_per_node"
oracle = "A three-node compact cycle with a leaf beside each node: `compact_closest_hit` and `compact_any_hit` test primitives 0, 1 and 2 and refuse the fourth interior visit with `InvalidNodeIndex`, so a bound one looser (a fourth primitive test) or one tighter (no test of primitive 2) fails."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-501"
requirements = ["TRACE-REQ-041", "TRACE-REQ-042"]
evidence = "property"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_501_hits_match_an_exact_rational_oracle_on_generated_rays"
oracle = "Exact rational oracle in independent arbitrary-precision integers: discriminant as b^2 - a c, each rounded root found by bisection on the exact predicate k <= root, no square root; the normal checked without the kernel's formula, by exact sign decisions of p + q sqrt(disc): each component within 1/2 + 2^-24 raw units of the exact unit normal at the exact root, a positive dot product with the exact offset, and FixedUnitVec3::try_from_unit. 6 000 cases from seed 0x5EED_5F4E_0358_0001, one in three with its range moved onto its own exact roots (t_min on the near root or one past it, t_max on the near root or one before it, or both around the far root) by a second stream from seed 0x5EED_5F4E_0422_0001 (4 244 misses, 925 front hits, 831 back hits, 232 hits at a radius of at most 2^8 raw, 128 distances on t_min, 235 on t_max, 235 misses with a root one raw unit past t_max; floors 3 183, 693, 623, 174, 96, 176, 176), the seed and case printed on failure."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-502"
requirements = ["TRACE-REQ-041", "TRACE-REQ-043"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_502_a_tangent_ray_hits_once_and_a_ray_one_raw_unit_outside_misses"
oracle = "Constructed: the line y = 1 touches the unit sphere at distance 4.0 with normal (0, 1, 0), front face; one raw unit further out misses; one raw unit in is checked against the oracle and lies within the hand-computed half chord."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-503"
requirements = ["TRACE-REQ-041", "TRACE-REQ-043"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_503_an_origin_inside_the_sphere_hits_the_far_side_from_inside"
oracle = "Constructed: from the centre of a radius-2 sphere the hit is at 2.0, back face, normal (1, 0, 0); off-centre with a half-length direction the distance is 3.464.. (2 sqrt 3) and equals the oracle; t_min past the exit misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-504"
requirements = ["TRACE-REQ-041", "TRACE-REQ-043"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_504_a_sphere_behind_the_origin_is_not_hit"
oracle = "Constructed: roots at -11 and -9 miss with t_min = 0 and give the near root -11.0, front face, normal (-1, 0, 0) with t_min = -20; a ray pointing away misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-505"
requirements = ["TRACE-REQ-040"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_505_spheres_outside_the_trace_domain_are_refused_with_a_typed_error"
oracle = "Constructed: each bound one raw unit past, and i64 extremes, give the stated error variant; the largest accepted sphere round-trips its centre and radius and its bounds are hand-computed."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-506"
requirements = ["TRACE-REQ-040", "TRACE-REQ-041"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_506_the_widest_inputs_match_the_oracle_without_overflow"
oracle = "The exact oracle of TRACE-TP-501 on every domain corner against the largest spheres, with the longest, shortest and a 2^32 + 1 unit direction, under overflow-checked test arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-507"
requirements = ["TRACE-REQ-041"]
evidence = "test"
test = "crates/bunny-query/src/trace/wide.rs::tests::isqrt_is_the_floor_root_around_squares"
oracle = "Hand-computed: products of the largest values, carries across limbs, refused overflow, and isqrt of k^2 - 1, k^2 and k^2 + 1 up to k = 2^128 - 1."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-508"
requirements = ["TRACE-REQ-040", "TRACE-REQ-041", "TRACE-REQ-042"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#sphere-mutation-checks"
oracle = "Each rule broken on purpose makes a case above fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-701"
requirements = ["TRACE-REQ-042"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_701_a_one_raw_unit_sphere_hit_from_its_centre_has_a_unit_normal"
oracle = "Constructed (independent review of flyingrobots/bunny#375): radius 1 raw, origin at the centre, direction (1, 1, 1); distance 0, back face, each component 2479700524 or 2479700525 raw (2^32 / sqrt 3 = 2479700524.87..) and the normal oracle of TRACE-TP-501 holds. The code at the merge of flyingrobots/bunny#375 returned (0, 0, 0)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-702"
requirements = ["TRACE-REQ-042"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_702_a_sphere_of_radius_one_hundredth_has_a_unit_normal"
oracle = "Constructed (same review): centre raw (12345, -777, 4242), radius raw 42949673, origin (-1, -1, -1), direction (1, 1, 1); distance raw 4270175561, front face, and the normal oracle holds. The code at the merge of flyingrobots/bunny#375 returned (-2480407998, -2479095798, -2479597698), 2^32 + 56.2 raw units long."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-703"
requirements = ["TRACE-REQ-042"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_703_the_seeds_seven_old_zero_normals_are_unit_normals"
oracle = "Cases 22, 129, 757, 988, 1275, 1628 and 2635 of the base corpus of TRACE-TP-501 (radius 1 raw each): distance and face equal the oracle and the normal oracle holds on all seven; a failure lists every one that breaks. All seven were (0, 0, 0) at the merge of flyingrobots/bunny#375."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-704"
requirements = ["TRACE-REQ-041"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_704_the_distance_limits_are_inclusive_at_both_ends"
oracle = "Constructed: the unit sphere from (-4, 0, 0) along +x has exact roots 3.0 and 5.0; each root exactly on t_min and on t_max hits, t_min == t_max == 3.0 hits, one raw unit past t_max misses, and one raw unit above the near root moves the hit to the far root (back face, normal (1, 0, 0))."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-705"
requirements = ["TRACE-REQ-042"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_705_the_normal_bound_holds_from_the_smallest_radius_to_the_largest"
oracle = "Constructed: a skew direction through an off-axis centre at radius 1 raw, 0.01, 1.0 and the largest admitted, from outside (front face) and from the centre (back face), each against the distance oracle and the normal oracle of TRACE-TP-501."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-706"
requirements = ["TRACE-REQ-041"]
evidence = "test"
test = "crates/bunny-query/tests/trace_sphere_tests.rs::trace_tp_706_the_contract_identity_is_public"
oracle = "RAY_SPHERE_ID equals bunny-ray-sphere/1."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-707"
requirements = ["TRACE-REQ-041", "TRACE-REQ-042"]
evidence = "test"
test = "crates/bunny-query/src/trace/wide.rs::tests::wide_shifts_keep_every_bit_or_refuse"
oracle = "Hand-computed: U256::checked_shl by 0, 128, 129, 200, 255 and 256 bits keeps every bit or refuses the overflow."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-540"
requirements = ["TRACE-REQ-066", "TRACE-REQ-067", "TRACE-REQ-068"]
evidence = "property"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_540_hits_match_an_exact_rational_oracle_on_generated_rays"
oracle = "Exact rational oracle in independent arbitrary-precision integers that lists every point where the line meets a surface (a side root whose height is between the caps, a cap crossing inside its disc, each checked exactly) and takes the earliest and latest, a cap winning a tie; discriminant as b^2 - a c, each side root's floor by bisection on an exact predicate, no square root; each normal checked without the kernel's formula (a cap's is the exact axis; a side's has y = 0, each component within one raw unit of the exact unit normal a (o + d t) / (a r) built from the oracle's own root, and a positive dot product with the exact offset); 6 000 cases from seed 0x5EED_C71D_0360_0001, one in ten parallel to the axis, one in ten parallel to the caps (half of those in a cap's plane), one in ten through a rim point, one in five from inside (3 908 misses, 407 cap entries, 884 side entries, 441 cap exits, 360 side exits; floors 2 931, 305, 663, 330, 270), the seed and case printed on failure."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-541"
requirements = ["TRACE-REQ-066", "TRACE-REQ-068"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_541_a_grazing_ray_touches_the_side_once_and_one_raw_unit_out_misses"
oracle = "Constructed: the line z = 1 touches the unit cylinder's side at distance 4.0 with normal (0, 0, 1), front face; one raw unit further out misses; one raw unit in is checked against the oracle and lies within the hand-computed half chord; one raw unit above the top cap's plane misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-542"
requirements = ["TRACE-REQ-066", "TRACE-REQ-068"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_542_an_origin_inside_hits_the_exit_from_inside"
oracle = "Constructed: from the centre the exits are at 1.0 through the side (1, 0, 0), 1.0 through the top cap and 2.0 through the bottom cap with a half-length direction, all back face; off-centre the distance is 1.732.. (sqrt 3) and is checked against the oracle; t_min past the exit misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-543"
requirements = ["TRACE-REQ-066", "TRACE-REQ-068"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_543_a_cylinder_behind_the_origin_is_not_hit"
oracle = "Constructed: side crossings at -11 and -9 miss with t_min = 0 and give -11.0, front face, normal (-1, 0, 0) with t_min = -20; a ray below the bottom cap pointing down misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-544"
requirements = ["TRACE-REQ-067"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_544_at_the_rim_the_cap_wins_the_tie"
oracle = "Constructed: a line through the top rim at 1.0 and the bottom rim at 3.0 reports the top cap's normal at the entry and the bottom cap's at the exit; a line touching only the rim point (-1, 1, 0) reports the top cap, front face; one raw unit higher enters through the cap, one lower through the side (equal to the oracle)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-545"
requirements = ["TRACE-REQ-066", "TRACE-REQ-068"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_545_a_ray_parallel_to_the_axis_hits_only_a_cap"
oracle = "Constructed: inside the radius the entry is the bottom cap at 4.0 and the exit the top cap at 6.0; on the side itself the bottom rim, with the cap's normal; one raw unit outside the radius misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-546"
requirements = ["TRACE-REQ-066", "TRACE-REQ-068"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_546_a_ray_in_a_caps_plane_hits_the_side"
oracle = "Constructed: along y = 1 and y = -1 the hit is the side at 3.0 with normal (-1, 0, 0), and the exit the side at 5.0; one raw unit below the bottom plane misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-547"
requirements = ["TRACE-REQ-065"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_547_cylinders_outside_the_trace_domain_are_refused_with_a_typed_error"
oracle = "Constructed: each bound one raw unit past, i64 extremes, equal and reversed caps give the stated error variant, a bad radius before a bad cap; the largest and smallest accepted cylinders round-trip and their bounds are hand-computed."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-548"
requirements = ["TRACE-REQ-065", "TRACE-REQ-066"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_548_the_widest_inputs_match_the_oracle_without_overflow"
oracle = "The exact oracle of TRACE-TP-540 on every domain corner against the largest, the thinnest and a one-raw-unit-tall cylinder, with the longest, shortest, axis-parallel, cap-parallel and a 2^32 + 1 unit direction, under overflow-checked test arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-550"
requirements = ["TRACE-REQ-066"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_550_the_distance_limits_are_inclusive_at_both_ends"
oracle = "Constructed: the side is entered at exactly 3.0 and left at exactly 5.0; the entry is hit with t_max = 3.0 (and t_min = t_max = 3.0) and missed with t_max one raw unit below; the exit is hit with t_max = 5.0 and with t_min = 5.0, and missed one raw unit past either limit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-551"
requirements = ["TRACE-REQ-069"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_551_the_contract_identity_is_published"
oracle = "Constructed: RAY_CYLINDER_ID equals the string bunny-ray-cylinder/1 written in the test."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-552"
requirements = ["TRACE-REQ-067"]
evidence = "test"
test = "crates/bunny-query/tests/trace_cylinder_tests.rs::trace_tp_552_side_normals_of_the_smallest_cylinder_are_unit_and_outward"
oracle = "Constructed: a cylinder of radius one raw unit hit by rays through its axis in five directions; each side normal passes the normal check of TRACE-TP-540 (within one raw unit of the exact unit normal, outward, y = 0)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-549"
requirements = ["TRACE-REQ-065", "TRACE-REQ-066", "TRACE-REQ-067", "TRACE-REQ-068", "TRACE-REQ-069"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#cylinder-mutation-checks"
oracle = "Each rule broken on purpose makes a case above fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-509"
requirements = ["TRACE-REQ-045", "TRACE-REQ-046", "TRACE-REQ-047"]
evidence = "property"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_509_hits_match_an_exact_rational_oracle_on_generated_rays"
oracle = "Exact rational oracle in its own reduced fractions: entry as the largest and exit as the smallest slab crossing, the face named by which face planes the exact hit point lies on; 8 000 cases from seed 0x5EED_B0C5_0359_0001 (4 851 misses, 2 415 front hits, 734 back hits, 791 hits with a zero direction component; floors 3 638, 1 811, 550, 593), aimed, inside, parallel, exact-feature and random rays, flat boxes, random ranges; the seed and case printed on failure."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-510"
requirements = ["TRACE-REQ-045"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_510_a_grazing_ray_hits_and_one_raw_unit_outside_misses"
oracle = "Constructed on the box [-1, 1]^3: a ray in the plane y = 1 hits at 4.0 and one raw unit outside misses; a diagonal that touches only the edge hits at 2.0, one raw unit higher misses and one raw unit lower clips through the x face; a ray along an edge line hits the end face and one raw unit off misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-511"
requirements = ["TRACE-REQ-045", "TRACE-REQ-047"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_511_an_origin_inside_the_box_hits_the_exit_from_inside"
oracle = "Constructed: from the centre the exit is at 1.0, back face, normal +x; a quarter-length direction gives 4.0; off-centre the nearer face wins; t_min past the exit misses; an origin on a face gives distance 0 front or back; a flat box in its own plane."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-512"
requirements = ["TRACE-REQ-045", "TRACE-REQ-047"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_512_a_box_behind_the_origin_is_not_hit"
oracle = "Constructed: crossings at -6 and -4 miss with t_min = 0 and give -6.0, front face, normal -x, with t_min = -20; a ray pointing away misses; an entry below t_min reports the exit from inside; an entry above t_max misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-513"
requirements = ["TRACE-REQ-046"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_513_edge_and_corner_hits_report_the_lowest_axis"
oracle = "Constructed: edges xy, yz and xz and corners hit from both sides report the lowest axis, entering (-x, -y, -x, -x, +x) and leaving; two faces whose floors are both 1 but whose true crossings differ by 2^-32 report the later face, in both axis orders."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-514"
requirements = ["TRACE-REQ-047"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_514_a_ray_parallel_to_a_slab_is_decided_by_the_origin_in_it"
oracle = "Constructed: an origin one raw unit outside a slab misses, an origin exactly on either bound plane hits through the other axes, a parallel axis never supplies the face, two parallel axes, and a zero-thickness box in and across its plane."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-515"
requirements = ["TRACE-REQ-044"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_515_boxes_outside_the_trace_domain_are_refused_with_a_typed_error"
oracle = "Constructed: each bound one raw unit past, i64 extremes, a reversed box and a corner both reversed and out of domain give the stated error variant; the largest box and a point box are accepted and round-trip."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-516"
requirements = ["TRACE-REQ-044", "TRACE-REQ-045"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_516_the_widest_inputs_match_the_oracle_without_overflow"
oracle = "The exact oracle on every domain corner against the largest box and unit boxes, with the longest, shortest and axis-only directions and a 2^32 + 1 unit direction, under overflow-checked test arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-517"
requirements = ["TRACE-REQ-046"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_517_the_oracle_names_the_face_it_expects"
oracle = "Hand-computed hits checked against the oracle itself, so a shared mistake cannot hide."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-518"
requirements = ["TRACE-REQ-044", "TRACE-REQ-045", "TRACE-REQ-046", "TRACE-REQ-047"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#box-mutation-checks"
oracle = "Each rule broken on purpose makes a case above fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-519"
requirements = ["TRACE-REQ-045"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_519_the_distance_limits_are_inclusive_exactly_and_one_step_past"
oracle = "Constructed on [-1, 1]^3 from (-5, 0, 0) along +x: entry 4.0, exit 6.0. t_max = 4.0 hits the entry, 4.0 - 1 raw misses; t_min = 4.0 reports the entry, 4.0 + 1 raw the exit; the exit is on t_max = 6.0 and one raw unit short misses; a non-whole entry floor(4/3 * 2^32) = 5726623061 on t_min and t_max and one step past each."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-520"
requirements = ["TRACE-REQ-045"]
evidence = "test"
test = "crates/bunny-query/tests/trace_box_tests.rs::trace_tp_520_the_contract_identity_is_public"
oracle = "RAY_BOX_ID equals the literal bunny-ray-box/1."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-560"
requirements = ["TRACE-REQ-061", "TRACE-REQ-062", "TRACE-REQ-063"]
evidence = "property"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_560_hits_match_an_exact_rational_oracle_on_generated_rays"
oracle = "Exact rational oracle in independent arbitrary-precision integers: the meeting point kept as the fraction tn / td, each hit coordinate as (o td + d tn) / td, the squared lengths compared exactly, the distance rounded by truncating division with a remainder correction; 6 000 cases from seed 0x5EED_D15C_0363_0001 of three kinds: random (half, aimed near the disc), rim-built after a whole number of direction lengths (a quarter), and rim-built with a non-integer distance (a quarter; d.x = d.z u, d.y = d.z v, any d.z down to 1 raw, either sign of distance), the rim kinds moved by -1, 0 or +1 raw unit and using the whole i64 distance range half the time (3 718 misses, 1 184 front hits, 1 098 back hits, 221 hits behind the origin, 172 cases where the exact radius test and a test on the rounded hit point disagree; floors 2 750, 880, 820, 165 and 129, all asserted), the seed and case printed on failure."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-561"
requirements = ["TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_561_the_rim_is_inside_and_one_raw_unit_beyond_it_is_outside"
oracle = "Constructed: a ray falling onto (3, 4, 0) of a radius-5 disc, and onto (-5, 0, 0), hits at 2.0, front face; one raw unit beyond the rim misses and one inside hits; a slanted ray with exact meeting point (3, 4, 0) hits at 4.0 and one raw unit off misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-562"
requirements = ["TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_562_grazing_rays_decide_on_the_exact_hit_not_the_rounded_one"
oracle = "Constructed: with d.z = -1 raw the meeting point 3.0 away is on the rim of radius 3.0 and outside radius 3.0 minus one raw unit; with d.z = -3 raw the exact meeting point 2863311530.67 raw is outside radius 2863311530 raw (whose rounded hit point is on the rim) and inside radius 2863311531; the shortest and barely tilted directions equal the oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-563"
requirements = ["TRACE-REQ-062"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_563_a_ray_parallel_to_the_plane_misses_even_in_the_plane"
oracle = "Constructed: rays in the plane through the centre and the rim, from the centre, and rays one raw unit above and below the plane all miss, with any distance range; one raw unit of tilt makes the ray a hit at 1.0."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-564"
requirements = ["TRACE-REQ-063"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_564_an_origin_on_or_over_the_disc_hits_it_from_the_right_side"
oracle = "Constructed: an origin on the disc, on its rim or above its centre hits at 0, 0 and 3.0 with the front face when the direction points to -z; one raw unit past the rim misses; from below the hit is a back face at 6.0 for a half-length direction."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-565"
requirements = ["TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_565_a_disc_behind_the_origin_is_not_hit"
oracle = "Constructed: a disc at z = -4 behind a ray toward +z misses with t_min = 0 and hits at -4.0, back face, with t_min = -8; a distance of -1 raw misses at t_min = 0 and hits with t_min = -1; -1/3 of a direction length is -1431655766 raw (floor, not truncation); a ray pointing away misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-566"
requirements = ["TRACE-REQ-060"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_566_discs_outside_the_trace_domain_are_refused_with_a_typed_error"
oracle = "Constructed: each bound one raw unit past, and i64 extremes, give the stated error variant; the largest accepted disc round-trips its height and radius and its flat bounds are hand-computed."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-567"
requirements = ["TRACE-REQ-060", "TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_567_the_widest_inputs_match_the_oracle_without_overflow"
oracle = "The exact oracle of TRACE-TP-560 on every domain corner against the largest discs at the extreme heights, with the longest, shortest and a 2^32 + 1 unit direction, under overflow-checked test arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-569"
requirements = ["TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_569_the_distance_limits_are_inclusive_at_both_ends"
oracle = "Constructed: a ray falling 2.0 onto the disc hits at exactly 2 * 2^32 raw; a range with t_min equal to it, t_max equal to it, both equal to it, hits; t_min one raw unit above or t_max one raw unit below misses."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-570"
requirements = ["TRACE-REQ-061"]
evidence = "test"
test = "crates/bunny-query/tests/trace_disc_tests.rs::trace_tp_570_the_contract_identity_is_public"
oracle = "The public constant RAY_DISC_ID is the string bunny-ray-disc/1."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-568"
requirements = ["TRACE-REQ-060", "TRACE-REQ-061", "TRACE-REQ-062", "TRACE-REQ-063"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#disc-mutation-checks"
oracle = "Each rule broken on purpose makes a case above fail."
tier = "manual"
status = "implemented"

[[case]]
id = "TRACE-TP-720"
requirements = ["TRACE-REQ-070", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_720_an_origin_outside_the_domain_is_named"
oracle = "Constructed, on each axis and sign: an origin coordinate of 2^40 - 1 raw is accepted and round-trips, 2^40 and 2^40 + 1 raw and the i64 extremes give OriginOutsideTraceDomain with the origin as given; the Option form is None exactly then. The limit is a literal, checked equal to TRACE_COORD_LIMIT_RAW."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-721"
requirements = ["TRACE-REQ-070", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_721_a_direction_component_above_one_is_named"
oracle = "Constructed, on each axis and sign: direction components of 2^32 - 1 and 2^32 raw are accepted (and all three at 2^32 at once), 2^32 + 1 raw and i64::MIN give DirectionComponentTooLarge with the direction as given; the Option form is None exactly then."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-722"
requirements = ["TRACE-REQ-070", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_722_a_zero_direction_is_named"
oracle = "Constructed: (0, 0, 0) gives ZeroDirection; one raw unit of either sign on each axis is accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-723"
requirements = ["TRACE-REQ-070"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_723_the_first_failing_ray_condition_is_reported"
oracle = "Constructed: an origin outside the domain with a direction that is too large or zero gives OriginOutsideTraceDomain; a too-large direction gives DirectionComponentTooLarge."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-724"
requirements = ["TRACE-REQ-071", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_724_a_triangle_vertex_outside_the_domain_is_named_by_index"
oracle = "Constructed, for each vertex, axis and sign: 2^40 - 1 raw is accepted and the vertices round-trip, 2^40 and 2^40 + 1 raw give VertexOutsideTraceDomain with that index and vertex; with several vertices outside, the first in argument order is named."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-725"
requirements = ["TRACE-REQ-070", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_725_ray_option_and_result_forms_agree_on_generated_inputs"
oracle = "200 000 rays from seed 0xB002_0445_0000_0001 (134 691 accepted, 31 182 origin, 23 410 direction, 10 717 zero refusals; each at least 1 000), coordinates mostly in the domain and often on, or one raw unit either side of, each limit or at the i64 extremes: try_new equals the rule written in the test from the raw values, and new equals try_new(..).ok(). The seed and case are printed on failure."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-726"
requirements = ["TRACE-REQ-071", "TRACE-REQ-072"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_726_triangle_option_and_result_forms_agree_on_generated_inputs"
oracle = "200 000 triangles from seed 0xB002_0445_0000_07A0 (155 152 accepted; 16 299, 14 909 and 13 640 naming vertex 0, 1 and 2): try_new equals the rule written in the test, and new equals try_new(..).ok()."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-727"
requirements = ["TRACE-REQ-073"]
evidence = "test"
test = "crates/bunny-query/tests/trace_refusal_tests.rs::trace_tp_727_trace_refusals_display_their_raw_values_and_are_errors"
oracle = "The exact message of each TraceRayError and TraceTriangleError variant, written in the test with its raw values; each is a core::error::Error with no source."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-728"
requirements = ["TRACE-REQ-073"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_error_display_tests.rs::trace_tp_728_traversal_errors_display_their_condition_and_are_errors"
oracle = "The exact message of each TraversalError variant, written in the test; each is a core::error::Error with no source."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-729"
requirements = ["TRACE-REQ-073"]
evidence = "test"
test = "crates/bunny-broadphase/tests/traversal_error_display_tests.rs::trace_tp_729_compact_errors_display_their_condition_and_are_errors"
oracle = "The exact message of each CompactError variant, written in the test; each is a core::error::Error with no source."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-730"
requirements = ["TRACE-REQ-070", "TRACE-REQ-071", "TRACE-REQ-072"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#typed-refusal-mutation-checks"
oracle = "Each rule of the try_ forms broken on purpose, and the fast path's direction bound broken alone, makes a case above fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a container copy of the source, and
the trace tests were run (`cargo test --locked -p bunny-query --test
trace_tests`). Every mutation made at least one case fail; the restored source
passed all of them.

| Mutation in `trace.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Slab lower bound rounded up: `scaled(near, reciprocal + 1) + 1` | The box bound is a lower bound | TRACE-TP-007, TRACE-TP-008, TRACE-TP-010 |
| Slab upper bound rounded down: `scaled(far, reciprocal)` | The box test never rejects a reached box | TRACE-TP-010 |
| `v < 0` made `v <= 0` | Edge rule: the `v0`-`v1` edge is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |
| `0..=det` made `1..=det` for `u` | Edge rule: the `v0`-`v2` edge is inclusive | TRACE-TP-001, TRACE-TP-004 |
| `0..=det` made `0..det` for `u` | Edge rule: the corner `v1` is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |
| `u + v > det` made `u + v >= det` | Edge rule: the `v1`-`v2` edge is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |

The upper-bound mutation is caught only by the corner-graze golden case in
`TRACE-TP-010`: seeded rays almost never meet a box with exact entry equal to
exact exit. That case was added for this reason.

The overflow path of `ray_intersects_aabb` (`crates/bunny-query/src/ray/aabb_wide.rs`)
was checked the same way, running `ray_aabb_overflow_tests`, `ray_aabb_oracle_tests`
and `query_tests`:
| Mutation in `ray/aabb_wide.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Magnitude truncated instead of rounded to nearest | Bounds are correctly rounded | TRACE-TP-013, TRACE-TP-014 |
| Hit decided from the rounded, saturated bounds instead of exactly | Saturation never creates a hit | TRACE-TP-013, TRACE-TP-014, TRACE-TP-022 |
| Coordinate difference saturated to `i64` before the division | Differences are widened | TRACE-TP-014, TRACE-TP-021 |
| `t_exit >= 0` check dropped | Boxes behind the origin miss | TRACE-TP-013, TRACE-TP-014 |
Ties to even versus ties away from zero is not observable through the public
API: a quotient `diff * 2^32 / d` is an exact half only when `d` is a multiple
of `2^33`, and a unit direction component is at most `2^32 + 1` raw. Replacing
the tie rule therefore passes every case; `TRACE-TP-015` pins the oracle's tie
rule to `checked_div` directly instead;
tests were run (`cargo test --locked -p bunny-broadphase --test
closest_hit_tests`). Every mutation made at least one case fail; the restored
source passed all nine.

| Mutation in `closest_hit.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| `enter <= self.limit` made `enter < self.limit` in `pop` | A subtree whose bound equals the best distance is searched | TRACE-TP-108 |
| `candidate < (best.distance, best.primitive)` made `candidate.0 < best.distance` | Ties go to the lowest index | TRACE-TP-103, TRACE-TP-106, TRACE-TP-108 |
| Nearer child pushed first, farther child on top | The nearer child is searched first | TRACE-TP-107, TRACE-TP-108 |
| The stop at the first accepted hit removed from `any_hit` | `any_hit` stops at the first hit | TRACE-TP-109 |

The non-strict skip changes an answer only when a tie sits exactly on a
subtree's entry bound, the wrong push order changes no answer at all (pruning
stays sound in either order; the search only does more work), and without the
stop `any_hit` still returns the right value. All three survived the
brute-force cases `TRACE-TP-101` to `TRACE-TP-105`. `TRACE-TP-107`,
`TRACE-TP-108` and `TRACE-TP-109` were added to catch them, by checking which
primitives the search tests as well as what it returns.

The pre-check tried in flyingrobots/bunny#214 (reject when
`t >= (t_max + 1) * ceil(det / 2^32)`, not shipped) was mutated the same way, running
`bunny-query`, `bunny-broadphase` and `bunny-perf` tests: `t_max` for `t_max + 1`,
the floor of `det / 2^32` for its ceiling, `t + 1 >= bound`, and the ceiling computed with
`2^32 - 2`. Each made at least one of `TRACE-TP-201` to `TRACE-TP-205` fail; the last two
survived until `TRACE-TP-204` was added. The table is in the performance chapter's entry 6.

The compact layout (`compact.rs`, `compact_hit.rs`) was mutated the same way,
running `cargo test --locked -p bunny-broadphase --test compact_tests`, each
mutant under a 30-second wall-clock limit so a search that never ends counts
as caught instead of holding the machine. Every mutant was caught; the
restored source passed all seven cases.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Nearer child pushed first, farther child on top | The nearer child is searched first | TRACE-TP-301, TRACE-TP-302, TRACE-TP-305, TRACE-TP-306 |
| `enter <= self.limit` made `enter < self.limit` in `pop` | A subtree whose bound equals the best distance is searched | TRACE-TP-306 |
| Tie sends the right child first (`r <= l`) | Ties go to the left child, as in `closest_hit` | TRACE-TP-301, TRACE-TP-302, TRACE-TP-305, TRACE-TP-306 |
| The stop at the first accepted hit removed | `compact_any_hit` stops at the first hit | TRACE-TP-301, TRACE-TP-302, TRACE-TP-303, TRACE-TP-306 |
| The farther child dropped instead of pushed | A reachable subtree is never pruned without a bound | TRACE-TP-301, TRACE-TP-302, TRACE-TP-305, TRACE-TP-306 |
| The interior-visit bound removed | A reference cycle is an error, not an endless loop | TRACE-TP-305 (did not terminate) |
| Conversion writes the right subtree first | Depth-first, left-first order | TRACE-TP-304, TRACE-TP-307 |
| Conversion links the wrong child slot | Each reference names its own child | TRACE-TP-301, TRACE-TP-302, TRACE-TP-307 |

The first run of this table had no wall-clock limit: the wrong-slot mutant
produced a cyclic compact tree, the search looped, and the run had to be
killed. That exposed the missing guard, now the interior-visit bound, and the
dropped-child mutant first failed to compile rather than run; TRACE-TP-306
gained a case where the nearer leaf's box is entered but its triangle missed.

The interior-visit bound (flyingrobots/bunny#269) was mutated in a container
checkout of the final source. Each mutant was built once, then each test was run as
`timeout 120 cargo test --locked -q -p bunny-broadphase ...`, so a search that
never ends is recorded as caught by exit status 124. The loop of `traverse` holds
the bound for `intersect_aabb` and `intersect_ray`, the loop of `Search::run` for
`closest_hit` and `any_hit`, so removing one bound removes it from two functions.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Bound removed from `traverse` (`intersect_aabb`, `intersect_ray`) | A cycle is an error, not an endless loop | TRACE-TP-403, TRACE-TP-404 (did not terminate: 124), TRACE-TP-405 |
| Bound removed from `Search` (`closest_hit`, `any_hit`) | The same | TRACE-TP-401, TRACE-TP-402 (did not terminate: 124), TRACE-TP-405 |
| `traverse` budget one smaller (`(n / 2).saturating_sub(1)`) | A tree never reaches the bound | existing: `bvh_tests::test_bvh_build_and_traverse`, `bvh_allocation_tests`, `bvh_sah_tests` TP-009 and TP-010; new: TRACE-TP-403 to 406 |
| `Search` budget one smaller | The same | existing: TRACE-TP-107, 108, 109, 301 and the `closest_hit` doctest; new: TRACE-TP-405, 406 |
| Both budgets one larger (`n / 2 + 1`) | The bound is tight | TRACE-TP-403, 404, 405 |
| `traverse` spends the visit before `visit_matching_node` pushes the children | A full stack still reports `StackOverflow` first | TRACE-TP-406 (the truncated comb) |

The rewritten box test (flyingrobots/bunny#258) was checked the same way in a container
copy of the final source, running `box_entry_equivalence_tests`, `trace_tests`,
`closest_hit_tests` and the `bunny-perf` scene test `TRACE-TP-311`. Every mutation made at
least one case fail; the restored source passed every `bunny-query` and `bunny-broadphase`
test.

| Mutation in `trace.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Near and far planes swapped in `offsets` | Near and far selection by direction sign | TRACE-TP-308 to 304, TRACE-TP-005, 007, 008, 010, TRACE-TP-101 to 109 |
| `far >= origin` made `far > origin` | A far plane through the origin is reached | TRACE-TP-308 to 303, TRACE-TP-010 |
| Parallel-axis near check disabled (`direction == 0` made `direction == i64::MIN`) | A parallel ray outside the slab misses | TRACE-TP-308 to 303 |
| `!` not applied to the origin in `offsets` | The origin is flipped with the planes for a negative direction | TRACE-TP-308 to 304, TRACE-TP-005, 007, 008, 010, TRACE-TP-101 to 109 |
| Upper multiplier `r + 1` made `r` | The upper bound rounds up | TRACE-TP-309, 303 |
| Upper bound step `+ 1` made `+ 0` | The upper bound rounds up | TRACE-TP-309, 303 |
| Parallel-axis upper bound not `u64::MAX` | A parallel axis does not limit the exit | TRACE-TP-308 to 303, TRACE-TP-007, 008, 010, TRACE-TP-101 to 109 |
| `saturating_add` made `wrapping_add` in the upper bound | Saturation | TRACE-TP-308 to 303, TRACE-TP-007, 008, 010, TRACE-TP-101 to 109 |
| `enter <= limit` made `enter < limit` | The distance limit is inclusive | TRACE-TP-308 to 304, TRACE-TP-008 |
| `limit >= 0` dropped | A negative limit rejects | TRACE-TP-308 to 304, TRACE-TP-008 |
| The z-axis flag dropped | Every axis can reject | TRACE-TP-308 to 303, TRACE-TP-008 |
| `enter <= exit` made `enter < exit` | Touching contact is reached | TRACE-TP-309, 303 |
| Lower-bound saturation replaced by truncation | Saturation | TRACE-TP-308 to 303, TRACE-TP-010 |
| Near offset not clamped at zero behind the origin | An origin inside the slab enters at 0 | TRACE-TP-308 to 304, TRACE-TP-007, 008, 010, TRACE-TP-101 to 109 |

Three of these (`r + 1` made `r`, the `+ 0` step and the strict exit) pass every
case that existed before #258; only the equivalence corpora `TRACE-TP-309` and
`TRACE-TP-310` catch them. The first run of the parallel-axis mutation deleted the check
outright, which left an unused-parentheses lint that `deny(warnings)` turned into a build
error, so no test ran; it was redone as the comparison change above.

### Depth Mutation Checks

Each stack capacity below was changed in a container copy of
`crates/bunny-broadphase/src/`, every source file touched before the build,
and `timeout 120 cargo test --locked -q -p bunny-broadphase --no-fail-fast`
run per mutant.

The restored source passed every test of the crate.

| Mutation | Failing cases |
| --- | --- |
| `traversal.rs` stack of 65 entries | TRACE-TP-412 (depth 64 no longer overflows); also TRACE-TP-406 (the frozen copies keep 64) |
| `traversal.rs` stack of 63 entries | TRACE-TP-412 (depth 63 overflows) |
| `closest_hit.rs` stack of 65 or 63 entries | TRACE-TP-412 |
| `compact_hit.rs` stack of 65 or 63 entries | TRACE-TP-412 |
| `compact.rs` queue of 65 entries | TRACE-TP-413 (depth 65 converts) |
| `compact.rs` queue of 63 entries | TRACE-TP-413 (depth 64 is `TooDeep`) |
| Box queries' capacity check `>` for `>=` | None: an equivalent mutant. The bounded `get_mut` that follows returns `StackOverflow` at the same push. |

Before TRACE-TP-412 and TRACE-TP-413, the only depth cases were a 70-level
comb, a truncated 64-level comb (TRACE-TP-406, TRACE-TP-305) and `comb(60)`
and `comb(70)` in TRACE-TP-304, none at either boundary.

### Edge-Oracle Mutation Checks

Each rule below was broken in a container copy of `trace.rs`, the sources were touched, and
`timeout 120 cargo test --locked --no-fail-fast` was run. "Before" is `main` without
`TRACE-TP-315`, running the `bunny-query`, `bunny-broadphase` and `bunny-optics` tests; "after"
is this plan, running the `bunny-query` tests. The restored source passes both.

| Mutation in `hit_triangle` | Before: failing cases | After: failing cases |
| --- | --- | --- |
| All three edges relaxed by one unit (`-1..=det`, `v < -1`, `u + v > det + 1`) | TRACE-TP-201 only (the frozen copy) | TRACE-TP-315, TRACE-TP-201 |
| `0..=det` made `1..=det` for `u` | TRACE-TP-001, 004, 201, 204 | adds TRACE-TP-315 |
| `0..=det` made `0..det` for `u` | TRACE-TP-001, 004, 009, 201 | adds TRACE-TP-315 |
| `v < 0` made `v <= 0` | TRACE-TP-001, 004, 009, 201, 204; OPTICS-TP-046, 060, 079 | adds TRACE-TP-315 |
| `u + v > det` made `u + v >= det` | TRACE-TP-001, 004, 009, 201, 203 | adds TRACE-TP-315, TRACE-TP-005 |
| Distance rounded to nearest instead of floored | TRACE-TP-201, 204; OPTICS-TP-046 | adds TRACE-TP-315 |
| `front_face` inverted | TRACE-TP-001, 003, 009, 201 to 204; 12 optics cases | adds TRACE-TP-315, TRACE-TP-005 |

Before `TRACE-TP-315`, the relaxed edges and the rounded distance were caught only by tests
that compare the kernel with its own frozen copy (and, for the distance, one optics golden
case).

### Counting Mutation Checks

Each counting rule (flyingrobots/bunny#335) was broken in a container copy of the source, the
sources were touched, the tests built once, and
`timeout 120 cargo test --locked -q -p bunny-broadphase --test counted_traversal_tests` and
`--test compact_tests` run. The restored source passes every `bunny-broadphase` and `bunny-perf`
test.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| `box_tests` not incremented in the tally | Every box test is counted | TRACE-TP-415, 416 |
| `box_entries` incremented for a rejected box | A culled box is not an entry | TRACE-TP-415, 416 |
| A skipped pop counted as an interior visit | A culled entry is not a visit | TRACE-TP-415, 416 |
| `leaf()` called twice per leaf | A leaf is counted once | TRACE-TP-415, 416 |
| The last primitive test of each leaf not counted | Every call of `hit` is counted | TRACE-TP-415, 416 |
| `peak_stack` never updated | The deepest stack is recorded | TRACE-TP-415, 416 |
| `enter <= limit` made `enter < limit` for the counting tally only | The counted form prunes exactly as the uncounted one | TRACE-TP-415 (the tie tree) |

The last mutation changes only the counted form, so `compact_tests` (which runs the uncounted
functions) passes it. An entry bound equal to the limit at its pop needs a box bound exactly at
a hit distance, which the generated corpora of TRACE-TP-416 and 417 did not produce even with
duplicated triangles; the constructed tie tree in TRACE-TP-415 catches it. Its first run had no
tie tree and the mutant survived; the tie tree was added for that reason. Two mutants were first
written by deleting a tally call, which left a method unused and failed to compile under
`deny(warnings)`; they were redone as the changed bodies above.

### Sphere Mutation Checks

Each rule below was broken in a container copy of `trace/sphere.rs` or `trace/wide.rs`
(flyingrobots/bunny#358) and `cargo test --locked -q -p bunny-query --test trace_sphere_tests`
run. The restored source passes.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| The near root's `- 1` for a non-square `E` removed | The near root is the floor of an irrational value | TRACE-TP-501 |
| `front_face` true at the near root made false | Facing | TRACE-TP-501 to 504 |
| `radius <= 0` made `radius < 0` | A zero radius is refused | TRACE-TP-505 |
| `isqrt` starts its low bits at bit 62 instead of 63 | The square root is exact above `2^128` | two cases, TRACE-TP-501 among them |

The normal and the range limits (flyingrobots/bunny#422) were checked the same
way, in the working tree with the source restored after each run:

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| The old normal: the code at the merge of flyingrobots/bunny#375, the new tests run against it with `.into_inner()` and TRACE-TP-706 removed | Unit normal at the exact root | TRACE-TP-501, 503, 506, 701, 702, 703 (all seven cases), 705 |
| `t_min..=t_max` made `t_min..t_max` | The upper limit is inclusive | TRACE-TP-501, TRACE-TP-704 |
| The offset at the floored distance passed to `FixedUnitVec3::new` | The normal is taken at the exact root | TRACE-TP-501, 506, 701, 702, 703, 705 |
| The near and far roots swapped in the normal | Outward normal at the reported root | TRACE-TP-501 to 504, 506, 701 to 705 |
| The prescale made `[2^60, 2^61)` | none: every one of the 1 901 hits of the base corpus of TRACE-TP-501 and the TRACE-TP-705 normals are bit-identical under it, so no case can tell the two apart | not detected (equivalent on the corpus) |

### Cylinder Mutation Checks

Each rule below was broken in a container copy of `trace/cylinder.rs` or
`trace/cylinder/span.rs` (flyingrobots/bunny#360), the sources touched so that cargo
rebuilt them, and `cargo test --locked -q -p bunny-query --test trace_cylinder_tests`
run. The restored source passes.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| The side wins a tie at the entry | Tie rule at the rim | TRACE-TP-540, TRACE-TP-544 |
| The side wins a tie at the exit | Tie rule at the rim | TRACE-TP-540, TRACE-TP-544 |
| A single touching point at a rim reports the side | Tie rule at the rim | TRACE-TP-540, TRACE-TP-544 |
| Cap against the near root, negative `x`: the order reversed | Exact comparison | TRACE-TP-540, TRACE-TP-544, TRACE-TP-548 |
| Cap against the far root, nonnegative `x`: the order reversed | Exact comparison | TRACE-TP-540, TRACE-TP-548 |
| `bottom..=top` made `bottom..top` for a ray parallel to the caps | A ray in a cap's plane is inside the closed slab | TRACE-TP-540, TRACE-TP-546, TRACE-TP-548 |
| `<= r * r` made `<` for a ray parallel to the axis | A ray along the side is inside the closed cylinder | TRACE-TP-545 |
| An entry after the exit still hits | Miss when the intervals do not overlap | TRACE-TP-540, TRACE-TP-548 |
| The caps' order of travel swapped | Entry and exit caps | TRACE-TP-540, TRACE-TP-542, TRACE-TP-544, TRACE-TP-545, TRACE-TP-548 |
| `front_face` false at the entry | Facing | TRACE-TP-540, TRACE-TP-541, TRACE-TP-543 to TRACE-TP-546, TRACE-TP-548, TRACE-TP-550, TRACE-TP-552 |
| The upper distance limit made exclusive | Inclusive limits | TRACE-TP-550 |
| The side normal taken at the other root | Normal at the exact hit | TRACE-TP-540 to TRACE-TP-544, TRACE-TP-546, TRACE-TP-548, TRACE-TP-550, TRACE-TP-552 |
| The side normal not scaled before its square root (`s = 0`) | Unit normal for every radius | TRACE-TP-540, TRACE-TP-552 |
| `bottom >= top` made `bottom > top` | Equal caps are refused | TRACE-TP-547 |
| `radius <= 0` made `radius < 0` | A zero radius is refused | TRACE-TP-547 |

The generated case reaches the axis-parallel mutation only through TRACE-TP-545: a
generated axis-parallel ray lies exactly on the side with negligible probability.

### Box Mutation Checks

Each rule below was broken in a copy of `trace/box_hit.rs` (flyingrobots/bunny#359) in the
container, on the final source, and `cargo test --locked -q -p bunny-query --test trace_box_tests` run. The restored
source passes all eleven tests.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Entry replaced on `!= Less` instead of `== Greater` | The lowest axis wins an entry tie | TRACE-TP-509, 513, 516 |
| Exit replaced on `!= Greater` instead of `== Less` | The lowest axis wins an exit tie | TRACE-TP-509, 513, 516 |
| Zero-direction test made `low >= 0 \|\| high <= 0` | A face plane belongs to the box | TRACE-TP-509, 510, 511, 514, 516 |
| Entry normal sign not negated | The entry face points against the ray | TRACE-TP-509 to 514, 516 |
| Crossings compared by their floors | Ties are of true distances, not rounded ones | TRACE-TP-513 only |
| Hit test `entry < exit` instead of `<=` | Touching is a hit | TRACE-TP-509, 510, 514, 516 |
| Negative-direction slab ends swapped | A negative direction enters at the maximum face | TRACE-TP-509, 510, 511, 513, 516 |
| Floor replaced by truncation toward zero | The distance is rounded down | TRACE-TP-509 only |
| `front_face` true at the entry made false | Facing | TRACE-TP-509 to 514, 516 |
| `t_max` made exclusive (`..` for `..=`) | The upper limit is inclusive | TRACE-TP-519 only |
| Entry skipped when it equals `t_min` | The lower limit is inclusive | TRACE-TP-509, 511, 516, 519 |

The generated cases never put a distance on `t_max` (a reviewer counted none in 8 000), so the upper limit rests on TRACE-TP-519 alone. Three mutants were caught by one test each: comparing floors by TRACE-TP-513, whose pair of faces
was built for it (the generated rays did not reach a tie of floors without a tie of crossings),
truncation by the generated oracle, which reaches negative non-integer distances, and an exclusive `t_max` by TRACE-TP-519.

### Disc Mutation Checks

Each rule below was broken in a container copy of `trace/disc.rs` (flyingrobots/bunny#363) and
`cargo test --locked -q -p bunny-query --test trace_disc_tests` run. The restored source passes
all ten tests.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| `dz == 0` made `dz == 7` | A ray parallel to the plane misses | TRACE-TP-560, 563 |
| `outside <= reach` made `outside < reach` | The rim is inside | TRACE-TP-560 to 562, 564 |
| `front_face: dz < 0` made `dz > 0` | Facing | TRACE-TP-560 to 565, 567, 569 |
| `div_euclid` made truncating division | The distance is the floor | TRACE-TP-560, 565 |
| The radius test made on the point at the rounded distance (floor per coordinate) instead of the exact meeting point | The test is on the exact hit point | TRACE-TP-560, 562 |
| `o * dz - d * f` made `o * dz + d * f` | The hit point is the exact meeting point | TRACE-TP-560, 561, 567 |
| `r * dz` made `r * 2^32` | The radius test is scaled by the same `d.z` | TRACE-TP-560 to 562, 567 |
| `radius <= 0` made `radius < 0` | A zero radius is refused | TRACE-TP-566 |
| `t_min` made exclusive | The limits are inclusive | TRACE-TP-560, 564, 565, 569 |
| `t_max` made exclusive (`..=` made `..`) | The upper limit is inclusive | TRACE-TP-569 only (generated ranges never end exactly on a hit) |

### Typed Refusal Mutation Checks

Each rule below was broken in the working tree (flyingrobots/bunny#445),
`cargo test --locked -q --no-fail-fast -p bunny-query --test trace_refusal_tests`
run, and the file restored from git; the restored source passes all eight tests.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| `try_new` accepts an origin coordinate of `2^40` | The domain bound is exclusive | TRACE-TP-720, 723, 725 |
| `try_new` refuses a direction component of exactly `2^32` | The direction bound is inclusive | TRACE-TP-720, 721, 725 |
| `try_new` drops the zero-direction check | A zero direction is refused | TRACE-TP-722, 725 |
| `try_new` checks the direction before the origin | Check order | TRACE-TP-723, 725 |
| `TraceTriangle::try_new` names the last vertex outside | The first vertex is named | TRACE-TP-724, 726 |
| `TraceTriangle::try_new` ignores the third vertex | Every vertex is checked | TRACE-TP-724, 726 |
| The fast path `TraceRay::new` (through `prepare`) refuses a direction component of exactly `2^32`, `try_new` unchanged | The two paths agree | TRACE-TP-720, 721, 725 |

## Determinism Obligations And Evidence

The kernels, the traversals and their tests use only integer arithmetic: `i64`, `u64`, `i128`
and `u128` additions, multiplications, shifts and divisions with defined
results. There is no floating point, randomness from the host, time,
filesystem access, locale-sensitive formatting, parallel scheduling, map
iteration, stdout, stderr or logging in any oracle.

Test builds check integer overflow, so `TRACE-TP-005` turns any overflow in
the domain corners into a test failure.

## Known Failures

The current executable surface has no known failing cases.

## Edge Cases And Unusual Inputs

Current tests cover:

- origin and vertex coordinates one raw unit inside and outside the domain;
- direction components at 1.0, above 1.0, one raw unit, and zero;
- axis-parallel and nearly axis-parallel directions against boxes;
- an origin on a box face and inside a box;
- boxes behind the ray and a negative distance limit;
- a ray grazing a box edge and a box corner;
- hits on a triangle corner and on a shared edge;
- back-face hits;
- distances and weights that are not exact in Q32.32;
- collinear and duplicate-vertex (zero-area) triangles;
- a ray lying in the triangle's plane;
- a crossing whose distance does not fit `i64`;
- equal-distance hits in one leaf and in sibling subtrees, including a sibling
  whose entry bound equals the best distance exactly;
- trees of different shapes over the same primitives, including a single leaf;
- empty trees, dangling child indices and leaf ranges past the primitive
  buffer;
- cyclic buffers (a node that is its own left or right child, a three-node
  cycle) and shared subtrees within and beyond the interior-visit bound;
- primitive tests that report distances beyond the limit or below zero.
- for `box_entry` against its retained reference: box coordinates at and near
  `i64::MIN` and `i64::MAX`, inverted boxes, faces through the origin,
  direction components of 1 raw unit, -1, 0 and `2^32`, and limits of
  `i64::MIN`, -1, 0 and exactly the entry bound;
- for `ray_intersects_aabb`: direction components of 1 to 4 000 raw units,
  coordinate differences past `i64`, slab distances past Q32.32 on one or both
  sides, two saturated entries, an origin on a face at the edge of the range,
  inverted boxes (`min > max` on an axis) and grazing contacts whose rounded
  bounds tie.

Future traversal work must add cases for ties between equal-distance hits.

## Stress And Fuzz

No fuzz target exists today. `TRACE-TP-001`, `TRACE-TP-002`, `TRACE-TP-004`
and `TRACE-TP-007` are seeded property cases over 4 000 to 40 000 inputs each.
`TRACE-TP-013` and `TRACE-TP-014` are seeded property cases over 200 000 inputs
each (SplitMix64, seeds `0xB002_0211_0000_0001` and `0xB002_0211_0000_0002`);
their failure messages name the seed and the case index.
and `TRACE-TP-007` are seeded property cases over 4 000 to 40 000 inputs each;
`TRACE-TP-101`, `TRACE-TP-102`, `TRACE-TP-103` and `TRACE-TP-106` are seeded
traversal cases over 1 500 to 3 000 rays each.

`TRACE-TP-309` (300 000 cases) and `TRACE-TP-310` (a 42 525-case sweep and 200 000 seeded
cases) compare `box_entry` with its retained reference at ten limits each; their failure
messages name the case index, the ray, the box and the limit.

`TRACE-TP-315` compares `hit_triangle` with the signed-volume oracle over 40 000 seeded
cases; its failure message names the seed, the case index, the ray and the triangle. It has
no shrinking.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Before/after timing of the kernels and the traversals. | Benchmark cases (flyingrobots/bunny#210). |
| A tree builder tuned for ray tracing; the traversals run on `build_bvh` trees today. | flyingrobots/bunny#213. |
| Closest-point-to-mesh queries and committed mesh fixture data. | Remaining scope of flyingrobots/bunny#123. |
| Canonical hit record, plane, segment, capsule and OBB intersections. | Remaining scope of flyingrobots/bunny#159. |
