# Rotation Test Plan

This document defines how Bunny proves [`bunny-rotation/1`](README.md):
`FixedMat3::rotation_x`, `rotation_y`, `rotation_z` and `rotation_axis_angle`. It answers
flyingrobots/bunny#311.

## Scope

- The three axis rotations and the axis-angle rotation in `bunny-linalg`: their sign
  convention, their exact cases, the single rounding of the axis-angle entries, and the
  orthonormality and determinant bounds in raw units.

Out of scope: composing rotations (`checked_mul_mat3` already exists and this contract adds
nothing to it), quaternions (now [`bunny-quaternion/1`](../quaternion/README.md)), and translate, rotate and scale
composition (flyingrobots/bunny#312).

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| ROT-REQ-001 | Positive angles follow the right-hand rule of the coordinate law, with column vectors: a positive quarter turn about a coordinate axis `a` maps each perpendicular `v` onto `a x v` (`rotation_y` maps `+X` onto `-Z` and `+Z` onto `+X`), the sense `FixedAngle` documents about `+Z` and the convention `try_look_rotation` documents. | `README.md#convention` |
| ROT-REQ-002 | The axis rotations copy `FixedAngle::sin_cos`: entries within one raw unit of the exact value, exactly `0`, `1` and `-1` at the cardinal angles, the identity at angle zero; dot products between rows or columns exactly zero, lengths within 3/2 raw of one, the determinant exactly `c^2 + s^2` and within 3 raw of one. | `README.md#axis-rotations` |
| ROT-REQ-003 | Each `rotation_axis_angle` entry is the exact value of `c I + s [u]x + (1 - c) u u^T` on the raw axis over `2^64`, rounded once to nearest, ties to even; it is within `3/2 + 2^-30` raw of the formula at the exact angle and within `19/2 + 2^-28` raw of the exact rotation about the axis direction. | `README.md#axis-angle-rotation` |
| ROT-REQ-004 | `rotation_axis_angle` is exactly the identity at angle zero for every axis, exactly the transpose at the negated angle, and exactly the axis rotation for each of the six signed unit axis constants. | `README.md#axis-angle-rotation` |
| ROT-REQ-005 | For every `FixedUnitVec3` axis and every angle, `rotation_axis_angle` has row and column lengths within 11 raw of one, dot products between rows or between columns within 12 raw of zero and a determinant within 15 raw of one; it never refuses and never panics. | `README.md#orthonormality-and-determinant` |

```toml
[[requirement]]
id = "ROT-REQ-001"
summary = "Positive angles follow the right-hand rule with column vectors: a positive quarter turn about axis a maps a perpendicular v onto a x v, as FixedAngle and try_look_rotation document."
status = "active"

[[requirement]]
id = "ROT-REQ-002"
summary = "Axis rotations copy sin_cos: exact at cardinal angles, identity at zero, zero dot products, lengths within 3/2 raw, determinant exactly c^2 + s^2 within 3 raw."
status = "active"

[[requirement]]
id = "ROT-REQ-003"
summary = "Each axis-angle entry is the exact Rodrigues value on the raw axis over 2^64 rounded once, ties to even; within 3/2 + 2^-30 raw of the formula at the exact angle and 19/2 + 2^-28 raw of the exact rotation."
status = "active"

[[requirement]]
id = "ROT-REQ-004"
summary = "The axis-angle rotation is the identity at zero, the transpose at the negated angle, and the axis rotation for each signed unit axis constant, exactly."
status = "active"

[[requirement]]
id = "ROT-REQ-005"
summary = "For every FixedUnitVec3 axis, row and column lengths are within 11 raw of one, dot products within 12 raw of zero, the determinant within 15 raw of one; no refusal, no panic."
status = "active"
```

## Oracles

The test file writes its own exact reference and shares no code with the kernel.
`oracle_axis_angle` rebuilds every entry from `FixedAngle::sin_cos` (the kernel's input, not
the kernel) as `t u_i u_j`, plus `c 2^64` on the diagonal or minus `epsilon(i, j, k) s u_k
2^32` off it, and rounds by `div_euclid`, `rem_euclid` and a comparison of twice the
remainder with `2^64`. The kernel builds each entry from named terms and rounds by an
arithmetic shift and a mask. Lengths, dot products and determinants are decided exactly on
the raw `i128` sums, without rounding.

A separate Python model, written from the [chapter](README.md) with unbounded integers,
ports `FixedAngle::sin_cos` (checked against its 20 golden vectors), applies the documented
rules to the corpus below, and checks every result in `Decimal` at 40 digits against the
exact sine and cosine of the exact angle. Its FNV-1a digest over the result stream is
`0xB74D_1C11_AF2A_0E65`, which ROT-TP-008 pins, and the goldens of ROT-TP-005 and ROT-TP-009
come from it. Over the corpus the largest values it saw, in raw units, were 0.99 (entry
against the formula on the given axis, bound `3/2 + 2^-30`), 8.11 (entry against the exact
rotation about the axis direction, bound `19/2 + 2^-28`), 8.16 and 8.10 (row and column
length), 8.07 and 7.89 (row and column dot product) and 10.54 (determinant); for the axis
rotations at the same angles, 0.70 (length) and 1.39 (determinant). The model is posted on flyingrobots/bunny#311's pull request, not kept in the
tree.

## Fixtures

None. ROT-TP-007 and ROT-TP-008 draw 10 000 cases from seed `0xB002_0311_0000_0001` with a
64-bit LCG: a uniform raw angle, then an axis whose raw sum of squares is aimed at the top of
the `FixedUnitVec3` band (`(2^32 + 2)^2 - 1`, a quarter of the cases), its bottom
(`(2^32 - 1)^2`, a quarter) or a uniform point between; two components are uniform within the
remaining length, the third is its floor square root (raised by one if that leaves the band),
with a random sign and a random rotation of the three. ROT-TP-003 and ROT-TP-004 use the
first 1 000 draws of the same stream. Every failure message names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| ROT-TP-001 | Hand-derived | ROT-REQ-001 | From the coordinate law's `X x Y = Z`, `Y x Z = X`, `Z x X = Y`: `rotation_y(quarter)` maps `(1, 0, 0)` to `(0, 0, -1)` and `(0, 0, 1)` to `(1, 0, 0)`; the three quarter-turn matrices are pinned; `try_look_rotation(+X, +Y)` equals `rotation_y` of three quarters. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_001_quarter_turns_follow_the_right_hand_rule` |
| ROT-TP-002 | Exact cases | ROT-REQ-002 | For each axis and `k` in `0..4`, the angle `k` quarters and one angle unit either side equal the `k`-th power of the quarter turn, with entries in `{0, 1, -1}` and determinant exactly 1; angle zero is the identity. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_002_cardinal_angles_give_exact_zeros_and_ones` |
| ROT-TP-003 | Property | ROT-REQ-004 | The eight multiples of 45 degrees and 1 000 seeded angles: `rotation_axis_angle(+X, a) == rotation_x(a)`, likewise `+Y` and `+Z`, and `-X`, `-Y`, `-Z` give the axis rotation of `-a`, bit for bit. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_003_axis_angle_about_the_coordinate_axes_matches_the_axis_rotations` |
| ROT-TP-004 | Property | ROT-REQ-004 | 1 000 seeded axes and angles: angle zero is exactly the identity, and the negated angle exactly the transpose, for the axis-angle and the three axis rotations. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_004_zero_is_the_identity_and_a_negated_angle_is_the_transpose` |
| ROT-TP-005 | Pinned golden | ROT-REQ-002, ROT-REQ-003 | `rotation_z(from_degrees(30))` copies the angle goldens; 60 degrees about the rounded `(2/7, 3/7, 6/7)` gives pinned raw rows, each within 1/2 raw of the Decimal ideal on that axis, and equal to the oracle. Runs in the WebAssembly gate. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_005_pinned_goldens` |
| ROT-TP-006 | Ties | ROT-REQ-003 | At 60 degrees `1 - cos` is exactly `2^31` raw, so about `(1, y 2^-32, 0)` the entries `m01` and `m10` are exactly `y / 2` raw: `y = 1, -1, 3, -3, 5, -5` give `0, 0, 2, -2, 2, -2`. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_006_entries_on_a_half_round_to_even` |
| ROT-TP-007 | Seeded bound | ROT-REQ-002 | 10 000 seeded angles, each axis rotation: dot products between rows and between columns exactly zero, lengths within 3/2 raw (exact on squares), determinant exactly `c^2 + s^2` and within 3 raw of one. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_007_axis_rotations_are_orthogonal_within_the_sine_and_cosine_error` |
| ROT-TP-008 | Seeded oracle and bound | ROT-REQ-003, ROT-REQ-005 | 10 000 seeded axes across the `FixedUnitVec3` band and angles: every entry equals `oracle_axis_angle`; lengths within 11, dot products within 12 and the determinant within 15 raw, decided exactly; digest equal to the Python model's, which found every entry within the two ROT-REQ-003 bounds. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_008_seeded_axis_angle_rotations_match_the_oracle_and_the_bounds` |
| ROT-TP-009 | Boundary | ROT-REQ-003, ROT-REQ-005 | The longest `FixedUnitVec3` along `x`, `(2^32 + 1, 92681, 0)` raw, is accepted and one raw more in either component is not; at the half turn it gives pinned rows with `m00 = 1 + 4` raw and a row more than 7 raw longer than one, within the bounds. The shortest, `(2^32 - 1, 0, 0)`, is accepted and `2^32 - 2` is not; it gives `m00 = 1 - 4` raw. | `crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_009_the_longest_and_shortest_axes_bound_the_error` |
| ROT-TP-010 | Mutation | ROT-REQ-001, ROT-REQ-002, ROT-REQ-003, ROT-REQ-004, ROT-REQ-005 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "ROT-TP-001"
requirements = ["ROT-REQ-001"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_001_quarter_turns_follow_the_right_hand_rule"
oracle = "Hand-derived quarter turns from the coordinate law's cross products; the shipped look rotation."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-002"
requirements = ["ROT-REQ-002"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_002_cardinal_angles_give_exact_zeros_and_ones"
oracle = "Powers of the hand-derived quarter turn at and one angle unit either side of each cardinal angle."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-003"
requirements = ["ROT-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_003_axis_angle_about_the_coordinate_axes_matches_the_axis_rotations"
oracle = "Bit equality with the axis rotations for the six signed unit axes."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-004"
requirements = ["ROT-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_004_zero_is_the_identity_and_a_negated_angle_is_the_transpose"
oracle = "Bit equality with the identity and with the transpose."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-005"
requirements = ["ROT-REQ-002", "ROT-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_005_pinned_goldens"
oracle = "Angle goldens; raw rows from the Python model, each within 1/2 raw of the Decimal ideal, equal to the i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-006"
requirements = ["ROT-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_006_entries_on_a_half_round_to_even"
oracle = "Hand-derived exact halves at 60 degrees rounded to even."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-007"
requirements = ["ROT-REQ-002"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_007_axis_rotations_are_orthogonal_within_the_sine_and_cosine_error"
oracle = "Exact i128 dot products, squared lengths and determinants over 10 000 seeded angles."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-008"
requirements = ["ROT-REQ-003", "ROT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_008_seeded_axis_angle_rotations_match_the_oracle_and_the_bounds"
oracle = "Independent i128 oracle and exact bounds over 10 000 seeded cases; digest equal to the Decimal-checked Python model."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-009"
requirements = ["ROT-REQ-003", "ROT-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/rotation_tests.rs::rot_tp_009_the_longest_and_shortest_axes_bound_the_error"
oracle = "Hand-derived half turns about the extreme FixedUnitVec3 axes, one raw unit inside the type's acceptance."
tier = "fast"
status = "implemented"

[[case]]
id = "ROT-TP-010"
requirements = ["ROT-REQ-001", "ROT-REQ-002", "ROT-REQ-003", "ROT-REQ-004", "ROT-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/rotation/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel, `rotation_tests` and `look_rotation_tests` run, and
the kernel restored; each fails the named cases.

| Mutation | Fails |
| --- | --- |
| `rotation_y` transposed (signs of `s` swapped) | ROT-TP-001, ROT-TP-003 |
| `rotation_z` transposed | ROT-TP-001, ROT-TP-003, ROT-TP-005 |
| `rotation_x` handedness flipped | ROT-TP-001, ROT-TP-003 |
| `rotation_x` with `m00 = c` | ROT-TP-001, ROT-TP-002, ROT-TP-003, ROT-TP-007 |
| Rodrigues skew term negated (the transposed matrix) | ROT-TP-003, ROT-TP-005, ROT-TP-008 |
| Rodrigues with `t = c - 1` | ROT-TP-003, ROT-TP-005, ROT-TP-006, ROT-TP-008, ROT-TP-009 |
| Rodrigues `m10` with `-z` (symmetric off-diagonal pair) | ROT-TP-003, ROT-TP-004, ROT-TP-005, ROT-TP-008 |
| Floor instead of rounding to nearest | ROT-TP-005, ROT-TP-006, ROT-TP-008, ROT-TP-009 |
| Ties up instead of to even | ROT-TP-006 |
| Ties away from zero | ROT-TP-006 |

Only ROT-TP-006 decides ties: no seeded case lands exactly on a half.

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic; the tests use a fixed seed and no clock. They run
natively and in the WebAssembly gate.
