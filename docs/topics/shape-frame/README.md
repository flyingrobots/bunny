# Shape Frame

This chapter is the current reference for `ShapeFrame` in `bunny-query`, the rigid
placement an analytic shape (sphere, box, cylinder and so on) is traced in. The
[test plan](test-plan.md) says how it is verified. It answers flyingrobots/bunny#357.

## Contract Identity

`SHAPE_FRAME_ID` is `bunny-shape-frame/1`. It names the output bits of
`ShapeFrame::to_object` and `ShapeFrame::to_world`, refusals included. Any change to one of
those bits is a new version.

## The Frame

A frame is three unit axes `x`, `y` and `z` and an origin, all given in world coordinates:

- `ShapeFrame::new([x, y, z], origin)` takes the axes as `FixedUnitVec3` values;
- `ShapeFrame::translation(origin)` uses the world axes.

The world point `p` has the object coordinates
`((p - origin) . x, (p - origin) . y, (p - origin) . z)`. There is no scale and no shear: a
shape carries its own dimensions (a radius, half extents), so distances along a ray are the
same in both spaces up to the rounding below.

`new` refuses, with a `ShapeFrameError`:

| Refusal | When |
| --- | --- |
| `OriginOutsideTraceDomain` | an origin coordinate has `abs(raw) >= 2^40` (256 units), the trace domain of `docs/topics/ray-tracing/README.md#trace-domain` |
| `AxesNotOrthogonal` | the exact dot product of two axes, in units of `2^-64`, exceeds `SHAPE_FRAME_AXIS_DOT_LIMIT_RAW` (`2^40`) in magnitude: the cosine between them is above `2^-24` |
| `AxesLeftHanded` | the exact `(x x y) . z` is not positive |

The orthogonality limit is wide enough for axes made by normalizing the rows of a rotation
(each component off by at most half a raw unit, so a dot product off by a few times
`2^-32`), and narrow enough that the frame is rigid to about `2^-24`. Axes are used as
given; they are not re-orthogonalized.

## Mapping A Ray Into The Object Frame

`to_object(&TraceRay) -> Result<ObjectRay, ShapeFrameError>` returns the ray whose origin is
`(ray.origin - origin) . axis` and whose direction is `(ray.direction . axis) / 2^k`, for the
x, y and z axes in turn, together with `k` (`ObjectRay::direction_shift`).

**Rounding rule.** Each of the six components is one exact integer dot product: the raw
axis components (Q32.32) times raw coordinates (Q32.32), summed in `i128` as a Q64.64 value,
then (for the direction) divided by `2^k`, and rounded once to Q32.32 to nearest, ties to
even (`docs/NUMERIC_CONSTITUTION.md#rounding-policy`). The division is part of the one
rounding, not a shift of a rounded value. No intermediate is rounded or saturated. Every
product is below `2^74` (origin) and `2^66` (direction), so the sum fits `i128`.

**The direction shift.** `k` is the smallest shift that keeps every object direction
component within `TRACE_UNIT_DIRECTION_LIMIT_RAW` (`2^32 + 1` raw), the largest the trace
kernels accept. Rays are not unit length: a camera ray or a shadow ray toward a light has
components up to 1.0 and a length up to `sqrt(3)`, and a rotation can move more than 1.0
onto one axis. Such a ray is scaled, not refused. The bound: a `TraceRay` direction has
components at most `2^32 + 1` raw, so its length is below `sqrt(3) * (1 + 2^-32)`; an axis
accepted by `FixedUnitVec3` is shorter than `1 + 2^-31`; each exact object component is
therefore below 2.0, and half of it is below 1.0. So `k` is 0 or 1
(`SHAPE_FRAME_MAX_DIRECTION_SHIFT`).

**Distances.** The object point at distance `t` along the object ray is the world point at
distance `t / 2^k` along the world ray, to within the rounding of the object ray's
components (at most half a raw unit each) and the axes' departure from an exact rotation.

- `ObjectRay::world_distance(t)` is `floor(t / 2^k)` in raw units, exact. Shape kernels
  report the floor of the true object distance, and `floor(floor(x) / 2^k)` is
  `floor(x / 2^k)`, so the composition is the floor of the exact world parameter of the
  object-ray hit: the same rounding direction as `TraceRay::hit_triangle` on the world ray.
  Hits on shapes and triangles therefore compare in world-ray parameters.
- `ObjectRay::object_distance(t)` is `t * 2^k`, saturating at the Q32.32 range, for passing
  a world distance limit to a kernel. `world_distance(object_distance(t))` is `t`. A
  saturated limit is beyond every distance a kernel reports in the trace domain.

`to_object` refuses:

| Refusal | When |
| --- | --- |
| `RayOutsideTraceDomain` | an object origin coordinate has `abs(raw) >= 2^40`; a ray anywhere in the trace domain is up to `2^41 * sqrt(3)` raw from a frame origin, so this is reachable |
| `DirectionOutsideTraceDomain` | every object direction component rounds to zero; defensive, since a nonzero `TraceRay` direction has a component of at least one raw unit |

### Decision: scale, do not refuse

The coordinator of the Lumen shape plan decided this (2026-11-16) on flyingrobots/bunny#374.
The rejected alternative was to refuse such rays and have every caller normalize its
directions first. Camera and shadow rays would then be refused for every rotated shape, and
the normalizing arithmetic, which decides pixels, would move into Lumen.

## Mapping A Normal Back To World Space

`to_world(normal: FixedVec3) -> Result<FixedVec3, ShapeFrameError>` returns
`normal.x * x + normal.y * y + normal.z * z`. Each world component is the exact sum of three
Q32.32 products as a Q64.64 value, rounded once to Q32.32 to nearest, ties to even, the same
rule as `to_object`. Every product is below `2^96`, so the sum fits `i128`.

Any vector in object coordinates is accepted, not only unit normals, and the result is not
renormalized: a caller that needs a `FixedUnitVec3` normalizes it. `NormalOutOfRange` is
returned when a world component does not fit `i64`.

## Degenerate And Boundary Cases

- A ray that starts at the frame origin has the object origin `(0, 0, 0)` exactly.
- A ray parallel to an object plane keeps a zero direction component in that plane when the
  axes map it exactly (for example axis permutations); under a general rotation the
  component is the rounded exact value, which may be a few raw units.
- A lengthened direction is halved (`k = 1`), never refused; distances map back through
  `world_distance`.
- With axes that map it exactly (for example axis permutations), a ray whose frame origin is
  behind it maps to an object ray that also moves away from the object origin. With inexact
  axes, a ray nearly perpendicular to its offset from the frame origin can map to one that
  moves slightly toward it. Either way the map does not change the ray's distance range.
- Nothing wraps or panics: every overflow is one of the refusals above.
