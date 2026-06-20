# Coordinate Law Test Plan

This document defines how Bunny proves the coordinate-law contract.

The current coordinate-law chapter is
[`README.md`](README.md). This test plan is a living verification design, not a
historical proposal.

## Scope

This plan covers Bunny's coordinate-space conventions:

- right-handed 3D basis orientation
- canonical `XY` plane and winding
- unit policy
- coordinate-space naming conventions
- transform, rotation, bounds, and projection reservations
- external-format boundary expectations

Only conventions backed by implemented APIs can have executable tests today.
Reserved future APIs are tracked as open gaps until those APIs exist.

## Test Goals

- Prove that existing vector operations implement the right-handed basis
  required by the coordinate law.
- Prove that `XY` counter-clockwise winding produces a `+Z` normal.
- Keep the tests native and WASM-compatible.
- Assert public behavior through `bunny-linalg` public types.
- Avoid private helpers, stdout, stderr, logs, and documentation prose as test
  oracles.

## Non-Goals

- Matrix, transform, projection, viewport, camera, quaternion, and angle tests
  are not executable until those APIs exist.
- This plan does not test renderer-specific coordinate conventions.
- This plan does not test external file-format conversion until boundary
  adapters explicitly expose those conversions.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| CL-REQ-001 | Bunny's canonical 3D frame is right-handed. | `README.md#canonical-3d-frame` |
| CL-REQ-002 | `+X cross +Y` is `+Z`, with cyclic basis products following the right-hand rule. | `README.md#canonical-3d-frame` |
| CL-REQ-003 | Reversing cross-product operands negates the basis result. | `README.md#canonical-3d-frame` |
| CL-REQ-004 | Bunny's default 2D plane is the `XY` plane. | `README.md#canonical-2d-plane` |
| CL-REQ-005 | Counter-clockwise `XY` winding viewed from `+Z` produces a `+Z` normal. | `README.md#canonical-2d-plane` |
| CL-REQ-006 | Bunny coordinates are unitless fixed-point Bunny units. | `README.md#units` |
| CL-REQ-007 | Future transforms are named `target_from_source`. | `README.md#coordinate-spaces` |
| CL-REQ-008 | Future matrix transforms use column-vector semantics and right-to-left composition. | `README.md#transform-convention` |
| CL-REQ-009 | Positive rotation follows the right-hand rule. | `README.md#rotation-and-angle-direction` |
| CL-REQ-010 | Bounds require `min <= max`, inclusive boundary contact, and explicit emptiness. | `README.md#bounds-and-boundary-contact` |
| CL-REQ-011 | Future canonical NDC uses `x/y` in `[-1, 1]`, `z` in `[0, 1]`, and positive `y` up. | `README.md#projection-and-ndc-reservations` |
| CL-REQ-012 | External boundary adapters document source convention and exact conversion into Bunny convention. | `README.md#external-format-boundaries` |

## Fixtures

No external fixtures are required for the current executable tests.

The basis vectors are constructed directly from public fixed-point values:

- `FixedQ32_32::from_raw(ONE_RAW)`
- `FixedVec3::new`
- public `FixedVec3::cross`

Future projection, transform, mesh, and external-format tests may add checked-in
fixtures. Each fixture must document:

- source
- generation command or hand-computation proof
- expected canonical convention
- regeneration policy
- whether bytes or structured values are the oracle

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| CL-TP-001 | Golden path, determinism | CL-REQ-001, CL-REQ-002, CL-REQ-003 | Exact `FixedVec3` equality for public cross-product outputs. | `coordinate_law_tests::cl_tp_001_canonical_basis_is_right_handed` |
| CL-TP-002 | Golden path, winding | CL-REQ-004, CL-REQ-005 | Exact `FixedVec3` equality for `(b - a).cross(c - a)` and reversed winding. | `coordinate_law_tests::cl_tp_002_xy_counter_clockwise_winding_points_toward_positive_z` |

## Determinism Proof

The current tests use only public deterministic fixed-point operations:

- integer raw construction
- vector subtraction
- vector cross product
- exact equality on raw fixed-point values through `FixedVec3`

There is no randomness, time, filesystem access, locale-sensitive formatting,
parallel scheduling, map iteration, stdout, stderr, or logging in the test
oracle.

The tests run in native Rust test mode and are annotated with
`wasm_bindgen_test`, so the same assertions participate in the headless WASM
gate.

## Known Failures

The current executable surface has no known failing coordinate-law cases.

Future implementation work must add explicit negative tests for:

- invalid transform composition
- point/vector misuse where type APIs can reject it
- inverted bounds where a bounds API owns the rejection
- external-format adapters that omit required convention metadata
- viewport conversions with mismatched origin or `y` direction

## Edge Cases And Unusual Inputs

Current basis tests intentionally use exact unit basis vectors because they
prove convention, not numeric range handling. Numeric range and overflow edges
belong to the numeric constitution and the saturating-arithmetic audit.

Future coordinate APIs must add boundary cases at:

- `min == max` bounds
- zero-length directions
- zero-area triangles
- identity transforms
- inverse transforms
- 180-degree rotations
- near-plane and far-plane NDC limits
- viewport edges and origin flips

## Stress And Fuzz

No fuzz target exists for coordinate law alone today.

Future transform and projection APIs should add seeded property or fuzz tests
for:

- transform inverse round trips
- point versus vector transform differences
- stable projection and unprojection examples
- external-format conversion round trips where the source format is bounded

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| `target_from_source` composition examples. | Matrix and transform primitives. |
| Point versus vector translation behavior. | Typed point/vector/transform APIs. |
| Positive rotation direction beyond cross-product orientation. | Angle, quaternion, or rotation APIs. |
| NDC and viewport conversion examples. | Projection and viewport APIs. |
| External source convention conversion tests. | Format-specific adapter APIs that expose convention conversion. |
