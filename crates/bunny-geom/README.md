# bunny-geom

Geometry primitives for the Bunny graphics commons.

This crate provides basic bounding structures and shapes used in intersection
queries, collision detection, and ray-casting.

## Core Features

* **Ray3**: Represents a 3D ray with finite origin and direction components
  through `Ray3::try_new`.
* **Aabb3**: Represents Axis-Aligned Bounding Boxes (AABBs) with finite minimum
  and maximum bounds through `Aabb3::try_new`.
* **Sphere3**: Represents spheres with finite centers and finite non-negative
  radii through `Sphere3::try_new`.
* **Boundary conversions**: Float-to-fixed ingress is fallible and validating
  through `try_into_fixed`, `try_from_float`, and `TryFrom`; fixed-to-float
  egress is infallible through `into_float` and `From<Fixed*>`. Ingress rejects
  non-finite and finite out-of-range values before fixed-point conversion.
* **Safe & Portable**: Compiles under `#![deny(unsafe_code)]` with zero runtime
  dependencies.

## Planned Features

* Ray-Sphere, Ray-AABB, and Ray-Triangle intersection solvers.
* Swept volume collision checks.
* Stable bounding volume hierarchy (BVH) structures.

## License

Apache-2.0.
