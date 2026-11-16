# Ray Tracing

This chapter defines Bunny's current contract for ray tracing: the kernels in
`bunny-query` (a prepared ray, a prepared triangle, a conservative ray-box test
and an exact ray-triangle test) and the closest-hit and any-hit BVH traversals
in `bunny-broadphase` that run them.

The kernels are the per-node and per-primitive tests a closest-hit traversal
runs millions of times, so they avoid division wherever the answer allows it.
The general-purpose solvers in `crates/bunny-query/src/ray.rs`
(`ray_intersects_aabb`, `ray_intersects_triangle`) and the all-hits traversal
`bunny_broadphase::intersect_ray` keep their own contracts; the overflow
behaviour of `ray_intersects_aabb`, which `intersect_ray` uses for every node,
is described in
[General-Purpose Ray-AABB Solver](#general-purpose-ray-aabb-solver).

`PreparedRay3` with `ray_intersects_aabb_prepared` (`bunny-query`) is the exact
counterpart: the same `(enter, exit)` bits as `ray_intersects_aabb` on every input, with a
prepared divisor per axis. It is for callers that need those exact distances for many boxes
per ray. For traversal, `TraceRay::box_entry` is the faster test; the measured comparison is
ledger entry 5 in [Performance](../performance/README.md).

## Status

This is a living topic chapter, not a proposal.

The kernels are implemented in `crates/bunny-query/src/trace.rs` and exported
as:

- `TraceRay` (`new`, `from_ray`, `origin`, `direction`, `box_entry`,
  `hit_triangle`);
- `TraceTriangle` (`new`, `vertices`, `bounds`);
- `TriangleHit` (`distance`, `front_face`, `barycentrics`,
  `barycentric_numerators`);
- `TraceSphere` (`new`, `center`, `radius`, `bounds`), `TraceSphereError`,
  `SphereHit` (`distance`, `front_face`, `normal`) and `TraceRay::hit_sphere`,
  in `crates/bunny-query/src/trace/sphere.rs` (see [Ray-Sphere](#ray-sphere));
- `TraceBox` (`new`, `min`, `max`, `bounds`), `TraceBoxError`, `BoxHit`
  (`distance`, `front_face`, `normal`) and `TraceRay::hit_box`, in
  `crates/bunny-query/src/trace/box_hit.rs` (see [Ray-Box](#ray-box));
- `BarycentricNumerators` (`new`, `u`, `v`, `det`), in
  `crates/bunny-query/src/barycentric.rs`;
- `TRACE_COORD_LIMIT_RAW`, `TRACE_DIRECTION_LIMIT_RAW` and
  `TRACE_UNIT_DIRECTION_LIMIT_RAW`.

The traversals are implemented in `crates/bunny-broadphase/src/closest_hit.rs`
and exported as:

- `closest_hit` and `any_hit`;
- `BvhView` (`new`, `nodes`, `prim_indices`), the borrowed BVH buffers they
  search;
- `ClosestHit` (`primitive`, `distance`).

A second node layout for the same searches is implemented in
`crates/bunny-broadphase/src/compact.rs` and `compact_hit.rs` and exported as:

- `compact_bvh`, which converts a built tree into it, and `CompactError`;
- `CompactNode` (`bounds`, `children`), `CompactChild` (`first`, `prim_count`,
  `is_leaf`) and `CompactRoot` (`bounds`, `child`);
- `compact_closest_hit`, `compact_any_hit` and `CompactView` (`root`, `nodes`,
  `prim_indices`);
- `compact_closest_hit_counted`, `compact_any_hit_counted` and `TraversalCounts`
  (in `crates/bunny-broadphase/src/counts.rs`).

See [Compact Layout](#compact-layout) and [Counting Traversals](#counting-traversals).

Evidence is listed in the [test plan](test-plan.md). Benchmarks of the kernels
and the traversals are not part of this chapter yet.

All arithmetic is integer arithmetic on raw Q32.32 values (`i64`, widened to
`u128` or `i128`). There is no floating point, no `unsafe` code and no
target-specific path, so every supported target returns the same bits.

## Trace Domain

The kernels accept only values inside a fixed trace domain:

| Value | Bound | Constant |
| --- | --- | --- |
| Ray origin and triangle vertex coordinates | `abs(raw) < 2^40` (below 256.0) | `TRACE_COORD_LIMIT_RAW` |
| Ray direction components, `TraceRay::new` | `abs(raw) <= 2^32` (at most 1.0) | `TRACE_DIRECTION_LIMIT_RAW` |
| Ray direction components, `TraceRay::from_ray` | `abs(raw) <= 2^32 + 1` | `TRACE_UNIT_DIRECTION_LIMIT_RAW` |

`TraceRay::new` returns `None` when an origin coordinate or a direction
component is outside the domain, or when the direction is zero.
`TraceTriangle::new` returns `None` when any vertex coordinate is outside the
domain. `TraceBox::new` returns `Err(TraceBoxError)` for a corner outside it. `TraceRay::from_ray` accepts a `FixedRay3` and returns `None` only for
an origin outside the domain: `FixedUnitVec3::try_from_unit` allows the length
to be off by one raw unit, so a unit direction can have one component of
magnitude `2^32 + 1` (the other two then have squares summing to at most
`2^33 + 2`, since the sum of all three must be less than `(2^32 + 2)^2`), and `from_ray` accepts it although `new`
refuses it. `FixedUnitVec3::new` (normalization) never produces a component
above `2^32`: the floored root is at least the largest component, so the
quotient is at most 1.0 (TRACE-TP-409 counts this over generated inputs).

**Widths with direction components up to `2^32 + 1`.** Each line of the argument
was redone for this bound; none is tight:

- The reciprocal `floor(2^63 / abs(d))` is `2^31 - 1` for `abs(d) = 2^32 + 1`
  (`(2^32 + 1)(2^31 - 1) = 2^63 - 2^31 - 1`, remainder `2^31 + 1 < 2^32 + 1`).
  The box test's products are `near * r` and `far * (r + 1)`, with `r` below
  `2^63` and `near`, `far` below `2^64`: below `2^127` whatever `d` is.
- Edge and origin-offset components are below `2^41` (at most `2^41 - 2`).
  Each component of `p = d x e2` is at most `2 (2^32 + 1)(2^41 - 2)`, below
  `2^75` (below `2^74` when `abs(d) <= 2^32`).
- `det = e1 . p = d . (e2 x e1)`; each component of `e2 x e1` is below `2^83`, so
  `abs(det)` is below `3 (2^32 + 1) 2^83 = 3 * 2^115 + 3 * 2^83`, below `2^117`.
- `u = s . p` is below `3 * 2^41 * 2^75`, and `v = d . q` is below
  `3 (2^32 + 1) 2^83`, with `q = s x e1` below `2^83` per component: both below
  `2^117` before the comparisons with `det`.
- `t = e2 . q` does not involve `d`: below `3 * 2^41 * 2^83 = 3 * 2^124`, below `2^126`.
- `u + v` is below `2^118`; all of these fit `i128` (below `2^127`) with at least
  nine bits to spare.

The bounds exist so that every intermediate of the triangle test fits `i128`
exactly: edges and origin offsets are below `2^41`, cross products below
`2^84`, and the largest numerator below `2^126`. Box coordinates are not
restricted; `box_entry` accepts any `FixedAabb3`.

The direction need not be a unit vector. Every distance in this chapter is a
multiple of the direction: the point at distance `t` is
`origin + direction * t`, and distances are raw Q32.32 values of `t`.

## Conservative Box Test

`TraceRay::box_entry(aabb, t_max)` returns a lower bound on the distance at
which the ray enters `aabb`, or `None` when the ray certainly does not reach the
box within distances `0..=t_max`. A negative `t_max` always returns `None`.

`TraceRay::new` stores one reciprocal per direction component,
`r = floor(2^63 / abs(d))` (zero for a zero component). For each axis with a
nonzero component, with `near` and `far` the raw coordinate differences from
the origin to the slab planes in the direction of travel:

- the slab lower bound is `floor(near * r / 2^31)`, with a negative `near`
  (origin already past the near plane) taken as zero;
- the slab upper bound is `floor(far * (r + 1) / 2^31) + 1`, saturating;
- a negative `far` (the slab is behind the origin) rejects the box.

An axis with a zero component accepts the whole distance range when the origin
coordinate lies within `min..=max` on that axis (inclusive) and rejects the box
otherwise. The entry bound is the largest slab lower bound; the box is rejected
when it exceeds the smallest slab upper bound or `t_max`.

The exact slab distances are `near * 2^32 / abs(d)` and `far * 2^32 / abs(d)`.
Because `r <= 2^63 / abs(d) < r + 1`, each lower bound is at or below its exact
value and each upper bound is above its exact value. Therefore:

- **It never rejects a box the ray reaches.** If the ray reaches the box at an
  exact entry `T <= t_max`, every computed lower bound is at most `T`, every
  computed upper bound is above the exact exit, and the test passes.
  Touching a face, edge or corner counts as reaching the box.
- **It may accept a box the ray narrowly misses**, by at most the rounding
  slack below. Callers must treat an accepted box as "may contain a hit", which
  is what a traversal needs.
- **The bound is within `2^11 + 2` raw units plus `T / 2^30` below the exact
  entry `T`.** That is the stated contract. The per-axis derivation gives a
  tighter figure: the reciprocal loses less than one unit of `2^63 / abs(d)`,
  so a slab lower bound falls short by less than `near / 2^31 + 1` raw units,
  and `near <= T` because `abs(d) <= 2^32`. For a `from_ray` direction
  component of `2^32 + 1`, `near <= T + T / 2^32`, so the shortfall is below
  `T / 2^31 + 1 + T / 2^63`, which is below `T / 2^31 + 2` for `T < 2^63`. The
  test checks both the contract and the tighter `T / 2^31 + 2`.

The box test needs no division at query time: one multiplication and one shift
per slab plane.

How the implementation computes this (flyingrobots/bunny#258; the results are
bit for bit those of the earlier `i128` version, which the tests keep as a
reference oracle):

- **Differences in `i64`.** For a negative direction component, bitwise not
  (`!x = -x - 1`) reverses the order of `i64` and keeps `!a - !b = b - a`, so
  the slab becomes one crossed forwards: the box test applies `!` to the
  origin and the slab's planes on such axes. A signed comparison then decides
  whether a plane is ahead of the origin, and the difference of two `i64`
  values `a >= b` lies in `0..2^64`, so the wrapping `i64` subtraction read as
  `u64` is exact for any box coordinates, including the ends of the `i64`
  range.
- **Parallel axes in the same path.** A zero component is treated as a forward
  crossing whose reciprocal is zero: its far-plane check is `origin <= max`, a
  near plane ahead of the origin (`min > origin`) rejects, its lower bound is
  0 and its upper bound saturates to `u64::MAX`.
- **Straight-line code.** All three axes, both bounds and the comparison with
  `t_max` are computed without early returns and combined into one validity
  flag, so the test has no data-dependent branch of its own; arithmetic on a
  rejected axis cannot overflow and its result is discarded.

## Exact Triangle Test

`TraceRay::hit_triangle(triangle, t_min, t_max)` decides exactly whether the
ray hits the triangle at a distance in `t_min..=t_max`, and returns a
`TriangleHit` when it does.

The test follows the Moller-Trumbore construction in exact integers. With
`e1 = v1 - v0`, `e2 = v2 - v0`, `d` the direction and `s = origin - v0`, all
products in `i128`:

```text
p   = d x e2         det = e1 . p
q   = s x e1         u   = s . p      v = d . q      t = e2 . q
```

After the sign of `det` is folded into `det`, `u`, `v` and `t` (so `det > 0`),
the ray's line crosses the triangle when `0 <= u`, `0 <= v` and `u + v <= det`.
These comparisons are exact: no rounding happens before the hit is decided.

- **Edge rule.** Edges and corners belong to the triangle (all three
  comparisons are inclusive). Two triangles that share an edge both report a
  ray through that edge, so a ray through a shared edge of a closed mesh never
  passes between them. A closest-hit caller must break the resulting tie.
- **Parallel and degenerate cases.** `det == 0` is a miss. That covers rays
  parallel to the triangle's plane, including rays lying in that plane, and
  zero-area triangles (duplicate or collinear vertices).
- **Behind the origin.** A crossing with `t < 0` is a miss whatever `t_min` is.

Only after a crossing is established does the test divide, once:

- **Distance rounding.** `TriangleHit::distance` is
  `floor(t * 2^32 / det)`, the floor of the true distance in raw units. A
  crossing whose distance does not fit `i64` is a miss; this can only happen
  with very short directions.
- **Distance limits** are inclusive and apply to the rounded distance:
  the hit is reported when `t_min <= distance <= t_max`.
- **Barycentrics** are computed on demand by `TriangleHit::barycentrics`:
  `(floor(u * 2^32 / det), floor(v * 2^32 / det))`, the weights of `v1` and
  `v2`. The hit point is `v0 + (v1 - v0) * u + (v2 - v0) * v`. Each weight lies
  in `0.0..=1.0`, and their sum is at most 1.0.
- **Exact numerators.** `TriangleHit::barycentric_numerators` returns the
  `u`, `v` and `det` the test compared, unrounded, as a
  `BarycentricNumerators`. Each is below `2^117` for a ray and triangle in the
  trace domain: direction components are at most `2^32 + 1` and edge components
  below `2^41`, so each component of `p = d x e2` is below `2^75` and
  `det = e1 . p` is below `3 (2^32 + 1) 2^83`, below `2^117`; `u` and `v` are at
  most `det`.
  `BarycentricNumerators::new(u, v, det)` builds the same type from any values
  that describe a point on a triangle (`det > 0`, `u >= 0`, `v >= 0`,
  `u + v <= det`) and returns `None` otherwise, so a caller can state weights
  without a ray. Neither changes `barycentrics`.

The division is a long division of the `i128` numerator by the `i128`
denominator that brings the 32 fractional bits down in steps the denominator's
width allows, so it is exact for every value in the trace domain.

## Facing

The triangle is two-sided: both faces report hits.
`TriangleHit::front_face` is `true` when the ray arrives on the side from
which `v0, v1, v2` appear counter-clockwise, that is when
`d . (e1 x e2) < 0`, so the ray travels against the right-handed normal
`e1 x e2`. This matches the winding convention of
[the coordinate law](../coordinate-law/README.md).

## Accessors

`TraceRay::origin` and `TraceRay::direction` return the values given to `new`;
the direction is not normalized. `TraceTriangle::vertices` returns the three
vertices given to `new`, and `TraceTriangle::bounds` returns the smallest
`FixedAabb3` that contains them.

## Ray-Sphere

`TraceRay::hit_sphere(sphere, t_min, t_max)` is the exact ray-sphere test.
Its contract identity is **`bunny-ray-sphere/1`**. The general-purpose
`ray_intersects_sphere` in `ray.rs` is a separate solver with its own
contract and is unchanged.

**The sphere.** `TraceSphere::new(center, radius)` takes a centre and a
radius, so a sphere anywhere in the trace domain needs no change of frame; a
caller with spheres in an object frame passes a centre at the origin of that
frame. Every centre coordinate and the radius must be inside the trace domain
(`abs(raw) < 2^40`), and the radius must be positive. Any other input is
refused with a typed `TraceSphereError`: `CenterOutsideTraceDomain`,
`RadiusNotPositive` or `RadiusOutsideTraceDomain`. Nothing wraps, saturates or
panics. `TraceSphere::bounds` is the box `center -+ radius`.

**The hit** has the form of `TriangleHit`, so a renderer compares distances
across kinds directly:

- `SphereHit::distance` is in multiples of the ray direction, as for
  triangles;
- `SphereHit::front_face` is `true` at the near root (the ray arrives from
  outside, or touches a tangent point) and `false` at the far root (the ray
  leaves from inside);
- `SphereHit::normal` is the outward normal.

**The arithmetic.** With `f = origin - center`, `d` the direction and `r` the
radius, all raw, the point at distance `t` lies on the sphere when
`a t^2 + 2 b t + (f . f - r^2) = 0`, with `a = d . d` and `b = f . d`. The
quarter discriminant is computed as `a r^2 - |f x d|^2` (Lagrange's identity),
exactly, in 256-bit unsigned integers (`crates/bunny-query/src/trace/wide.rs`).
The line misses when it is negative. Otherwise `E = 2^64 * discriminant` and
the roots in raw distance units are `(-2^32 b -+ sqrt(E)) / a`.

`floor(sqrt(E))` is exact: below `2^128` it is `FixedQ32_32::sqrt_u128`,
Bunny's exact integer square root; above, the root of the high 128 bits gives
the top 64 bits of the result and the low 64 bits are set one at a time,
largest first, while the square stays at most `E`.

Widths in the trace domain: `abs(f) < 2^41` and `abs(d) <= 2^32 + 1` per
component, so `a < 2^66`, `abs(b) < 2^75`, each component of `f x d` is below
`2^75`, `a r^2 < 2^146`, `E < 2^210` and its root below `2^105`;
`2^32 b -+ root` is below `2^108`. Every value fits its type with room to
spare.

**Rounding rule of `bunny-ray-sphere/1`.**

- Each root is the floor of the true root in raw units:
  `floor((-2^32 b - sqrt(E)) / a)` and `floor((-2^32 b + sqrt(E)) / a)`, with
  the square root real, not rounded. Over integers this is
  `floor((-2^32 b - g - 1) / a)` for the near root when `E` is not a perfect
  square and `floor((-2^32 b - g) / a)` when it is, and
  `floor((-2^32 b + g) / a)` for the far root, where `g = floor(sqrt(E))`.
  This is the rule of `TriangleHit::distance`.
- The distance limits are inclusive and apply to the rounded distance. The
  near root is reported when `t_min <= near <= t_max`; otherwise the far root
  when `t_min <= far <= t_max`; otherwise there is no hit.
- The normal is `floor((f * 2^32 + d * distance) / r)` per component: the
  offset of the point at the rounded distance from the centre, over the
  radius, rounded down. Its length is within a few raw units of 1.0, not
  exactly 1.0.

**Cases.**

- **Tangent rays.** A discriminant of zero is a hit at the one root, front
  face. Contact is inclusive: a ray whose line touches the sphere hits.
- **Origin inside.** The near root is negative, so with `t_min >= 0` the far
  root is reported, with `front_face == false`.
- **Behind the origin.** When both rounded roots are below `t_min` there is no
  hit; a negative `t_min` lets a caller see them.

## Ray-Box

`TraceRay::hit_box(box, t_min, t_max)` is the exact ray-box test for a box
aligned with the axes of the frame the ray is in. Its contract identity is
**`bunny-ray-box/1`**. `TraceRay::box_entry` (the conservative traversal test) and the
general-purpose `ray_intersects_aabb` are separate and unchanged. A box that is
rotated is traced by a caller that carries the ray into the box's object space
first.

**The box.** `TraceBox::new(min, max)` takes two corners. Every coordinate of
both must be inside the trace domain (`abs(raw) < 2^40`) and no minimum may be
above its maximum. Any other input is refused with a typed `TraceBoxError`:
`CornerOutsideTraceDomain` (reported first) or `MinimumAboveMaximum`. Nothing
wraps, saturates or panics. Equal bounds on an axis are allowed: the box is then
flat (a rectangle, a segment or a point). `TraceBox::bounds` is the same box as a
`FixedAabb3`.

**The hit** has the form of `SphereHit`:

- `BoxHit::distance` is in multiples of the ray direction, rounded down;
- `BoxHit::front_face` is `true` at the entry (the ray arrives from outside, or
  touches the box) and `false` at the exit (the ray leaves from inside);
- `BoxHit::normal` is the outward normal of the face hit: exactly `1.0` or
  `-1.0` (raw `2^32` or `-2^32`) in one component and `0` in the other two.

**The arithmetic.** On each axis with a nonzero direction component `d`, the
ray crosses the faces at the raw distances `(bound - origin) * 2^32 / d`. For a
positive `d` it enters at the minimum face and leaves at the maximum face; for a
negative `d` the reverse. The ray is inside the box between the largest entry
and the smallest exit over the three axes, and hits the closed box exactly when
that entry is not after that exit. These crossings are kept as exact fractions
and compared by cross-multiplication, never by their rounded values. Widths in
the trace domain: each offset `bound - origin` is below `2^41`, each numerator
below `2^73`, each denominator at most `2^32 + 1`, so every product is below
`2^107` and fits `i128`.

**Rounding rule of `bunny-ray-box/1`.**

- The distance is the floor of the true crossing in raw units,
  `floor((bound - origin) * 2^32 / d)`. The floor is monotone, so the floor of
  the largest entry is the largest of the floors.
- The distance limits are inclusive and apply to the rounded distance. The entry
  is reported when `t_min <= entry <= t_max`; otherwise the exit when
  `t_min <= exit <= t_max`; otherwise there is no hit. A start clipped by `t_min`
  therefore reports the exit with `front_face == false`, as `hit_sphere` does.

**Tie rule for edges and corners.** When the ray crosses two or three faces at
exactly the same true distance (it hits an edge or a corner), the face reported
is the one on the lowest axis, in the order x, y, z. This holds for the entry and
for the exit alike. Equal rounded distances are not a tie: the face crossed
later by the exact fractions is the one hit (TRACE-TP-513 has two faces whose
floors are both 1).

**A ray parallel to a slab** (a zero direction component) never supplies a face.
Its axis rules the box out when the origin's coordinate is outside the closed
slab, even by one raw unit, and constrains nothing otherwise. A ray lying
exactly in the plane of a face, or on the line of an edge, is inside the closed
box's slab, so it can hit through the faces of the other axes. The direction is
never zero, so at least one axis supplies a crossing.

**Cases.**

- **Grazing rays.** Contact is inclusive: a ray that touches only a face, edge
  or corner hits. One raw unit outside it misses.
- **Origin inside.** The entry is negative, so with `t_min >= 0` the exit is
  reported, `front_face == false`, with the outward normal of the face left.
- **Origin on a face.** Entering gives distance `0`, front face; leaving gives
  distance `0`, back face.
- **Behind the origin.** When the rounded entry and exit are both below `t_min`
  there is no hit; a negative `t_min` lets a caller see them.

## General-Purpose Ray-AABB Solver

`ray_intersects_aabb(ray, aabb)` in `crates/bunny-query/src/ray.rs` returns
`Some((t_enter, t_exit))` or `None`. It is not a trace kernel: it divides, its
ray is a `FixedRay3` with a unit direction, and its coordinates are not
restricted. `bunny_broadphase::intersect_ray` visits a node exactly when this
function returns `Some` for the node's bounds.

**Bounds.** For each axis with a non-zero direction component the two slab
distances are `(bound - origin) / direction`, rounded to the nearest raw unit
with ties to even, exactly as `FixedQ32_32::checked_div` rounds. `t_enter` is
the largest near-slab distance, `t_exit` the smallest far-slab distance. An
axis with a zero component does not bound the interval and rejects the box when
the origin lies outside `min..=max` on that axis. A side that no axis bounds is
`i64::MIN` or `i64::MAX` raw.

**Overflow.** A slab distance that does not fit Q32.32 saturates to `i64::MIN`
or `i64::MAX` raw by its sign. The coordinate difference `bound - origin` is
taken in `i128`, so a difference past `i64` is never saturated before the
division. Such distances arise from a direction component of a few raw units
(for a slab of half-width `k` centred on the origin, any component of at most `2k`
raw units), or from a bound about `2^31` units or more from the origin.

**Hit decision.** When every slab distance fits, the hit is decided from the
rounded distances (`t_enter <= t_exit` and `t_exit >= 0`), so a ray that misses
by less than half a raw unit can report a hit whose bounds are equal or whose
exit is zero. When some slab distance does not fit, the function decides the hit
exactly: every slab distance is an `i128` ratio, compared by
cross-multiplication (all products stay below `2^127`), and only then are the
two bounds rounded and saturated. A saturated bound therefore never turns a miss
into a hit; two entries that both saturate to the maximum are still compared
exactly.

**History.** Before flyingrobots/bunny#211 the function returned `None` whenever
a coordinate difference or slab distance overflowed, so rays through a box were
reported as misses. The change touches only those inputs: every hit reported
before is reported with the same bits, which a differential test against the
retained old implementation checks. The analysis is in
[`docs/design/0006-ray-aabb-slab-overflow.md`](../../design/0006-ray-aabb-slab-overflow.md).

## Closest-Hit Traversal

`closest_hit(bvh, ray, t_max, hit)` returns the primitive the ray hits first
within distances `0..=t_max`, as a `ClosestHit { primitive, distance }`, or
`None` when nothing is hit. `any_hit(bvh, ray, t_max, hit)` returns whether
anything is hit in that range.

`bvh` is a `BvhView`: the node buffer (truncated to the nodes `build_bvh`
used; the root is node 0) and the primitive-index buffer. Both stay owned by
the caller, and the traversal allocates nothing; its stack is a fixed array of
64 entries on the call stack.

The primitive test is a caller closure, so the traversal works for any
primitive type:

- `closest_hit` calls `hit(primitive, limit)`. It returns the hit distance, or
  `None` for a miss. `limit` is the best distance found so far (at first
  `t_max`); a triangle closure passes it to `TraceRay::hit_triangle` as
  `t_max` with `t_min` zero. A reported distance outside `0..=limit` is
  ignored.
- `any_hit` calls `hit(primitive)`, which returns whether the primitive is hit
  within `0..=t_max`. After the first `true` the search ends and the closure is
  not called again.

### Search Order And Pruning

The search is depth first and front to back:

1. The root is pushed with its `box_entry` bound if the ray can reach it.
2. At an interior node, both children are tested with
   `TraceRay::box_entry(child, limit)`. A child the ray cannot reach within
   the current limit is dropped. The farther child is pushed first and the
   nearer one on top, so the nearer one is searched first; when both bounds
   are equal the left child is searched first.
3. A node popped from the stack is skipped when its entry bound is strictly
   greater than the current limit, which may have shrunk since it was pushed.
   A node whose bound equals the limit is still searched.
4. At a leaf, each primitive is tested in the order the primitive-index buffer
   lists it. A hit replaces the best one when its distance is smaller, or equal
   with a lower primitive index. The limit becomes the best distance.

Malformed buffers are reported as `TraversalError::InvalidNodeIndex` or
`TraversalError::InvalidPrimitiveRange` when the search reaches the malformed
node; a malformed node the search prunes is not inspected. A search that needs
more than its 64 stack entries returns `TraversalError::StackOverflow`; which
trees can need that is stated in
[Depth And The Traversal Stack](#depth-and-the-traversal-stack). An empty
node buffer is a miss. A buffer that is not a tree is bounded as described
next.

### Malformed Buffers

`BvhNode` buffers are public data, so a caller can hand any of the four plain
traversals (`intersect_aabb`, `intersect_ray`, `closest_hit`, `any_hit`) a
buffer whose child index names an ancestor (a cycle) or in which two interior
nodes name the same child pair (a shared subtree). Every index can be in range
and the stack need never fill, so without a bound a cycle is searched forever.

Each traversal therefore allows at most `nodes.len() / 2` interior visits and
returns `TraversalError::InvalidNodeIndex` when a visit would exceed them. The
count is a countdown in a local of the search loop. The box queries spend it
after an interior visit has pushed its children, so a full stack still reports
`StackOverflow` first; `closest_hit` and `any_hit` spend it when they pop an
interior node.

**Why a tree never reaches the bound.** In a tree every node has at most one
parent and the root has none, so a search reaches each node at most once, and
`i` interior visits whose children are all inside the slice name `2i + 1`
distinct nodes: `2i + 1 <= n`, so `i <= n / 2`. A child outside the slice ends
the search with the error it always had. The box queries pop the right child
next, which is out of range whenever either child is (the children are `first`
and `first + 1`), with no callback in between. In `closest_hit` and `any_hit`
the count can run out only at an interior node whose children are not both in
range (the visits before it already name every node), and that visit would
return the same `InvalidNodeIndex` before calling anything. So on every buffer in
which no node is reached twice, including ones whose only faults are
out-of-range indices, bad leaf ranges or a depth past 64, the results, errors
and callback calls are exactly those of the functions before
flyingrobots/bunny#269, which a differential test against frozen copies checks.

**What a buffer that is not a tree returns.** A cycle is `InvalidNodeIndex`
after at most `n / 2` interior visits, with the leaf callbacks those visits
reached. A cycle the stack used to overflow on first now returns
`InvalidNodeIndex` instead of `StackOverflow`. A shared subtree is searched as
the tree it unfolds into while the unfolding needs at most `n / 2` interior
visits (a leaf shared by two parents is then reported twice) and is
`InvalidNodeIndex` beyond that; with a query that reaches every node this is
exactly the rule by which `compact_bvh` returns `NotATree`. The bound limits
work; it is not a check that the buffer is a tree, and it does not inspect
nodes the search does not reach. `InvalidNodeIndex` does not say whether the
index was out of range or the bound ran out.

### Depth And The Traversal Stack

The depth of a tree is the number of edges from the root to its deepest leaf:
the root is at depth 0, and a tree of depth `d` has `d + 1` levels.

All six searches (`intersect_aabb`, `intersect_ray`, `closest_hit`,
`any_hit`, `compact_closest_hit` and `compact_any_hit`) keep a fixed stack of
64 entries. Every entry waiting on the stack is the other child of a node on
the path to the node being visited, at most one per level, so a visit at depth
`k` that pushes both children holds at most `k + 2` entries. The deepest
interior node of a tree of depth `d` is at depth `d - 1`, so:

- **A tree of depth at most 63 (64 levels) is always searched completely.**
  It needs at most 64 entries.
- **A tree of depth 64 or more may not be.** A search returns
  `TraversalError::StackOverflow` at the first push that would need a 65th
  entry. Whether that happens depends on the tree's shape and the query: a
  comb whose deeper level is searched first keeps one waiting leaf per level
  and overflows at depth 64 in every search (TRACE-TP-412), while
  `closest_hit` on a 90-level tree completes along a ray that meets each
  split-off leaf before the rest of its level (BVH-TP-024). `closest_hit` and `any_hit` and their compact forms push only the
  children the ray can enter within the limit.

`StackOverflow` ends the search with no partial result. On any tree, then,
each search returns its complete answer or an error, never a different
answer. `build_bvh_sah` builds no tree deeper than 60, so every search holds
its trees; `build_bvh` has no depth cap and can exceed 63 even inside the
trace domain
([BVH construction](../bvh-construction/README.md#depth)).

### Correctness: The Answer Does Not Depend On The Tree

The result is the hit a test of every primitive would choose: of all
primitives whose test reports a distance in `0..=t_max`, the one with the
smallest distance, and among those the lowest primitive index. That holds for
every tree over the same primitives whose depth the stack holds (at most 63;
see [above](#depth-and-the-traversal-stack)), whatever its shape and whatever
order its leaves list primitives in. A deeper tree gives the same answer or
`StackOverflow`. The argument, for a tree whose node bounds contain
the bounds of every primitive below them (as `build_bvh` guarantees) and a
closure that reports `floor(T)` for a hit at true distance `T` (as
`hit_triangle` does):

1. **The box bound is a lower bound.** `box_entry` returns an integer raw
   value at or below the exact entry distance of the box
   ([Conservative Box Test](#conservative-box-test)). Its computed slab upper
   bounds are above the exact exits, so a box the ray reaches is rejected only
   when that lower bound exceeds the distance limit.
2. **A primitive inside a node is no nearer than the node.** The hit point at
   true distance `T` lies in the primitive's box, so in every ancestor's box;
   the ray is inside that box at `T`, so the exact entry `E` is at most `T`.
   The computed bound `b` therefore satisfies `b <= E <= T`.
3. **The floor keeps the order.** The reported distance is `floor(T)`, and `b`
   is an integer at most `T`, so `b <= floor(T)`: the floor of a value that is
   at least an integer bound is itself at least that bound.
4. **Ties are kept.** Let the winner be the primitive the full test chooses,
   with reported distance `d`. Until it is found, the limit is at least `d`
   (the limit only ever becomes the distance of a hit that was accepted, and no
   accepted hit beats the winner). Every ancestor of the winner has bound
   `b <= d <= limit`, and a node is dropped or skipped only when its bound is
   strictly greater than the limit, so every ancestor is searched and the
   winner is tested. Its test reports `d <= limit`, and no hit already kept can
   beat `(d, index)`, so it is kept; nothing found later beats it either.
5. **Nothing better is invented.** Every kept hit was reported by the closure
   within `0..=t_max`, so the result is never a primitive the full test would
   not consider.

`any_hit` runs the same search with the limit held at `t_max` until the first
hit, which ends it. Every ancestor of a primitive hit at true distance `T` with
`floor(T) <= t_max` has bound `b <= floor(T) <= t_max` (steps 1 to 3), so it is
searched: `any_hit` returns `true` exactly when testing every primitive would
find a hit.

**What the caller must guarantee.** The claim needs three things of the
closure and the tree:

- every reported distance is at least the floor of some distance at which the
  ray is inside the primitive's box (for `any_hit`: a primitive is reported
  hit only if the ray is inside its box at a distance whose floor is at most
  `t_max`), and the tree's node bounds contain the box of every primitive
  below them;
- the closure answers the same way for a primitive whatever `limit` it
  receives, except that it may return `None` (or a distance above the limit)
  for hits beyond the limit, as `hit_triangle` does;
- it has no other dependence on call order (it is a function of its inputs).

A closure that reports distances larger than the floor of the true distance
(a conservative overestimate) still satisfies the first condition. A closure
that reports a distance below the box bound of a primitive's subtree breaks
step 4: that primitive is found only if the search happens to reach its leaf
before the limit drops below the subtree's bound, so the answer can depend on
the tree's shape and differ from the full test. It is still a primitive the
closure reported within `0..=t_max`, and the traversal remains deterministic
for a fixed tree.

### Compact Layout

`compact_bvh(nodes, out)` converts a tree that `build_bvh` filled into a second
layout in which each interior node holds both of its children's boxes, so a
search step reads one node and tests two boxes from one place:

- **`CompactNode`** is `bounds: [FixedAabb3; 2]` and
  `children: [CompactChild; 2]`, left then right: 112 bytes, 8-byte aligned.
- **`CompactChild`** is `first` and `prim_count`. A leaf (`prim_count > 0`)
  stores its primitive range in the parent, so leaves have no node of their
  own and a leaf visit reads no node. An interior child's `first` is the index
  of its `CompactNode`.
- **`CompactRoot`** is the root's box and the root itself as a `CompactChild`:
  compact node 0 for an interior root, or a leaf for a one-leaf tree, which
  then has no compact node at all. An empty tree converts to `None`.

A tree of `n` nodes has `(n - 1) / 2` interior nodes and needs that many
compact nodes; the scene's 55 277 nodes of 56 bytes (3.10 MB) become 27 638
compact nodes of 112 bytes (3.10 MB), because each box is still stored once
(in its parent) and the leaves' ranges move into their parents. The nodes are
written to the front of `out` in depth-first order, each node followed by its
left subtree and then its right subtree. The conversion is integer-only,
copies bounds and ranges without arithmetic, and writes the same bytes on
every target. It returns the root and the number of nodes written, or a
`CompactError`: `InvalidNodeIndex` when a child index is outside the node
slice, `OutputTooSmall` when `out` is shorter than the tree needs, `NotATree`
when more interior nodes are reached than a tree of that size has (a node
reached twice), and `TooDeep` when more than 64 interior subtrees wait to be
written at once. The conversion writes each node's left subtree first and
queues its interior children; at most one subtree waits per level above the
node being written, so a tree of depth `d` queues at most `d`, and every tree
of depth at most 64 converts. A tree of depth 65 can be `TooDeep` (a spine
whose right child at every level is an interior node), and a deeper one need
not be (a comb whose right child is the next level converts at depth 101):
`TooDeep` is about waiting subtrees, not depth alone. A tree of depth 64
converts although its searches may overflow; the conversion does not check
that the searches hold the tree. Leaf primitive ranges are copied
unchecked and are checked by the search when it reaches them, as
`closest_hit` checks them.

`compact_closest_hit` and `compact_any_hit` take a `CompactView` (the root,
the compact nodes and the source tree's primitive-index buffer) and run the
[search above](#search-order-and-pruning) step for step: the same box tests
in the same order, the nearer child first with ties to the left, the same
pop-time skip, the same leaf order and the same acceptance rule. A stack
entry carries the child's reference with its entry bound. On the tree
`compact_bvh` made from a source tree, both therefore call the primitive test
with the same primitives and limits in the same order as `closest_hit` and
`any_hit` on the source tree, and return the same result; the correctness
argument above applies unchanged, because every node box is the source box.
Errors are the same `TraversalError` values: `InvalidNodeIndex` for a
reference to a missing compact node, `InvalidPrimitiveRange` for a leaf range
past the buffer, `StackOverflow` past 64 entries. Because `CompactView`'s
fields are public, a hand-built compact tree can contain a reference cycle
(a child that names an ancestor); a search that pushes only one child per
visit would then loop forever without filling the stack. A tree visits each
node at most once, so the search counts interior visits and returns
`InvalidNodeIndex` once they exceed the number of compact nodes, the
counterpart of the [bound on the plain traversals](#malformed-buffers).

The layout is a second representation to keep in step with the first: a
change to the search order or pruning rule must be made in both
`closest_hit.rs` and `compact_hit.rs`. Its speed is recorded in the
performance chapter's ledger.

### Counting Traversals

`compact_closest_hit_counted` and `compact_any_hit_counted` run the compact
searches and also say how much work the search did:

```rust,ignore
pub fn compact_closest_hit_counted<F>(
    bvh: CompactView<'_>, ray: &TraceRay, t_max: FixedQ32_32, hit: F,
) -> Result<(Option<ClosestHit>, TraversalCounts), TraversalError>
where F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>;

pub fn compact_any_hit_counted<F>(
    bvh: CompactView<'_>, ray: &TraceRay, t_max: FixedQ32_32, hit: F,
) -> Result<(bool, TraversalCounts), TraversalError>
where F: FnMut(u32) -> bool;
```

They return exactly what `compact_closest_hit` and `compact_any_hit` return and
call `hit` with the same primitives and limits in the same order, on every
view, malformed ones included. On an error they return the same
`TraversalError` and no counts: the counts of a search that met a malformed
node describe no answer, and a caller tracing a tree `compact_bvh` built never
sees an error.

**One loop.** Both forms run one search loop, generic over a private tally:
the uncounted functions instantiate it with a zero-sized tally whose methods
are empty, the counted ones with `TraversalCounts`. There is no second copy of
the search to keep in step; the pruning rule, order and visit bound are the
same code. On the uncounted functions, `compact_closest_hit` measured
unchanged and `compact_any_hit` 6 % to 7 % faster with the shared loop, against
about 2 % with one codegen unit or a separate copy of the loop; the extra
4 to 5 % appears only with the shared loop, and its cause was not established
(the performance chapter's [contract change cost
check](../performance/README.md#counting-compact-traversals-flyingrobotsbunny335)).

**What is counted.** `TraversalCounts` is a `Copy` record of `u64` counters,
each incremented at one place of the loop:

| Field | Counted when |
| --- | --- |
| `box_tests` | `TraceRay::box_entry` is called: once for the root's box, and twice per interior visit (both children's boxes sit in the parent node). |
| `box_entries` | such a call returns a bound, so the child is pushed. |
| `popped_skipped` | a popped entry is dropped because its bound now exceeds the limit (the limit shrank after the push). |
| `interior_visits` | an interior child is popped and searched. |
| `leaf_visits` | a leaf is popped and its primitive range read. |
| `primitive_tests` | `hit` is called. |
| `peak_stack` | after each push: the most entries the stack held at once (at most 64). |

In both searches `box_tests = 1 + 2 * interior_visits` for a non-empty tree.
A closest-hit search pops every entry it pushes, so also `box_entries =
popped_skipped + interior_visits + leaf_visits`; an any-hit search stops at
its first accepted primitive and leaves the rest of the stack unpopped. An empty
tree counts nothing; a ray that misses the root box counts one box test.
These are the definitions of the benchmark profile's replica (`box_calls`,
`box_some`, `skipped_at_pop`, `interior_visited`, `leaf_visited`, `tri_tests`,
`stack_peak` in `crates/bunny-perf/src/profile.rs`), which counts on the
`BvhNode` tree: a compact tree makes the same box tests in the same order, and
`TRACE-TP-418` checks that every count and answer agrees with the replica on
all 1 024 benchmark rays (20 238 box tests over the `closest_hit` searches, the
profile table's 19.76 per ray). The profile's "nodes popped" is
`popped_skipped + interior_visits + leaf_visits`.

**Adding counts.** `+` and `+=` sum each counter, saturating at `u64::MAX`,
and keep the larger `peak_stack`, so a sum over rays is a total per counter
and the deepest stack of any ray. The record is `#[non_exhaustive]`: read its
fields, start a sum from `TraversalCounts::default()`.

**Overflow.** Within one search, interior visits are at most `nodes.len()`
(the visit bound), box tests at most `1 + 2 * nodes.len()`, and entries,
skipped pops and leaf visits at most the box tests. A slice of 112-byte
compact nodes has fewer than `2^57` elements, so none of these can reach
`2^64`. Primitive tests are at most `leaf_visits * prim_indices.len()` on a
malformed tree whose leaves share ranges (on a tree each primitive is tested
at most once), which is bounded below `2^64` only by the time `2^64` calls of
`hit` take; every increment saturates, so even that case is defined.

**Which to call.** Use the uncounted functions for rendering. Use the counted
ones where the work per ray is the output: statistics, a per-pixel cost view,
or comparing trees. Only the compact layout has counted forms. A caller with a
`BvhNode` tree can convert it with `compact_bvh` and count there, because the
compact search makes the same box tests, visits and primitive calls as
`closest_hit` and `any_hit`; counted forms of the plain traversals would add a
second generic loop to keep for no count the compact forms cannot give.

### Example

The rustdoc of `closest_hit` builds a BVH over two triangles with `build_bvh`
and traces a ray with a closure over `TraceRay::hit_triangle`.
