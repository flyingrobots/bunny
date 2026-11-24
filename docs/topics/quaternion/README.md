# Quaternion

`FixedUnitQuat` in `bunny-linalg` is a rotation stored as four Q32.32 components. It is
normalised by one exact rule, multiplied by the Hamilton product, applied to vectors as
`q v q^-1`, converted to a `FixedMat3` and built from an axis and a `FixedAngle`. The contract
is named `bunny-quaternion/1` (`QUATERNION_CONTRACT_ID`). Every rounding is one rounding to the
nearest raw unit of an exact wide-integer value, and none can land on an exact half
([No Exact Halves](#no-exact-halves)). The [test plan](test-plan.md) lists the evidence.

## Representation And Storage Order

A quaternion `w + x i + y j + z k` is stored as the components `(x, y, z, w)`: the vector part
first, in the order of `FixedVec3`, then the scalar part `w`. This order is the contract:
`to_array()` returns `[x, y, z, w]`, and the accessors are `x()`, `y()`, `z()` and `w()`. The
fields are private, so every value comes from a constructor or an operation below. The frame
and the sign of an angle are those of [`bunny-rotation/1`](../rotation/README.md#convention):
right-handed, column vectors, and the right-hand rule.

`q` and `-q` are the same rotation. They are different values: `==` and `Hash` compare the
stored components, and no operation changes the sign of its result to a canonical one.
`canonical()` does it on request (see [Sign Operations](#sign-operations)).

## The Length Band

An exactly unit quaternion is rare in fixed point: four integers whose squares sum to exactly
`2^64` are few (`(2^32, 0, 0, 0)`, `(2^31, 2^31, 2^31, 2^31)` and their signs and
permutations). So the type does not require unit length. Every value satisfies, on raw
components,

```text
(2^32 - 1)^2 <= x^2 + y^2 + z^2 + w^2 <= (2^32 + 1)^2
```

(`QUAT_LENGTH_SQUARED_MIN_RAW` and `QUAT_LENGTH_SQUARED_MAX_RAW`), so its exact length is within
one raw unit, `2^-32`, of one. The band is symmetric and decided exactly on the squares. It is
narrower than the band of `FixedUnitVec3`, whose floor-square-root check admits lengths from
`1 - 2^-32` up to but not including `1 + 2^-31`; every operation below produces a value in
this band.

`try_from_unit(x, y, z, w)` accepts components already in the band, unchanged, and refuses
anything else with `QuatError::NotUnit`; a sum of squares that would pass `u128` (each square is
at most `2^126`) is out of the band. It is `const`. There is no constructor without a check.

## Normalisation

`try_normalize(x, y, z, w)` accepts any nonzero components. Each result component is the exact
`r / |p|` of its input component `r` and the input vector `p`, rounded once to the nearest raw
unit, with the sign of `r`:

```text
n = nearest integer to |r| 2^32 / sqrt(S),   S = x^2 + y^2 + z^2 + w^2  (raw)
```

The kernel finds `m = floor(2 |r| 2^32 / sqrt(S))`, the largest `m` with `m^2 S <= 4 r^2 2^64`,
by bisection over `[0, 2^33]` on exact 256-bit integers, and returns `(m + 1) / 2` floored. For
inputs below `2^66` in magnitude (every caller's), `r^2 2^66` is below `2^198`, `S` below
`2^134` and `m^2 S` below `2^200`.

The result is in the length band for every nonzero input. The exact unit vector `u = p 2^32 /
|p|` has length `2^32` raw, and each component of the result is within 1/2 of it, so the result
is within 1 raw unit of `u` in Euclidean distance and its length within 1 raw unit of one. So
no input is too small or too large: one raw unit (`(1, 0, 0, 0)` normalises to `(2^32, 0, 0,
0)`) and the extremes of the raw range (`(i64::MIN, ..)`, `(i64::MAX, ..)` and every mix) are
accepted, and the only refusal is `QuatError::Zero` for four zero components. There is no
overflow: every result component is at most `2^32` raw in magnitude.

Consequences, each exact:

- Only the direction matters: `try_normalize(k p) == try_normalize(p)` for every positive
  integer `k` with `k p` representable.
- `try_normalize(-p) == -try_normalize(p)`.
- An exactly unit input is returned unchanged. Another value of the type may change, by at most
  one raw unit per component (its length is within `2^-32` of one), so normalisation is not
  idempotent in general.

`FixedUnitVec3::new` normalises differently: it divides each component by the floor square root
of the sum of squares, rounds the quotient and then checks the length. That rule refuses short
vectors (it has no result for `(1, 1, 0)` raw) and its result length is checked, not derived.
Rounding the exact quotient instead gives a result for every nonzero input and a band that
follows from the rounding.

## No Exact Halves

No rounding in this contract can land on an exact half, so no tie rule is needed and none is
implemented. Each case rests on the 2-adic
valuation `v2`. Divide an integer quaternion `p` by the largest power of two `2^mu` that divides
all four components: some component is odd, and a sum of four squares with an odd one is `1`,
`2`, `3` or `4` modulo 8, so `v2(S) <= 2 mu + 2`.

- **Normalisation.** `2 |r| 2^32 / sqrt(S)` is an odd integer only if `S` is a square `s^2` with
  `v2(s) = v2(r) + 33`, so `v2(S) = 2 v2(r) + 66 >= 2 mu + 66`. That contradicts `v2(S) <= 2 mu
  + 2`.
- **Matrix entries.** `P_ij 2^32 / S` is an odd multiple of 1/2 only if `v2(S) = v2(P_ij) + 33`,
  and every entry `P_ij` (below) is a sum of products of two components, so `v2(P_ij) >= 2 mu`.
- **Vector rotation.** `(P v)_i / S` is an odd multiple of 1/2 only if `v2(S) = v2((P v)_i) +
  1`. With some component odd: if `S` is odd it has no half; if `S = 2` mod 4, exactly two
  components are odd and every entry of `P` is even, while `S / 2` is odd; if `S = 4` mod 8, all
  four are odd and every entry of `P` is a multiple of 4, while `S / 2` is `2` mod 4. If `S` is a
  multiple of 8, every component is even and halving all four leaves `P / S` unchanged, so the
  argument repeats.

The tests check every rounding against an oracle that would accept an exact half only for an even
result; QUAT-TP-010 also checks directly, on quaternions whose components share up to 16 factors
of two, that twice each error is strictly less than the divisor.

## Product And Composition

`a * b` (also `a.product(b)`) is the Hamilton product of the stored values, normalised in the
same single rounding: the four exact components

```text
x = aw bx + ax bw + ay bz - az by
y = aw by - ax bz + ay bw + az bx
z = aw bz + ax by - ay bx + az bw
w = aw bw - ax bx - ay by - az bz
```

are formed on raw values in `i128` (each product is below `(2^32 + 1)^2 < 2^65`, and each
component at most `|a| |b| < 2^65` raw squared) and passed to the [normalisation](#normalisation).
It is one rounding, not a rounded product followed by a renormalisation. So:

- each component is within 1/2 raw unit of the exact unit quaternion in the direction of the
  exact product;
- each component is within `5/2 + 2^-32` raw units of the exact, unnormalised product `a b`,
  because `|a| |b|` is within `2^-31 + 2^-64` of one and the normalisation moves no component by
  more than that;
- the result is in the length band, so it is a `FixedUnitQuat`, and the product never refuses or
  overflows (`|a b| = |a| |b| > 0`).

**Composition is right to left.** The rotation of `a * b` applies `b` first, then `a`: `(a b) v
(a b)^-1 = a (b v b^-1) a^-1`. This is the order of the matrix product on column vectors, `A B v
= A (B v)`, so the quarter turns about `+X` and `+Y` give `(x_quarter * y_quarter).to_mat3() ==
rotation_x(q) * rotation_y(q)` exactly. The product does not commute.

Exact cases: `i j = k`, `j k = i`, `k i = j`, `j i = -k` and `i i = -1` on the exactly unit
basis; `a * a.conjugate()` is exactly `IDENTITY` for every `a` (the exact product is the
positive scalar `|a|^2`); `(-a) * b == a * (-b) == -(a * b)`. `IDENTITY * a` and `a * IDENTITY`
are `try_normalize(a)`, equal to `a` only when `a` is exactly unit.

## Drift

Each product renormalises, so a chain of products never leaves the length band: the length does
not drift. The direction does. Let `c_k` be the computed product of the first `k` factors of a
chain and `e_k` its Euclidean distance, in raw units, from the exact unit quaternion in the
direction of the exact product of the same stored factors. Each step rounds once (at most 1 raw
unit, Euclidean), and the previous error passes through the next product scaled by the factor's
length and through the exact normalisation at most `1 / sqrt(|c_(k-1)|)`, below `1 + 2^-32`:

```text
e_k <= 1 + (1 + 2^-32) e_(k-1),   e_1 <= 1
```

So after `M` products, for `M <= 2^20`, `e <= (M + 1)(1 + 2^-11)` raw, and `M (1 + 2^-11)` when
the first factor is exactly unit. That is the proven worst case; rounding errors that cancel make
the measured error far smaller. QUAT-TP-007 multiplies an exactly unit quaternion by 1 000 seeded
pairs `b`, `b*` (2 000 products, whose exact normalised product is the start), and measured a
largest Euclidean error of 12.04 raw units (squared error 145), against the bound of 2 000.
Long chains should still be renormalised from an authoritative source (for example rebuilt
from an angle) when the accumulated direction error matters; renormalising a computed value
again does not remove it.

## Vector Rotation

With raw components `(x, y, z, w)`, `S = x^2 + y^2 + z^2 + w^2` and

```text
P = | ww+xx-yy-zz   2(xy - wz)    2(xz + wy)  |
    | 2(xy + wz)    ww-xx+yy-zz   2(yz - wx)  |
    | 2(xz - wy)    2(yz + wx)    ww-xx-yy+zz |
```

`q v q*` is `P v` exactly, and `q v q^-1 = q v q* / S`. `P / S` is exactly the rotation by the
direction `q / |q|`: orthogonal with determinant one, whatever the length of `q`.

`rotate_vec3(v)` returns each component `(P v)_i / S` on raw values, rounded once to nearest. It
is not two rounded quaternion products. Each `|P_ij| <= S < 2^65` and `|v_j| <= 2^63`, so each
term is below `2^128` and the three are summed by sign in 256 bits; the quotient by `S` (at least
`2^63`) is below `2^67` before the range check. So:

- each component is within 1/2 raw unit of the exact rotation of `v` by the stored quaternion's
  direction;
- the result's length is within `sqrt(3) / 2` raw units of `|v|`, an absolute bound for every
  `|v|`. The length of `q` does not enter: dividing by `S` removes it exactly, instead of scaling
  `v` by `|q|^2` as `P v / 2^64` would;
- `q` and `-q` give the same bits, because `P` and `S` are even in `q`.

It returns `None` when a rounded component is outside the Q32.32 range. An exact rotation
preserves length, so that needs `|v|` near `2^63` raw: it never happens when every component of
`v` is at most `2^62` raw in magnitude (`|v| <= sqrt(3) 2^62 < 2^63 - 1`). The half turn about
`z` maps `(-(2^63 - 1), 0, 0)` raw to `(2^63 - 1, 0, 0)` and refuses `(-2^63, 0, 0)`, whose image
`2^63` is one raw unit past the range.

Exact cases: `IDENTITY` leaves every vector unchanged; the quarter turn about a coordinate axis,
`(0, 0, c, c)` with `c = 3 037 000 500 = round(2^32 / sqrt 2)` about `z`, has `P / S` exactly the
axis rotation (the two equal components cancel every error), so quarter and half turns map basis
vectors, and every vector, exactly.

## Rotation Matrix

`to_mat3()` returns the matrix `P / S` for column vectors, each entry `P_ij 2^32 / S` rounded
once to nearest (`|P_ij| 2^32 < 2^97`, exact in `i128`). Each entry is within 1/2 raw unit of the
exactly orthonormal `P / S`, so, with dot products taken as the exact raw sum divided by `2^32`:

| Quantity | Proven | Contract | Seeded maximum | `bunny-rotation/1` |
| --- | --- | --- | --- | --- |
| Length of each row or column | `sqrt(3) / 2` of one | within 1 | 0.85 | within 11 |
| Dot product of two rows, or two columns | `sqrt(3) + 2^-32` of zero | within 2 | 1.44 | within 12 |
| Determinant | `3 sqrt(3) / 2 + 2^-30` of one | within 3 | 1.73 | within 15 |

The seeded maxima are the Python model's over 20 000 quaternions across the band. The
determinant bound is `det(I + F) - 1` for `F = R^T E`, whose trace is at most `|R|_F |E|_F =
sqrt(3) 3/2` raw. A matrix from a quaternion therefore meets the published bounds of
`rotation_axis_angle` (11, 12 and 15) with room to spare, so it is accepted wherever those bounds
are the acceptance test, such as the oriented box basis proposed in flyingrobots/bunny#482. The
matrix of `q` and of `-q` is the same bit for bit, and `conjugate().to_mat3()` is exactly the
transpose of `to_mat3()`.

## Axis-Angle Input

`from_axis_angle(axis: FixedUnitVec3, angle: FixedAngle)` is `(sin(h) u, cos(h))` for the half
angle `h`:

- **The half angle.** The angle is read as signed, in `[-1/2, 1/2)` turn, and halved toward zero,
  so `|h|` is at most a quarter turn and `w = cos(h) >= 0`. Halving is exact for an even raw
  angle; for an odd one `h` is `2^-65` turn nearer zero than the exact half, which moves the
  sine and cosine by less than `2^-29` raw. `(s, c) = h.sin_cos()`, each within one raw unit of
  the exact value.
- **One rounding.** The exact `(s u_x, s u_y, s u_z, c 2^32)` on raw values (each below `2^65`)
  is passed to the [normalisation](#normalisation). The result is within 1/2 raw unit per
  component of the exact normalised `(s u, c)` and in the length band, whatever the axis's own
  length in the `FixedUnitVec3` band; it is not renormalised again.
- **Against the exact rotation.** With `u = |u| u^` and `|u|` within `2^-31` of one, `|s |u| -
  sin h|` is at most `3 + 2^-31` raw and `|c - cos h|` at most 1, so `(s u, c)` is within `sqrt((3
  + 2^-31)^2 + 1) < 3.17` raw of the exact `(sin(h) u^, cos(h))`; after the normalisation and the
  rounding the quaternion is within 4.17 raw units (Euclidean). The Python model measured 2.20.
- **Agreement with `rotation_axis_angle`.** A rotation-matrix entry is a quadratic form of norm
  one in a unit quaternion, so it moves at most twice the quaternion's distance: each entry of
  `from_axis_angle(u, a).to_mat3()` is within `1/2 + 2 (4.17) < 8.84` raw of the exact rotation,
  and each entry of `rotation_axis_angle(u, a)` within `19/2 + 2^-28`. The two paths round
  differently, so they are not bit equal: their entries differ by at most 18 raw units (below
  18.4). QUAT-TP-013 checks this over 10 000 seeded axes across the `FixedUnitVec3` band; the
  largest difference measured was 8.

Exact cases: the angle zero gives exactly `IDENTITY` for every axis; the negated angle gives
exactly the conjugate, for every angle but the half turn (which is its own negation); the half
turn gives the normalised `(-u, 0)`; the quarter turn about a coordinate axis gives `(c, 0, 0, c)`
and its permutations with `c = round(2^32 / sqrt 2)`, because `sin_cos` of an eighth turn has
equal components bit for bit.

## Sign Operations

`conjugate()` is `(-x, -y, -z, w)`, and `-q` negates all four; both are exact (every component is
at most `2^32 + 1` raw) and stay in the band. The conjugate is the inverse rotation.
`canonical()` returns `q` when the first nonzero component in the order `w, x, y, z` is positive,
and `-q` otherwise: so `w > 0` after it, and at `w = 0` the tie goes to `x`, then `y`, then `z`.
`q.canonical() == (-q).canonical()` for every `q`, and canonical is idempotent.

## Conversions Not Provided

These are outside `bunny-quaternion/1`, and nothing here claims them:

- quaternion to axis-angle (it needs an inverse trigonometric function of a rounded length);
- matrix to quaternion;
- spherical or normalised linear interpolation between quaternions (slerp, nlerp);
- the shortest-arc rotation between two vectors, and Euler angles;
- an unnormalised quaternion type, and a product without renormalisation.
