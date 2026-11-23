# Transform Composition Test Plan

This document defines how Bunny proves [`bunny-transform/1`](README.md):
`FixedAffine3::from_trs`, `FixedAffine3::rotate_about_point` and
`FixedAffine3::scale_about_point`, and the composition bound of `checked_mul_affine`. It
answers flyingrobots/bunny#312.

## Scope

- The three constructors in `bunny-linalg`: the scale, rotate, translate order, the pivot
  formulas, the single rounding of each product, the bounds in raw units, the overflow
  thresholds, and the acceptance of zero and negative scales.
- The bound on the two groupings of a product of three affine transforms.

Out of scope: normals under a non-uniform scale, decomposing a transform, and the ray and
bounds helpers of flyingrobots/bunny#108.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| TRS-REQ-001 | `from_trs(t, R, s)` is `T * R * S` with column vectors and right-to-left composition: a point is scaled, then rotated, then translated; the linear part is `R * diag(s)` and the translation is `t`, copied. | planned: `README.md` |
| TRS-REQ-002 | Each `from_trs` linear entry is `R_ij * s_j` rounded once to nearest, ties to even, within 1/2 raw of the exact product; the result is `None` exactly when a rounded entry is outside the `i64` raw range; zero and negative scales are accepted. | planned: `README.md` |
| TRS-REQ-003 | `rotate_about_point(p, R)` is `T(p) * R * T(-p)`: linear part `R`, translation `p - R p` with `R p` from `checked_mul_vec3`, within 3/2 raw of the exact value; `checked_transform_point(p)` is exactly `p`; `None` exactly when a rounded product, a partial sum or the subtraction leaves the range. | planned: `README.md` |
| TRS-REQ-004 | `scale_about_point(p, s)` is `T(p) * S * T(-p)`: linear part `diag(s)`, translation `p_i - s_i p_i` rounded once, within 1/2 raw; the pivot maps onto itself exactly; `None` exactly on overflow; zero and negative scales are accepted. | planned: `README.md` |
| TRS-REQ-005 | For three affine transforms, each entry of `(A * B) * C` and of `A * (B * C)` by `checked_mul_affine` is within the documented bound of the exact product `A B C`. | planned: `README.md` |

```toml
[[requirement]]
id = "TRS-REQ-001"
summary = "from_trs is T * R * S: scale, then rotate, then translate; linear part R * diag(s), translation copied."
status = "active"

[[requirement]]
id = "TRS-REQ-002"
summary = "Each from_trs entry is R_ij * s_j rounded once, ties to even, within 1/2 raw; None exactly when an entry leaves the i64 raw range; zero and negative scales accepted."
status = "active"

[[requirement]]
id = "TRS-REQ-003"
summary = "rotate_about_point is T(p) R T(-p): translation p - R p within 3/2 raw, the pivot fixed exactly, None exactly on overflow."
status = "active"

[[requirement]]
id = "TRS-REQ-004"
summary = "scale_about_point is T(p) S T(-p): translation p_i - s_i p_i rounded once within 1/2 raw, the pivot fixed exactly, None exactly on overflow; zero and negative scales accepted."
status = "active"

[[requirement]]
id = "TRS-REQ-005"
summary = "Both groupings of a product of three affine transforms are within the documented bound of the exact product."
status = "active"
```

## Oracles

The test file writes its own exact reference and shares no code with the kernel.
`oracle_trs` and `oracle_about` multiply raw values in `i128`, round by `div_euclid`,
`rem_euclid` and a comparison of twice the remainder with `2^32`, and check every value, partial
sum and difference against the `i64` range with `i64::try_from`. The kernel uses
`FixedQ32_32::checked_mul`, which rounds the magnitude by a shift and a mask. Bounds are
decided exactly on the unrounded `i128` values. `exact_product` forms the exact `A B C` over
`2^64` raw with checked `i128` arithmetic, which panics if a seeded input ever left the widths
it was chosen for.

A separate Python model, written from the [chapter](README.md) with unbounded integers,
applies the documented rules to the golden inputs of TRS-TP-004 and checks in `Fraction` that
each `from_trs` entry is within 1/2 raw and each pivot translation within 3/2 raw of the exact
value, and that each pivot maps onto itself. The goldens come from it; it also found the
`from_trs` thresholds of TRS-TP-006 by bisection. The model is posted on the pull request for
flyingrobots/bunny#312, not kept in the tree.

## Fixtures

None. TRS-TP-005 draws 10 000 cases from seed `0xB002_0312_0000_0001` with a 64-bit LCG: a
matrix, a translation, a scale and a pivot, each raw component a uniform 64-bit pattern shifted
right by 0 to 63 bits, so every magnitude up to the range limit occurs. Of the cases, 4 682
`from_trs` results are in range; of the 20 000 pivot constructions, 11 717 are in range and
8 283 are `None`. The test pins those counts. TRS-TP-008 draws 10 000 triples from seed
`0xB002_0312_0000_0001 ^ 8`, each transform `from_trs` of a translation within 4096, the
product `rotation_x(a) * rotation_z(b)` of two seeded angles and a scale within 4 in each
component (zero and negative included). Every failure message names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| TRS-TP-001 | Hand-derived | TRS-REQ-001 | Derived on paper in the test: `from_trs((1, 0, 0), rotation_y(quarter), (2, 3, 4))` maps `(1, 0, 0)` to `(1, 0, -2)`; rotating before scaling gives `(1, 0, -4)`; the vector `(1, 0, 0)` maps to `(0, 0, -2)`; the result equals `T * (R * S)` by `checked_mul_affine`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_001_scale_then_rotate_then_translate` |
| TRS-TP-002 | Hand-derived | TRS-REQ-003, TRS-REQ-004 | `rotate_about_point((1, 0, 0), rotation_y(half))` maps `(2, 0, 0)` to the origin and the pivot to itself, and equals `T(p) * (R * T(-p))`; `scale_about_point((1, 2, 3), (2, -1, 0))` maps `(2, 2, 2)` to `p + s (q - p) = (3, 2, 3)` and the pivot to itself. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_002_rotation_and_scale_about_a_point_fix_the_pivot` |
| TRS-TP-003 | Ties | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | At 60 degrees `rotation_z` has `m00` exactly one half, so `m00 s` is `s / 2` raw: `s = 1, -1, 3, -3, 5` give `0, 0, 2, -2, 2`, and the pivot translations `s - round(s / 2)` are `1, -1, 1, -1, 3`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_003_products_on_a_half_round_to_even` |
| TRS-TP-004 | Pinned golden | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | Raw linear parts, translations and mapped points for `from_trs` with `rotation_z(30 degrees)` and the 60 degree axis-angle rotation of ROT-TP-005, two `rotate_about_point` and one `scale_about_point`, from the Python model. Runs in the WebAssembly gate. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_004_pinned_goldens` |
| TRS-TP-005 | Seeded oracle | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | 10 000 seeded full-range cases: each constructor's `Option` and bits equal the oracle's; in-range results are within 1/2 raw (`from_trs`) and 3/2 raw (pivot translations) of the exact value, and every accepted pivot maps onto itself exactly; the in-range counts are pinned. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_005_seeded_constructors_match_the_oracle_and_fix_the_pivot` |
| TRS-TP-006 | Boundary | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | One raw unit either side of each threshold: `-1 * s` at `s = i64::MIN + 1` and `i64::MIN`; the longest-axis half turn (`m00 = 1 + 4` raw) at `0x7FFF_FFFE_0000_0007` and `-0x7FFF_FFFE_0000_0008`; translations at `i64::MIN` and `i64::MAX` copied; pivots `2^62 - 1`, `2^62`, `-2^62`, `-2^62 - 1` for the half turn, the scale `-1` and the scale 2. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_006_overflow_is_none_one_raw_unit_past_each_threshold` |
| TRS-TP-007 | Exact cases | TRS-REQ-002, TRS-REQ-004 | A zero scale gives determinant exactly zero and no inverse; the scale `(-1, 1, 1)` gives determinant exactly `-1` and the mirror image; a zero scale about a pivot maps every point onto the pivot. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_007_zero_and_negative_scales_are_accepted` |
| TRS-TP-008 | Seeded bound | TRS-REQ-005 | 10 000 seeded triples: every entry of `(A * B) * C` and `A * (B * C)` is within its documented bound of the exact `A B C`, decided exactly in `i128`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_008_composition_is_associative_within_the_rounding_bound` |
| TRS-TP-009 | Mutation | TRS-REQ-001, TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "TRS-TP-001"
requirements = ["TRS-REQ-001"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_001_scale_then_rotate_then_translate"
oracle = "Hand-derived scale, rotate, translate of one point; the swapped order; the literal T * R * S composition."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-002"
requirements = ["TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_002_rotation_and_scale_about_a_point_fix_the_pivot"
oracle = "Hand-derived half turn and scale about a pivot; the literal T(p) R T(-p) composition."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-003"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_003_products_on_a_half_round_to_even"
oracle = "Hand-derived exact halves at 60 degrees rounded to even."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-004"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_004_pinned_goldens"
oracle = "Raw goldens from the Python model, each checked in Fraction against the exact value."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-005"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_005_seeded_constructors_match_the_oracle_and_fix_the_pivot"
oracle = "Independent i128 oracle and exact bounds over 10 000 seeded full-range cases."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-006"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_006_overflow_is_none_one_raw_unit_past_each_threshold"
oracle = "Hand-derived and bisected thresholds, one raw unit either side."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-007"
requirements = ["TRS-REQ-002", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_007_zero_and_negative_scales_are_accepted"
oracle = "Exact determinants and images of zero and negative scales."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-008"
requirements = ["TRS-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_008_composition_is_associative_within_the_rounding_bound"
oracle = "Exact i128 product of three transforms over 10 000 seeded triples."
tier = "fast"
status = "planned"

[[case]]
id = "TRS-TP-009"
requirements = ["TRS-REQ-001", "TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/transform/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "planned"
```

## Mutations

Each mutation was applied to the kernel, `transform_tests` and `affine_transform_tests` run, and
the kernel restored; each fails the named cases.

| Mutation | Fails |
| --- | --- |
| `from_trs` applies `S` after `R` (`diag(s) * R`) | TRS-TP-001 (maps `(1, 0, 0)` to `(1, 0, -4)`), TRS-TP-004, TRS-TP-005 |
| `from_trs` drops the translation | TRS-TP-001, TRS-TP-004, TRS-TP-005, TRS-TP-006 |
| `from_trs` floors the product instead of rounding to nearest | TRS-TP-003, TRS-TP-005, TRS-TP-006 |
| `from_trs` saturates instead of refusing on overflow | TRS-TP-005, TRS-TP-006 |
| Pivot translation `p + R p` | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006 |
| Pivot translation `R p - p` | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006, TRS-TP-007 |
| `rotate_about_point` stores the transposed rotation | TRS-TP-003, TRS-TP-005 |
| `scale_about_point` permutes the scale components | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006 |

TRS-TP-008 tests the existing `checked_mul_affine` and no kernel line of this contract.

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic; the tests use fixed seeds and no clock. They run
natively and in the WebAssembly gate.
