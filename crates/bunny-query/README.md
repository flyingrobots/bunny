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
* Ray-tracing kernels (`TraceRay`, `TraceTriangle`): a division-free
  conservative ray-box test and an exact integer ray-triangle test with
  inclusive edges; see `docs/topics/ray-tracing/`.
* Raw Q32.32 receipt tests for cross-target determinism.

## License

Apache-2.0.
