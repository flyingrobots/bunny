# Coordinate Law

This document defines Bunny's current coordinate-space contract.

When this document and implementation disagree, one of them is wrong. Usually
both need tests.

`docs/NUMERIC_CONSTITUTION.md` defines how Bunny represents and computes
canonical numbers. This document defines what those numbers mean when they are
coordinates, vectors, directions, normals, bounds, rays, transforms, cameras,
or viewport values.

## Status

This is a living topic chapter, not a historical design proposal.

Design notes, goalpost documents, issues, and pull requests may explain why a
choice was made. This file states the current contract. Any pull request that
changes Bunny's coordinate behavior must update this file and
[`test-plan.md`](test-plan.md) in the same change or explicitly state why the
coordinate law is unaffected.

## Scope

The coordinate law applies to canonical math and geometry inside Bunny core
crates:

- `bunny-linalg`
- `bunny-geom`
- `bunny-query`
- `bunny-broadphase`
- `bunny-mesh`
- `bunny-codec`
- future transform, projection, collision, visibility, optics, and SIMD crates

Boundary formats may use external conventions. When Bunny imports or exports an
external convention, the boundary API must name that convention and convert into
or out of Bunny's canonical convention explicitly.

## Canonical 3D Frame

Bunny's canonical 3D frame is right-handed:

- `+X` points right.
- `+Y` points up.
- `+Z` is `+X cross +Y`.

With the usual screen diagram where `+X` points right and `+Y` points up,
`+Z` points toward the viewer.

This is a geometry convention, not a camera convention. Future camera APIs may
define a default camera looking down `-Z`, but that belongs to camera and
projection contracts rather than the global geometry frame.

Required invariant:

```text
UNIT_X cross UNIT_Y = UNIT_Z
UNIT_Y cross UNIT_Z = UNIT_X
UNIT_Z cross UNIT_X = UNIT_Y
```

## Canonical 2D Plane

Bunny's default 2D plane is the `XY` plane embedded in the 3D frame.

Positive 2D winding is counter-clockwise when viewed from `+Z` toward the
origin. For a triangle `(a, b, c)` in the `XY` plane:

```text
(b - a) cross (c - a)
```

points toward `+Z` when `(a, b, c)` is counter-clockwise.

Clockwise winding points toward `-Z`.

## Units

Bunny's canonical coordinates are unitless fixed-point Bunny units.

`FixedQ32_32::ONE` means exactly one canonical unit. Downstream users may
interpret that unit as meters, centimeters, pixels, tiles, scene units, or
another project-specific scale. Bunny APIs must not silently assume a physical
scale.

Rules:

- Public Bunny APIs should say "units" unless the API explicitly imports an
  external physical or display convention.
- Conversion from domain-specific units belongs at the boundary.
- Algorithms must not hide scale-sensitive thresholds unless the threshold is
  documented as part of the API contract.

## Coordinate Spaces

Current Bunny geometry operates in one caller-owned Euclidean space unless an
API explicitly says otherwise.

Future APIs that move values between spaces must name transforms as:

```text
target_from_source
```

Examples:

- `world_from_local`
- `view_from_world`
- `clip_from_view`
- `mesh_from_quantized`

This naming makes the direction of conversion visible at the call site.

## Transform Convention

Future matrix and transform APIs must use column-vector semantics:

```text
p_target = target_from_source * p_source
```

Composition is right-to-left:

```text
world_from_mesh = world_from_actor * actor_from_mesh
p_world = world_from_mesh * p_mesh
```

The left transform consumes the output space of the right transform. Reversing
that order is a bug unless an API explicitly documents a different algebra.

## Value Kinds

Points, vectors, directions, rays, and normals are different concepts.

- Points have position and are affected by translation.
- Vectors represent displacement and are not affected by translation.
- Directions are vectors with a unit-length invariant.
- Rays pair a point origin with a direction.
- Normals are orientation covectors and may need inverse-transpose handling
  under non-uniform transforms when that transform API exists.

Current APIs do not yet type every distinction at the Rust type level. New APIs
should prefer types that make invalid mixing hard to express.

## Rotation and Angle Direction

Positive rotation follows the right-hand rule.

With the thumb pointing along the positive axis of rotation, positive angles
curl in the direction of the fingers.

The full deterministic angle and trigonometry profile is owned by future angle
work. That work must preserve this sign convention.

## Bounds and Boundary Contact

Axis-aligned bounds use component-wise minimum and maximum corners in the
current coordinate space.

Unless an API says otherwise:

- `min <= max` is required on every axis.
- Boundary contact is inclusive.
- Inverted bounds are invalid input.
- Empty bounds must be represented explicitly by an API that names emptiness.

Queries may define stricter behavior for degeneracy. Degeneracy policy belongs
with the shape or query API and must cite this coordinate law when the behavior
depends on orientation, winding, units, or boundary contact.

## Projection and NDC Reservations

Bunny does not yet expose canonical projection or viewport APIs. Future APIs
must use these reserved defaults unless their names explicitly say otherwise:

- Normalized device coordinate `x` range: `[-1, 1]`.
- Normalized device coordinate `y` range: `[-1, 1]`.
- Normalized device coordinate `z` range: `[0, 1]`.
- Positive NDC `y` points up.

Viewport conversion must explicitly name the viewport origin and `y` direction.
OpenGL, WebGPU, DirectX, image buffers, and UI systems disagree here; Bunny must
not hide that disagreement behind ambient defaults.

## External Format Boundaries

Mesh, scene, renderer, and file formats often carry their own conventions.

Boundary adapters must document:

- source handedness
- source up axis
- source unit scale, if known
- source winding order
- source depth range, if projection data is imported
- the exact conversion into Bunny's canonical convention

Lossy display or diagnostic conversion may use floats, but canonical geometry
truth remains fixed-point.

## Required Tests

The current repository must keep tests for conventions that existing code can
enforce:

- basis orientation: `+X cross +Y = +Z`
- cyclic right-handed basis products
- reversed cross product signs
- `XY` counter-clockwise winding produces a `+Z` normal

Future transform, projection, angle, and camera APIs must add tests for:

- `target_from_source` naming examples
- matrix composition order
- point versus vector translation behavior
- positive rotation direction
- NDC and viewport conversion examples

## Documentation Rule

This topic folder is the source of truth for current coordinate law.

Historical design documents must not become competing references. If a design
document is superseded, leave it as historical evidence and point readers here
for the current contract.
