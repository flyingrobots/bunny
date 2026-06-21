# Matrix Types Test Plan

This document defines how Bunny verifies the fixed-point matrix contract.

The current matrix chapter is [`README.md`](README.md). This test plan is a
living verification design. It records active requirements, executable
evidence, and open gaps for future transform and projection work.

## Scope

This plan covers `FixedMat2`, `FixedMat3`, and `FixedMat4` in `bunny-linalg`:

- row-major layout;
- row and column accessors;
- column-vector multiplication semantics;
- checked matrix-vector and matrix-matrix multiplication;
- identity and transpose;
- determinant and inverse behavior;
- singular and overflowing input handling.

Transform-specific point, vector, normal, bounds, projection, and viewport
semantics are out of scope until later roadmap slices expose those APIs.

## Test Goals

- Prove that matrix layout and multiplication semantics are stable through
  public APIs.
- Assert exact raw Q32.32 outputs for representative products and inverses.
- Prove that determinant and inverse APIs return `None` for singular or
  overflowing cases instead of panicking or silently saturating.
- Keep the evidence native and WASM-compatible through normal `bunny-linalg`
  test coverage.
- Avoid private helpers, stdout, stderr, logs, and documentation prose as test
  oracles.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| MT-REQ-001 | Matrix entries are row-major and public row/column accessors preserve that layout. | `README.md#layout` |
| MT-REQ-002 | Matrix-vector multiplication uses column-vector semantics. | `README.md#multiplication` |
| MT-REQ-003 | Matrix-matrix multiplication composes right-to-left for column vectors. | `README.md#multiplication` |
| MT-REQ-004 | Multiplication APIs return `None` when checked Q32.32 arithmetic overflows. | `README.md#multiplication` |
| MT-REQ-005 | Identity and transpose are stable for 2x2, 3x3, and 4x4 matrices. | `README.md#identity-and-transpose` |
| MT-REQ-006 | Determinant and inverse APIs return exact values when representable and `None` for singular or overflowing cases. | `README.md#determinant-and-inverse` |
| MT-REQ-007 | `FixedMat4` does not yet claim transform, projection, normal, or bounds propagation semantics. | `README.md#transform-boundaries` |

```toml
[[requirement]]
id = "MT-REQ-001"
summary = "Matrix entries are row-major and public row/column accessors preserve that layout."
status = "active"

[[requirement]]
id = "MT-REQ-002"
summary = "Matrix-vector multiplication uses column-vector semantics."
status = "active"

[[requirement]]
id = "MT-REQ-003"
summary = "Matrix-matrix multiplication composes right-to-left for column vectors."
status = "active"

[[requirement]]
id = "MT-REQ-004"
summary = "Multiplication APIs return None when checked Q32.32 arithmetic overflows."
status = "active"

[[requirement]]
id = "MT-REQ-005"
summary = "Identity and transpose are stable for 2x2, 3x3, and 4x4 matrices."
status = "active"

[[requirement]]
id = "MT-REQ-006"
summary = "Determinant and inverse APIs return exact values when representable and None for singular or overflowing cases."
status = "active"

[[requirement]]
id = "MT-REQ-007"
summary = "FixedMat4 does not yet claim transform, projection, normal, or bounds propagation semantics."
status = "active"
```

## Fixtures

No external fixtures are required.

All current matrix cases construct values directly from public Q32.32 raw
values:

- `FixedQ32_32::from_raw`;
- `FixedVec2::new`;
- `FixedVec3::new`;
- `FixedMat2::new`;
- `FixedMat3::from_rows`;
- `FixedMat4::from_rows`.

Future transform or projection slices may add checked-in fixtures. Each fixture
must document its source, generation command or hand-computation proof, oracle,
and regeneration policy.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| MT-TP-001 | Layout, golden path | MT-REQ-001, MT-REQ-002, MT-REQ-005 | Exact row/column vectors and raw Q32.32 matrix-vector output. | `crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_layout_transpose_and_raw_vector_multiply_are_stable` |
| MT-TP-002 | Determinant, inverse, composition | MT-REQ-003, MT-REQ-006 | Exact determinant, inverse rows, and identity product. | `crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_determinant_inverse_and_matrix_multiply_are_stable` |
| MT-TP-003 | 3x3 determinant and inverse | MT-REQ-002, MT-REQ-003, MT-REQ-006 | Exact raw product, integer inverse rows, and identity product. | `crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat3_determinant_inverse_and_raw_vector_multiply_are_stable` |
| MT-TP-004 | 4x4 identity, transpose, determinant, inverse | MT-REQ-003, MT-REQ-005, MT-REQ-006, MT-REQ-007 | Exact determinant, transpose accessors, inverse rows, and identity product. | `crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat4_identity_transpose_and_matrix_multiply_are_stable` |
| MT-TP-005 | Negative and overflow behavior | MT-REQ-004, MT-REQ-006 | Singular matrices and overflowing products return `None`. | `crates/bunny-linalg/tests/matrix_tests.rs::matrix_inverse_returns_none_for_degenerate_or_overflowing_cases` |
| MT-TP-006 | Fractional inverse raw outputs | MT-REQ-006 | Exact raw Q32.32 half-unit inverse entries. | `crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_fractional_inverse_uses_q32_32_raw_outputs` |

```toml
[[case]]
id = "MT-TP-001"
requirements = ["MT-REQ-001", "MT-REQ-002", "MT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_layout_transpose_and_raw_vector_multiply_are_stable"
oracle = "Exact row/column vectors and raw Q32.32 matrix-vector output."
tier = "fast"
status = "implemented"

[[case]]
id = "MT-TP-002"
requirements = ["MT-REQ-003", "MT-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_determinant_inverse_and_matrix_multiply_are_stable"
oracle = "Exact determinant, inverse rows, and identity product."
tier = "fast"
status = "implemented"

[[case]]
id = "MT-TP-003"
requirements = ["MT-REQ-002", "MT-REQ-003", "MT-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat3_determinant_inverse_and_raw_vector_multiply_are_stable"
oracle = "Exact raw product, integer inverse rows, and identity product."
tier = "fast"
status = "implemented"

[[case]]
id = "MT-TP-004"
requirements = ["MT-REQ-003", "MT-REQ-005", "MT-REQ-006", "MT-REQ-007"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat4_identity_transpose_and_matrix_multiply_are_stable"
oracle = "Exact determinant, transpose accessors, inverse rows, and identity product."
tier = "fast"
status = "implemented"

[[case]]
id = "MT-TP-005"
requirements = ["MT-REQ-004", "MT-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::matrix_inverse_returns_none_for_degenerate_or_overflowing_cases"
oracle = "Singular matrices and overflowing products return None."
tier = "fast"
status = "implemented"

[[case]]
id = "MT-TP-006"
requirements = ["MT-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/matrix_tests.rs::fixed_mat2_fractional_inverse_uses_q32_32_raw_outputs"
oracle = "Exact raw Q32.32 half-unit inverse entries."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

Current matrix tests use only public deterministic fixed-point operations:

- integer raw construction;
- checked Q32.32 multiplication, addition, subtraction, negation, and division;
- exact equality on fixed-point matrix and vector values;
- exact raw Q32.32 comparisons for representative outputs.

There is no randomness, time, filesystem access, locale-sensitive formatting,
parallel scheduling, map iteration, stdout, stderr, or logging in the oracle.

## Known Failures

The current executable surface has no known failing matrix cases.

Future implementation work must add explicit negative tests for:

- point/vector transform misuse;
- invalid affine transform construction;
- non-invertible transform wrappers;
- projection parameters that would produce invalid clip-space mappings;
- bounds propagation cases where transformed corners overflow.

## Edge Cases And Unusual Inputs

Current tests cover:

- singular 2x2, 3x3, and 4x4 matrices;
- checked multiplication overflow;
- fractional inverse entries that must compare by raw Q32.32 value;
- identity composition for 4x4 matrices.

Future slices should add boundary cases for:

- near-singular matrices with nonzero determinants;
- large translation components;
- non-uniform scale and inverse-transpose normal handling;
- projection near and far plane limits;
- viewport edge coordinates.

## Stress And Fuzz

No matrix fuzz target exists today.

Future matrix and transform property tests should cover:

- identity round trips;
- inverse round trips for bounded invertible matrices;
- associativity within the checked fixed-point domain;
- transform-specific point/vector distinctions;
- projection and unprojection round trips where the input domain is bounded.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Typed affine transforms and point/vector distinction. | Slice 2.2 transform types. |
| Transform-aware bounds propagation. | Slice 2.3 bounds propagation APIs. |
| Projection, unprojection, and viewport mapping. | Slice 2.4 projection APIs. |
| Quaternion-to-matrix conversion. | v0.6.0-GP3 orientation APIs. |
