# Ray Tracing Kernels

This chapter defines Bunny's current contract for the ray-tracing kernels in
`bunny-query`: a prepared ray, a prepared triangle, a conservative ray-box test
and an exact ray-triangle test.

They are the per-node and per-primitive tests a closest-hit traversal runs
millions of times, so they avoid division wherever the answer allows it. The
general-purpose solvers in `crates/bunny-query/src/ray.rs`
(`ray_intersects_aabb`, `ray_intersects_triangle`) keep their own contracts;
the overflow behaviour of `ray_intersects_aabb`, which
`bunny_broadphase::intersect_ray` uses for every node, is described in
[General-Purpose Ray-AABB Solver](#general-purpose-ray-aabb-solver).

## Status

This is a living topic chapter, not a proposal.

Current behavior is implemented in `crates/bunny-query/src/trace.rs` and
exported as:

- `TraceRay` (`new`, `from_ray`, `origin`, `direction`, `box_entry`,
  `hit_triangle`);
- `TraceTriangle` (`new`, `vertices`, `bounds`);
- `TriangleHit` (`distance`, `front_face`, `barycentrics`);
- `TRACE_COORD_LIMIT_RAW` and `TRACE_DIRECTION_LIMIT_RAW`.

Evidence is listed in the [test plan](test-plan.md). A BVH closest-hit
traversal built on these kernels, and their benchmarks, are not part of this
chapter yet.

All arithmetic is integer arithmetic on raw Q32.32 values (`i64`, widened to
`u128` or `i128`). There is no floating point, no `unsafe` code and no
target-specific path, so every supported target returns the same bits.

## Trace Domain

The kernels accept only values inside a fixed trace domain:

| Value | Bound | Constant |
| --- | --- | --- |
| Ray origin and triangle vertex coordinates | `abs(raw) < 2^40` (below 256.0) | `TRACE_COORD_LIMIT_RAW` |
| Ray direction components | `abs(raw) <= 2^32` (at most 1.0) | `TRACE_DIRECTION_LIMIT_RAW` |

`TraceRay::new` returns `None` when an origin coordinate or a direction
component is outside the domain, or when the direction is zero.
`TraceTriangle::new` returns `None` when any vertex coordinate is outside the
domain. `TraceRay::from_ray` accepts a `FixedRay3`, whose unit direction is
always inside the direction bound, and so returns `None` only for an origin
outside the domain.

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
  and `near <= T` because `abs(d) <= 2^32`. The test checks both the contract
  and the tighter `T / 2^31 + 2`.

The box test needs no division at query time: one multiplication and one shift
per slab plane.

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
