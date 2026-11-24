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
* **FixedFrustum**: Six planes facing into the frustum; `contains_point`
  gives `Option<bool>`, and `classify_aabb` (`Option<FrustumClass>`) and
  `classify_sphere` (`Result<FrustumClass, FrustumSphereError>`) give
  `Inside`, `Intersecting` or `Outside`, with `Outside` certain and `Intersecting`
  conservative, and `from_clip_matrix` extracts the planes of a clip matrix
  exactly for Bunny's `[0, 1]` depth range; see `docs/topics/frustum/README.md`.
* **Exact predicates** (`bunny-predicates/1`): `orientation2` and
  `orientation3` return the exact sign of the 2D and 3D orientation
  determinants for every representable input; `doubled_signed_area2` and
  `sextupled_signed_volume3` return the exact values, and the `_rounded` forms
  round them once to Q32.32. `barycentric2` and `barycentric3_coplanar` return
  exact barycentric numerators with an exact inside, on-edge, on-vertex or
  outside classification; see `docs/topics/predicates/README.md`.
* **2D primitives** (`bunny-planar/1`): `FixedSegment2`, `FixedRay2`,
  `FixedAabb2` and `FixedCircle2` with the 3D types' constructors and errors,
  and exact closed-boundary queries for every representable input: box
  containment, overlap and closest point, circle containment and overlap with
  a circle or a box, segment containment and intersection from the exact
  `orientation2`, and the closest point on a segment rounded once per axis,
  ties to even; see `docs/topics/planar-geometry/README.md`.
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

* `bunny-query` owns the 3D ray, triangle, closest-point, and overlap solvers;
  the 2D queries of `bunny-planar/1` are methods on the 2D types here.
* `bunny-broadphase` owns BVH and sweep-and-prune acceleration structures.
* Future collision, swept-volume, and richer shape coverage is tracked in
  `ROADMAP.md` rather than claimed as current `bunny-geom` behavior.

## License

Apache-2.0.
