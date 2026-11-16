# Shape Frame Test Plan

This document defines how Bunny proves the [shape frame contract](README.md).

## Scope

- `ShapeFrame::new` and `ShapeFrame::translation` and their refusals
- `ShapeFrame::to_object` and `ShapeFrame::to_world`: every output bit, the rounding rule
  and the refusals
- the contract identity `bunny-shape-frame/1`

## Test Goals

- Compare every output with an exact rational oracle written in the test file
  (`nearest_even` over exact `i128` sums), which shares no code with the kernel.
- Name the geometric cases the issue calls out: grazing, inside-origin and behind-origin
  rays.
- Show each rounding direction, each inclusive limit and each refusal is load bearing by
  breaking it on purpose.
- Run natively and in the WASM gate.

## Non-Goals

- Scale and shear (not part of the contract).
- Shape intersection kernels, which have their own chapters.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| SF-REQ-001 | Every component of `to_object` and `to_world` is the exact Q64.64 dot product of the raw axes and raw inputs, rounded once to Q32.32 to nearest, ties to even. | `README.md#mapping-a-ray-into-the-object-frame` |
| SF-REQ-002 | `new` refuses an origin outside the trace domain, axes whose exact dot product exceeds `2^40 / 2^64` in magnitude (the limit inclusive) and left-handed axes, each with its typed error. | `README.md#the-frame` |
| SF-REQ-003 | `to_object` refuses an object origin outside the trace domain and an object direction component above `2^32 + 1` raw (the limit inclusive); `to_world` refuses a component outside `i64`; nothing wraps or panics. | `README.md#mapping-a-ray-into-the-object-frame` |
| SF-REQ-004 | Grazing, inside-origin and behind-origin rays map exactly as the rounding rule says. | `README.md#degenerate-and-boundary-cases` |
| SF-REQ-005 | `SHAPE_FRAME_ID` is `bunny-shape-frame/1`. | `README.md#contract-identity` |

```toml
[[requirement]]
id = "SF-REQ-001"
summary = "Every component of to_object and to_world is the exact Q64.64 dot product of the raw axes and raw inputs, rounded once to Q32.32 to nearest, ties to even."
status = "active"

[[requirement]]
id = "SF-REQ-002"
summary = "new refuses an origin outside the trace domain, axes whose exact dot product exceeds 2^40 / 2^64 in magnitude (the limit inclusive) and left-handed axes, each with its typed error."
status = "active"

[[requirement]]
id = "SF-REQ-003"
summary = "to_object refuses an object origin outside the trace domain and an object direction component above 2^32 + 1 raw (the limit inclusive); to_world refuses a component outside i64; nothing wraps or panics."
status = "active"

[[requirement]]
id = "SF-REQ-004"
summary = "Grazing, inside-origin and behind-origin rays map exactly as the rounding rule says."
status = "active"

[[requirement]]
id = "SF-REQ-005"
summary = "SHAPE_FRAME_ID is bunny-shape-frame/1."
status = "active"
```

## Fixtures

None. Generated frames are rotations of integer quaternions with components in
`-2^15..=2^15`, whose matrix entries are exact rationals over `|q|^2`, rounded to the
nearest raw unit; a row that `FixedUnitVec3::try_from_unit` refuses discards the quaternion.
The seed is `0x5AFE_F4A3_0357_0001`; every failure message names it, the frame and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| SF-TP-001 | Exact oracle | SF-REQ-001, SF-REQ-003 | 300 generated frames with origins anywhere in the trace domain, 40 rays each (origins anywhere, direction bounds from 1.0 down to one raw unit) and 40 normals (magnitudes up to `2^62`): `to_object` and `to_world`, results and refusals, equal the oracle; at least one refusal and 1 000 maps occur. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_001_maps_equal_the_exact_rational_oracle` |
| SF-TP-003 | Grazing | SF-REQ-004, SF-REQ-001 | Under an axis permutation a ray in the object plane `z = 0` maps to hand-computed values with zero `z` origin and direction; a nearly grazing ray under a rotation equals the oracle. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_003_grazing_ray_stays_in_the_object_plane` |
| SF-TP-004 | Inside origin | SF-REQ-004 | A ray from the frame origin has object origin `(0, 0, 0)` and the oracle's direction. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_004_ray_from_the_frame_origin_starts_at_object_zero` |
| SF-TP-005 | Behind origin | SF-REQ-004 | Under an axis permutation a ray moving away from the frame origin maps to hand-computed values with `origin . direction > 0`. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_005_frame_origin_behind_the_ray_stays_behind` |
| SF-TP-006 | Ties | SF-REQ-001 | With an axis x component of exactly `2^31` raw, offsets of ±1 and ±3 raw give ±0.5 and ±1.5 raw units: 0 and ±2 from both maps; rounding half up fails. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_006_ties_round_to_even` |
| SF-TP-007 | Frame refusals | SF-REQ-002 | Origins at `±2^40` refused and `2^40 - 1` accepted; swapped axes left-handed; a repeated axis and a dot just above the limit not orthogonal, one at the limit accepted. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_007_invalid_frames_are_refused` |
| SF-TP-008 | Map refusals | SF-REQ-003 | An object origin `2^41 - 2` raw away refused, one inside accepted; `(1, 1, 0)` rotated by 45 degrees refused; a direction of `2^32 + 1` raw accepted and its image `2^32 + 2` refused; `(MAX, MAX, 0)` rotated refused as the oracle says. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_008_unrepresentable_results_are_refused` |
| SF-TP-009 | Identity | SF-REQ-005 | The string `bunny-shape-frame/1`. | `crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_009_contract_identity_is_versioned` |

```toml
[[case]]
id = "SF-TP-001"
requirements = ["SF-REQ-001", "SF-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_001_maps_equal_the_exact_rational_oracle"
oracle = "Exact rational oracle on 300 generated frames, 12 000 rays and normals, refusals included."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-003"
requirements = ["SF-REQ-004", "SF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_003_grazing_ray_stays_in_the_object_plane"
oracle = "Hand-computed values under an axis permutation; the exact rational oracle under a rotation."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-004"
requirements = ["SF-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_004_ray_from_the_frame_origin_starts_at_object_zero"
oracle = "Object origin (0, 0, 0) exactly; direction equals the exact rational oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-005"
requirements = ["SF-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_005_frame_origin_behind_the_ray_stays_behind"
oracle = "Hand-computed values under an axis permutation, with origin . direction > 0."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-006"
requirements = ["SF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_006_ties_round_to_even"
oracle = "Exact half-unit ties give the even neighbour from both maps."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-007"
requirements = ["SF-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_007_invalid_frames_are_refused"
oracle = "Typed refusals at and beside each frame limit."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-008"
requirements = ["SF-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_008_unrepresentable_results_are_refused"
oracle = "Typed refusals at and beside each map limit; the oracle's refusal for the large normal."
tier = "fast"
status = "implemented"

[[case]]
id = "SF-TP-009"
requirements = ["SF-REQ-005"]
evidence = "test"
test = "crates/bunny-query/tests/shape_frame_tests.rs::sf_tp_009_contract_identity_is_versioned"
oracle = "The literal identity string."
tier = "fast"
status = "implemented"
```

## Mutations

Each mutation was applied to `crates/bunny-query/src/frame.rs` and the test file run; each
fails the named cases.

| Mutation | Fails |
| --- | --- |
| Ties round half up | SF-TP-006 |
| Floor instead of nearest | SF-TP-001, SF-TP-003, SF-TP-004 |
| Handedness check accepts negative | SF-TP-007 |
| Orthogonality limit exclusive | SF-TP-007 |
| Object origin domain check removed | SF-TP-001, SF-TP-008 |
| Direction limit exclusive | SF-TP-008 |
| `to_world` uses rows instead of columns | SF-TP-001 |
