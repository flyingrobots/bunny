# Rotation

`FixedMat3::rotation_x`, `rotation_y` and `rotation_z` build the rotation by a `FixedAngle`
about a coordinate axis, and `FixedMat3::rotation_axis_angle` the rotation about a
`FixedUnitVec3` axis by Rodrigues' formula. The contract is named `bunny-rotation/1`
(`ROTATION_CONTRACT_ID` in `bunny-linalg`). The only trigonometry is
[`FixedAngle::sin_cos`](../angle-trigonometry/README.md), so no rounded `pi` is involved.

## Convention

The frame and the algebra are those of the [coordinate law](../coordinate-law/README.md):
right-handed, column vectors (`p_target = rotation * p_source`), and positive angles follow
the right-hand rule. That is the sense `FixedAngle` documents about `+Z` (counter-clockwise
seen from `+Z`) and the convention of [`try_look_rotation`](../look-rotation/README.md), whose
forward `+X` with up `+Y` is documented as -90 degrees about `+Y` and equals
`rotation_y` of three quarter turns bit for bit.

A positive quarter turn about a unit axis `a` maps each vector `v` perpendicular to it onto
`a x v`. With the coordinate law's `X x Y = Z`, `Y x Z = X` and `Z x X = Y`, with `(s, c) =
angle.sin_cos()`:

| Constructor | Rows | Quarter turn |
| --- | --- | --- |
| `rotation_x(angle)` | `(1, 0, 0)`, `(0, c, -s)`, `(0, s, c)` | `+Y` to `+Z`, `+Z` to `-Y` |
| `rotation_y(angle)` | `(c, 0, s)`, `(0, 1, 0)`, `(-s, 0, c)` | `+Z` to `+X`, `+X` to `-Z` |
| `rotation_z(angle)` | `(c, -s, 0)`, `(s, c, 0)`, `(0, 0, 1)` | `+X` to `+Y`, `+Y` to `-X` |

Composition is not part of this contract: `checked_mul_mat3` composes right to left as it
always has, and its products round.

## Axis Rotations

The entries are the `sin_cos` values copied, with `-s` exact (both are within `[-1, 1]`), so
each entry is within one raw unit of the exact sine or cosine, the bound `sin_cos` gives, and
nothing is rounded here. `sin_cos` is exact at the cardinal angles, so every multiple of a
quarter turn (and one angle unit either side of it) gives entries of exactly `0`, `1` and
`-1`, and angle zero is exactly the identity. `rotation_*(-a)` is exactly the transpose of
`rotation_*(a)`, because `sin_cos` is odd and even bit for bit.

Fixed point cannot make the matrix exactly orthonormal. With dot products taken as the exact
sum of raw products divided by `2^32`:

- the dot products between rows, and between columns, are exactly zero (`cs - sc`);
- each row and column that is not the axis has length `sqrt(c^2 + s^2)`; since `c^2 + s^2 - 1
  = 2(a cos + b sin) + a^2 + b^2` for errors `|a|, |b| <= 2^-32`, it is within `2 sqrt 2 +
  2^-31` raw of one, so the length is within 3/2 raw of one;
- the determinant is exactly `c^2 + s^2`, within 3 raw of one.

## Axis-Angle Rotation

`rotation_axis_angle(axis, angle)` is Rodrigues' formula `c I + s [u]x + (1 - c) u u^T` on
the axis `u` as given; it is not renormalized. On raw values (`t = 2^32 - c`), each entry is
one exact `i128` numerator over `2^64`:

| Entries | Numerator |
| --- | --- |
| `m00`, `m11`, `m22` | `c 2^64 + t u_i u_i` |
| `m01`, `m10` | `t x y - s z 2^32`, `t x y + s z 2^32` |
| `m02`, `m20` | `t x z + s y 2^32`, `t x z - s y 2^32` |
| `m12`, `m21` | `t y z - s x 2^32`, `t y z + s x 2^32` |

Each is rounded once to nearest, ties to even (the
[numeric constitution's rule](../NUMERIC_CONSTITUTION.md#rounding-policy)). With `|c|, |s|
<= 2^32`, `t <= 2^33` and every axis component below `2^32 + 2` raw, every term is below
`2^99`, so the sum fits `i128`; every entry is below `2^35` raw, so nothing overflows and the
function has no error: every `FixedUnitVec3` and every angle give a matrix.

Exact cases: angle zero gives exactly the identity for every axis (`c = 1`, `s = 0`, `t =
0`); the negated angle gives exactly the transpose (the same numerators with the sign of `s`
swapped); and the six constants `UNIT_X` to `NEG_UNIT_Z` give exactly the axis rotations,
`rotation_x(a)` about `UNIT_X` and `rotation_x(-a)` about `NEG_UNIT_X`, and so on. So
`rotation_axis_angle(UNIT_Y, a) == rotation_y(a)` for every `a`.

**Error bound against the formula.** The `sin_cos` errors enter an entry with weights
`1 - u_i^2` (diagonal) and `|u_i u_j| + |u_k|` (off it), each below `1 + 2^-30` for a
`FixedUnitVec3`, and the rounding adds 1/2 raw. Each entry is within `3/2 + 2^-30` raw of the
formula on the given axis at the exact angle.

**The axis length dominates.** A `FixedUnitVec3` is accepted when the floor square root of its
raw sum of squares is within one raw unit of `2^32`, so its length is at least `1 - 2^-32`
and below `1 + 2^-31`, and `|u|^2 - 1` is below `4 + 2^-30` raw. The formula is a rotation only when `|u| = 1`; its
`(1 - c) u u^T` term grows with `|u|^2`, so each entry is within `19/2 + 2^-28` raw of the exact
rotation about the axis direction. At the half turn about the longest axis along `x`, `(2^32 +
1, 92681, 0)` raw, `m00 = -1 + 2 x^2` rounds to `1 + 4` raw.

### Orthonormality And Determinant

With `sigma = c^2 + s^2` and `rho = |u|^2 - 1`, the unrounded matrix `M` satisfies `M M^T =
(sigma + s^2 rho) I + ((1 - sigma) + t^2 rho) u u^T`, and `M^T M` is the same (it is `M` at
`-s`), with `det M = (1 + t rho)(sigma + s^2 rho)`. Bounding `|sigma - 1| <= 2 sqrt 2` raw,
`|rho| < 4 + 2^-30` raw and the rounding by 1/2 raw per entry gives, in raw units:

| Quantity | Proven | Contract | Seeded maximum |
| --- | --- | --- | --- |
| Length of each row or column | below 10.3 of one | within 11 | 8.16 |
| Dot product of two rows, or of two columns | below 11.2 of zero | within 12 | 8.07 |
| Determinant (exact on the raw entries) | below 14.5 of one | within 15 | 10.54 |

The seeded maxima are from the 10 000 cases of the test plan, whose axes cover the whole
`FixedUnitVec3` band. Almost all of each bound is the `rho` term, which comes from the axis length the type
allows, not from the rounding: about the six exactly unit axis constants the matrices are the
axis rotations, with the bounds of [Axis Rotations](#axis-rotations).
