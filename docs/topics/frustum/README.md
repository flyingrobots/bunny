# Frustum Classification

This chapter defines Bunny's fixed-point frustum: six planes, the classification of points,
boxes and spheres against them, and the extraction of the planes from a clip matrix.

The surface lives in `bunny-geom`: `FixedFrustum`, `FrustumClass` and `FrustumSphereError`.
It is built on the plane rules of [plane clipping](../plane-clipping/README.md):
`FixedPlane3::signed_distance`, `classify`, `classify_aabb` and `PlaneSpan`, which it does
not change.

## Status

This is a living topic chapter, not a proposal. The verification design is
[`test-plan.md`](test-plan.md).

## The Frustum

`FixedFrustum { left, right, bottom, top, near, far }` holds six `FixedPlane3` values.
`FixedFrustum::new([FixedPlane3; 6])` and `planes()` use that order.

Orientation: each plane's front faces into the frustum. The inside is the closed set of
points in front of or on all six planes, the same closed front half-space that clipping
keeps, so a point on a face is inside.

The planes are held as given. Nothing checks that they bound a region, that the region is
non-empty, or that opposite planes face each other. An empty set of six planes (for
example `x >= 1` and `x <= -1`) is a valid value with every point outside; six planes with
a zero normal and a negative offset are a valid value with every point inside. The names
are slots, not a promise that `left` is to the left of anything. Planes need not be unit
length and are never normalised; a signed distance is then the true distance times the
length of the normal.

## Classification

`FrustumClass` has three outcomes:

| Outcome | Meaning |
| --- | --- |
| `Inside` | Every plane has the whole shape in its closed front half-space. |
| `Outside` | Some plane has the whole shape behind it. |
| `Intersecting` | Neither: no plane has the shape behind it, and at least one is straddled. |

The six per-plane answers are `PlaneSpan` values (`Front`, `Behind`, `Straddling`,
`Coplanar`) or `None` for an overflowing signed distance. They combine by one rule, in this
order:

1. any plane `Behind`: `Outside`;
2. otherwise any plane `None`: no answer (`None`, or `Err(Overflow)` for spheres);
3. otherwise any plane `Straddling`: `Intersecting`;
4. otherwise (every plane `Front` or `Coplanar`): `Inside`.

Rule 1 comes first because a `Behind` plane is certain on its own: the shape has no point in
front of that plane, whatever the other planes give. A culler that reads `Outside` never
discards a shape it should keep, even when another plane overflowed; one that reads `None`
must keep the shape or decide in a wider domain. Evaluation order is not observable: every
plane is evaluated and the rule reads all six.

Boundary contact, which follows `PlaneSpan`'s rule that a shape touching a plane without
crossing it takes the side of its other points:

- touching a face from inside (a corner, an edge, a face or a sphere's point on the plane,
  the rest inside) is `Inside`; a flat box lying in a face is `Coplanar` with it, and so
  inside;
- touching a face from outside (the rest behind it) is `Outside`. The shape and the closed
  frustum can share the touching points, so `Outside` means "no point in front of the
  culling plane", not "no common point".

`contains_point(p)` is the same rule at one point, through `classify`: `Some(true)` for
inside, `Some(false)` when some plane has the point `Behind` (even if another overflows),
`None` otherwise on overflow.

Classification is exact in the rounded signed distances, as `classify` is; it is not the
classification of the real-valued shape against real-valued planes.

## Boxes

`classify_aabb(FixedAabb3)` takes each plane's `FixedPlane3::classify_aabb`, which equals
the point rule at the box's eight corners, `None` included, and combines them by the rule.

- `Outside` is exact: the culling plane has all eight corners `Behind` or `On` and at least
  one `Behind`, and every representable point of the box lies between its two extreme
  corners' distances (plane clipping, Boxes).
- `Inside` is exact: every corner, and so every representable point of the box, is in
  front of or on every plane.
- `Intersecting` is conservative. A box can be in front of or straddling every plane and
  still meet no point of the frustum, when it sits outside near an edge or a corner. For
  the view-space pyramid below, the box `x` in `[-5, -7/2]`, `y` in `[-1/2, 1/2]`, `z` in
  `[-4, -2]` straddles the left plane `x >= z` and the far plane `z >= -3`, but a frustum
  point has `x >= z >= -3` and the box has `x <= -7/2`. The six-plane test reports
  `Intersecting`. This never culls a visible box; it keeps some invisible ones.

An exact box-frustum intersection test needs the frustum's edges and a separating-axis
test; it is not provided (see the last section).

An inverted box (`min > max` on an axis) is the box with the same eight corners, and a
box with `min == max` is one point and gets `contains_point`'s answer, both inherited from
`classify_aabb`. `FixedAabb3::try_new` remains the way to refuse inverted bounds.

## Spheres

`classify_sphere(FixedSphere3)` returns `Result<FrustumClass, FrustumSphereError>`.

Against one plane, with raw values (value times `2^32`) `D` the rounded signed distance of
the centre from `signed_distance`, `R` the radius and `N` the normal, the true distance is
`d / |n|`, so the sphere is clear of the plane when `d^2 >= r^2 |n|^2`, which in raw units is

```text
D^2 * 2^64  >=  R^2 * (Nx^2 + Ny^2 + Nz^2)
```

When it holds, the sphere is `Behind` for `D < 0` and `Front` for `D >= 0`; when it does not,
the sphere is `Straddling`. A sphere touching the plane (equality) takes the side of its
centre, which is the side of its other points: from inside it is `Front`, from outside
`Behind`. The comparison needs no square root and no normalisation, and it is exact in the
rounded `D`: the only rounding is `signed_distance`'s, at most 3/2 raw units from the exact
distance of the centre.

Widths: `D^2 <= 2^126`, so the left side is at most `2^190`; `R^2 < 2^126` and the sum of
squares is at most `3 * 2^126 < 2^128`, so the right side is below `3 * 2^252`. Both sides
are held as 256-bit `(high, low)` pairs of `u128` and compared lexicographically; nothing
overflows, so the only overflow is the centre distance.

The six answers combine by the same rule. `Inside` and `Outside` are exact in the rounded
centre distances. `Intersecting` is conservative as for boxes: the sphere at `(-4, 0, -4)`
with radius 5/4 straddles the pyramid's left and far planes, but the frustum point nearest
its centre is `(-3, 0, -3)`, at distance `sqrt 2 > 5/4`.

Radius rules, following `FixedSphere3::try_new`: a zero radius is the point rule, exactly
`contains_point`; a negative radius is `Err(NegativeRadius)`, checked before any plane, since
it describes no set. `Err(Overflow)` means a centre distance overflowed and no plane had
the sphere behind it.

## Extraction From A Clip Matrix

`FixedFrustum::from_clip_matrix(clip_from_space)` returns the frustum of a `clip_from_space`
matrix, in that space: `clip_from_view` gives the view-space frustum, `clip_from_world`
(the projection times `view_from_world`) the world-space one.

The coordinate law puts visible points at clip coordinates with `-w <= x <= w`,
`-w <= y <= w` and `0 <= z <= w` (depth `0` at near, `1` at far). With rows `r0` to `r3` of
the matrix, each clip coordinate is a row times `(p, 1)`, so the six inequalities are rows:

| Plane | Inequality | Row |
| --- | --- | --- |
| left | `x + w >= 0` | `r3 + r0` |
| right | `w - x >= 0` | `r3 - r0` |
| bottom | `y + w >= 0` | `r3 + r1` |
| top | `w - y >= 0` | `r3 - r1` |
| near | `z >= 0` | `r2` |
| far | `w - z >= 0` | `r3 - r2` |

The near plane is `r2`, not `r3 + r2`: that is the `[-1, 1]` depth range, which is not
Bunny's. A row `(a, b, c, d)` is the plane with normal `(a, b, c)` and offset `-d`, so its
front is the inequality's side. Left and right together give `w >= 0`.

Each coefficient is an exact sum or difference of two entries, by `checked_add` or
`checked_sub`, and the offset is `checked_neg` of `d`. Nothing is rounded and nothing is
normalised. `None` exactly when a sum leaves the `i64` raw range or `d` is `i64::MIN`; a
`d` of `2^63` is refused even though its negation would fit, because `d` is formed first.

For the projection `try_perspective(1, 1, 1, 3)` (tangent, aspect, near, far) the planes are
`x - z >= 0`, `-x - z >= 0`, `y - z >= 0`, `-y - z >= 0`, `-3/2 z - 3/2 >= 0` (that is
`z <= -1`) and `z / 2 + 3/2 >= 0` (`z >= -3`), the pyramid used above.

### What Extraction Guarantees

Let `E` be the exact value of a plane's inequality at a point `p` under the stored matrix
entries (the exact clip coordinates of `p`, with no rounding). The plane's coefficients are
exact, so `E` is also the exact value of `normal . p - offset`. `signed_distance` rounds
three products, each by at most half a raw unit, and adds exactly, so the computed distance
is within 3/2 raw units of `E`. Therefore:

- when no plane's signed distance overflows, a point with `E > 3/2` raw for all six planes
  is inside;
- a point with `E < -3/2` raw for some plane whose signed distance does not overflow is
  outside, whatever the other planes do;
- when a signed distance overflows and no plane is `Behind`, there is no answer (`None`),
  however clear of the faces the point is. A plane whose distance overflows cannot be the
  one that proves the point outside;
- a point within 3/2 raw units of a face can be classified either way. Faces are not exact.

Example: for the pyramid, one raw unit beyond the far face, `z = -3 - 2^-32`, gives
`(1/2) z` exactly halfway between two raw values; ties to even rounds it to `-3/2`, the
distance is `0`, and the point is inside. Two raw units beyond it is outside. Faces whose
normal components are integers (the four side faces here) have exact products and are
exact.

The guarantee is relative to the matrix as stored. The matrix itself is rounded by its
builders (each entry once, the coordinate law) and by `checked_mul_mat4` (per product), so
the extracted frustum differs from the frustum of the exact camera by that rounding. The
seeded camera test shows the scale: for 1 000 cameras from `try_perspective` or
`try_orthographic` times `try_look_at`, with tangents 1/4 to 4, aspects 1/2 to 2, near 1/16
to 4 and far up to 1004 units, eyes within 100 units, every point built in view space at
least 1/64 of a face's extent inside the volume classifies inside, and every point at least
1/64 beyond one face classifies outside. That margin is a tested property of those
parameter ranges, not a bound.

## Overflow

Every signed distance is `FixedPlane3::signed_distance`, which refuses any product or partial
sum outside `i64`. Its domain is order dependent (plane clipping, Intermediate widths), and
the frustum inherits it unchanged. Nothing saturates and nothing panics.

## What Is Deliberately Not Here

- No renderer policy: no depth-range switch (`[-1, 1]` is converted at a boundary, as the
  coordinate law says), no reversed-depth mode, no cull-mode or face-culling enumeration,
  no API named after a graphics API, no assumption beyond the coordinate law.
- No exact box or sphere test. The six-plane test is conservative; an exact test needs the
  frustum's edges, corner points and a separating-axis test.
- No frustum against frustum. It needs the eight corners, each the intersection of three
  planes, which is a rational point, not a Q32.32 one; an exact version would classify
  those rational corners with exact wide arithmetic. flyingrobots/bunny#146 lists
  "frustum classification" and it remains open for that reading.
- No normalised planes, no plane-coherency or culling-plane index, no corner points.

## Required Tests

The repository keeps constructed point, box and sphere cases on and one raw unit either side
of each face, the conservative false positives, flat, inverted and zero-extent boxes, zero
and negative radii, non-unit normals, overflow with and without a culling plane, extraction
goldens and face ties, and seeded sweeps against independent oracles: the combination rule
over six `classify_aabb` results and an `i128` eight-corner model, a 32-bit-limb sphere
model, `i128` exact clip coordinates for the extraction bound, and seeded cameras through
the repository's builders. They run natively and under WASM through `wasm_bindgen_test`.
