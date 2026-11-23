# Shape Library

This chapter is the current reference for the geometry-level shape types of `bunny-geom`
that the shape library adds beside the ray, the box and the sphere: the line, the segment,
the plane, the triangle, the capsule and the oriented box. For each it states the
representation, the invariants, which degenerate cases are accepted and which are refused,
and how it relates to the trace-domain types of `bunny-query`. The [test plan](test-plan.md)
says how it is verified. It answers flyingrobots/bunny#115.

These are types, not queries. Intersection, distance and containment between shapes are
not part of this chapter. Plane classification and clipping are in
[plane clipping](../plane-clipping/README.md).

## Conventions

Every shape is in Q32.32 coordinates over the full raw `i64` range unless a section says
otherwise, and derives `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`. Equality is
representation equality: two values that describe the same set of points but are stored
differently (another point on a line, a reversed segment, a permuted triangle) are not `==`.
Nothing is canonicalized.

Constructors follow the crate's existing pattern. `new` is infallible and `const`; where the
type has something to refuse, `try_new` refuses it with a typed error. The new shapes refuse
with `ShapeError`, which is `#[non_exhaustive]`; `GeomError` is unchanged.

| Shape | Type | Fixed constructors | Float constructor | Refusals |
| --- | --- | --- | --- | --- |
| Line | `FixedLine3` | `new`, `try_new` | `try_from_float(point, direction)` | `InvalidLineDirection` |
| Segment | `FixedSegment3` | `new` | `try_from_float(start, end)` | none beyond the float boundary |
| Plane | `FixedPlane3` | `new` | `try_from_float(normal, offset)` | none beyond the float boundary |
| Triangle | `FixedTriangle3` | `new`, `try_new` | `try_from_float(vertices)` | `DegenerateTriangle` |
| Capsule | `FixedCapsule3` | `new`, `try_new` (`const`) | `try_from_float(start, end, radius)` | `NegativeCapsuleRadius` |
| Oriented box | `FixedObb3` | `try_new` only | `try_from_float(center, axes, half_extents)` | `NegativeHalfExtent`, `AxisLengthOutOfTolerance`, `AxesNotOrthogonal` |

## Line

`FixedLine3 { point: FixedVec3, direction: FixedUnitVec3 }` is the set `point + t *
direction` for every real `t`. It has the representation of `FixedRay3` and differs from it
only in meaning: a ray keeps `t >= 0`, a line has no restriction on `t`.

- `new(point, direction)` takes an already normalized `FixedUnitVec3`; every input is a line.
- `try_new(point, direction: FixedVec3)` normalizes with `FixedUnitVec3::new`, exactly as
  `FixedRay3::try_new` does, and refuses with `InvalidLineDirection` exactly when that returns
  `None`.

**Invariant.** The direction is a `FixedUnitVec3`: its floor square-root length is within one
raw unit of `2^32`. It is not exactly unit length.

**Degenerate cases.** The zero direction is refused. So are directions the ray's rule cannot
normalize: a vector whose raw length exceeds `i64::MAX` (`(i64::MAX, i64::MAX, 0)`, and
`(i64::MIN, 0, 0)`, whose magnitude is `2^63`), and a short vector whose normalized length
falls outside the band, such as `(1, 1, 0)` raw (its floor square-root length is 1 raw unit,
so it normalizes to `(1, 1, 0)` units). `(1, 0, 0)` raw and `(i64::MAX, 0, 0)` raw are
accepted. Short directions with uneven components are often refused: 9 534 of the 20 000
seeded directions of the test plan, of magnitudes up to `2^61` raw, were. A line may pass
through any point.

## Segment

`FixedSegment3 { start, end }` is the closed segment between two points, in that order. It
existed before this chapter; see [plane clipping](../plane-clipping/README.md) for its use.

- `new(start, end)` accepts every pair. A zero-length segment (`start == end`) is a point
  and is accepted.

**Invariant.** None beyond the coordinates. Endpoint order is part of the representation.

## Plane

`FixedPlane3 { normal, offset }` is the plane `normal . p = offset`, with signed distance
`normal . p - offset` and positive distance in front. It existed before this chapter; its
classification and clipping are in [plane clipping](../plane-clipping/README.md).

- `new(normal, offset)` accepts every input. The normal need not be unit length (the signed
  distance is then scaled by its length).

**Degenerate cases.** A zero normal is accepted: every point then has signed distance
`-offset`, so every point classifies to the same side.

## Triangle

`FixedTriangle3 { v0, v1, v2 }` is the triangle with those vertices, in order. The order is
the winding and is significant: `(v0, v1, v2)` and `(v1, v0, v2)` are different values.

- `new(v0, v1, v2)` accepts every input, degenerate triangles included.
- `try_new(v0, v1, v2)` refuses a degenerate triangle with `DegenerateTriangle`.
- `vertices()` returns `[v0, v1, v2]`.
- `is_degenerate()` says whether the triangle is degenerate.

**Degenerate cases.** A triangle is degenerate when its vertices are collinear or two or
three coincide, decided exactly: the cross product `(v1 - v0) x (v2 - v0)` of the raw
vertices is zero. It is accepted by `new` and refused by `try_new`, as `PlaneSpan`
classifies a degenerate triangle rather than refusing it.

**Exactness and widths.** Each edge difference has up to 65 bits; it is held in `i128` with
magnitude below `2^64`. A product of two has up to 129 bits; it is held as a sign and a
`u128` magnitude below `2^128`. A cross component is the difference of two products and
needs 130 bits, beyond `i128`, so it is never formed: the component is zero exactly when its
two products are equal, which is compared as sign and magnitude (zero has no sign). Nothing
rounds. A rounded cross product would be wrong here: the edges `(1, 0, 0)` and `(0, 1, 0)`
raw have a Q32.32 cross product that rounds to zero, but the triangle is not degenerate. The
answer does not depend on vertex order, because a permutation of the vertices only changes
the sign of the exact cross product.

**Trace domain.** `TraceTriangle` in `bunny-query` is a different type: its vertices must lie
in the trace domain (`abs(raw) < 2^40`, `docs/topics/ray-tracing/README.md#trace-domain`) and
it stores `v0` and two edges for the exact ray test. It accepts degenerate triangles too. Any
`TraceTriangle`'s vertices make a `FixedTriangle3`; the reverse holds only inside the domain.

## Capsule

`FixedCapsule3 { segment: FixedSegment3, radius: FixedQ32_32 }` is every point within
`radius` of the closed segment.

- `new(segment, radius)` does not validate, as `FixedSphere3::new` does not, so it can carry
  a negative radius.
- `try_new(segment, radius)` refuses a negative radius (raw `< 0`) with
  `NegativeCapsuleRadius`; it is `const`.

**Invariant** (for a value from `try_new` or `try_from_float`): `radius >= 0`.

**Degenerate cases**, all accepted: a zero radius is the segment itself; a zero-length
segment is the sphere of that radius about the point; both together are a single point.
These follow the sphere, whose `try_new` accepts a zero radius.

**Trace domain.** `bunny-query` has no capsule. Its `TraceSphere` and `TraceCylinder` refuse a
zero radius (`RadiusNotPositive`) and work only inside the trace domain.

## Oriented Box

`FixedObb3` is the set `center + axes * l` for every local point `l` with `abs(l_i) <=
half_extents_i`, where the axes are the three columns of the `FixedMat3` `axes`. Its fields
are private, so every value satisfies the invariant; `center()`, `axes()` and
`half_extents()` return them. It has no unvalidated `new`.

- `try_new(center, axes, half_extents)` checks, in this order: each half extent is
  non-negative (`NegativeHalfExtent { axis }`, the first negative one); each column's length
  is in tolerance (`AxisLengthOutOfTolerance { column }`, the first failing one); each pair's
  dot product is in tolerance (`AxesNotOrthogonal { first, second }`, the first failing pair
  in the order `(0, 1)`, `(0, 2)`, `(1, 2)`).

**Invariant.** Fixed point cannot hold an exactly orthonormal basis, so the box does not
claim one. With lengths and dot products taken exactly on the raw entries (a dot product is
the exact sum of raw products divided by `2^32`, as in the
[rotation](../rotation/README.md#orthonormality-and-determinant) and
[look rotation](../look-rotation/README.md#rounding-and-orthonormality) chapters):

| Quantity | Bound | Constant | Exact test on raw columns `c` |
| --- | --- | --- | --- |
| Length of each column | within 11 raw units of one | `OBB_AXIS_LENGTH_TOLERANCE_RAW` | `(2^32 - 11)^2 <= c . c <= (2^32 + 11)^2` |
| Dot product of two columns | within 12 raw units of zero | `OBB_AXIS_DOT_TOLERANCE_RAW` | `abs(a . b) <= 12 * 2^32` |
| Each half extent | non-negative | | `raw >= 0` |

The bounds are the column contract of `FixedMat3::rotation_axis_angle` (`bunny-rotation/1`,
within 11 and 12), the widest of the repository's rotation constructors; the axis rotations
(within 3/2 and exactly 0) and `FixedMat3::try_look_rotation` (`bunny-look-rotation/1`,
within 2 and 2) are inside them. So every matrix those constructors return is accepted; the
test plan checks this on seeded samples, which shows it for the samples and rests on those
chapters' proven bounds for the rest. Rows are not checked: the axes are the columns.
Handedness is not checked: a reflection (determinant near `-1`) is accepted, and describes
the same set of points as the rotation it mirrors.

**Widths.** A raw square is at most `2^126`, so a column's sum of three squares fits `u128`
for every entry, `i64::MIN` included. The dot products are formed only after every length
has passed; each component then has magnitude at most `2^32 + 11`, each product is below
`2^65` and each dot product fits `i128`. Nothing rounds and nothing overflows.

**Degenerate cases.** A zero half extent is accepted: one zero gives a flat box, two a
segment, three a point. A zero, short, long, sheared or repeated axis is refused by the
tolerances. The corners are not exposed, because `center + axes * l` rounds.

**Trace domain.** `TraceBox` in `bunny-query` is axis-aligned in the trace domain. Traced
inside a `ShapeFrame` (`docs/topics/shape-frame/README.md`), it is an oriented box with its
own, different axis rule: `FixedUnitVec3` axes, pairwise dot products within `2^-24` (256
raw units, wider than this chapter's 12) and right-handed (which this chapter does not
require). Neither rule implies the other.

## Float Boundary

Each new shape, and the segment and the plane, has a `try_from_float` that takes `f32`
components (`Vec3`, `Scalar`) and returns `Result<Self, ShapeError>`. It is built only from
the existing component conversions, `FixedVec3::try_from_float` and
`FixedQ32_32::try_from_f32`, so each float is rounded once from its exact value times `2^32`
to nearest, ties to even
([fixed-point rounding](../fixed-point-rounding/README.md), the
[numeric constitution's float rule](../../NUMERIC_CONSTITUTION.md)), with no float arithmetic
and no `std`-only float function. The refusals come in this order:

1. `NonFiniteCoordinate` when any coordinate is `NaN` or infinite (for the plane, the offset
   too), before any range check;
2. `NonFiniteRadius` when the capsule radius is `NaN` or infinite;
3. `NegativeCapsuleRadius` or `NegativeHalfExtent { axis }` when that float is below zero,
   before rounding, as the sphere's float path does: `-1e-12` is refused although it rounds
   to zero, and `-0.0` is accepted;
4. `FixedValueOutOfRange` when a finite value is outside Q32.32 after rounding (`2^31` is
   refused; `2^31 - 128`, the largest `f32` below it, and `-2^31` are accepted);
5. the fixed `try_new` refusals, on the rounded values: a line direction that rounds to
   zero or cannot be normalized, a triangle whose rounded vertices are degenerate, and the
   oriented box's axis tolerances. The oriented box takes its float axes as the columns.

There are no float shape structs for the new types and no float egress; the existing `Ray3`,
`Aabb3` and `Sphere3` boundary types are unchanged.
