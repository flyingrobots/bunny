# Exact Geometric Predicates Test Plan

This document defines how Bunny proves [`bunny-predicates/1`](README.md): `orientation2`,
`doubled_signed_area2`, `signed_area2_rounded`, `orientation3`, `sextupled_signed_volume3`,
`signed_volume3_rounded`, `barycentric2`, `barycentric3_coplanar` and the `ExactBarycentric`
classification and rounded weights, all in `bunny-geom`. It answers flyingrobots/bunny#145.

## Scope

- the 2D orientation and doubled signed area of three `FixedVec2` points, exact, and the
  signed area rounded once to Q32.32;
- the 3D orientation and sextupled signed volume of four `FixedVec3` points, exact, and the
  signed volume rounded once to Q32.32;
- exact barycentric numerators and denominator of a point against a 2D triangle, and of a
  point exactly in the plane of a 3D triangle, the exact location classification
  (`TriangleLocation`) with its precedence, the closed and open containment forms, and the
  weights rounded once each to Q32.32.

Out of scope: barycentric coordinates of a point that is not exactly in the plane of a 3D
triangle (refused), predicates over more than four points (in-circle, in-sphere), and any
change to `FixedPlane3::classify`, `BarycentricNumerators` or the trace kernels.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| PRED-REQ-001 | `orientation2(a, b, c)` is the sign of the exact `D2 = (b - a) x (c - a)` on raw values for every representable input, `CounterClockwise` when positive (canonical `+Y` up), `Clockwise` when negative and `Collinear` when exactly zero. | `README.md#2d-orientation` |
| PRED-REQ-002 | `doubled_signed_area2` returns `D2` exactly (`\|D2\| < 2^129`, raw squared units); `signed_area2_rounded` is `round_half_even(D2 / 2^33)` as Q32.32 raw, `None` outside `i64`; it is zero exactly when `\|D2\| <= 2^32` and otherwise has the sign of `orientation2`. | `README.md#signed-area` |
| PRED-REQ-003 | `orientation3(a, b, c, d)` is the sign of the exact `D3 = (d - a) . ((b - a) x (c - a))` for every representable input, `Front` when positive (on the side the right-hand normal of `(a, b, c)` points to), `Behind` when negative and `Coplanar` when exactly zero. | `README.md#3d-orientation` |
| PRED-REQ-004 | `sextupled_signed_volume3` returns `D3` exactly (`\|D3\| < 6 2^192`, raw cubed units); `signed_volume3_rounded` is `round_half_even(D3 / (6 2^64))` as Q32.32 raw, `None` outside `i64`; it is zero exactly when `\|D3\| <= 3 2^64` and otherwise has the sign of `orientation3`. | `README.md#signed-volume` |
| PRED-REQ-005 | `barycentric2(p, a, b, c)` is `None` exactly when `D2(a, b, c) = 0`; otherwise its numerators are `D2(p, b, c)`, `D2(a, p, c)`, `D2(a, b, p)` and its denominator `D2(a, b, c)`, all negated together when the denominator is negative, so the denominator is positive and the numerators sum to it exactly. | `README.md#barycentric-numerators` |
| PRED-REQ-006 | `location()` is exact: `Outside` when any numerator is negative, otherwise `OnVertex` when two are zero, `OnEdge` when one is zero and `Inside` when none is, naming the vertex or edge; `contains_closed` is "not `Outside`" and `contains_open` is "`Inside`". | `README.md#location` |
| PRED-REQ-007 | `barycentric3_coplanar(p, a, b, c)` is `None` when the triangle has zero area or `orientation3(a, b, c, p)` is not `Coplanar`; otherwise it equals `barycentric2` of the projection that drops the first axis (`x`, `y`, `z`) in which the normal is nonzero. | `README.md#coplanar-3d-points` |
| PRED-REQ-008 | `weights_rounded()` rounds each numerator times `2^32` over the denominator once, to nearest with ties to even, `None` when any is outside `i64`; when defined, the three raw weights sum to `2^32` within one raw unit, and no weight is adjusted. | `README.md#rounded-weights` |
| PRED-REQ-009 | The orientations are antisymmetric under a swap of two points and invariant under a cyclic rotation (2D) or an even permutation (3D), and the barycentric numerators have the signs of the orientations of the sub-triangles. | `README.md#consistency` |

```toml
[[requirement]]
id = "PRED-REQ-001"
summary = "orientation2 is the exact sign of (b - a) x (c - a) for every representable input; CounterClockwise is positive in the canonical +Y-up plane."
status = "active"

[[requirement]]
id = "PRED-REQ-002"
summary = "doubled_signed_area2 is exact; signed_area2_rounded is round_half_even(D2 / 2^33), None outside i64, zero exactly when |D2| <= 2^32."
status = "active"

[[requirement]]
id = "PRED-REQ-003"
summary = "orientation3 is the exact sign of (d - a) . ((b - a) x (c - a)) for every representable input; Front is the right-hand normal side."
status = "active"

[[requirement]]
id = "PRED-REQ-004"
summary = "sextupled_signed_volume3 is exact; signed_volume3_rounded is round_half_even(D3 / (6 2^64)), None outside i64, zero exactly when |D3| <= 3 2^64."
status = "active"

[[requirement]]
id = "PRED-REQ-005"
summary = "barycentric2 is None exactly for a zero-area triangle; otherwise exact sub-triangle numerators over a positive denominator that they sum to."
status = "active"

[[requirement]]
id = "PRED-REQ-006"
summary = "location is exact with the precedence Outside, OnVertex, OnEdge, Inside; contains_closed includes the boundary and contains_open excludes it."
status = "active"

[[requirement]]
id = "PRED-REQ-007"
summary = "barycentric3_coplanar refuses a zero-area triangle or a point not exactly coplanar, and otherwise equals barycentric2 of the stated projection."
status = "active"

[[requirement]]
id = "PRED-REQ-008"
summary = "weights_rounded rounds each weight once, ties to even, None outside i64; the defined raw weights sum to 2^32 within one raw unit."
status = "active"

[[requirement]]
id = "PRED-REQ-009"
summary = "Orientations are antisymmetric and rotation invariant, and barycentric numerator signs match the sub-triangle orientations."
status = "active"
```

## Oracles

`crates/bunny-geom/tests/predicates_oracle/mod.rs` is an arbitrary-precision signed integer
written for the tests: a sign and a growable magnitude in 32-bit digits (the kernel uses a
fixed sign and four 64-bit limbs), with schoolbook addition, subtraction, multiplication and
comparison and no division. It computes `D2` and `D3` from the raw `i64` coordinates by the
textbook formulas and shares no helper with the kernel. A rounded result `r` is checked
without dividing: with the denominator `d > 0`, `r` is the round-half-even quotient of `n`
exactly when `2 |n - r d| < d`, or `2 |n - r d| = d` and `r` is even; a refusal is checked
as `2 n >= (2^64 - 1) d` or `2 n < (-2^64 - 1) d`.

The golden vectors are raw values inline in the tests, each with its derivation, and each is
also checked against the oracle. A separate Python model of the same goldens (Python's
unbounded integers) is posted on the pull request, not kept in the tree.

Every test is a `wasm_bindgen_test`, so the WebAssembly job (`wasm-pack test --node
crates/bunny-geom`) runs the goldens and sweeps as well as the native run.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| PRED-TP-001 | PRED-REQ-001 | Constructed goldens: three collinear points give `Collinear`, and one raw unit on one coordinate gives each definite sign, at small magnitudes (`D2 = +-1` raw squared) and at the ends of the raw range (`i64::MIN`, `0`, `i64::MAX` on the diagonal), repeated points, and the identity string. | `crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_001_collinear_and_one_raw_unit_off_in_2d` |
| PRED-TP-002 | PRED-REQ-002 | Hand-derived exact `D2` values and rounded areas: `D2 = 1` (positive orientation, rounded area zero), `2^32` (a tie to 0), `2^32 + 1`, `3 2^32` (a tie to 2), their negatives, the largest `D2 = 2 (2^64 - 1)^2` magnitudes, and the `i64` refusal on and one raw unit either side of both ends. | `crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_002_doubled_and_rounded_signed_area` |
| PRED-TP-003 | PRED-REQ-003 | Constructed goldens: four coplanar points give `Coplanar`, and one raw unit off the plane gives `Front` and `Behind`, at small magnitudes and at the ends of the raw range, on an axis plane and on the tilted plane `x = y`; the largest `D3 = (2^64 - 1)^3`. | `crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_003_coplanar_and_one_raw_unit_off_in_3d` |
| PRED-TP-004 | PRED-REQ-004 | Hand-derived rounded volumes: `D3 = 1`, `3 2^64` (a tie to 0), `3 2^64 + 1`, `9 2^64` (a tie to 2), their negatives, and the `i64` refusal at the largest volumes. | `crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_004_sextupled_and_rounded_signed_volume` |
| PRED-TP-005 | PRED-REQ-005, PRED-REQ-006 | Constructed goldens: a point on each vertex and on each edge, one raw unit inside and outside each edge, a point on an edge's extension beyond a vertex (`Outside`), a clockwise triangle normalized to a positive denominator, a zero-area triangle refused, and the same at the ends of the raw range; exact numerators pinned. | `crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_005_barycentric_locations_on_and_off_the_boundary` |
| PRED-TP-006 | PRED-REQ-007 | A tilted triangle with a coplanar point equal to `barycentric2` of its projection, a point one raw unit off the plane refused, triangles in each axis plane (each projection used), and a zero-area triangle refused. | `crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_006_coplanar_3d_barycentrics` |
| PRED-TP-007 | PRED-REQ-008 | Hand-derived rounded weights: thirds summing to `2^32 - 1`, `(2/3, 1/6, 1/6)` summing to `2^32 + 1`, ties of half a raw unit going to even, a vertex exactly `(1.0, 0, 0)`, and a weight on and past `2^63` refused. | `crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_007_rounded_weights_and_their_sum` |
| PRED-TP-008 | PRED-REQ-001, PRED-REQ-002 | 60 000 seeded triples (seed printed) built deliberately: exactly collinear, one raw unit off collinear, small, and of every magnitude up to the ends of the raw range; the exact `D2` and the rounded area equal the oracle, and the outcome counts are pinned. | `crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_008_seeded_2d_orientations_match_the_oracle` |
| PRED-TP-009 | PRED-REQ-003, PRED-REQ-004 | 40 000 seeded quadruples (seed printed) built the same way (exactly coplanar, one raw unit off, every magnitude); the exact `D3` and the rounded volume equal the oracle, and the outcome counts are pinned. | `crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_009_seeded_3d_orientations_match_the_oracle` |
| PRED-TP-010 | PRED-REQ-005, PRED-REQ-006, PRED-REQ-008 | 40 000 seeded point and triangle pairs (seed printed; points on vertices, on edges, one raw unit off an edge, degenerate triangles and every magnitude): numerators, denominator and rounded weights equal the oracle, the location equals the oracle's sign rule, the sum bound holds; outcome counts pinned. | `crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_010_seeded_barycentrics_match_the_oracle` |
| PRED-TP-011 | PRED-REQ-009 | Over the seeded inputs of PRED-TP-008 to -010: every swap negates, every cyclic rotation (2D) and even permutation (3D) preserves the exact value, and each normalized numerator's sign equals the orientation of its sub-triangle times the triangle's orientation. | `crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_011_orientations_are_consistent_under_permutation` |
| PRED-TP-012 | PRED-REQ-001, PRED-REQ-002, PRED-REQ-003, PRED-REQ-004, PRED-REQ-005, PRED-REQ-006, PRED-REQ-007, PRED-REQ-008 | Each rule broken on purpose, including the exact sign replaced by a rounded computation, fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "PRED-TP-001"
requirements = ["PRED-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_001_collinear_and_one_raw_unit_off_in_2d"
oracle = "Constructed raw goldens, each also checked against the independent big-integer oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-002"
requirements = ["PRED-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_002_doubled_and_rounded_signed_area"
oracle = "Hand-derived raw values at each rounding and range threshold, each also checked against the oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-003"
requirements = ["PRED-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_003_coplanar_and_one_raw_unit_off_in_3d"
oracle = "Constructed raw goldens, each also checked against the independent big-integer oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-004"
requirements = ["PRED-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_orientation_tests.rs::pred_tp_004_sextupled_and_rounded_signed_volume"
oracle = "Hand-derived raw values at each rounding and range threshold, each also checked against the oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-005"
requirements = ["PRED-REQ-005", "PRED-REQ-006"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_005_barycentric_locations_on_and_off_the_boundary"
oracle = "Constructed raw goldens on and one raw unit either side of each edge and vertex."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-006"
requirements = ["PRED-REQ-007"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_006_coplanar_3d_barycentrics"
oracle = "barycentric2 of the stated projection, and constructed refusals."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-007"
requirements = ["PRED-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_barycentric_tests.rs::pred_tp_007_rounded_weights_and_their_sum"
oracle = "Hand-derived raw weights, each also checked against the oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-008"
requirements = ["PRED-REQ-001", "PRED-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_008_seeded_2d_orientations_match_the_oracle"
oracle = "Independent big-integer oracle over 60 000 seeded, deliberately near-degenerate triples."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-009"
requirements = ["PRED-REQ-003", "PRED-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_009_seeded_3d_orientations_match_the_oracle"
oracle = "Independent big-integer oracle over 40 000 seeded, deliberately near-coplanar quadruples."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-010"
requirements = ["PRED-REQ-005", "PRED-REQ-006", "PRED-REQ-008"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_010_seeded_barycentrics_match_the_oracle"
oracle = "Independent big-integer oracle over 40 000 seeded point and triangle pairs."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-011"
requirements = ["PRED-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/predicate_sweep_tests.rs::pred_tp_011_orientations_are_consistent_under_permutation"
oracle = "Permutation identities of the determinant over seeded near-degenerate inputs."
tier = "fast"
status = "planned"

[[case]]
id = "PRED-TP-012"
requirements = ["PRED-REQ-001", "PRED-REQ-002", "PRED-REQ-003", "PRED-REQ-004", "PRED-REQ-005", "PRED-REQ-006", "PRED-REQ-007", "PRED-REQ-008"]
evidence = "manual-audit"
artifact = "docs/topics/predicates/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
```

## Mutation Checks

Planned: each rule of `crates/bunny-geom/src/predicates.rs`,
`crates/bunny-geom/src/predicates/barycentric.rs` and
`crates/bunny-geom/src/predicates/wide.rs` broken on purpose in a local copy, the
predicate tests run, and the source restored. The first mutation replaces the exact sign
with one computed from Q32.32-rounded products.
