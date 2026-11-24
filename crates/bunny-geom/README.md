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
* **Bounds helpers** (`bunny-bounds/1`): `FixedAabb3::from_points`,
  `from_triangle`, `merge`, `expand`, `expand_axes`, `from_sphere` and
  `bounding_sphere`, exact on raw values, with typed `BoundsError` refusals for
  no points, inverted boxes, inverting shrinks, negative radii and the raw range;
  the bounding sphere's radius is rounded outward so it contains every corner.
  See `docs/topics/bounding-volumes/README.md`.
* **Ray and box transforms**: `FixedRay3::checked_transform` maps a ray by a
  `FixedAffine3` into a `TransformedRay3` whose direction is not renormalised,
  so a parameter `t` keeps its meaning: each component of the point at `t` on
  the transformed ray is within `3/2 (1 + |t|)` raw of the exact image of the
  point at `t` on the original. That bounds the position, not `t`: near a
  grazing hit a small change in position can move or remove the hit;
  `FixedAabb3::checked_transform` returns the hull of the eight mapped
  corners. Typed `TransformError` refusals for inverted boxes, overflow and a
  zero direction; see `docs/topics/transform/README.md#rays-boxes-and-normals`.
* **FixedLine3 / FixedTriangle3 / FixedCapsule3 / FixedObb3**: the infinite
  line (the ray's unit-direction rule), the triangle with an exact degeneracy
  test over the full raw range, the capsule (segment and non-negative radius)
  and the oriented box (centre, axis columns within the rotation contracts'
  length and orthogonality tolerances, non-negative half extents), refusing
  with `ShapeError`; each, and the segment and plane, has a `try_from_float`.
  See `docs/topics/shape-library/README.md`.
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

## Examples

| Command | Shows |
| --- | --- |
| `cargo run -p bunny-geom --example float_boundary` | A float ray and box converted into `FixedRay3` and `FixedAabb3` with their exact raw values, and one typed `GeomError` refusal each for NaN, an infinity, a value outside Q32.32, a zero ray direction and an inverted float and fixed box. |

## License

Apache-2.0.
