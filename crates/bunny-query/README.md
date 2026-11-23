# bunny-query

Fixed-point ray-casting and closest-point query solvers for Bunny.

This crate evaluates spatial queries against Bunny geometry primitives using
the deterministic Q32.32 numeric profile. It is intended for repeatable
collision, picking, and contact-query calculations across native and WASM
targets.

## Features

* Ray intersections for spheres, AABBs, and triangles.
* Closest-point queries for AABBs, triangles, and segment pairs.
* AABB-sphere overlap with deterministic contact point output.
* Checked ray-intersection arithmetic: `ray_intersects_sphere` and
  `ray_intersects_triangle` return `None` when an intermediate Q32.32
  operation overflows instead of accepting saturated geometry as a hit.
* `ray_intersects_aabb` does not miss on overflow: a slab distance that does
  not fit Q32.32 saturates by sign, coordinate differences are taken in
  `i128`, and on that path the hit is decided exactly before the bounds are
  rounded (flyingrobots/bunny#211). See its rustdoc and
  `docs/topics/ray-tracing/README.md`.
* `PreparedRay3` and `ray_intersects_aabb_prepared`: the exact results of
  `ray_intersects_aabb` (overflow inputs included) with one prepared reciprocal
  per direction component, for many exact box tests per ray. Ray tracing that
  only needs an entry bound should use `TraceRay::box_entry`.
* Ray-tracing kernels (`TraceRay`, `TraceTriangle`): a division-free
  conservative ray-box test and an exact integer ray-triangle test with
  inclusive edges; see `docs/topics/ray-tracing/`.
* `TraceSphere` and `TraceRay::hit_sphere` (contract `bunny-ray-sphere/1`): the
  exact ray-sphere test, returning the rounded-down distance, facing and outward
  normal in the form of `TriangleHit`; inputs outside the trace domain are refused
  with `TraceSphereError`. See `docs/topics/ray-tracing/README.md#ray-sphere`.
* `TraceCylinder` and `TraceRay::hit_cylinder` (contract `bunny-ray-cylinder/1`,
  `RAY_CYLINDER_ID`):
  the exact test of a capped cylinder about the object-space `+Y` axis, returning
  the rounded-down distance, facing and outward unit normal in the form of
  `SphereHit`, a cap winning a tie with the side at the rim; inputs outside the
  trace domain are refused with `TraceCylinderError`. See
  `docs/topics/ray-tracing/README.md#ray-cylinder`.
* `TraceBox` and `TraceRay::hit_box` (contract `bunny-ray-box/1`): the exact ray-box
  test for an axis-aligned box, returning the rounded-down distance, facing and
  face normal (the lowest axis wins an edge or corner tie); inputs outside the trace
  domain are refused with `TraceBoxError`; identity constant `RAY_BOX_ID`. See `docs/topics/ray-tracing/README.md#ray-box`.
* `TraceDisc` and `TraceRay::hit_disc` (contract `bunny-ray-disc/1`, `RAY_DISC_ID`): the exact
  ray-disc test for a disc in the plane `z = height` about the `z` axis,
  returning the rounded-down distance and facing in a `DiscHit`; the rim is
  inside, a ray parallel to the plane misses, and inputs outside the trace
  domain are refused with `TraceDiscError`. See
  `docs/topics/ray-tracing/README.md#ray-disc`.
* `TriangleHit::barycentric_numerators`: the exact `u`, `v` and `det` the
  triangle test decided a hit with, as a `BarycentricNumerators` (also
  buildable from any point on a triangle with `BarycentricNumerators::new`).
* `ShapeFrame` (`bunny-shape-frame/1`): a rigid frame (orthogonal unit axes
  and an origin) that maps a `TraceRay` into a shape's object space and a
  normal back to world space, each component one exact dot product rounded
  once, ties to even, with typed refusals. A direction lengthened by the
  rotation is halved and the `ObjectRay` maps hit distances back to the
  world ray; see `docs/topics/shape-frame/`.
* Raw Q32.32 receipt tests for cross-target determinism.

## License

Apache-2.0.
