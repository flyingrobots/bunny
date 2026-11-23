# Transform Composition

`FixedAffine3::from_trs`, `FixedAffine3::rotate_about_point` and
`FixedAffine3::scale_about_point` build the affine transforms that local and world space need:
translate, rotate and scale in a fixed order, and a rotation or scale about a pivot. The
contract is named `bunny-transform/1` (`TRANSFORM_CONTRACT_ID` in `bunny-linalg`). The
constructors return the existing `FixedAffine3`; there is no separate transform type.

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
[numeric constitution's rule](../NUMERIC_CONSTITUTION.md#rounding-policy)), by
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

## Limitations

These constructors transform points and vectors only. Under a non-uniform scale, a surface
normal needs the inverse transpose of the linear part; no normal transform is provided. There
is no decomposition of a `FixedAffine3` back into translation, rotation and scale, and no
inverse beyond the general `FixedAffine3::try_inverse`. A composed linear part is not exactly
a rotation times a scale: its entries carry the rounding above.
