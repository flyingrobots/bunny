# Transform Composition

`FixedAffine3::from_trs`, `FixedAffine3::rotate_about_point` and
`FixedAffine3::scale_about_point` build the affine transforms that local and world space need:
translate, rotate and scale in a fixed order, and a rotation or scale about a pivot. The
contract is named `bunny-transform/1` (`TRANSFORM_CONTRACT_ID` in `bunny-linalg`). The
constructors return the existing `FixedAffine3`; there is no separate transform type. The
[ray, box and normal transforms](#rays-boxes-and-normals) carry rays, axis-aligned boxes and
surface normals through a `FixedAffine3`, and [non-uniform scale](#non-uniform-scale) explains
what it does to normals and bounds.

## Convention

The conventions are those of the [matrix types](../matrix-types/README.md) and the
[coordinate law](../coordinate-law/README.md): column vectors, row-major `mRC` entries,
right-to-left composition (`outer * inner` applies `inner` first), points translated and
vectors not. Rotations come from [`bunny-rotation/1`](../rotation/README.md) or
[`try_look_rotation`](../look-rotation/README.md), with the right-hand rule; any `FixedMat3`
is accepted.

`from_trs(translation, rotation, scale)` is `T * R * S`. A point is **scaled first, then
rotated, then translated**: `p' = rotation * (scale * p) + translation`, so the linear part is
`rotation * diag(scale)`.

Worked example: `from_trs((1, 0, 0), rotation_y(quarter), (2, 3, 4))`. The quarter turn about
`+Y` has rows `(0, 0, 1)`, `(0, 1, 0)`, `(-1, 0, 0)`; it maps `+X` to `-Z`. The point `(1, 0,
0)` is scaled to `(2, 0, 0)`, rotated to `(0, 0, -2)` and translated to `(1, 0, -2)`. The
other order, rotate then scale, would rotate it to `(0, 0, -1)`, scale `z` by 4 to `(0, 0,
-4)` and give `(1, 0, -4)`. A uniform scale commutes with a rotation, so only a non-uniform
scale tells the orders apart.

`rotate_about_point(pivot, rotation)` is `T(pivot) * R * T(-pivot)` and
`scale_about_point(pivot, scale)` is `T(pivot) * S * T(-pivot)`: move the pivot to the origin,
rotate or scale, move it back. `rotate_about_point((1, 0, 0), rotation_y(half))` has
translation `(2, 0, 0)` and maps `(2, 0, 0)` to the origin and the pivot to itself.

## Rounding And Bounds

Rounding is to nearest, ties to even (the
[numeric constitution's rule](../../NUMERIC_CONSTITUTION.md#rounding-policy)), by
`FixedQ32_32::checked_mul`: the exact `i128` product of two raw values (below `2^126`)
divided by `2^32` and rounded once.

| Constructor | Linear part | Translation |
| --- | --- | --- |
| `from_trs` | `rotation.mij * scale_j`: one rounding, within 1/2 raw of the exact product | `translation`, copied |
| `rotate_about_point` | `rotation`, copied | `pivot - rotation * pivot`: three roundings, within 3/2 raw of the exact value |
| `scale_about_point` | `diag(scale)`, copied | `pivot_i - scale_i * pivot_i`: one rounding, within 1/2 raw of the exact value |

`rotation * pivot` is `FixedMat3::checked_mul_vec3`: each of the three products rounded once
and the sum exact. `scale_about_point` is `rotate_about_point` with the diagonal matrix, whose
zero products are exact. Because the translation subtracts exactly what
`checked_transform_point` adds back, **the pivot maps onto itself exactly** under
`checked_transform_point`, for every input the constructors accept.

The bounds above are against the exact product of the given entries. Against an exact
rotation they add the rotation's own error, weighted by what multiplies it. If each entry of
`rotation` is within `e` raw of the exact rotation (`e = 1` for the axis rotations, `19/2 +
2^-28` for `rotation_axis_angle`; see [Rotation](../rotation/README.md)), then each `from_trs`
entry is within `|scale_j| e + 1/2` raw of the exact `R s_j`, and each `rotate_about_point`
translation component within `3/2 + e (|p_x| + |p_y| + |p_z|)` raw, with `|s_j|` and `|p_j|`
the real values. Nothing tighter follows from the inputs.

## Refusals

Each constructor returns `None` on overflow and has no other refusal. Zero and negative scales
are accepted: a zero gives a singular linear part (its `try_inverse` is `None`), a negative
one a reflection.

- `from_trs`: `None` when a rounded entry `rotation.mij * scale_j` is outside `[i64::MIN,
  i64::MAX]` raw. A rotation entry of at most one in magnitude never overflows, except `-1`
  times `scale_j = i64::MIN` raw, which rounds to `2^63`; `i64::MIN + 1` gives `i64::MAX`. The
  longest-axis half turn of [Rotation](../rotation/README.md#axis-angle-rotation) has `m00 = 1
  + 4` raw, and its threshold is `scale_x = 0x7FFF_FFFE_0000_0007` raw (`m00 = i64::MAX`; one
  more is `None`) and `-0x7FFF_FFFE_0000_0008` (`m00 = i64::MIN`; one less is `None`). The
  translation is copied, so every translation is accepted.
- `rotate_about_point` and `scale_about_point`: `None` when a rounded product, a partial sum of
  `checked_mul_vec3`, or the subtraction is outside the range. With `m00 = -1` (the half turn
  about `+Y`, or the scale `-1`) the translation is `2 pivot_x`: `pivot_x = 2^62 - 1` raw gives
  `2^63 - 2`, `2^62` is `None`, `-2^62` gives `i64::MIN` and `-2^62 - 1` is `None`. With the
  scale 2, the product `2 pivot_x` is the first to leave the range, at the same pivots.

No constructor panics.

## Composition

Composing these transforms uses the existing `checked_mul_affine`, which is not changed. Each
entry of `A.checked_mul_affine(B)` is within 3/2 raw of the exact product of the given
entries: three rounded products, summed exactly, for both the linear entries and the
translation `A.linear * B.translation + A.translation`.

Fixed-point products are not associative, but the two groupings stay close to the exact
product `A B C` of the given entries. With `|.|` the real values of the given entries, each
linear entry `(i, j)` and translation component `i` is within, in raw units:

| Grouping | Linear entry `(i, j)` | Translation `i` |
| --- | --- | --- |
| `(A * B) * C` | `3/2 (1 + sum_k abs(C.linear_kj))` | `3/2 (2 + sum_k abs(C.translation_k))` |
| `A * (B * C)` | `3/2 (1 + sum_k abs(A.linear_ik))` | `3/2 (1 + sum_k abs(A.linear_ik))` |

The first rounding error is carried through the next product by the other factor, so the two
groupings agree within the sum of their bounds.

## Rays, Boxes And Normals

Three helpers carry other kinds of geometry through a `FixedAffine3`. They are built on
`checked_transform_point` and `checked_transform_vector`, which are not changed: each output
component is three products, each rounded once to nearest, ties to even, from the exact `i128`
product, summed exactly (plus the translation for a point), so it is **within 3/2 raw of the
exact value**. The point or vector is refused when a rounded product, a partial sum (in the
order `x`, `y`, `z`) or the translation sum leaves the `i64` raw range, so a few inputs whose
finished value would fit are refused. No helper saturates and none panics. They carry no
contract identity of their own.

The ray and box helpers are in `bunny-geom` and return the typed `TransformError`
(`InvertedBox`, `OutOfRange`, `DegenerateDirection`); the normal helper is in `bunny-linalg`
and returns `None`, like the point and vector transforms.

### Rays

`FixedRay3::checked_transform(&transform)` maps the origin as a point and the stored unit
direction as a vector, and returns a `TransformedRay3 { origin, direction }`. The direction is
**not renormalised**: a scale changes its length, so the parameter of a hit keeps its meaning.
For every real `t`, each component of `origin' + t direction'` is within `3/2 (1 + abs(t))` raw
of the exact image of `origin + t direction`: 3/2 raw from the origin and `3/2 abs(t)` from the
direction. The bound is against the stored direction, which `FixedUnitVec3` holds within one raw
unit of unit length.

Worked example: scale `(2, 1, 1)`, then translate by `(1, 0, 0)`. The ray from `(1, 0, 0)` along
`+X` maps to origin `(3, 0, 0)` and direction `(2, 0, 0)`. The original point at `t = 3` is
`(4, 0, 0)`, whose image `(9, 0, 0)` is `origin' + 3 direction'`.

`FixedRay3` requires a unit direction, so the image is a separate type.
`TransformedRay3::try_into_ray` renormalises it with `FixedRay3::try_new`; after that the
parameter no longer carries over (in the example, `t = 3` then reaches `(6, 0, 0)`). Refusals,
checked in this order:

- `OutOfRange` when the origin's point transform or the direction's vector transform is
  `None`. With `m00 = i64::MAX` raw, the unit `+X` (`2^32` raw) maps to exactly `i64::MAX`, and
  the unit vector `(2^32 + 1, 0, 0)` raw, which `FixedUnitVec3` accepts, is `OutOfRange`.
- `DegenerateDirection` when the rounded direction is the zero vector: a singular transform, or
  products that round to zero. With `m00 = 1` raw and every other entry zero, the image of `d_x`
  is `round(d_x / 2^32)` raw: `d_x = 2^31` is a tie and rounds to even, 0, so it is refused;
  `2^31 + 1` gives 1 raw, a nonzero direction, accepted.

### Boxes

`FixedAabb3::checked_transform(&transform)` returns the axis-aligned box that contains the
mapped box: the per-axis minimum and maximum of the **eight corners** mapped by
`checked_transform_point`. It contains every rounded corner exactly, by construction. Against
the exact image, each face is within 3/2 raw of the face of the exact image's axis-aligned hull,
inside or outside it: a box that must contain the exact image needs 2 raw more on each face, and
one that must contain the rounded point transform of every point of the box needs 3 raw more.
This helper adds no padding.

- `InvertedBox` when a minimum is above its maximum on some axis, the rule of
  `FixedAabb3::try_new`, checked before any arithmetic. An equal minimum and maximum is accepted.
- `OutOfRange` when a corner's point transform is `None`.
- A point box gives the point transform exactly, as a point box.

### Normals

`FixedAffine3::checked_transform_normal(normal)` maps a surface normal by the inverse transpose
of the linear part, `L^-T n`; the translation is ignored. The result is `try_inverse()` of the
linear part, transposed, times `n` by `checked_mul_vec3`, so each component is within 3/2 raw of
the exact product of the **rounded inverse** and `n`. The rounded inverse carries `try_inverse`'s
own rounding: rounded minors and determinant, then one division per entry rounded to nearest,
ties to even. That error has no bound in raw units independent of the matrix (it grows as the
determinant shrinks), so no bound against the exact `L^-T n` is claimed. Where the inverse
entries and every product are exact, as for power-of-two scales and signed permutations, the
result is exact.

The result is **not renormalised**; its length changes with the scale. `None` when
`try_inverse` is `None` (its rounded determinant is exactly zero, or the inverse overflows: the
entry `1 / (2 raw)` is `2^63` raw) or when a rounded product or partial sum leaves the range.
The rounded determinant is not the exact one, in both directions. Some nonsingular matrices are
refused: `diag(1, 1, 1)` raw has a minor that rounds to zero. Some exactly singular matrices
are not: raw rows `(2^32, 2^32, 2^32)`, `(2^31, 2^30, 2^29)` and `(12, 10, 9)` have a third
row equal to `8 / 2^32` times the sum of the first two, a rounded determinant of 1 raw, and an
inverse that `try_inverse` returns. A result is therefore not proof that the linear part is
invertible.

## Non-Uniform Scale

A rotation and a uniform scale change lengths by one factor in every direction and keep angles.
A non-uniform scale does not, and that changes how normals and bounds transform.

**Normals.** A normal `n` is perpendicular to every tangent `t` of its surface. The tangent maps
as a vector, `L t`. The normal must map by the inverse transpose, because `(L^-T n) . (L t) = n
. t`, which is zero. Mapping the normal by `L` itself keeps it perpendicular only when `L` is a
rotation times a uniform scale, where `L^-T` is `L` divided by the square of the scale.

Worked example: scale `(2, 1, 1)`. The surface through the origin with normal `(1, 1, 0)` holds
the tangent `(1, -1, 0)`, which maps to `(2, -1, 0)`. The inverse transpose maps the normal to
`(1/2, 1, 0)`; the dot product is `1 - 1 = 0`. The linear part alone gives `(2, 1, 0)`, whose dot
product with the tangent is `4 - 1 = 3`: no longer perpendicular. The inverse transpose also
changes the normal's length (here from `sqrt 2` to `sqrt 5 / 2`), so a caller that needs a unit
normal renormalises it, with `FixedUnitVec3::new` for example. A reflection keeps the outward
side: under the scale `(-1, 1, 1)` the outward normal `+X` maps to `-X`.

**Bounds.** An affine map takes a box to a parallelepiped. Its axis-aligned box is a hull of
the image and in general larger than it. Worked example: an eighth turn about `+Z` (entries `c =
round(2^32 / sqrt 2)` raw, `c - c` and `c + c` exact) maps the unit cube's corners to `x` in
`{0, c, -c, 0}` and `y` in `{0, c, c, 2c}`, so the hull is `(-c, 0, 0)` to `(c, 2c, 1)`: about
`1.414` by `1.414` by 1, twice the cube's volume. Composing several such transforms and taking
the hull after each can grow the box at every step; taking the hull once, of the corners mapped by
the composed transform, does not. A bounding sphere does not stay a sphere under a non-uniform
scale: its image is an ellipsoid, and a sphere that contains it needs the radius times the
largest scale factor (the largest singular value of `L`). No sphere transform is provided.

## Limitations

There is no decomposition of a `FixedAffine3` back into translation, rotation and scale, and no
inverse beyond the general `FixedAffine3::try_inverse`, whose error has no stated bound. A
composed linear part is not exactly a rotation times a scale: its entries carry the rounding
above. The box transform has no padded or faster centre-and-extents form, and spheres,
triangles and meshes have no transform helper.
