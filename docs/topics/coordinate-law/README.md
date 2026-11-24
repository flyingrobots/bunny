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

`FixedMat3::rotation_x`, `rotation_y`, `rotation_z` and `rotation_axis_angle`
follow it: a positive quarter turn about a unit axis `a` maps each perpendicular
`v` onto `a x v` (see [Rotation](../rotation/README.md)).

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

## Projection and NDC

`FixedMat4::try_look_at` builds the matching `view_from_world` matrix: rows
right, up and `-forward` of the right-handed frame below, so the view axis maps
to `-Z`.

`FixedMat4::try_orthographic` and `FixedMat4::try_perspective` in
`bunny-linalg` are the canonical projection builders. Each returns a
`clip_from_view` matrix that follows the transform convention above
(`p_clip = clip_from_view * p_view`, column vectors).

Canonical normalized device coordinates (NDC):

- `x` range: `[-1, 1]`.
- `y` range: `[-1, 1]`.
- `z` (depth) range: `[0, 1]`.
- Positive NDC `y` points up.

View space is the right-handed canonical frame with the camera at the origin
looking down `-Z`. A point at distance `d` in front of the camera has
`z_view = -d`, `+X` is to the camera's right, and `+Y` is up. `near` and `far`
are positive distances in front of the camera, with `near < far`.

Depth range choice: the near plane maps to NDC `z = 0` and the far plane to
`z = 1`. Reason: the coordinate law already reserved `[0, 1]` for NDC depth
(it is the WebGPU and Direct3D range and keeps fixed-point depth precision
non-negative), and the right-handed frame with a `-Z` camera requires no
handedness flip. The `[-1, 1]` OpenGL range is not canonical; a boundary
adapter that needs it must convert explicitly (`z_gl = 2 * z - 1`).

Orthographic matrix (`half_width`, `half_height`, `near`, `far`, with
`depth = far - near`):

```text
| 1/half_width  0              0            0           |
| 0             1/half_height  0            0           |
| 0             0              -1/depth     -near/depth |
| 0             0              0            1           |
```

Perspective matrix (`tan_half_fov_y`, `aspect = width / height`):

```text
| 1/(aspect*tan)  0        0              0                  |
| 0               1/tan    0              0                  |
| 0               0        -far/depth     -far*near/depth    |
| 0               0        -1             0                  |
```

The perspective builder takes the tangent of half the vertical field of view,
so Bunny needs no trigonometry to build it. After the matrix, `w_clip =
-z_view`, and NDC is the component-wise division by `w_clip`. A point behind
the camera has `w_clip < 0`; callers clip before dividing. Bunny does not yet
expose a clip or divide helper; viewport conversion is defined in
[Viewport Conversion](#viewport-conversion) and its inverse to a world ray in
[Unprojection](#unprojection).

Both builders use checked Q32.32 arithmetic and return `ProjectionError`
instead of panicking or saturating:

- `NonPositiveExtent`: a half extent, tangent, or aspect is zero or negative.
- `InvalidNear`: `near < 0` (orthographic) or `near <= 0` (perspective).
- `InvalidDepthRange`: `near >= far`.
- `Overflow`: a checked operation overflowed, including `far * near`, or a
  divisor rounded to zero.

Each matrix entry is rounded to the nearest Q32.32 value, so a matrix is exact
only when its entries are representable. Golden vectors therefore use
power-of-two parameters.

## Viewport Conversion

OpenGL, WebGPU, DirectX, image buffers, and UI systems disagree about the
viewport origin and the `y` direction, so Bunny names them explicitly.
`Viewport::try_new(width, height)` in `bunny-linalg` takes a size in device
pixels (`u32`), and `try_ndc_to_viewport` and `try_viewport_to_ndc` convert
between NDC and viewport coordinates as `FixedVec3` values.

The coordinate law already fixed that a conversion names its origin and `y`
direction; the rest of this section is a choice recorded here, with its reason.

- Origin: the top-left corner of the viewport. Pixel `y` runs down. NDC
  `(-1, +1)` is `(0, 0)` and NDC `(+1, -1)` is `(width, height)`. Reason: the
  issue specified it, and it matches image buffers and WebGPU framebuffer
  coordinates, which are the consumers.
- Pixel corners, not centres: a viewport coordinate is continuous, integers lie
  on pixel corners, pixel `i` spans `[i, i + 1)` and its centre is `i + 1/2`.
  The NDC edges `-1` and `+1` land exactly on `0` and `width`, and the NDC
  origin lands on `width / 2`. Reason: it keeps the map linear and invertible
  with no half-pixel offset baked in; deciding which pixel a point falls in
  (snapping, fill rules) stays with the rasterizer.
- Depth: viewport `z` equals NDC `z`, in `[0, 1]`, unchanged.
- No clipping or clamping: points outside the NDC cube map linearly, so values
  below `0` or above the size are possible.

Forward mapping, with `x` and `y` in raw Q32.32 units and `ONE = 2^32`:

```text
x_px = floor((x_ndc + ONE) * width  / 2)
y_px = floor((ONE - y_ndc) * height / 2)
```

The product is exact in `i128`, so the only rounding is one floor of at most
half a raw unit. Floor (round toward negative infinity, an arithmetic right
shift) is used rather than truncation or round-half-up so that the rule is the
same on both sides of zero. At NDC `-1`, `0` and `+1` and on power-of-two sizes
(and any even size) the result is exact.

Inverse mapping: `x_ndc = floor(2 * x_px / width) - ONE` and
`y_ndc = ONE - floor(2 * y_px / height)`, flooring the Euclidean quotient.

Round trip: the exact integer reasoning is `a = x_ndc + ONE`,
`p = floor(a * width / 2)`, then `floor(2p / width)`. When `width` is even, or
`a` is even, `a * width` is even and the round trip returns `a`. When both are
odd, `2p = a * width - 1` and the round trip returns exactly `a - 1`. So the
round trip is within one raw unit per axis: `x` never comes back above its
original and `y` never below, and it is exact unless the size and the shifted
NDC raw value are both odd.

Errors (`ViewportError`): `EmptyExtent` for a zero width or height, and
`Overflow` when a result does not fit Q32.32. Sizes are unsigned, so a negative
size cannot be expressed. Nothing panics or saturates.

## Unprojection

`Viewport::try_unproject(coordinate, clip_from_view, view_from_world)` in
`bunny-linalg` (`bunny-unproject/1`, `UNPROJECT_CONTRACT_ID`) returns the
world-space ray through a viewport coordinate, for picking code that holds only
the matrices. The cameras in `bunny-optics` generate rays from their own
parameters; this is the matrix-side inverse, and its rays agree with theirs
(below).

**Input.** `coordinate` is a continuous viewport coordinate (`FixedVec2`) in
[Viewport Conversion](#viewport-conversion)'s convention: origin at the
top-left corner, `y` down, integers on pixel corners, so the centre of pixel
`(i, j)` is `(i + 1/2, j + 1/2)` and a camera's `PixelSample` is
`(x + sub_x, y + sub_y)`. The closed rectangle `[0, width] x [0, height]` is
accepted, edges included, as boundary contact is inclusive elsewhere in this
law. `clip_from_view` must have one of the two shapes the builders produce,
with any entry values: perspective rows `(m00, 0, 0, 0)`, `(0, m11, 0, 0)`,
`(0, 0, m22, m23)`, `(0, 0, -1, 0)`, or orthographic with last row
`(0, 0, 0, 1)`. `view_from_world` must be affine (last row `(0, 0, 0, 1)`)
with any invertible linear part.

**Output.** A `ViewportRay { origin, direction }`: `origin` is the world point
the coordinate unprojects to at NDC depth `0` (the near plane) and
`origin + direction` the point at depth `1` (the far plane), so `t` in
`[0, 1]` spans the depth range. The direction is not normalized; its length is
the near-to-far distance along that line (`far - near` on the axis of an
orthographic view, longer off axis for a perspective one). `TraceRay::new`
accepts direction components up to `1.0`, so a caller that wants a trace ray
scales it.

**The exact ray.** Write `ONE = 2^32` and raw values `X`, `Y`, `W`, `H` and
`p, q, r, s` for `m00, m11, m22, m23`. The viewport step is the exact inverse
of `try_ndc_to_viewport`'s formula before its floor, not `try_viewport_to_ndc`
(which floors): `x_ndc = ax / (W ONE)` with `ax = 2 X - W ONE`, and
`y_ndc = ay / (H ONE)` with `ay = H ONE - 2 Y`. The projection step is the
analytic inverse, divide included, at depths `0` and `1`:

```text
perspective   depth-0 point  (ax s / (W p r), ay s / (H q r), -s / r)
              depth-1 point  the depth-0 point times r / (r + ONE)
orthographic  depth-0 point  (ax / (W p), ay / (H q), -s / r)
              direction      (0, 0, ONE / r)
```

in units. With `E = W H p q r` every view-space value is an integer over `E`
(or over `E (r + ONE)` and `r` for the directions). The view step is
`A^-1 (v - t)` with the exact adjugate inverse of the raw linear part `A`
(`A^-1 = ONE adj(A) / det(A)` in units), not the transpose. So each output is
one exact ratio:

```text
origin_i    = ONE  * sum_j adj_ij (ONE N_j - t_j E) / (E det A)
direction_i = ONE^2 * sum_j adj_ij D_j / (F det A)
```

with `N / E` the depth-0 point and `D / F` the direction in view space.

**Rounding and bound.** Nothing is rounded before the end. Each of the six
components is its exact ratio rounded once to nearest, ties to even: within
half a raw unit of the exact value for these matrices. This is the general
inverse of `clip_from_world = clip_from_view * view_from_world`, not an
approximation of it: CL-TP-018 checks the result bit for bit against Cramer's
rule on the full product in unbounded integers. `FixedMat4::try_inverse` is
not used; it rounds every minor and has no stated bound.

**Widths.** With every raw entry below `2^63` in magnitude and `W, H < 2^32`:
`|ax|, |ay| < 2^64`, `|E| < 2^253`, `|F| < 2^317`, `|adj_ij| <= 2^127` and
`|det A| < 2^192`. The largest numerator is below `2^478` and the largest
denominator below `2^509`, so the kernel's 512-bit signed integers hold every
intermediate exactly. No float is used.

**Refusals** (`UnprojectError`), checked in this order, each tested on and one
raw unit either side of its threshold (CL-TP-015 to CL-TP-017):

| Variant | Condition |
| --- | --- |
| `OutsideViewport` | `X < 0`, `X > W ONE`, `Y < 0` or `Y > H ONE` (raw) |
| `UnsupportedProjection` | an entry outside the shape is nonzero, or the last row is neither `(0, 0, -1, 0)` nor `(0, 0, 0, 1)` exactly |
| `SingularProjection` | `m00 = 0`, `m11 = 0`, or perspective `m23 = 0`, or orthographic `m22 = 0` |
| `PointAtInfinity` | perspective homogeneous `w = (z_ndc + m22) / m23` is zero: `m22 = 0` (depth 0) or `m22 = -1.0` (depth 1) |
| `PointBehindCamera` | that `w` is negative at depth `0` or `1`: the sign of `m22` or of `m22 + 1.0` differs from the sign of `m23` |
| `UnsupportedView` | the last row of `view_from_world` is not `(0, 0, 0, 1)` exactly |
| `SingularView` | `det A = 0`, decided on the exact raw entries |
| `Overflow` | a rounded component is outside `i64` (origin first, then direction) |
| `DegenerateRay` | the rounded direction is zero (the exact one never is) |

A builder's matrix has the accepted shape and signs (`m23 < 0` and
`m22 <= -1.0` for a perspective one), but its rounded entries can still be
refused: `m22` rounds to exactly `-1.0` (`PointAtInfinity`) when
`far / (far - near)` is within half a raw unit of one, and `m23`, or an
orthographic `m22`, rounds to zero (`SingularProjection`) for extreme depth
ranges. A perspective matrix with `m23 > 0` and `m22 > 0` (a
reversed depth range, near and far swapped) is accepted, and its ray still runs
from the depth-0 point to the depth-1 point, toward the camera.

**Agreement with the cameras.** For 2 400 seeded samples each, the ray
unprojected through `try_look_at(position, position + look, up)`,
`try_perspective(tan, W / H, 2^-10, 1024)` (or `try_orthographic` with the
camera's half extents, near `0`) and `Viewport::try_new(W, H)` lies along
`PinholeCamera::ray` (or `OrthographicCamera::ray`) of the same `PixelSample`
(CL-TP-020, CL-TP-021): the directions point the same way, the camera direction is
within `2^4` raw of the unprojected line, the unprojected origin within `2^4`
raw of the camera ray, and the camera's origin within `2^4` raw of the
unprojected line, worst over both cameras. The asserted tolerance, `2^6` raw
for each, is measured, not proven: the two sides round differently (the camera
floors its direction to raw units, the builders round each entry and the
look-at basis), and no closed-form bound is derived for the difference. There
is no convention mismatch; a flipped handedness, `y` direction or pixel centre
misses by many orders of magnitude. For the perspective camera the unprojected
origin is on the near plane, not at the eye, so the origins are compared as
distances from each other's lines.

**Round trip.** A world point projected exactly, rounded once to raw NDC and
mapped by `try_ndc_to_viewport` (floor) lands within `W / 4 + 1` raw pixel
units of its exact viewport coordinate on `x` (`H / 4 + 1` on `y`). At the
point's clip `w` that moves the line by at most `2 delta w / (W |m00|)` view
units on `x` (and `y` alike), carried to world by `A^-1`; the kernel adds under
`1 + |t|` raw at the point's parameter `t`. CL-TP-019 checks each point's
distance from the unprojected ray against that bound, derived per case in exact
rationals from those step bounds.

## Camera Image Convention

The pinhole camera in `bunny-optics` names its image convention explicitly:
its basis is right-handed (`right = forward x up`), so a camera looking down
`-Z` with up `+Y` has right `+X`; pixel `(0, 0)` is the top-left pixel,
columns grow toward right and rows grow toward down, as in
[Viewport Conversion](#viewport-conversion).
The contract and its evidence live in
[the optics chapter](../optics/README.md).

## External Format Boundaries

Mesh, scene, renderer, and file formats often carry their own conventions.

Boundary adapters must document:

- source handedness
- source up axis
- source unit scale, if known
- source winding order
- source depth range, if projection data is imported (Bunny's is `[0, 1]`)
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

Projection builders keep tests that map near and far plane points to NDC
`z = 0` and `z = 1`, and that check the `-Z` camera and `+Y` up convention.

Viewport conversion keeps golden vectors on power-of-two and odd sizes, exact
raw rounding at the edges, and a seeded round-trip property test against exact
integer arithmetic.

Future transform, angle, and camera APIs must add tests for:

- `target_from_source` naming examples
- matrix composition order
- point versus vector translation behavior
- positive rotation direction

## Documentation Rule

This topic folder is the source of truth for current coordinate law.

Historical design documents must not become competing references. If a design
document is superseded, leave it as historical evidence and point readers here
for the current contract.
