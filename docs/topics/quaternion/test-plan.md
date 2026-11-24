# Quaternion Test Plan

This document defines how Bunny proves [`bunny-quaternion/1`](README.md): `FixedUnitQuat`, its
constructors, the Hamilton product, vector rotation, the rotation matrix and axis-angle input.
It answers flyingrobots/bunny#109.

## Scope

- The storage order and representation equality; the length band and `try_from_unit`; the
  normalisation and its refusal; the absence of exact halves; the product, its composition order
  and its drift; `rotate_vec3` and its overflow rule; `to_mat3` and its orthonormality bounds;
  `from_axis_angle` and its agreement with `rotation_axis_angle`; the sign operations.

## Non-Goals

- Quaternion to axis-angle, matrix to quaternion, slerp and nlerp, shortest-arc rotations and
  Euler angles: not provided (see [Conversions Not Provided](README.md#conversions-not-provided)).
- The rotation matrices of `bunny-rotation/1`, covered by the
  [rotation test plan](../rotation/test-plan.md); they are used here as a comparison only.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| QUAT-REQ-001 | `FixedUnitQuat` stores `(x, y, z, w)` with `w` the scalar part (`to_array` and the accessors in that order); `==` compares stored components, so `q` and `-q` are different values of the same rotation and nothing canonicalises the sign implicitly; the contract identity is `bunny-quaternion/1`. | `README.md#representation-and-storage-order` |
| QUAT-REQ-002 | Every value satisfies `(2^32 - 1)^2 <= x^2 + y^2 + z^2 + w^2 <= (2^32 + 1)^2` on raw values; `try_from_unit` accepts exactly that band unchanged and refuses anything else, including sums past `u128`, with `QuatError::NotUnit`. | `README.md#the-length-band` |
| QUAT-REQ-003 | `try_normalize` rounds each exact `r / \|p\|` once to the nearest raw unit with the sign of `r`; it refuses only four zeros (`QuatError::Zero`), accepts every other input including the extremes of the raw range, never overflows, lands in the band, and is odd and invariant under positive integer scaling. | `README.md#normalisation` |
| QUAT-REQ-004 | No rounding of the contract (normalisation, matrix entries, vector rotation) can meet an exact half. | `README.md#no-exact-halves` |
| QUAT-REQ-005 | `a * b` is the exact Hamilton product of the stored values normalised in one rounding: within 1/2 raw per component of the exact normalised product and `5/2 + 2^-32` of the exact product, always in the band; `a * b` applies `b` first (right to left, as `A B v`); `a a*` is exactly `IDENTITY`, and negating either factor negates the product. | `README.md#product-and-composition` |
| QUAT-REQ-006 | After `M <= 2^20` chained products the result is within `(M + 1)(1 + 2^-11)` raw units, Euclidean, of the exact normalised product of its factors (`M (1 + 2^-11)` from an exactly unit start), and every intermediate is in the band. | `README.md#drift` |
| QUAT-REQ-007 | `rotate_vec3` rounds each exact `(P v)_i / S` once to nearest: within 1/2 raw of the exact rotation by the stored direction, for every `\|v\|`; `q` and `-q` give the same bits; `None` exactly when a rounded component leaves the raw range, never when every component of `v` is at most `2^62` raw; quarter and half turns about the axes map vectors exactly. | `README.md#vector-rotation` |
| QUAT-REQ-008 | `to_mat3` rounds each exact `P_ij 2^32 / S` once to nearest: entries within 1/2 raw of an exact rotation, lengths within 1 raw of one, dot products within 2 of zero and the determinant within 3 of one, inside `bunny-rotation/1`'s 11, 12 and 15; `q` and `-q` give the same matrix and the conjugate its transpose. | `README.md#rotation-matrix` |
| QUAT-REQ-009 | `from_axis_angle` halves the signed angle toward zero and normalises the exact `(s u, c)` in one rounding; it is in the band with `w >= 0`, exactly `IDENTITY` at angle zero, exactly the conjugate at the negated angle (except the half turn), the normalised `(-u, 0)` at the half turn, and its matrix is within 18 raw per entry of `rotation_axis_angle`. | `README.md#axis-angle-input` |
| QUAT-REQ-010 | `conjugate` and negation are exact and stay in the band; `canonical` returns the representative whose first nonzero component in the order `w, x, y, z` is positive, the same for `q` and `-q`. | `README.md#sign-operations` |

```toml
[[requirement]]
id = "QUAT-REQ-001"
summary = "FixedUnitQuat stores (x, y, z, w) with w scalar; == compares stored components, q and -q differ; no implicit canonical sign; identity bunny-quaternion/1."
status = "active"

[[requirement]]
id = "QUAT-REQ-002"
summary = "Every value has raw x^2+y^2+z^2+w^2 in [(2^32-1)^2, (2^32+1)^2]; try_from_unit accepts exactly that band and refuses the rest with NotUnit."
status = "active"

[[requirement]]
id = "QUAT-REQ-003"
summary = "try_normalize rounds each exact r/|p| once to nearest; refuses only zero; accepts every other input incl. raw extremes; in the band; odd and scale invariant."
status = "active"

[[requirement]]
id = "QUAT-REQ-004"
summary = "No rounding of the contract can meet an exact half."
status = "active"

[[requirement]]
id = "QUAT-REQ-005"
summary = "a * b is the exact Hamilton product normalised once: 1/2 raw of the normalised product, 5/2 + 2^-32 of the exact product, in the band; b applies first; a a* is IDENTITY."
status = "active"

[[requirement]]
id = "QUAT-REQ-006"
summary = "After M <= 2^20 chained products the error is within (M + 1)(1 + 2^-11) raw Euclidean of the exact normalised product; every intermediate in the band."
status = "active"

[[requirement]]
id = "QUAT-REQ-007"
summary = "rotate_vec3 rounds each exact (P v)_i / S once: within 1/2 raw of the exact rotation; q and -q identical; None only out of range, never for components up to 2^62; axis quarter and half turns exact."
status = "active"

[[requirement]]
id = "QUAT-REQ-008"
summary = "to_mat3 rounds each exact P_ij 2^32 / S once: lengths within 1 raw, dots within 2, determinant within 3; q and -q identical; the conjugate gives the transpose."
status = "active"

[[requirement]]
id = "QUAT-REQ-009"
summary = "from_axis_angle halves the signed angle toward zero and normalises (s u, c) once; in the band, w >= 0; identity at zero; conjugate at the negated angle; within 18 raw of rotation_axis_angle."
status = "active"

[[requirement]]
id = "QUAT-REQ-010"
summary = "conjugate and negation are exact; canonical picks the representative whose first nonzero of w, x, y, z is positive, the same for q and -q."
status = "active"
```

## Oracles

The oracle is `crates/bunny-linalg/tests/quaternion_oracle/mod.rs` and shares no code with the
kernel. It works in signed arbitrary-precision integers over 32-bit limbs (the kernel uses a
two-word 256-bit integer). Its Hamilton product is the sum over the sixteen pairs of basis
elements from the multiplication table `ij = k`, `ji = -k`, `i^2 = -1` and so on (the kernel
writes the four expanded components). Its rotation is the sandwich `q v q*` of two such
products (the kernel expands the matrix `P` of degree-two polynomials). Each rounding is checked
by its defining inequality instead of being computed: a normalised component `n` of `r` in `p`
must satisfy `(2|n| - 1)^2 S < 4 r^2 2^64 < (2|n| + 1)^2 S`, and a rounded quotient `n` of
`N / S` must satisfy `|2 N - 2 n S| <= S` with equality only for even `n` (the kernel bisects
and divides). Lengths, dot products and determinants are decided exactly on the raw entries.

A separate Python model, written from the [chapter](README.md) with unbounded integers,
`Fraction` and `Decimal`, ports `FixedAngle::sin_cos` (checked against nine of its golden
vectors), normalises by `Decimal` square roots cross-checked against an integer square root,
checks its matrix polynomials against the sandwich product, and reproduces the QUAT-TP-014 corpus
and the QUAT-TP-007 chain. Its FNV-1a digest over 2 000 cases is `0xA1BB_1276_9435_F52F`, which
QUAT-TP-014 pins, and case 0's goldens come from it. Over 20 000 cases it measured, in raw
units: 2.22 (product against the exact product; bound `5/2 + 2^-32`), 2.20 (axis-angle
quaternion against the exact half-angle quaternion in `Decimal` trigonometry; bound 4.17), 4.31
(its matrix against the exact rotation; bound 8.84), 8 (agreement with `rotation_axis_angle`;
bound 18), 0.85, 1.44 and 1.73 (matrix length, dot and determinant; bounds 1, 2, 3); and 12.04
(drift after 2 000 products; bound 2 000). It found no exact half. The model is posted on
flyingrobots/bunny#109's pull request, not kept in the tree.

## Fixtures

None. Every seeded case uses the 64-bit LCG of the rotation tests from seed
`0xB002_0109_0000_0001`, with the case number mixed into the low bits (`SEED ^ nn` for
QUAT-TP-0nn; QUAT-TP-004 and QUAT-TP-005 use the seed itself). Band quaternions are built without
the kernel: the raw sum of squares is aimed at the top of the band, its bottom or a uniform point
between, three components are uniform within the remaining length and the fourth is its floor
square root (raised by one if that leaves the band), with a random sign and position. Raw inputs
have a uniform bit width from 0 to 64 and a random sign, clamped to `i64`, so both extremes
occur. Axes are drawn across the whole `FixedUnitVec3` band as in the rotation tests. Counts:
10 000 cases for QUAT-TP-004, -006, -009, -011 and -013; 1 000 for QUAT-TP-005, -008, -012 and
-015; 2 000 for QUAT-TP-010 and QUAT-TP-014; 1 000 pairs for QUAT-TP-007. Every failure message
names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| QUAT-TP-001 | Hand-derived | QUAT-REQ-001, QUAT-REQ-005 | Storage order of `(1/2, -1/2, 1/2, 1/2)` and of `IDENTITY`; `q != -q` with the same matrix and rotation; the unit rules `ij = k`, `jk = i`, `ki = j`, `ji = -k`, `ii = -1`, `ijk = -1` exactly. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_001_storage_order_and_representation_equality` |
| QUAT-TP-002 | Boundary | QUAT-REQ-002 | `S` at `(2^32 - 1)^2 - 1` (by Euler's four-square identity), `(2^32 - 1)^2`, `(2^32 + 1)^2` and one more: refused, accepted unchanged, accepted unchanged, refused; `(2^32 + 2, 0, 0, 0)` (in the `FixedUnitVec3` band) refused; sums past `u128` and zero refused. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_002_the_length_band_one_raw_unit_either_side` |
| QUAT-TP-003 | Exact cases | QUAT-REQ-003 | Zero refused; one raw unit, `i64::MIN`, `i64::MAX` and mixes of them normalise to the hand-derived `(2^32, ..)` and `(2^31, ..)` values; `(3, 4)` at two scales gives `0.6` and `0.8` rounded; `(1, 0, 0, 1)` gives `round(2^32 / sqrt 2)` where `FixedUnitVec3::new` has no result; exactly unit inputs are fixed points. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_003_normalisation_refuses_only_zero_and_accepts_the_extremes` |
| QUAT-TP-004 | Seeded oracle | QUAT-REQ-003, QUAT-REQ-004 | 10 000 seeded inputs of random bit width (both extremes occur): every result satisfies `(2\|n\| - 1)^2 S < 4 r^2 2^64 < (2\|n\| + 1)^2 S` strictly, is in the band, is odd, is unchanged by scaling by 3, and renormalises within one raw unit. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_004_seeded_normalisation_matches_the_oracle` |
| QUAT-TP-005 | Composition | QUAT-REQ-005 | Quarter turns about `+X` and `+Y`: their product is exactly `(1/2, 1/2, 1/2, 1/2)`, its matrix equals `rotation_x(q) * rotation_y(q)` and it rotates the basis as `b` then `a`; `b a != a b`. 1 000 seeded pairs: `(-a) b == a (-b) == -(a b)`, `a a* == IDENTITY`, `IDENTITY a == a IDENTITY == try_normalize(a)`. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_005_products_compose_right_to_left` |
| QUAT-TP-006 | Seeded oracle and bound | QUAT-REQ-005 | 10 000 seeded pairs across the band: the product is the nearest normalisation of the oracle's Hamilton product (written from the unit rules), in the band, and within `5/2 + 2^-32` raw of the exact product. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_006_seeded_products_match_the_oracle_and_the_bounds` |
| QUAT-TP-007 | Drift | QUAT-REQ-006 | An exactly unit start multiplied by 1 000 seeded pairs `b`, `b*`: after every pair the squared Euclidean error from the start is within `(M (1 + 2^-11))^2` for `M` products, and every value is in the band; the largest error, 12.04 raw after 2 000 products, was measured by the Python model. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_007_drift_over_a_chain_stays_within_the_derived_bound` |
| QUAT-TP-008 | Exact cases | QUAT-REQ-007, QUAT-REQ-008 | Quarter and half turns about `+X`, `+Y`, `+Z` from `from_axis_angle` map each basis vector exactly onto its hand-derived image; the quarter turn about `+Z` has exactly the matrix `rotation_z`; `IDENTITY` leaves extreme vectors unchanged; 1 000 seeded `q` and `-q` give the same rotation and matrix bits. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_008_quarter_and_half_turns_map_the_basis_exactly` |
| QUAT-TP-009 | Seeded oracle and boundary | QUAT-REQ-007 | 10 000 seeded quaternions and vectors of random width: each component is the nearest integer to the oracle's sandwich `q v q*` over `S`, or `None` exactly when a rounded component would leave `i64`; halved vectors (components at most `2^62`) never refuse. The half turn about `z` maps `-(2^63 - 1)` to `2^63 - 1` and refuses `-2^63`; the quarter turn likewise one raw unit apart. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_009_seeded_rotations_match_the_oracle_and_overflow_at_the_range` |
| QUAT-TP-010 | Ties | QUAT-REQ-004 | 2 000 seeded quaternions whose components share 0 to 16 factors of two, with vectors: twice each rotation and matrix rounding error is strictly below `S`, as the 2-adic proof says. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_010_no_rounding_meets_an_exact_half` |
| QUAT-TP-011 | Seeded oracle and bound | QUAT-REQ-008 | 10 000 seeded quaternions: each entry is the nearest integer to the oracle's `(q e_j q*)_i 2^32 / S`; lengths within 1, dot products within 2 and the determinant within 3 raw, decided exactly. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_011_seeded_matrices_match_the_oracle_and_the_bounds` |
| QUAT-TP-012 | Exact cases | QUAT-REQ-009 | 1 000 seeded axes and angles: zero gives `IDENTITY`, `w >= 0`, the negated angle the conjugate, the half turn the normalised `(-u, 0)`; the quarter turn about `+Z` is `(0, 0, c, c)`; an odd angle where `sin_cos` steps between adjacent half angles shows the halving is toward zero, not floor. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_012_axis_angle_exact_cases` |
| QUAT-TP-013 | Seeded oracle and agreement | QUAT-REQ-009 | 10 000 seeded axes across the `FixedUnitVec3` band and angles: the result is the nearest normalisation of the exact `(s u, c)`, in the band, and its matrix within 18 raw per entry of `rotation_axis_angle` (largest measured 8). | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_013_seeded_axis_angle_matches_the_oracle_and_rotation_axis_angle` |
| QUAT-TP-014 | Pinned golden | QUAT-REQ-003, QUAT-REQ-005, QUAT-REQ-007, QUAT-REQ-008, QUAT-REQ-009 | Case 0 of the corpus pinned (normalisation, product, rotation, matrix, axis-angle); FNV-1a digest over 2 000 seeded cases equal to the Python model's. Runs in the WebAssembly gate. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_014_pinned_goldens_and_digest` |
| QUAT-TP-015 | Exact cases | QUAT-REQ-010 | Conjugate and canonical of hand-picked values, including every tie at `w = 0`; 1 000 seeded values: double conjugate and double negation are the identity, canonical agrees for `q` and `-q`, is idempotent and is `q` or `-q`, and the conjugate's matrix is the transpose. | `crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_015_conjugate_negation_and_canonical_sign` |
| QUAT-TP-016 | Unit | QUAT-REQ-003, QUAT-REQ-007 | The kernel's 256-bit product, sum, difference, shift, comparison and division against hand-computed values, including the carries of `(2^128 - 1)^2`. | `crates/bunny-linalg/src/fixed_quat/wide.rs::tests::quat_tp_016_wide_products_sums_and_quotients_are_exact` |
| QUAT-TP-017 | Mutation | QUAT-REQ-001, QUAT-REQ-002, QUAT-REQ-003, QUAT-REQ-004, QUAT-REQ-005, QUAT-REQ-006, QUAT-REQ-007, QUAT-REQ-008, QUAT-REQ-009, QUAT-REQ-010 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "QUAT-TP-001"
requirements = ["QUAT-REQ-001", "QUAT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_001_storage_order_and_representation_equality"
oracle = "Hand-derived storage order and the unit rules of the quaternion basis."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-002"
requirements = ["QUAT-REQ-002"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_002_the_length_band_one_raw_unit_either_side"
oracle = "Constructed sums of squares on and one raw unit either side of each end of the band."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-003"
requirements = ["QUAT-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_003_normalisation_refuses_only_zero_and_accepts_the_extremes"
oracle = "Hand-derived normalisations at the extremes of the raw range, and the defining inequality."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-004"
requirements = ["QUAT-REQ-003", "QUAT-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_004_seeded_normalisation_matches_the_oracle"
oracle = "The defining inequality of the nearest integer, decided exactly in arbitrary precision."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-005"
requirements = ["QUAT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_005_products_compose_right_to_left"
oracle = "The matrix product of bunny-rotation/1 axis rotations; exact algebraic identities."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-006"
requirements = ["QUAT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_006_seeded_products_match_the_oracle_and_the_bounds"
oracle = "Arbitrary-precision Hamilton product from the basis multiplication table; exact bound."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-007"
requirements = ["QUAT-REQ-006"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_007_drift_over_a_chain_stays_within_the_derived_bound"
oracle = "The exact normalised product of each prefix is the start, because b b* is a positive scalar."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-008"
requirements = ["QUAT-REQ-007", "QUAT-REQ-008"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_008_quarter_and_half_turns_map_the_basis_exactly"
oracle = "Hand-derived images of the basis from the coordinate law."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-009"
requirements = ["QUAT-REQ-007"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_009_seeded_rotations_match_the_oracle_and_overflow_at_the_range"
oracle = "Arbitrary-precision sandwich product and exact range decision; constructed overflow boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-010"
requirements = ["QUAT-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_010_no_rounding_meets_an_exact_half"
oracle = "Exact strict inequality on the arbitrary-precision numerators."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-011"
requirements = ["QUAT-REQ-008"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_011_seeded_matrices_match_the_oracle_and_the_bounds"
oracle = "Sandwich products of the basis vectors in arbitrary precision; exact length, dot and determinant."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-012"
requirements = ["QUAT-REQ-009"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_012_axis_angle_exact_cases"
oracle = "Exact identities, and a constructed odd angle at a step of sin_cos."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-013"
requirements = ["QUAT-REQ-009"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_013_seeded_axis_angle_matches_the_oracle_and_rotation_axis_angle"
oracle = "Defining inequality on the documented input; the shipped bunny-rotation/1 matrices."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-014"
requirements = ["QUAT-REQ-003", "QUAT-REQ-005", "QUAT-REQ-007", "QUAT-REQ-008", "QUAT-REQ-009"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_014_pinned_goldens_and_digest"
oracle = "Goldens and digest from the independent Python model."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-015"
requirements = ["QUAT-REQ-010"]
evidence = "test"
test = "crates/bunny-linalg/tests/quaternion_tests.rs::quat_tp_015_conjugate_negation_and_canonical_sign"
oracle = "Hand-derived signs and exact identities."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-016"
requirements = ["QUAT-REQ-003", "QUAT-REQ-007"]
evidence = "test"
test = "crates/bunny-linalg/src/fixed_quat/wide.rs::tests::quat_tp_016_wide_products_sums_and_quotients_are_exact"
oracle = "Hand-computed 256-bit values."
tier = "fast"
status = "implemented"

[[case]]
id = "QUAT-TP-017"
requirements = ["QUAT-REQ-001", "QUAT-REQ-002", "QUAT-REQ-003", "QUAT-REQ-004", "QUAT-REQ-005", "QUAT-REQ-006", "QUAT-REQ-007", "QUAT-REQ-008", "QUAT-REQ-009", "QUAT-REQ-010"]
evidence = "manual-audit"
artifact = "docs/topics/quaternion/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel, `quaternion_tests` and the library's unit tests run,
and the kernel restored; each fails the named cases.

| Mutation | Fails |
| --- | --- |
| Product operands swapped (`b * a`) | QUAT-TP-001, QUAT-TP-005, QUAT-TP-006, QUAT-TP-014 |
| Product cross-term sign flipped (`- ay bz + az by` in `x`) | QUAT-TP-001, QUAT-TP-006, QUAT-TP-007, QUAT-TP-014 |
| Normalisation by truncation (`m / 2` instead of `(m + 1) / 2`) | QUAT-TP-003, QUAT-TP-004, QUAT-TP-005, QUAT-TP-006, QUAT-TP-007, QUAT-TP-012, QUAT-TP-013, QUAT-TP-014 |
| Rotation rounded by truncation | QUAT-TP-009, QUAT-TP-010, QUAT-TP-014 |
| Matrix entries rounded by floor | QUAT-TP-010, QUAT-TP-011, QUAT-TP-014 |
| Rotation polynomial `2(xy - wz)` with `+ wz` | QUAT-TP-005, QUAT-TP-008, QUAT-TP-009, QUAT-TP-010, QUAT-TP-011, QUAT-TP-013, QUAT-TP-014, QUAT-TP-015 |
| Rotation divided by `2^64` instead of `S` | QUAT-TP-009, QUAT-TP-010, QUAT-TP-014 |
| Axis-angle with the full angle instead of the half | QUAT-TP-005, QUAT-TP-008, QUAT-TP-012, QUAT-TP-013, QUAT-TP-014 |
| Axis-angle halved by floor instead of toward zero | QUAT-TP-012 |
| Band minimum exclusive | QUAT-TP-002 |
| Band maximum exclusive | QUAT-TP-002 |
| Canonical tie order `y` before `x` | QUAT-TP-015 |
| Conjugate negating `w` instead of `z` | QUAT-TP-005, QUAT-TP-007, QUAT-TP-012, QUAT-TP-015 |
| 256-bit product dropping the middle carry | QUAT-TP-016 |

One mutation is equivalent and fails nothing, by design: bisecting over `[0, 2^33)` instead of
`[0, 2^33]` returns `2^33 - 1` where `floor(2q) = 2^33`, and both round to `2^32`. Tie-rule
mutations (ties up, ties away from zero) are equivalent too, because no exact half can occur
(QUAT-TP-010); the kernel has no tie branch to mutate.

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic and `core`; the tests use fixed seeds and no clock. They
run natively and in the WebAssembly gate.
