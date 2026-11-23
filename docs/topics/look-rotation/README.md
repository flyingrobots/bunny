# Look Rotation

`FixedMat3::try_look_rotation(forward, up)` builds the rotation that turns an
object's local `-Z` onto a `forward` direction, with an `up` hint fixing the
roll. It is the orientation half of a
[look-at view matrix](../matrix-types/README.md#look-at-view-matrix): the same
basis, the same rounding and the same refusals, without an eye. The contract is
named `bunny-look-rotation/1`.

## The Rotation

The frame is right-handed and follows the camera convention of the
[coordinate law](../coordinate-law/README.md#projection-and-ndc): local `-Z` is
forward, `+Y` is up and `+X` is right. With

- `f = normalize(forward)`,
- `right = normalize(f x up)`,
- `true_up = normalize(right x f)`,
- `back = -f`,

the matrix's columns are `right`, `true_up` and `back`, so for column vectors
`world = rotation * local`: local `-Z` maps to `f`, local `+Y` to `true_up` and
local `+X` to `right`. The rotation is the transpose of the upper 3x3 of
`FixedMat4::try_look_at(origin, forward, up)`.

Forward `(1, 0, 0)` with up `+Y` gives `right = (0, 0, 1)`, `true_up = (0, 1,
0)` and `back = (-1, 0, 0)`, so the rows are `(0, 0, -1)`, `(0, 1, 0)` and `(1,
0, 0)`: a quarter turn of -90 degrees about `+Y`, determinant 1. Forward `-Z`
with up `+Y` is the identity. The result is not exactly scale invariant: each
input is rounded when it is normalized, so scaling `forward` or `up` by a
positive factor can change its rounded direction by one raw unit. The matrix
can then differ within the bounds below, or the call can be refused under the
rounded-forward rule in [Refusals](#refusals). For up `(1, 0, 0)`, forward
`(2^61, 2^28 - 1, 0)` raw rounds to `(2^32, 1, 0)` and is accepted; three
times it rounds to `(2^32, 0, 0)` and is refused with `ParallelUp` (LR-TP-006).

## Rounding And Orthonormality

Each cross product is exact on the raw `i128` components. Each normalization
first rescales the exact vector so its largest component has 61 bits (rounding
half up when it shifts right), takes the floor integer square root of the sum of
squares, and divides each component, rounding half away from zero. Every
component of `f`, `right` and `true_up` is therefore within `1/2 + 2^-26` raw
units of the exact unit direction of the vector it normalized. Axis-aligned
inputs give exact results.

Fixed point cannot make the basis exactly orthonormal. The guaranteed bounds, in
raw units, with dot products taken as the exact sum of raw products divided by
`2^32`, are:

| Quantity | Bound |
| --- | --- |
| Length of each column (`right`, `true_up`, `back`) | within 2 of one |
| Dot product of two columns | within 2 of zero |
| Length of each row | within 3 of one |
| Dot product of two rows | within 4 of zero |

The column bounds are the look-at bounds, since the columns are the look-at
rows. Each column `c` is `c* + e` with `|e| <= sqrt 3 (1/2 + 2^-26) < 0.87`,
where `c*` is exactly perpendicular to the inputs of its cross product. The row
bounds follow by writing the matrix as an exactly orthogonal `Q` (columns
`right*`, `right* x g` and `-g`, with `g` the exact direction of `f`) plus an
error whose columns are below 0.87, 1.74 and 0.87: the error's Frobenius norm is
below 2.14, so each row's length is within 2.14 of one and each pair of rows has
a dot product within `sqrt 2 * 2.14 < 3.03` of zero. The bounds hold however
close `up` is to `forward`.

## Refusals

The function returns `LookAtError` and never panics:

- `CoincidentEyeTarget`: `forward` is exactly zero, the case `target == eye` of
  `try_look_at`. Any nonzero raw vector is a direction, so one raw unit along
  any axis is accepted.
- `ParallelUp`: `up` is zero, or exactly parallel or antiparallel to `forward`
  (decided on the exact raw inputs with 256-bit products), or exactly parallel
  to the rounded `f` (its exact cross product with `f` is zero). The last case
  arises when `forward` is so close to `up`'s direction that rounding makes them
  equal: `(2^40, 2^40 + 1, 0)` raw rounds to `(s, s, 0)` with `s = 3037000500`,
  so `up = (1, 1, 0)` is refused while `up = (1, 1 + 2^-32, 0)` is accepted. A
  nearly parallel `up` that survives these checks is accepted; the bounds hold,
  but the roll is fixed only to rounding noise.
- `Overflow` is never returned: every component of `f` is within one raw unit
  of `[-1, 1]`, so negating it cannot overflow.
