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

`to_object(&TraceRay) -> Result<TraceRay, ShapeFrameError>` returns the ray whose origin is
`(ray.origin - origin) . axis` and whose direction is `ray.direction . axis`, for the x, y
and z axes in turn.

**Rounding rule.** Each of the six components is one exact integer dot product: the raw
axis components (Q32.32) times raw coordinates (Q32.32), summed in `i128` as a Q64.64 value,
then rounded once to Q32.32 to nearest, ties to even
(`docs/NUMERIC_CONSTITUTION.md#rounding-policy`). No intermediate is rounded or saturated.
Every product is below `2^74` (origin) and `2^66` (direction), so the sum fits `i128`.

The direction is not renormalized. A distance `t` reported by a shape kernel on the object
ray is a multiple of the object direction, which differs from the world direction by the
rounding of its components (at most half a raw unit each) and by the axes' departure from
an exact rotation; the world point is `ray.origin + ray.direction * t` to that accuracy.

`to_object` refuses:

| Refusal | When |
| --- | --- |
| `RayOutsideTraceDomain` | an object origin coordinate has `abs(raw) >= 2^40`; a ray anywhere in the trace domain is up to `2^41 * sqrt(3)` raw from a frame origin, so this is reachable |
| `DirectionOutsideTraceDomain` | an object direction component exceeds `TRACE_UNIT_DIRECTION_LIMIT_RAW` (`2^32 + 1`) in magnitude, or every component is zero |

The direction limit is the largest the trace kernels accept. A direction from
`TraceRay::new` may have components up to 1.0 and a length up to `sqrt(3)`, so a rotation
can move more than 1.0 onto one axis; such a ray is refused rather than clamped, because a
clamped direction would change every distance. A unit direction maps to one of length
within a few raw units of 1.0, so it is refused only when a component lands above
`2^32 + 1` raw (an axis-aligned ray under axes at the long end of the unit tolerance).

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
- A ray whose frame origin is behind it maps to an object ray that also moves away from the
  object origin; nothing about the ray's distance range changes.
- Nothing wraps or panics: every overflow is one of the refusals above.
