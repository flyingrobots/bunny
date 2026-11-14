# Plane Classification And Segment Clipping Test Plan

This document defines how Bunny verifies the fixed-point plane and segment
clipping contract.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design.

## Scope

This plan covers `FixedPlane3`, `PlaneSide`, `FixedSegment3`, and `SegmentClip`
in `bunny-geom`:

- signed distance and three-way point classification;
- the rule for a point exactly on the plane;
- clipping one segment to the closed front half-space;
- the rounding of the clipped endpoint;
- checked overflow behavior.

Polygon clipping is out of scope (flyingrobots/bunny#204).

## Test Goals

- Prove classification and clipping agree on which side a point on the plane
  belongs to.
- Assert exact raw Q32.32 outputs where the intersection is representable.
- Lock the clipped endpoint's rounding with golden raw vectors.
- Prove overflow returns `None` instead of panicking or saturating.
- Compare a seeded random sweep against an exact `i128` reference.
- Stay native and WASM compatible through `wasm_bindgen_test`.

## Requirements

The table is the human-readable view. The fenced TOML block after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| PC-REQ-001 | Classification is Front for positive signed distance, Behind for negative, On for exactly zero. | `README.md#classification` |
| PC-REQ-002 | Clipping keeps the closed front half-space: a point with signed distance zero is kept. | `README.md#on-plane-rule` |
| PC-REQ-003 | A crossing segment returns the front part, original endpoint order preserved, with one checked division. | `README.md#clipping` |
| PC-REQ-004 | Fully behind segments return Outside; fully front or coplanar segments return unchanged. | `README.md#clipping` |
| PC-REQ-005 | The clipped endpoint rounds the parameter with ties-to-even and each coordinate offset with ties-to-even. | `README.md#rounding-of-the-clipped-endpoint` |
| PC-REQ-006 | Overflow in distance, parameter or interpolation returns None. | `README.md#overflow` |
| PC-REQ-007 | The clipped endpoint stays within a stated raw bound of the exact intersection. | `README.md#rounding-of-the-clipped-endpoint` |

```toml
[[requirement]]
id = "PC-REQ-001"
summary = "Classification is Front for positive signed distance, Behind for negative, On for exactly zero."
status = "active"

[[requirement]]
id = "PC-REQ-002"
summary = "Clipping keeps the closed front half-space: a point with signed distance zero is kept."
status = "active"

[[requirement]]
id = "PC-REQ-003"
summary = "A crossing segment returns the front part, original endpoint order preserved, with one checked division."
status = "active"

[[requirement]]
id = "PC-REQ-004"
summary = "Fully behind segments return Outside; fully front or coplanar segments return unchanged."
status = "active"

[[requirement]]
id = "PC-REQ-005"
summary = "The clipped endpoint rounds the parameter with ties-to-even and each coordinate offset with ties-to-even."
status = "active"

[[requirement]]
id = "PC-REQ-006"
summary = "Overflow in distance, parameter or interpolation returns None."
status = "active"

[[requirement]]
id = "PC-REQ-007"
summary = "The clipped endpoint stays within a stated raw bound of the exact intersection."
status = "active"
```

## Fixtures

No external fixtures. Every case builds values from `FixedQ32_32::from_raw`.
Exact cases use the plane `z = 1` and endpoints on a power-of-two grid so the
intersection is representable.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| PC-TP-001 | Classification | PC-REQ-001 | Front, Behind and On for axis and non-unit normals. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_001_classification_has_three_outcomes` |
| PC-TP-002 | Crossing, exact | PC-REQ-003 | Plane `z = 1`, intersection at t = 1/4, exact raw points, both directions. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_002_crossing_segment_keeps_front_part_exactly` |
| PC-TP-003 | Both front, both behind | PC-REQ-004 | Unchanged / Outside. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_003_fully_front_is_kept_and_fully_behind_is_outside` |
| PC-TP-004 | Touching at one end | PC-REQ-002 | On-plane end is kept: degenerate segment or whole segment. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_004_touching_end_belongs_to_front` |
| PC-TP-005 | Coplanar | PC-REQ-002, PC-REQ-004 | Segment inside the plane is returned unchanged. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_005_coplanar_segment_is_kept` |
| PC-TP-006 | Golden, inexact | PC-REQ-005 | t = 1/3 gives raw 1431655765 and endpoint raw 4294967295. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_006_golden_third_locks_raw_outputs` |
| PC-TP-007 | Ties-to-even | PC-REQ-005 | Constructed ties at 1.5 and 0.5 raw round to 2 and 0. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_007_parameter_ties_round_to_even` |
| PC-TP-008 | Overflow | PC-REQ-006 | Distance and interpolation overflow return None. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_008_overflow_returns_none` |
| PC-TP-009 | Seeded sweep | PC-REQ-001..004, PC-REQ-007 | Exact i128 rational intersection, classification consistency, retained endpoints on the front. | `crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_009_seeded_sweep_matches_exact_reference` |

```toml
[[case]]
id = "PC-TP-001"
requirements = ["PC-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_001_classification_has_three_outcomes"
oracle = "Front, Behind and On for axis and non-unit normals."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-002"
requirements = ["PC-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_002_crossing_segment_keeps_front_part_exactly"
oracle = "Plane z = 1, intersection at t = 1/4, exact raw points, both directions."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-003"
requirements = ["PC-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_003_fully_front_is_kept_and_fully_behind_is_outside"
oracle = "Unchanged for fully front, Outside for fully behind."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-004"
requirements = ["PC-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_004_touching_end_belongs_to_front"
oracle = "On-plane end is kept: degenerate segment or whole segment."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-005"
requirements = ["PC-REQ-002", "PC-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_005_coplanar_segment_is_kept"
oracle = "Segment inside the plane is returned unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-006"
requirements = ["PC-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_006_golden_third_locks_raw_outputs"
oracle = "t = 1/3 gives raw 1431655765 and endpoint raw 4294967295."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-007"
requirements = ["PC-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_007_parameter_ties_round_to_even"
oracle = "Constructed ties at 1.5 and 0.5 raw round to 2 and 0."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-008"
requirements = ["PC-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_008_overflow_returns_none"
oracle = "Distance and interpolation overflow return None."
tier = "fast"
status = "implemented"

[[case]]
id = "PC-TP-009"
requirements = ["PC-REQ-001", "PC-REQ-002", "PC-REQ-003", "PC-REQ-004", "PC-REQ-007"]
evidence = "test"
test = "crates/bunny-geom/tests/plane_clip_tests.rs::pc_tp_009_seeded_sweep_matches_exact_reference"
oracle = "Exact i128 rational intersection, classification consistency, retained endpoints on the front."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

Tests use only integer raw construction, checked Q32.32 operations and exact
equality. The sweep uses a fixed-seed xorshift generator; there is no time,
filesystem, locale or logging in any oracle.

## Known Failures

- The clipped endpoint can lie behind the plane by rounding error (PC-TP-006
  shows one raw unit). Clipping does not repair it; the stated bound is the
  contract.

## Edge Cases And Unusual Inputs

- A zero normal gives the constant signed distance `-offset` for every point.
- A segment touching the plane at one end returns a degenerate segment.
- Minimum and maximum raw coordinates overflow into `None`.

## Stress And Fuzz

PC-TP-009 runs 2000 seeded segments against an exact reference. Any failure
must be minimized into a named regression case.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Polygon clipping. | flyingrobots/bunny#204. |
| Normalized or validated plane constructors. | A concrete consumer need. |
