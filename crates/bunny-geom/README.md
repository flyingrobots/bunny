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
* **FixedPlane3 / FixedSegment3**: Checked plane classification of points,
  segments, triangles and AABBs (`PlaneSpan`), segment clipping and convex
  polygon clipping (`clip_polygon`, into a caller buffer) to the closed front
  half-space; see `docs/topics/plane-clipping/README.md`.
* **Exact predicates** (`bunny-predicates/1`): `orientation2` and
  `orientation3` return the exact sign of the 2D and 3D orientation
  determinants for every representable input; `doubled_signed_area2` and
  `sextupled_signed_volume3` return the exact values, and the `_rounded` forms
  round them once to Q32.32. `barycentric2` and `barycentric3_coplanar` return
  exact barycentric numerators with an exact inside, on-edge, on-vertex or
  outside classification; see `docs/topics/predicates/README.md`.
* **Boundary conversions**: Float-to-fixed ingress is fallible and validating
  through `try_into_fixed`, `try_from_float`, and `TryFrom`; fixed-to-float
  egress is infallible through `into_float` and `From<Fixed*>`. Ingress rejects
  non-finite and finite out-of-range values before fixed-point conversion.
* **Safe & Portable**: Compiles under `#![deny(unsafe_code)]` with zero runtime
  dependencies.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | on | Links `std` and turns on the `std` feature of `bunny-linalg` and `bunny-num`. Off, the crate is `no_std` and needs only `core` (no `alloc`); no public item is removed. |

```toml
[dependencies]
bunny-geom = { version = "0.7", default-features = false }
```

See [`docs/topics/no-std/`](../../docs/topics/no-std/README.md) for the gate
that builds it for `thumbv7em-none-eabihf`, a target with no `std`.

## Related Crates

* `bunny-query` owns ray, triangle, closest-point, and overlap solvers.
* `bunny-broadphase` owns BVH and sweep-and-prune acceleration structures.
* Future collision, swept-volume, and richer shape coverage is tracked in
  `ROADMAP.md` rather than claimed as current `bunny-geom` behavior.

## License

Apache-2.0.
