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
* Raw Q32.32 receipt tests for cross-target determinism.

## License

Apache-2.0.
