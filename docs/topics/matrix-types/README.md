# Matrix And Affine Transform Types

This document defines Bunny's current fixed-point matrix and affine transform
contract.

The matrix and affine transform surface lives in `bunny-linalg` and is
intentionally small. It gives later projection, bounds, quaternion, and optics
work a deterministic algebraic base without introducing camera, normal, bounds,
or viewport behavior early.

## Status

This is a living topic chapter, not a proposal.

Current matrix behavior is implemented by `FixedMat2`, `FixedMat3`, and
`FixedMat4`. Current affine transform behavior is implemented by `FixedAffine2`
and `FixedAffine3`. Planned projection, normal, and bounds helpers must update
this chapter only after the behavior exists in code and executable evidence
exists in the test suite.

## Layout

Matrix entries are row-major and named `mRC`, where `R` is the zero-based row
and `C` is the zero-based column.

For a 3x3 matrix:

```text
[ m00  m01  m02 ]
[ m10  m11  m12 ]
[ m20  m21  m22 ]
```

`FixedMat2` exposes `new` and `from_rows`. `FixedMat3` exposes `from_rows`.
`FixedMat4` exposes `from_rows` with `FixedMat4Row` tuple rows.

The public row and column accessors return stable values and are part of the
layout contract.

## Multiplication

Matrix-vector multiplication uses column-vector semantics:

```text
v_target = target_from_source * v_source
```

Matrix-matrix composition is right-to-left:

```text
combined = outer * inner
```

When `combined` is applied to a column vector, `inner` is applied first and
`outer` is applied second.

All public matrix multiplication APIs in this slice are checked APIs:

- `FixedMat2::checked_mul_vec2`
- `FixedMat2::checked_mul_mat2`
- `FixedMat3::checked_mul_vec3`
- `FixedMat3::checked_mul_mat3`
- `FixedMat4::checked_mul_mat4`

They return `None` when intermediate Q32.32 multiplication or addition cannot
be represented exactly by the checked fixed-point arithmetic profile.

## Affine Transforms

`FixedAffine2` and `FixedAffine3` store a linear matrix and a translation vector
as separate fields. They inherit the same column-vector convention as the matrix
types.

Points are transformed by the linear part and then translated:

```text
p_target = linear * p_source + translation
```

Vectors are transformed by the linear part only:

```text
v_target = linear * v_source
```

This distinction is part of the public contract. Translation affects points
because points represent positions. Translation does not affect vectors because
vectors represent directions or displacements.

Affine composition is right-to-left:

```text
combined = outer * inner
```

When `combined` is applied to a point or vector, `inner` is applied first and
`outer` is applied second.

All public affine transform APIs in this slice are checked APIs:

- `FixedAffine2::checked_transform_point`
- `FixedAffine2::checked_transform_vector`
- `FixedAffine2::checked_mul_affine`
- `FixedAffine3::checked_transform_point`
- `FixedAffine3::checked_transform_vector`
- `FixedAffine3::checked_mul_affine`

They return `None` when intermediate Q32.32 arithmetic cannot be represented
exactly. `try_inverse` returns `None` when the linear part is singular or when
inverse computation overflows.

## Identity And Transpose

Each matrix type exposes an identity constant:

- `FixedMat2::IDENTITY`
- `FixedMat3::IDENTITY`
- `FixedMat4::IDENTITY`

Each matrix type also exposes `transpose`, which swaps rows and columns without
rounding or allocation.

Each affine transform type exposes an identity constant:

- `FixedAffine2::IDENTITY`
- `FixedAffine3::IDENTITY`

## Determinant And Inverse

Each matrix type exposes `determinant`.

Determinants use checked Q32.32 arithmetic. A determinant returns `None` when an
intermediate product, sum, or subtraction overflows the checked fixed-point
profile.

Each matrix type exposes `try_inverse`.

`try_inverse` returns:

- `Some(inverse)` when the matrix is invertible and all checked arithmetic
  succeeds;
- `None` when the determinant is zero;
- `None` when an intermediate cofactor, division, or multiplication overflows.

No matrix inverse API panics for singular input.

## Transform Boundaries

`FixedAffine2` and `FixedAffine3` are affine transform types only. They do not
claim camera transform, projection matrix, normal matrix, or bounds propagation
semantics.

`FixedMat4` is still only a matrix type. It does not yet claim to be an affine
transform, camera transform, projection matrix, normal matrix, or bounds
propagation API.

Future transform-specific APIs must name the semantic kind they transform:

- normals may need inverse-transpose handling;
- bounds require explicit propagation rules;
- projection and viewport conversion require their own depth and origin
  conventions.

Those semantics are reserved for later roadmap slices.

## Required Tests

The current repository must keep tests for:

- row-major layout and row/column accessors;
- column-vector matrix-vector multiplication;
- right-to-left matrix composition with identity matrices;
- exact raw Q32.32 golden outputs for representative products;
- determinant and inverse for 2x2, 3x3, and 4x4 matrices;
- exact fractional inverse outputs when the inverse is representable;
- singular matrices returning `None`;
- checked overflow returning `None` instead of saturating silently;
- affine point transforms including translation;
- affine vector transforms excluding translation;
- right-to-left affine composition;
- affine inverse round trips and singular inverse rejection.
