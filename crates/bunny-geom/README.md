# bunny-geom

Geometry primitives for the Bunny graphics commons.

This crate provides basic bounding structures and shapes used in intersection queries, collision detection, and ray-casting.

## Core Features

* **Ray3**: Representing a 3D ray with a finite origin and direction.
* **Aabb3**: Bounding volumes represented as Axis-Aligned Bounding Boxes (AABBs) with minimum and maximum bounds.
* **Sphere3**: Spherical volumes defined by a center point and a scalar radius.
* **Safe & Portable**: Compiles under `#![deny(unsafe_code)]` with zero runtime dependencies.

## Planned Features

* Ray-Sphere, Ray-AABB, and Ray-Triangle intersection solvers.
* Swept volume collision checks.
* stable bounding volume hierarchy (BVH) structures.

## License

Apache-2.0.
