# Look Rotation Test Plan

This document defines how Bunny proves the [look rotation contract](README.md).

## Scope

- `FixedMat3::try_look_rotation`: its convention, its rounding bounds and its
  refusals.

## Test Goals

- Pin the convention with a hand-derived matrix, so a flipped cross product or a
  transposed result fails.
- Decide every bound with exact `i128` arithmetic written in the test file,
  which shares no code with the kernel.
- Put each refusal threshold between two cases one raw unit apart.
- Show each rule is load bearing by breaking it on purpose.
- Run natively and in the WASM gate.

## Non-Goals

- The look-at view matrix, covered by the
  [matrix types test plan](../matrix-types/test-plan.md).
- Quaternions and shortest-arc rotations.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it
is the contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| LR-REQ-001 | The columns are `right = normalize(f x up)`, `true_up = normalize(right x f)` and `-f` with `f = normalize(forward)`, so local `-Z` maps onto `forward` in the right-handed frame of the coordinate law. | `README.md#the-rotation` |
| LR-REQ-002 | Each normalized component is within `1/2 + 2^-26` raw of the exact unit direction (rounding half away from zero); column lengths are within 2 raw of one and column dot products within 2 raw of zero; row lengths are within 3 raw of one and row dot products within 4 raw of zero. | `README.md#rounding-and-orthonormality` |
| LR-REQ-003 | An exactly zero `forward` is `CoincidentEyeTarget`; a zero `up`, an `up` exactly parallel or antiparallel to `forward`, or exactly parallel to the rounded `f`, is `ParallelUp`; `Overflow` is never returned and nothing panics. | `README.md#refusals` |

```toml
[[requirement]]
id = "LR-REQ-001"
summary = "The columns are right = normalize(f x up), true_up = normalize(right x f) and -f with f = normalize(forward), so local -Z maps onto forward in the right-handed frame of the coordinate law."
status = "active"

[[requirement]]
id = "LR-REQ-002"
summary = "Each normalized component is within 1/2 + 2^-26 raw of the exact unit direction (rounding half away from zero); column lengths are within 2 raw of one and column dot products within 2 raw of zero; row lengths are within 3 raw of one and row dot products within 4 raw of zero."
status = "active"

[[requirement]]
id = "LR-REQ-003"
summary = "An exactly zero forward is CoincidentEyeTarget; a zero up, an up exactly parallel or antiparallel to forward, or exactly parallel to the rounded f, is ParallelUp; Overflow is never returned and nothing panics."
status = "active"
```

## Fixtures

None. LR-TP-005 draws 10 000 cases from seed `0xB002_0310_0000_0001` with a
64-bit LCG: each raw component of `forward` has a random bit width from 1 to 63
and a random sign; even cases draw `up` the same way and odd cases use `+Y`.
Every failure message names the seed, the case and the inputs.

A separate Python model, written from the [chapter](README.md) with unbounded
integers, ran the same corpus and checked every result in `Decimal` at 80 digits
against the exact geometry: each component of `f`, `right` and `true_up` within
`1/2 + 2^-26` raw of the exact unit direction of the vector it normalized, and
the four orthonormality bounds. All held. The largest values seen were 0.49999
(component), 0.73 (column length), 0.65 (column dot), 0.67 (row length) and 1.09
(row dot). The model gave 9 798 rotations and 202 `ParallelUp` refusals (all with
`up = +Y`: 4 forwards exactly along `±Y` and 198 whose rounded `f` is `±Y`) and the FNV-1a digest
`0xF8FC_3439_149D_2CF2` over the result stream, which the Rust test pins. The
LR-TP-003 golden and its ideal columns also come from the model.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| LR-TP-001 | Hand-derived | LR-REQ-001 | Forward `(1, 0, 0)`, up `+Y` gives rows `(0, 0, -1)`, `(0, 1, 0)`, `(1, 0, 0)` exactly, also for a longer forward and up and a 1-raw forward; forward `-Z` is the identity, forward `+Z` the half turn. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_001_forward_x_with_up_y_is_the_hand_derived_quarter_turn` |
| LR-TP-002 | Handedness | LR-REQ-001 | Local `-Z`, `+Y`, `+X` map to forward, up and right; determinant exactly 1; looking down with up `-Z` gives the hand-derived -90 degree turn about `+X`. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_002_local_axes_map_onto_right_up_and_forward` |
| LR-TP-003 | Pinned golden | LR-REQ-001, LR-REQ-002 | Forward `(1, 2, 3)`, up `+Y`: pinned raw rows, each within 1/2 raw of the Decimal ideal. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_003_diagonal_forward_has_the_pinned_golden_matrix` |
| LR-TP-004 | Refusals | LR-REQ-003 | Zero forward refused, 1 raw along `±Z` accepted; zero up refused, 1 raw accepted; parallel and antiparallel up at several scales (and all `i64::MIN`) refused, 1 raw off accepted; `(2^40, 2^40 + 1, 0)` with up `(1, 1, 0)` refused although not parallel, up 1 raw away accepted. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_004_degenerate_inputs_are_refused_one_raw_unit_from_the_threshold` |
| LR-TP-005 | Seeded bound | LR-REQ-002, LR-REQ-003 | 10 000 seeded cases: exact `i128` length and dot bounds on every column and row; exactly 9 798 `Ok` and 202 `ParallelUp`, no other error; digest equal to the Python model's. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_005_seeded_directions_are_orthonormal_within_the_documented_bound` |
| LR-TP-006 | Scaling | LR-REQ-002, LR-REQ-003 | Up `(2^32, 0, 0)` raw: forward `(2^61, 2^28 - 1, 0)` is accepted with back `-(2^32, 1, 0)`; three times that forward is refused with `ParallelUp`, so the result is not scale invariant. | `crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_006_scaling_forward_can_change_the_rounded_direction` |

```toml
[[case]]
id = "LR-TP-001"
requirements = ["LR-REQ-001"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_001_forward_x_with_up_y_is_the_hand_derived_quarter_turn"
oracle = "Hand-derived quarter turn about +Y, identity for forward -Z, half turn for forward +Z."
tier = "fast"
status = "implemented"

[[case]]
id = "LR-TP-002"
requirements = ["LR-REQ-001"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_002_local_axes_map_onto_right_up_and_forward"
oracle = "Local axes map to forward, up and right; determinant exactly 1; hand-derived turn about +X."
tier = "fast"
status = "implemented"

[[case]]
id = "LR-TP-003"
requirements = ["LR-REQ-001", "LR-REQ-002"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_003_diagonal_forward_has_the_pinned_golden_matrix"
oracle = "Pinned raw rows for forward (1, 2, 3), each within 1/2 raw of the Decimal ideal."
tier = "fast"
status = "implemented"

[[case]]
id = "LR-TP-004"
requirements = ["LR-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_004_degenerate_inputs_are_refused_one_raw_unit_from_the_threshold"
oracle = "Each refusal class returns its LookAtError; one raw unit away is accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "LR-TP-005"
requirements = ["LR-REQ-002", "LR-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_005_seeded_directions_are_orthonormal_within_the_documented_bound"
oracle = "Exact i128 bounds on 10 000 seeded cases; counts and digest equal the Decimal-checked Python model."
tier = "fast"
status = "implemented"

[[case]]
id = "LR-TP-006"
requirements = ["LR-REQ-002", "LR-REQ-003"]
evidence = "test"
test = "crates/bunny-linalg/tests/look_rotation_tests.rs::lr_tp_006_scaling_forward_can_change_the_rounded_direction"
oracle = "Hand-derived rounding of the two forwards: (2^32, 1, 0) and (2^32, 0, 0)."
tier = "fast"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel and the test file run; each fails the
named cases.

| Mutation | Fails |
| --- | --- |
| `cross(up, forward)` for right (handedness flipped) | LR-TP-001, LR-TP-002, LR-TP-003, LR-TP-004, LR-TP-005 |
| Rows instead of columns (no transpose) | LR-TP-001, LR-TP-002, LR-TP-003, LR-TP-005 |
| `forward x right` for true up | LR-TP-001, LR-TP-002, LR-TP-003, LR-TP-004, LR-TP-005 |
| Forward instead of back as the third column | LR-TP-001, LR-TP-002, LR-TP-003, LR-TP-004, LR-TP-005 |
| Truncate instead of rounding half away from zero | LR-TP-003, LR-TP-005 |
| Exact parallel check removed | LR-TP-004 |
| Zero forward reported as `ParallelUp` | LR-TP-004 |

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic; the tests use a fixed seed and no
clock. They run natively and in the WebAssembly gate.
