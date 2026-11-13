# Angles And Trigonometry Test Plan

This document defines how Bunny proves the [angle and trigonometry contract](README.md).

## Scope

- the `FixedAngle` representation and its wrapping arithmetic
- conversions from turns, degrees and radians
- `sin`, `cos` and `sin_cos`: exact cases, symmetry, the error bound and golden bits

## Test Goals

- Lock the exact output bits of `sin_cos` for cardinal, diagonal, boundary and wraparound
  angles, so any change to the table, interpolation or quadrant logic is caught.
- Prove the one-raw-unit error bound against an independent oracle.
- Prove the exactness claims: cardinal values, negation symmetry, exact turn and
  quarter-degree conversions.
- Run natively and in the WASM gate.

## Non-Goals

- Tangent, inverse trigonometry and conversions out of `FixedAngle` (not implemented).
- Timing; see the performance chapter.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| AT-REQ-001 | Canonical angles are binary angles with `2^64` units per turn and wrapping arithmetic. | `README.md#canonical-angle` |
| AT-REQ-002 | `sin` and `cos` are within one raw Q32.32 unit of the exact value. | `README.md#sine-and-cosine` |
| AT-REQ-003 | Quarter-turn multiples give exactly `0`, `1` or `-1`. | `README.md#sine-and-cosine` |
| AT-REQ-004 | `sin(-a) == -sin(a)` and `cos(-a) == cos(a)` bit for bit. | `README.md#sine-and-cosine` |
| AT-REQ-005 | `from_turns` is exact and discards whole turns. | `README.md#conversions` |
| AT-REQ-006 | `from_degrees` is exact for whole multiples of 90 degrees. | `README.md#conversions` |
| AT-REQ-007 | `from_radians` and `from_degrees` are symmetric under negation. | `README.md#conversions` |
| AT-REQ-008 | `sin_cos` output bits are identical on every supported target. | `README.md#how-it-is-computed` |

```toml
[[requirement]]
id = "AT-REQ-001"
summary = "Canonical angles are binary angles with 2^64 units per turn and wrapping arithmetic."
status = "active"

[[requirement]]
id = "AT-REQ-002"
summary = "sin and cos are within one raw Q32.32 unit of the exact value."
status = "active"

[[requirement]]
id = "AT-REQ-003"
summary = "Quarter-turn multiples give exactly 0, 1 or -1."
status = "active"

[[requirement]]
id = "AT-REQ-004"
summary = "sin(-a) == -sin(a) and cos(-a) == cos(a) bit for bit."
status = "active"

[[requirement]]
id = "AT-REQ-005"
summary = "from_turns is exact and discards whole turns."
status = "active"

[[requirement]]
id = "AT-REQ-006"
summary = "from_degrees is exact for whole multiples of 90 degrees."
status = "active"

[[requirement]]
id = "AT-REQ-007"
summary = "from_radians and from_degrees are symmetric under negation."
status = "active"

[[requirement]]
id = "AT-REQ-008"
summary = "sin_cos output bits are identical on every supported target."
status = "active"
```

## Fixtures

The golden vectors are inline in `crates/bunny-num/tests/fixed_angle_tests.rs`. Each
expected value is the correctly rounded Q32.32 of the exact result. They were recorded from
the implementation and checked against 60-digit decimal arithmetic before being committed.
Regenerate them only for an intentional algorithm change, and re-check every value against
an independent high-precision oracle when you do.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| AT-TP-001 | Golden vectors | AT-REQ-008, AT-REQ-002 | Exact raw `sin` and `cos` for 20 cardinal, diagonal, boundary, segment and converted angles. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_001_golden_vectors_are_bit_exact` |
| AT-TP-002 | Exact values | AT-REQ-003 | Exact raw values at the four quarter turns. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_002_cardinal_angles_are_exact` |
| AT-TP-003 | Symmetry | AT-REQ-004, AT-REQ-001 | Bit equality of `sin(-a)` with `-sin(a)` and `cos(-a)` with `cos(a)` over 20 000 seeded angles. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_003_negation_symmetry_is_bit_exact` |
| AT-TP-004 | Error bound | AT-REQ-002 | At most one raw unit from `f64` sine and cosine over 20 000 seeded angles and 4 097 segment-aligned angles. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004_error_is_within_one_raw_unit` |
| AT-TP-005 | Conversion | AT-REQ-005, AT-REQ-001 | Exact raw angles for fractional, multi-turn and negative turns, and wrapping arithmetic. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_005_turns_wrap_exactly` |
| AT-TP-006 | Conversion | AT-REQ-006 | Exact raw angles for 0, ±90, 180, 270, 360 and 450 degrees. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_006_whole_quarter_degrees_convert_exactly` |
| AT-TP-007 | Conversion | AT-REQ-007 | `from_radians(-x)` equals the wrapped negation of `from_radians(x)` over 20 000 seeded values. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_007_radians_round_to_the_nearest_unit_symmetrically` |

```toml
[[case]]
id = "AT-TP-001"
requirements = ["AT-REQ-008", "AT-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_001_golden_vectors_are_bit_exact"
oracle = "Exact raw sin and cos for 20 cardinal, diagonal, boundary, segment and converted angles."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-002"
requirements = ["AT-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_002_cardinal_angles_are_exact"
oracle = "Exact raw values at the four quarter turns."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-003"
requirements = ["AT-REQ-004", "AT-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_003_negation_symmetry_is_bit_exact"
oracle = "Bit equality of sin(-a) with -sin(a) and cos(-a) with cos(a) over 20 000 seeded angles."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-004"
requirements = ["AT-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004_error_is_within_one_raw_unit"
oracle = "At most one raw unit from f64 sine and cosine over seeded and segment-aligned angles."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-005"
requirements = ["AT-REQ-005", "AT-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_005_turns_wrap_exactly"
oracle = "Exact raw angles for fractional, multi-turn and negative turns, and wrapping arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-006"
requirements = ["AT-REQ-006"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_006_whole_quarter_degrees_convert_exactly"
oracle = "Exact raw angles for 0, +-90, 180, 270, 360 and 450 degrees."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-007"
requirements = ["AT-REQ-007"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_007_radians_round_to_the_nearest_unit_symmetrically"
oracle = "from_radians(-x) equals the wrapped negation of from_radians(x) over 20 000 seeded values."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

`sin_cos` uses only integer arithmetic on a table computed at compile time by integer code:
no floating point, platform math library, randomness or hardware-dependent instruction is
involved, so the golden vectors in AT-TP-001 must hold bit for bit on every target. The
tests carry `wasm_bindgen_test`, so the same assertions run in the headless WASM gate. The
error-bound test (AT-TP-004) uses `f64` only as an oracle in test code; it never feeds a
result.

## Gaps

- AT-TP-004 samples angles; it is not exhaustive over all `2^64` angles. The interpolation
  error analysis (cubic Hermite error at most `h^4 / 384` for segment width
  `h = π / 2048`, about `1.4e-14`) is the argument for angles between samples.
