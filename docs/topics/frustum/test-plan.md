# Frustum Classification Test Plan

This document defines how Bunny verifies the [frustum chapter](README.md): `FixedFrustum`,
`FrustumClass` and `FrustumSphereError` in `bunny-geom`. It answers flyingrobots/bunny#146.

## Scope

- The six-plane frustum: plane order, orientation (front faces into the frustum) and the
  closed inside.
- The combination rule for points, boxes and spheres, with the overflow rule.
- Box classification by `FixedPlane3::classify_aabb` per plane, and its conservative
  `Intersecting`.
- Sphere classification, exact in the rounded centre distance, by a wide-integer comparison.
- Plane extraction from a clip matrix under Bunny's `[0, 1]` depth range, and its
  3/2 raw unit bound against the exact clip-space inequalities.

Out of scope: an exact (separating-axis) box or sphere test, frustum against frustum,
normalised planes, and any renderer policy. The chapter lists them under
[What is deliberately not here](README.md#what-is-deliberately-not-here).

## Requirements

The table is the human-readable view. The fenced TOML block after it is the contract graph
consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| FRUSTUM-REQ-001 | A frustum is six planes in the order left, right, bottom, top, near, far, each with its front facing into the frustum; the inside is the closed set in front of or on all six; the planes are held as given, and an empty or unbounded set of six planes is a valid value. | `README.md#the-frustum` |
| FRUSTUM-REQ-002 | A point, box or sphere is `Outside` when some plane has it `Behind`, even if another plane overflows; otherwise `None` (or `Overflow`) when a plane overflows; `Inside` when every plane has it `Front` or `Coplanar`; `Intersecting` otherwise. A shape touching a face from inside is `Inside`; touching it from outside is `Outside`. | `README.md#classification` |
| FRUSTUM-REQ-003 | Box classification uses `classify_aabb` per plane, bit for bit; `Outside` and `Inside` are exact in the rounded distances; `Intersecting` is conservative and includes boxes that meet no point of the frustum; inverted and zero-extent boxes follow `classify_aabb`. | `README.md#boxes` |
| FRUSTUM-REQ-004 | Sphere classification per plane is the exact comparison of `D^2 2^64` with `R^2 \|N\|^2` in raw units, behind when `D < 0` and front when `D >= 0` once the clearance reaches the radius; a zero radius gives the point rule; a negative radius is `NegativeRadius`; the test is conservative as for boxes. | `README.md#spheres` |
| FRUSTUM-REQ-005 | `from_clip_matrix` returns the planes `r3 + r0`, `r3 - r0`, `r3 + r1`, `r3 - r1`, `r2`, `r3 - r2`, each row `(a, b, c, d)` as normal `(a, b, c)` and offset `-d`, with no rounding and no normalisation, and `None` exactly when a sum or `-d` overflows. | `README.md#extraction-from-a-clip-matrix` |
| FRUSTUM-REQ-006 | Each extracted plane's computed signed distance is within 3/2 raw units of the exact clip-space inequality under the stored matrix entries; a point clearer than that of every face is classified as the exact inequalities say; points built inside or outside the view volume of the repository's projection and look-at builders by a stated margin classify inside or outside. | `README.md#what-extraction-guarantees` |
| FRUSTUM-REQ-007 | No renderer policy: no depth-range or reversed-depth switch, no cull-mode enumeration, no name of a graphics API, no assumption beyond the coordinate law. | `README.md#what-is-deliberately-not-here` |

```toml
[[requirement]]
id = "FRUSTUM-REQ-001"
summary = "Six planes in the order left, right, bottom, top, near, far, front facing in; the closed inside; held as given, empty or unbounded sets valid."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-002"
summary = "Outside when some plane has the shape Behind, even over overflow; then None; Inside when all Front or Coplanar; Intersecting otherwise."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-003"
summary = "Boxes use classify_aabb per plane; Outside and Inside exact in the rounded distances; Intersecting conservative; inverted and zero-extent boxes as classify_aabb."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-004"
summary = "Spheres compare D^2 2^64 with R^2 |N|^2 exactly; touching takes the side of the other points; zero radius is the point rule; negative radius refused; conservative."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-005"
summary = "from_clip_matrix gives r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2 exactly, unnormalised, None exactly on overflow."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-006"
summary = "Extracted plane distances are within 3/2 raw of the exact clip-space inequalities; seeded cameras classify points known inside and outside by a stated margin."
status = "active"

[[requirement]]
id = "FRUSTUM-REQ-007"
summary = "No renderer policy: no depth or cull-mode switches, no graphics API names, nothing beyond the coordinate law."
status = "active"
```

## Oracles

Constructed cases are derived on paper in the test file from two frusta: the box
`-1 <= x <= 1`, `-1 <= y <= 1`, `-3 <= z <= -1` with unit normals, and the view-space
perspective frustum of `tan = 1`, aspect 1, near 1 and far 3, whose planes the chapter
derives from the coordinate law's matrix.

The sweeps write their own exact references and share no helper with the kernel.
`model_distance` rounds each product in `i128` with `div_euclid`, `rem_euclid` and a
comparison with `2^31`, and refuses each partial sum outside `i64`; `model_box_span` applies
it at the eight corners. `model_sphere_span` compares `D^2 2^64` with `R^2 |N|^2` with a
schoolbook multiplication over 32-bit limbs; the kernel splits `u128` values into two 64-bit
halves. `model_combine` writes the combination rule out again. `exact_clip` forms the exact
clip coordinates `M (p, 1)` of the stored entries in `i128`, from which the six exact
inequalities `w + x`, `w - x`, `w + y`, `w - y`, `z` and `w - z` are taken.

## Fixtures

None. Seeds, case counts and the pinned tallies:

| Case | Seed | Cases | Pinned tally |
| --- | --- | --- | --- |
| FRUSTUM-TP-011 | `0x146F_0001_9E37_79B9` | 20 000 boxes | 942 inside, 8 033 intersecting, 6 975 outside, 4 050 overflow |
| FRUSTUM-TP-012 | `0x146F_0002_7F4A_7C15` | 20 000 spheres, and 20 000 with negative radii | 920 inside, 5 508 intersecting, 10 677 outside, 2 895 overflow |
| FRUSTUM-TP-013 | `0x146F_0003_D1B5_4A32` | 10 000 matrices, 4 points each where the frustum exists | 3 236 refused, 26 491 points decided, 25 within the bound of a face |
| FRUSTUM-TP-014 | `0x146F_0004_94D0_49BB` | 1 000 cameras, 8 inside and 6 outside points each | 1 000 cameras, none skipped, 14 000 points |

The generator is a 64-bit xorshift. Each of TP-011 and TP-012 draws one of four modes per
case: six random planes with coordinates up to 1000 units, six on a half-unit grid with
raw-unit jitter, six at the ends of the range (overflow), or the box or pyramid frustum with
each offset moved by up to one raw unit, with shapes on a quarter-unit grid around it. Every
failure message names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| FRUSTUM-TP-001 | Hand-derived | FRUSTUM-REQ-001 | `new` and `planes` keep the order and the fields; an empty six-plane set has every point and box outside; six always-front planes have everything inside. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_001_planes_are_held_as_given_in_order` |
| FRUSTUM-TP-002 | Boundary | FRUSTUM-REQ-002, FRUSTUM-REQ-003 | On each of the six faces of the box frustum: a box touching from outside is `Outside`, one raw unit through it `Intersecting`, one raw unit clear `Outside`; touching from inside `Inside`, one raw past `Intersecting`, one raw short `Inside`; the point on the face is inside and one raw outside is not. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_002_boxes_inside_outside_intersecting_and_on_each_face` |
| FRUSTUM-TP-003 | Degenerate | FRUSTUM-REQ-003 | A flat box in a face is `Inside` and one raw unit outside it `Outside`; each inverted box equals its upright box; each zero-extent box equals `contains_point`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_003_flat_inverted_and_zero_extent_boxes` |
| FRUSTUM-TP-004 | Conservative | FRUSTUM-REQ-003 | The box `x` in `[-5, -7/2]`, `y` in `[-1/2, 1/2]`, `z` in `[-4, -2]` against the pyramid meets no frustum point (proof in the test) and is reported `Intersecting`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_004_the_six_plane_box_test_is_conservative` |
| FRUSTUM-TP-005 | Boundary | FRUSTUM-REQ-002, FRUSTUM-REQ-004 | On each face of the box frustum, a sphere of radius 1/2 touching from outside is `Outside`, one raw unit nearer `Intersecting`, one raw farther `Outside`; touching from inside `Inside`, one raw past `Intersecting`, one raw short `Inside`; radius 1 at the centre touches all six faces and is `Inside`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_005_spheres_inside_outside_intersecting_and_on_each_face` |
| FRUSTUM-TP-006 | Boundary | FRUSTUM-REQ-004 | Against `x - z >= 0` (and the same plane times 3), the radius `floor(sqrt 2 * 2^32)` raw stays in front of or behind the plane and one raw unit more straddles; zero radius equals `contains_point`, the far-face tie included; negative radii are refused. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_006_sphere_radius_rules_and_non_unit_normals` |
| FRUSTUM-TP-007 | Conservative | FRUSTUM-REQ-004 | The sphere at `(-4, 0, -4)` with radius 5/4 is `sqrt 2` from the nearest frustum point and is reported `Intersecting`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_007_the_six_plane_sphere_test_is_conservative` |
| FRUSTUM-TP-008 | Overflow | FRUSTUM-REQ-002, FRUSTUM-REQ-004 | With one plane overflowing, a point, box or sphere behind another plane is still outside, and otherwise the result is `None` or `Overflow`; the widest sphere comparisons do not overflow. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_008_a_certain_outside_wins_over_overflow` |
| FRUSTUM-TP-009 | Pinned golden | FRUSTUM-REQ-005 | Hand-derived planes of `try_perspective(1, 1, 1, 3)`, `try_orthographic(2, 1, 1, 5)`, the identity and the perspective times `try_look_at` from `(0, 0, 5)`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_009_extracted_planes_are_exact_row_sums` |
| FRUSTUM-TP-010 | Boundary | FRUSTUM-REQ-005, FRUSTUM-REQ-006 | Extracted side faces are exact one raw unit either side; the near face keeps both neighbours by the ties-to-even rounding of `3/2` raw; one raw unit beyond the far face rounds to `On` and is inside, two are outside; each row sum and `-d` one raw unit past `i64` is `None`. | `crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_010_extracted_faces_one_raw_unit_either_side_and_overflow` |
| FRUSTUM-TP-011 | Seeded oracle | FRUSTUM-REQ-002, FRUSTUM-REQ-003 | 20 000 seeded boxes: the result equals the rule applied to the six `classify_aabb` results and to the independent eight-corner model, bit for bit; every `Outside` has a plane with all eight corners `Behind` or `On` by `classify`; every `Inside` has all eight corners inside; no other result has such a plane. | `crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_011_seeded_boxes_follow_the_rule_and_outside_is_certain` |
| FRUSTUM-TP-012 | Seeded oracle | FRUSTUM-REQ-002, FRUSTUM-REQ-004 | 20 000 seeded spheres, radii from zero to `i64::MAX` raw: the result equals the 32-bit-limb model; zero radii equal `contains_point`; negative radii are refused. | `crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_012_seeded_spheres_match_the_wide_integer_model` |
| FRUSTUM-TP-013 | Seeded oracle | FRUSTUM-REQ-005, FRUSTUM-REQ-006 | 10 000 seeded matrices, the range ends included: the planes equal the `i128` row sums, `None` included; for in-range points every computed distance is within 3/2 raw of the exact clip inequality, and points clear of every face by more are classified as the exact inequalities say. | `crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_013_seeded_extraction_is_exact_and_within_three_halves_raw` |
| FRUSTUM-TP-014 | Seeded property | FRUSTUM-REQ-006 | 1 000 seeded cameras, `try_perspective` or `try_orthographic` times `try_look_at`: view-space points inside by 1/64 of each face's extent, taken to world space through the look-at rows, are inside; points beyond one face by 1/64 of its extent are outside. | `crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_014_seeded_cameras_classify_points_known_inside_and_outside` |
| FRUSTUM-TP-015 | Mutation | FRUSTUM-REQ-002, FRUSTUM-REQ-003, FRUSTUM-REQ-004, FRUSTUM-REQ-005 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |
| FRUSTUM-TP-016 | Audit | FRUSTUM-REQ-007 | The public surface of `frustum.rs` names no graphics API, depth or cull mode, and the chapter lists what is left out. | [What is deliberately not here](README.md#what-is-deliberately-not-here) |

```toml
[[case]]
id = "FRUSTUM-TP-001"
requirements = ["FRUSTUM-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_001_planes_are_held_as_given_in_order"
oracle = "Hand-derived order, an empty and an unbounded six-plane set."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-002"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_002_boxes_inside_outside_intersecting_and_on_each_face"
oracle = "Hand-derived boxes on and one raw unit either side of each face."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-003"
requirements = ["FRUSTUM-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_003_flat_inverted_and_zero_extent_boxes"
oracle = "Flat boxes in a face, inverted boxes against their upright boxes, zero-extent boxes against the point rule."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-004"
requirements = ["FRUSTUM-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_004_the_six_plane_box_test_is_conservative"
oracle = "A hand-derived box outside near an edge, reported Intersecting."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-005"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_005_spheres_inside_outside_intersecting_and_on_each_face"
oracle = "Hand-derived spheres touching and one raw unit either side of each face."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-006"
requirements = ["FRUSTUM-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_006_sphere_radius_rules_and_non_unit_normals"
oracle = "The integer square root of 2^65 raw as the threshold radius; zero and negative radii."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-007"
requirements = ["FRUSTUM-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_007_the_six_plane_sphere_test_is_conservative"
oracle = "A hand-derived sphere outside near an edge, reported Intersecting."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-008"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_008_a_certain_outside_wins_over_overflow"
oracle = "A hand-built overflowing plane beside a culling plane; the widest sphere comparisons."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-009"
requirements = ["FRUSTUM-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_009_extracted_planes_are_exact_row_sums"
oracle = "Hand-derived planes of power-of-two projections, the identity and an axis-aligned look-at."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-010"
requirements = ["FRUSTUM-REQ-005", "FRUSTUM-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_tests.rs::frustum_tp_010_extracted_faces_one_raw_unit_either_side_and_overflow"
oracle = "Hand-derived ties at the near and far faces; overflow one raw unit past each threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-011"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_011_seeded_boxes_follow_the_rule_and_outside_is_certain"
oracle = "The rule over six classify_aabb results and over an independent i128 eight-corner model; corner certainty by classify."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-012"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_012_seeded_spheres_match_the_wide_integer_model"
oracle = "Schoolbook 32-bit-limb comparison over an independent i128 distance model."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-013"
requirements = ["FRUSTUM-REQ-005", "FRUSTUM-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_013_seeded_extraction_is_exact_and_within_three_halves_raw"
oracle = "i128 row sums and exact i128 clip coordinates of the stored matrix entries."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-014"
requirements = ["FRUSTUM-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/frustum_sweep_tests.rs::frustum_tp_014_seeded_cameras_classify_points_known_inside_and_outside"
oracle = "View-space points inside or outside by construction with a 1/64 margin, through the repository's builders."
tier = "fast"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-015"
requirements = ["FRUSTUM-REQ-002", "FRUSTUM-REQ-003", "FRUSTUM-REQ-004", "FRUSTUM-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/frustum/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"

[[case]]
id = "FRUSTUM-TP-016"
requirements = ["FRUSTUM-REQ-007"]
evidence = "manual-audit"
artifact = "docs/topics/frustum/README.md#what-is-deliberately-not-here"
oracle = "Review of the public surface and the chapter's list of what is left out."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel, `frustum_tests` and `frustum_sweep_tests` run with
`cargo test --locked -q -p bunny-geom --no-fail-fast`, and the kernel restored.

| Mutation | Fails |
| --- | --- |
| Overflow checked before a `Behind` plane (`None` beats `Outside`) | FRUSTUM-TP-008, FRUSTUM-TP-011, FRUSTUM-TP-012 |
| `Coplanar` counted as straddled | FRUSTUM-TP-002, -003, -006, -010, -011, -012 |
| A point `On` a face counted as behind | FRUSTUM-TP-002, -003, -006, -010, -011, -012 |
| A sphere touching a plane from behind counted as straddling | FRUSTUM-TP-005, -012 |
| A sphere touching a plane from inside counted as straddling | FRUSTUM-TP-005, -006, -012 |
| The sphere comparison assumes a unit normal (`\|N\|^2 = 2^64`) | FRUSTUM-TP-001, -006, -008, -012 |
| The 256-bit product drops the carry of its low word | FRUSTUM-TP-012 |
| A negative radius taken as its magnitude | FRUSTUM-TP-006, -012 |
| Near plane `r3 + r2` (the `[-1, 1]` depth range) | FRUSTUM-TP-009, -010, -013, -014 |
| Far plane `r3 + r2` | FRUSTUM-TP-009, -010, -013, -014 |
| Left and right planes swapped | FRUSTUM-TP-009, -013 |
| Offset `d` instead of `-d` | FRUSTUM-TP-009, -010, -013, -014 |

One mutation survives and is equivalent: dropping the carry of the 256-bit product's middle
sum. With `R^2 < 2^126` and `|N|^2 <= 3 * 2^126`, the middle sum is below `2^128`, so the
carry is never set for any input; `sphere.rs` states the bound. The helper keeps the carry so
that it is correct for any `u128` operands.

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic; the tests use fixed seeds and no clock. Every test
runs natively and under WebAssembly through `wasm_bindgen_test`.
