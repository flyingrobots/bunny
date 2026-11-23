# Shape Library Test Plan

This document defines how Bunny proves the [shape library](README.md): `FixedLine3`,
`FixedTriangle3`, `FixedCapsule3`, `FixedObb3` and `ShapeError`, the float boundary
constructors of those four and of `FixedSegment3` and `FixedPlane3`, and the accepted
degenerate cases of the segment and the plane. It answers flyingrobots/bunny#115.

## Scope

- The representation, constructors, invariants and degenerate cases of the six shapes.
- The exact triangle degeneracy test and the exact oriented-box axis check.
- The float boundary constructors, their refusal order and their rounding through the
  existing component conversions.

Out of scope: queries between shapes (intersection, distance, containment: flyingrobots/bunny#117
and flyingrobots/bunny#242), bounds (flyingrobots/bunny#151) and plane classification
(`docs/topics/plane-clipping/`), and the existing ray, box and sphere, whose tests are
unchanged.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| SHAPE-REQ-001 | `FixedLine3::new` stores its point and unit direction; `try_new` accepts a direction exactly when `FixedUnitVec3::new` does (the rule of `FixedRay3::try_new`) and returns that unit direction, else `InvalidLineDirection`; equality is representation equality. | `README.md#line` |
| SHAPE-REQ-002 | `FixedTriangle3::new` accepts every input and keeps vertex order; `is_degenerate` is true exactly when the exact cross product of the raw edges is zero, for every vertex order and over the full raw range; `try_new` refuses exactly those with `DegenerateTriangle`. | `README.md#triangle` |
| SHAPE-REQ-003 | `FixedCapsule3::new` does not validate; `try_new` refuses exactly a raw radius below zero with `NegativeCapsuleRadius` and accepts a zero radius and a zero-length segment. | `README.md#capsule` |
| SHAPE-REQ-004 | `FixedObb3::try_new` refuses the first negative half extent, then the first column whose exact squared length is outside `[(2^32 - 11)^2, (2^32 + 11)^2]`, then the first pair, in the order (0, 1), (0, 2), (1, 2), whose exact dot product exceeds `12 * 2^32` in magnitude; it accepts zero half extents and reflections, and never overflows. | `README.md#oriented-box` |
| SHAPE-REQ-005 | `FixedObb3::try_new` accepts the matrices returned by `rotation_x`, `rotation_y`, `rotation_z`, `rotation_axis_angle` and `try_look_rotation`. | `README.md#oriented-box` |
| SHAPE-REQ-006 | Each `try_from_float` refuses, in order, a non-finite coordinate, a non-finite capsule radius, a float radius or half extent below zero (before rounding; `-0.0` accepted), a value outside Q32.32 after rounding, then the fixed `try_new` refusals on the rounded values; an accepted result equals the fixed constructor on `FixedQ32_32::try_from_f32` of each component. | `README.md#float-boundary` |
| SHAPE-REQ-007 | A zero-length `FixedSegment3` and a zero-normal `FixedPlane3` are accepted, in fixed and float form. | `README.md#segment` |
| SHAPE-REQ-008 | `ShapeError` implements `core::error::Error` and displays one stated message per variant. | `README.md#conventions` |

```toml
[[requirement]]
id = "SHAPE-REQ-001"
summary = "FixedLine3::try_new accepts exactly what FixedUnitVec3::new (the ray's rule) accepts, else InvalidLineDirection; equality is representation equality."
status = "active"

[[requirement]]
id = "SHAPE-REQ-002"
summary = "FixedTriangle3 keeps vertex order; is_degenerate is the exact zero test of the raw cross product, order independent, over the full range; try_new refuses exactly degenerate triangles."
status = "active"

[[requirement]]
id = "SHAPE-REQ-003"
summary = "FixedCapsule3::try_new refuses exactly a negative raw radius; zero radius and zero-length segments are accepted; new does not validate."
status = "active"

[[requirement]]
id = "SHAPE-REQ-004"
summary = "FixedObb3::try_new refuses negative half extents, then column lengths outside 11 raw of one, then column dot products beyond 12 raw, each first failure named, exactly and without overflow."
status = "active"

[[requirement]]
id = "SHAPE-REQ-005"
summary = "FixedObb3::try_new accepts the matrices of the axis, axis-angle and look rotation constructors."
status = "active"

[[requirement]]
id = "SHAPE-REQ-006"
summary = "try_from_float refuses non-finite, then negative float radius or half extent, then out-of-range, then the fixed refusals; accepted results equal the fixed constructor on try_from_f32 components."
status = "active"

[[requirement]]
id = "SHAPE-REQ-007"
summary = "Zero-length segments and zero-normal planes are accepted, fixed and float."
status = "active"

[[requirement]]
id = "SHAPE-REQ-008"
summary = "ShapeError implements core::error::Error with one stated message per variant."
status = "active"
```

## Oracles

`crates/bunny-geom/tests/shape_oracle/mod.rs` holds the exact oracles and shares no code with
the kernel. `Wide` is a 256-bit two's-complement integer on eight 32-bit limbs with products
formed limb by limb in `u64`; every quantity is below `2^132` in magnitude, so arithmetic
modulo `2^256` is exact. `triangle_is_degenerate` forms all three cross components in full
(the kernel never forms one; it compares sign-and-magnitude `u128` products).
`obb_axes_verdict` forms every squared length and dot product in `Wide` for any entries, with
the documented order (the kernel squares in `u128` and forms dot products in `i128` only
after every length passes). For lines the oracle is the ray: `FixedRay3::try_new` and
`FixedUnitVec3::new`, whose rule the line adopts by definition. For floats it is the fixed
constructor on `FixedQ32_32::try_from_f32` of each component.

Seeded tests use the generator in the oracle module with the seed in every failure message:
`0x5EED_0115_0000_0001` (lines), `0x5EED_0115_0000_0003` (triangles), `0x5EED_0115_0000_0007`
(rotations) and `0x5EED_0115_0000_0008` (boxes). Every test is a `wasm_bindgen_test`, so the
WebAssembly job (`wasm-pack test --node crates/bunny-geom`) runs them all.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| SHAPE-TP-001 | SHAPE-REQ-001 | Stored fields; axis-aligned directions of any length give the exact unit axis; a line through another point, or reversed, is not `==`; the line and the ray share a representation. | `crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_001_lines_keep_their_representation` |
| SHAPE-TP-002 | SHAPE-REQ-001 | The zero direction refused and one raw unit either side of it accepted on each axis; `(1, 1, 0)` raw, `(i64::MAX, i64::MAX, 0)` and `(i64::MIN, 0, 0)` refused, `(i64::MAX, 0, 0)` accepted; 20 000 seeded directions (magnitudes up to `2^61`; 10 466 accepted, 9 534 refused) agree with `FixedRay3::try_new` and `FixedUnitVec3::new`. | `crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_002_line_directions_follow_the_ray_rule` |
| SHAPE-TP-003 | SHAPE-REQ-002 | Hand-built: coincident, collinear, collinear across `v0` and mixed-sign zero products degenerate; one raw unit off collinear not (its rounded cross product is zero); the whole-range diagonal and one raw unit off it; equal products near `2^127` and six one-raw-unit neighbours; a cross product of `-1` at `2^128`; each in all six vertex orders against the oracle; winding kept and significant. | `crates/bunny-geom/tests/shape_triangle_tests.rs::shape_tp_003_hand_built_degenerate_and_near_degenerate_triangles` |
| SHAPE-TP-004 | SHAPE-REQ-002 | 30 000 seeded triangles, each in all six orders against the oracle: 10 000 of independent full-range vertices (all proper), 10 000 constructed collinear (all degenerate) and 10 000 of those moved one raw unit (9 723 proper). | `crates/bunny-geom/tests/shape_triangle_tests.rs::shape_tp_004_seeded_triangles_match_the_exact_oracle` |
| SHAPE-TP-005 | SHAPE-REQ-003 | Radius `-1`, `0`, `1` raw and both ends of the range; a zero-length segment with and without radius; `new` keeps a negative radius; endpoint order significant; `try_new` in a constant. | `crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_005_capsule_radius_and_degenerate_capsules` |
| SHAPE-TP-006 | SHAPE-REQ-004 | Half extents `-1` and `0` raw per axis, the first negative named; each column's length at `2^32 +- 11` and one raw unit past, also reflected; a diagonal column on and one raw unit past the bound; dot products at `+-12` and `+-13` raw for each pair; lengths before dots; `i64::MIN` and `i64::MAX` columns; a zero axis; equal axes; a reflection; all against the oracle. | `crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_006_hand_built_boxes_on_and_beside_each_threshold` |
| SHAPE-TP-007 | SHAPE-REQ-005 | 3 000 seeded angles through each axis rotation, `rotation_axis_angle` about a seeded axis and about the two `FixedUnitVec3` extremes, and 3 000 seeded `try_look_rotation` results (all accepted by it), plus the half turn about the longest axis: each accepted by the kernel and the oracle. | `crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_007_every_sampled_rotation_constructor_result_is_accepted` |
| SHAPE-TP-008 | SHAPE-REQ-004 | 30 000 seeded matrices (rotations with every entry moved by up to 8, 16, 40 or `2^20` raw, and one in ten of random full-range entries) against the oracle: 5 919 accepted, 20 284 refused by length and 3 797 by a dot product. | `crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_008_seeded_boxes_match_the_exact_oracle` |
| SHAPE-TP-009 | SHAPE-REQ-006 | Each new shape from floats equals the fixed constructor on `try_from_f32` components; a direction and a triangle that round to zero refused by the fixed rule; radius and half extent `-0.0` accepted, `-1e-12` refused; `2^31` refused and `2^31 - 128` accepted in every input; `NaN` and both infinities in every input, before range and before sign; the box's float axes are columns. | `crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_009_float_constructors_round_then_validate` |
| SHAPE-TP-010 | SHAPE-REQ-006, SHAPE-REQ-007 | A zero-length segment and a zero-normal plane (distance `-offset`); their float forms equal the fixed ones; `2^31` refused and `-2^31` accepted, `-2^31 - 256` refused; `NaN` and infinities in every input. | `crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_010_segments_and_planes_accept_their_degenerate_cases` |
| SHAPE-TP-011 | SHAPE-REQ-008 | Every variant's message, and `ShapeError: core::error::Error`. | `crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_011_shape_error_is_an_error_with_messages` |
| SHAPE-TP-012 | SHAPE-REQ-002, SHAPE-REQ-003, SHAPE-REQ-004, SHAPE-REQ-006 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "SHAPE-TP-001"
requirements = ["SHAPE-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_001_lines_keep_their_representation"
oracle = "Stored fields and the unit axis constants."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-002"
requirements = ["SHAPE-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_002_line_directions_follow_the_ray_rule"
oracle = "FixedRay3::try_new and FixedUnitVec3::new on constructed and 20 000 seeded directions."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-003"
requirements = ["SHAPE-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_triangle_tests.rs::shape_tp_003_hand_built_degenerate_and_near_degenerate_triangles"
oracle = "Independent 256-bit cross product on hand-built triangles at the ends of the raw range."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-004"
requirements = ["SHAPE-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_triangle_tests.rs::shape_tp_004_seeded_triangles_match_the_exact_oracle"
oracle = "Independent 256-bit cross product on 30 000 seeded triangles in six vertex orders."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-005"
requirements = ["SHAPE-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_line_capsule_tests.rs::shape_tp_005_capsule_radius_and_degenerate_capsules"
oracle = "Hand-derived radius thresholds and degenerate capsules."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-006"
requirements = ["SHAPE-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_006_hand_built_boxes_on_and_beside_each_threshold"
oracle = "Independent 256-bit lengths and dot products on hand-built thresholds."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-007"
requirements = ["SHAPE-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_007_every_sampled_rotation_constructor_result_is_accepted"
oracle = "Independent 256-bit verdict on seeded rotation constructor results."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-008"
requirements = ["SHAPE-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_obb_tests.rs::shape_tp_008_seeded_boxes_match_the_exact_oracle"
oracle = "Independent 256-bit verdict on 30 000 seeded matrices."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-009"
requirements = ["SHAPE-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_009_float_constructors_round_then_validate"
oracle = "Fixed constructors on FixedQ32_32::try_from_f32 components; hand-derived f32 range edges."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-010"
requirements = ["SHAPE-REQ-006", "SHAPE-REQ-007"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_010_segments_and_planes_accept_their_degenerate_cases"
oracle = "Fixed constructors on FixedQ32_32::try_from_f32 components; the documented zero-normal distance."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-011"
requirements = ["SHAPE-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/shape_float_tests.rs::shape_tp_011_shape_error_is_an_error_with_messages"
oracle = "The stated messages."
tier = "fast"
status = "planned"

[[case]]
id = "SHAPE-TP-012"
requirements = ["SHAPE-REQ-002", "SHAPE-REQ-003", "SHAPE-REQ-004", "SHAPE-REQ-006"]
evidence = "manual-audit"
artifact = "docs/topics/shape-library/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "planned"
```

## Mutations

Each mutation was applied to `crates/bunny-geom/src/shapes/`, the four shape test files run,
and the source restored.

| Mutation | Fails |
| --- | --- |
| A zero product may be negative | SHAPE-TP-003, SHAPE-TP-004 |
| Only the `z` component of the cross product tested | SHAPE-TP-003, SHAPE-TP-004 |
| The rounded Q32.32 cross product tested for zero | SHAPE-TP-003, SHAPE-TP-004 |
| Products truncated to 64 bits | SHAPE-TP-003, SHAPE-TP-009 |
| A zero capsule radius refused | SHAPE-TP-005, SHAPE-TP-009 |
| Length tolerance 10 | SHAPE-TP-006, SHAPE-TP-008 |
| Dot tolerance 13 | SHAPE-TP-006, SHAPE-TP-008 |
| Dot bound exclusive (`>=`) | SHAPE-TP-006 |
| A zero half extent refused | SHAPE-TP-006, SHAPE-TP-007, SHAPE-TP-008, SHAPE-TP-009 |
| Rows checked instead of columns | SHAPE-TP-006, SHAPE-TP-008, SHAPE-TP-009 |
| A line direction refused only when zero | SHAPE-TP-002 |
| Line float: range checked before finiteness | SHAPE-TP-009 |
| Capsule float: sign not checked before rounding | SHAPE-TP-009 |
| Box float: axes taken as rows | SHAPE-TP-009 |

Two mutations are equivalent and survive by design: inverting the sign of every product
(both sides of each comparison flip together), and taking the first edge as `v2 - v1`
(the exact cross product is the same up to sign).

## Determinism Obligations And Evidence

The kernels use only integer arithmetic, and the float constructors only the existing
bit-level conversions; the tests use fixed seeds and no clock. They run natively and in the
WebAssembly gate.
