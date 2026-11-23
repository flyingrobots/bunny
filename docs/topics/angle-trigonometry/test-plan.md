# Angles And Trigonometry Test Plan

This document defines how Bunny proves the [angle and trigonometry contract](README.md).

## Scope

- the `FixedAngle` representation and its wrapping arithmetic
- conversions from turns, degrees and radians
- `sin`, `cos` and `sin_cos`: exact cases, symmetry, the error bound and golden bits
- conversions out of `FixedAngle` (`to_turns`, `to_degrees`, `to_radians`) and `atan2`
  (flyingrobots/bunny#244): rounding, bounds, round trips, exact cases, branch and golden bits

## Test Goals

- Lock the exact output bits of `sin_cos` for cardinal, diagonal, boundary and wraparound
  angles, so any change to the table, interpolation or quadrant logic is caught.
- Prove the one-raw-unit error bound against an independent oracle.
- Prove the exactness claims: cardinal values, negation symmetry, exact turn and
  quarter-degree conversions.
- Run natively and in the WASM gate.

## Non-Goals

- Tangent, `asin`, `acos` and `atan` of a single ratio (not implemented).
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
| AT-REQ-009 | The interpolated value, with 62 fractional bits, is rounded to Q32.32 to nearest with ties to even, and exact ties are reachable. | `README.md#how-it-is-computed` |
| AT-REQ-010 | `to_turns` and `to_degrees` are the exact value `raw / 2^64` turn or `raw * 360 / 2^64` degrees rounded once to Q32.32, to nearest with ties to even, in `[0, 1]` and `[0, 360]`. | `README.md#conversions-out` |
| AT-REQ-011 | `to_radians` is within `1/2 + 2^-30` raw Q32.32 units of the exact `raw * 2 pi / 2^64`. | `README.md#conversions-out` |
| AT-REQ-012 | `to_turns(from_turns(t))` is exactly the fractional part of `t` in `[0, 1)`, and `from_turns(to_turns(a))` is `a` rounded to the nearest multiple of `2^32` units, ties to even, wrapping. | `README.md#conversions-out` |
| AT-REQ-013 | `atan2(y, x)` is within `1/2 + 2^-9` angle units of the exact angle of `(x, y)`, and `None` only at the origin. | `README.md#atan2` |
| AT-REQ-014 | `atan2` gives exact results on the axes and diagonals, a half turn on the negative x axis, and is bit for bit symmetric under conjugation, mirroring and swapping, and invariant under doubling both inputs. | `README.md#atan2` |

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

[[requirement]]
id = "AT-REQ-009"
summary = "The interpolated value, with 62 fractional bits, is rounded to Q32.32 to nearest with ties to even, and exact ties are reachable."
status = "active"

[[requirement]]
id = "AT-REQ-010"
summary = "to_turns and to_degrees are the exact value rounded once to Q32.32, to nearest with ties to even, in [0, 1] and [0, 360]."
status = "active"

[[requirement]]
id = "AT-REQ-011"
summary = "to_radians is within 1/2 + 2^-30 raw Q32.32 units of the exact raw * 2 pi / 2^64."
status = "active"

[[requirement]]
id = "AT-REQ-012"
summary = "to_turns(from_turns(t)) is the fractional part of t exactly; from_turns(to_turns(a)) is a rounded to the nearest multiple of 2^32 units, ties to even."
status = "active"

[[requirement]]
id = "AT-REQ-013"
summary = "atan2(y, x) is within 1/2 + 2^-9 angle units of the exact angle of (x, y), and None only at the origin."
status = "active"

[[requirement]]
id = "AT-REQ-014"
summary = "atan2 is exact on the axes and diagonals, a half turn on the negative x axis, bit-exact symmetric under conjugation, mirroring and swapping, and invariant under doubling."
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
| AT-TP-004 | Error bound | AT-REQ-002 | At most one raw unit from `f64` sine and cosine over 20 000 seeded angles, 4 097 segment-aligned angles and five probes in each of the 4 096 segments (a quarter, half and three quarters across, and the units next to each boundary). | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004_error_is_within_one_raw_unit` |
| AT-TP-008 | Error bound | AT-REQ-002 | Without any oracle, `sin^2 + cos^2` stays within the drift two errors of one raw unit allow, over 20 000 seeded angles. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004b_pythagorean_identity_holds_within_the_bound` |
| AT-TP-005 | Conversion | AT-REQ-005, AT-REQ-001 | Exact raw angles for fractional, multi-turn and negative turns, and wrapping arithmetic. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_005_turns_wrap_exactly` |
| AT-TP-006 | Conversion | AT-REQ-006 | Exact raw angles for 0, ±90, 180, 270, 360 and 450 degrees. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_006_whole_quarter_degrees_convert_exactly` |
| AT-TP-007 | Conversion | AT-REQ-007 | `from_radians(-x)` equals the wrapped negation of `from_radians(x)` over 20 000 seeded values. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_007_radians_round_to_the_nearest_unit_symmetrically` |
| AT-TP-009 | Final rounding tie | AT-REQ-009 | Six quarter-wave positions whose interpolated sine is exactly half a raw unit above an integer (three even, three odd, found by bisection over a replica of the interpolation) give the even neighbour from `sin` and from `cos` at the mirrored angle; rounding ties away from zero or toward zero fails. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_009_an_exact_tie_in_the_last_rounding_goes_to_even` |
| AT-TP-010 | Conversion out | AT-REQ-010 | Exact rational rounded by `u128` quotient and remainder, over 20 000 seeded angles and the ties of each conversion with one unit either side. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_010_turns_and_degrees_are_rounded_once_to_nearest_even` |
| AT-TP-011 | Conversion out | AT-REQ-011 | Independent 124-bit oracle (`pi` by Machin's formula) over the same angles. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_011_radians_are_within_half_a_unit_and_two_to_the_minus_thirty` |
| AT-TP-012 | Round trip | AT-REQ-012 | `rem_euclid` of the turns and the exactly rounded multiple of `2^32`, over 20 000 seeded values each. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_012_turn_round_trips_are_exact` |
| AT-TP-013 | Error bound | AT-REQ-013 | Independent 124-bit oracle (Machin `pi`, Taylor `atan`) over 20 000 seeded directions of every magnitude and the directions on and one raw unit either side of the axes, the diagonals and the extremes. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_013_atan2_is_within_half_a_unit_and_two_to_the_minus_nine` |
| AT-TP-014 | Exact values and branch | AT-REQ-013, AT-REQ-014 | Hand-derived exact angles on the axes and diagonals, `None` at the origin, and one raw unit either side of the negative and positive x axis. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_014_atan2_axes_diagonals_branch_and_origin` |
| AT-TP-015 | Symmetry | AT-REQ-014 | Bit equality of `atan2(-y, x)`, `atan2(y, -x)`, `atan2(x, y)` and `atan2(2y, 2x)` with the wrapped transforms of `atan2(y, x)` over 20 000 seeded directions. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_015_atan2_symmetries_and_scale_are_bit_exact` |
| AT-TP-016 | Golden vectors | AT-REQ-008, AT-REQ-010, AT-REQ-011, AT-REQ-013 | Raw conversions of 11 angles and 12 `atan2` angles, each the correctly rounded value from a 60-digit decimal model. | `crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_016_golden_vectors_are_bit_exact` |
| AT-TP-017 | Mutation | AT-REQ-010, AT-REQ-011, AT-REQ-013, AT-REQ-014 | Each rule broken on purpose makes at least one of AT-TP-010 to AT-TP-016 fail. | [Mutation checks](#mutation-checks) |

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
id = "AT-TP-008"
requirements = ["AT-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004b_pythagorean_identity_holds_within_the_bound"
oracle = "sin^2 + cos^2 stays within the drift two errors of one raw unit allow."
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

[[case]]
id = "AT-TP-009"
requirements = ["AT-REQ-009"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_009_an_exact_tie_in_the_last_rounding_goes_to_even"
oracle = "Six quarter-wave positions whose interpolated sine is exactly half a raw unit above an integer (three even, three odd, found by bisection over a replica of the interpolation) give the even neighbour from `sin` and from `cos` at the mirrored angle; rounding ties away from zero or toward zero fails."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-010"
requirements = ["AT-REQ-010"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_010_turns_and_degrees_are_rounded_once_to_nearest_even"
oracle = "Exact rational rounded by u128 quotient and remainder, over seeded angles and every tie with one unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-011"
requirements = ["AT-REQ-011"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_011_radians_are_within_half_a_unit_and_two_to_the_minus_thirty"
oracle = "Independent 124-bit oracle with pi by Machin's formula."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-012"
requirements = ["AT-REQ-012"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_012_turn_round_trips_are_exact"
oracle = "rem_euclid of the turns and the exactly rounded multiple of 2^32 units."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-013"
requirements = ["AT-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_013_atan2_is_within_half_a_unit_and_two_to_the_minus_nine"
oracle = "Independent 124-bit oracle (Machin pi, Taylor atan) over seeded and boundary directions."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-014"
requirements = ["AT-REQ-013", "AT-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_014_atan2_axes_diagonals_branch_and_origin"
oracle = "Hand-derived exact angles, None at the origin, one raw unit either side of the x axis."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-015"
requirements = ["AT-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_015_atan2_symmetries_and_scale_are_bit_exact"
oracle = "Bit equality with the wrapped conjugate, mirror, swap and doubled results."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-016"
requirements = ["AT-REQ-008", "AT-REQ-010", "AT-REQ-011", "AT-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_inverse_tests.rs::at_tp_016_golden_vectors_are_bit_exact"
oracle = "Correctly rounded raw values from a 60-digit decimal model."
tier = "fast"
status = "implemented"

[[case]]
id = "AT-TP-017"
requirements = ["AT-REQ-010", "AT-REQ-011", "AT-REQ-013", "AT-REQ-014"]
evidence = "manual-audit"
artifact = "docs/topics/angle-trigonometry/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of
`crates/bunny-num/src/fixed_angle/conversions_out.rs` or `fixed_angle/atan2.rs`, and
`cargo test --locked --no-fail-fast -p bunny-num --test fixed_angle_inverse_tests` was run;
the source was restored after each. The unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| `to_turns` floors | Round to nearest | 010, 012, 016 |
| A `to_turns` tie goes half up | Ties to even | 010, 016 |
| A `to_degrees` tie goes half up | Ties to even | 010, 016 |
| A `to_degrees` tie goes down | Ties to even | 010, 016 |
| `to_radians` floors | Round to nearest | 011, 016 |
| `to_radians` uses `2 pi` with 32 fractional bits | `2 pi` with 61 fractional bits | 011, 016 |
| `atan2` runs 64 CORDIC steps | 72 steps | 013, 016 |
| `atan2` runs 70 CORDIC steps | 72 steps | 013 |
| `atan2` floors the sum of step angles | Round to nearest | 013, 016 |
| The first step angle is a sixteenth turn | `atan(1)` is an eighth turn | 013, 014, 015, 016 |
| A swapped octant is added to the quarter turn | `QUARTER_TURN - a` | 013, 015, 016 |
| A mirrored quadrant is added to the half turn | `HALF_TURN - a` | 013, 014, 015, 016 |
| The sign of `y` is ignored | Conjugation | 013, 014, 015, 016 |
| The origin gives `ZERO` | `None` at the origin | 014, 015 |

## Oracles For The Conversions Out And Atan2

`crates/bunny-num/tests/angle_oracle/mod.rs` is unsigned fixed point with 124 fractional
bits in `u128`. It derives `pi` by Machin's formula, `16 atan(1/5) - 4 atan(1/239)`, and
`atan(n / d)` by its Taylor series, reducing arguments above `1/2` by
`atan(z) = pi/4 - atan((1 - z) / (1 + z))`; division is binary long division. It shares no
algorithm, constant or helper with the kernel, which uses CORDIC, a compile-time step table
scaled by `round(2^127 / 2 pi)`, and `round(2 pi 2^61)`. Its own error is below `2^-100` of
an output unit; the tests allow two units of `2^-32` of an output unit for it. `to_turns`
and `to_degrees` are checked against the exact rational rounded by `u128` quotient and
remainder (the kernel rounds by a shift and a mask).

The golden vectors of AT-TP-016 are inline in
`crates/bunny-num/tests/fixed_angle_inverse_tests.rs`. Each is the correctly rounded value
from a separate 60-digit Python `decimal` model (Machin `pi`, Taylor `atan`), which is not
committed; it is posted on the pull request. For every `atan2` and `to_radians` golden the
exact value is at least 0.02 raw units from a rounding boundary, so the kernel's bound
forces the nearest value.

## Determinism Obligations And Evidence

`sin_cos` uses only integer arithmetic on a table computed at compile time by integer code:
no floating point, platform math library, randomness or hardware-dependent instruction is
involved, so the golden vectors in AT-TP-001 must hold bit for bit on every target. The
tests carry `wasm_bindgen_test`, so the same assertions run in the headless WASM gate. The
error-bound test (AT-TP-004) uses `f64` only as an oracle in test code; it never feeds a
result.

## Gaps

- AT-TP-010, AT-TP-011 and AT-TP-013 sample inputs; they are not exhaustive. The bounds of
  `to_radians` and `atan2` rest on the error analyses in the living reference, which the
  samples check.
- AT-TP-004 samples angles; it is not exhaustive over all `2^64` angles. The interpolation
  error analysis (cubic Hermite error at most `h^4 / 384` for segment width
  `h = π / 2048`, about `1.4e-14`) is the argument for angles between samples.
