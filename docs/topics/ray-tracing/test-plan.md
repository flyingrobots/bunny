# Ray Tracing Kernels Test Plan

This document defines how Bunny verifies the ray-tracing kernel contract.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design. It records active requirements, executable
evidence, and open gaps for the traversal and benchmarks that will build on
these kernels.

## Scope

This plan covers `TraceRay`, `TraceTriangle` and `TriangleHit` in
`bunny-query`:

- the trace domain and its refusals;
- the conservative `box_entry` test, its never-reject guarantee and its bound;
- the exact `hit_triangle` test, its edge rule and its facing;
- rounding of distances and barycentric weights;
- overflow freedom at the edges of the domain.

The existing solvers in `crates/bunny-query/src/ray.rs` keep their own
evidence and are out of scope. BVH closest-hit traversal and timing are out of
scope until those APIs exist.

## Test Goals

- Compare every result with an oracle that does not share the kernel's
  rounding: constructed exact cases, an exact rational slab test, and
  hand-computed golden vectors.
- Assert exact raw Q32.32 outputs, never approximate ones.
- Show that each rounding direction and each inclusive comparison is load
  bearing by breaking it on purpose.
- Keep the evidence native and WASM-compatible: every case is a
  `wasm_bindgen_test` that also runs as a native test.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| TRACE-REQ-001 | TraceRay and TraceTriangle refuse coordinates outside the trace domain, direction components above 1.0 and a zero direction. | `README.md#trace-domain` |
| TRACE-REQ-002 | box_entry never returns None for a box the ray reaches within 0..=t_max, including face, edge and corner contact. | `README.md#conservative-box-test` |
| TRACE-REQ-003 | box_entry returns a lower bound on the exact entry distance, short by at most 2^11 + 2 raw units plus entry / 2^30. | `README.md#conservative-box-test` |
| TRACE-REQ-004 | hit_triangle reports a hit exactly when the ray crosses the triangle at a non-negative distance within its limits, and reports the floor of the true distance. | `README.md#exact-triangle-test` |
| TRACE-REQ-005 | Triangle edges and corners are inclusive, so a ray through an edge shared by two triangles hits both. | `README.md#exact-triangle-test` |
| TRACE-REQ-006 | Distance limits of box_entry and hit_triangle are inclusive. | `README.md#exact-triangle-test` |
| TRACE-REQ-007 | Barycentric weights are the floors of the exact weights of v1 and v2. | `README.md#exact-triangle-test` |
| TRACE-REQ-008 | front_face is true when the ray arrives on the side from which the vertices appear counter-clockwise. | `README.md#facing` |
| TRACE-REQ-009 | No intermediate overflows for any input inside the trace domain. | `README.md#trace-domain` |
| TRACE-REQ-010 | Parallel rays (including rays in the triangle's plane), zero-area triangles and crossings whose distance does not fit i64 are misses. | `README.md#exact-triangle-test` |

```toml
[[requirement]]
id = "TRACE-REQ-001"
summary = "TraceRay and TraceTriangle refuse coordinates outside the trace domain, direction components above 1.0 and a zero direction."
status = "active"

[[requirement]]
id = "TRACE-REQ-002"
summary = "box_entry never returns None for a box the ray reaches within 0..=t_max, including face, edge and corner contact."
status = "active"

[[requirement]]
id = "TRACE-REQ-003"
summary = "box_entry returns a lower bound on the exact entry distance, short by at most 2^11 + 2 raw units plus entry / 2^30."
status = "active"

[[requirement]]
id = "TRACE-REQ-004"
summary = "hit_triangle reports a hit exactly when the ray crosses the triangle at a non-negative distance within its limits, and reports the floor of the true distance."
status = "active"

[[requirement]]
id = "TRACE-REQ-005"
summary = "Triangle edges and corners are inclusive, so a ray through an edge shared by two triangles hits both."
status = "active"

[[requirement]]
id = "TRACE-REQ-006"
summary = "Distance limits of box_entry and hit_triangle are inclusive."
status = "active"

[[requirement]]
id = "TRACE-REQ-007"
summary = "Barycentric weights are the floors of the exact weights of v1 and v2."
status = "active"

[[requirement]]
id = "TRACE-REQ-008"
summary = "front_face is true when the ray arrives on the side from which the vertices appear counter-clockwise."
status = "active"

[[requirement]]
id = "TRACE-REQ-009"
summary = "No intermediate overflows for any input inside the trace domain."
status = "active"

[[requirement]]
id = "TRACE-REQ-010"
summary = "Parallel rays (including rays in the triangle's plane), zero-area triangles and crossings whose distance does not fit i64 are misses."
status = "active"
```

## Fixtures

No external fixtures are required. Every case builds rays, triangles and boxes
from raw Q32.32 values in the test file.

Seeded cases use a fixed 64-bit linear congruential generator with the seed
`TRACE_SEED` (`0xB002_7ACE_0000_0001`), varied per case by an exclusive-or with
the case number. Failure messages name the seed and the case index.

Constructed triangles have edges that are multiples of 64 raw-unit steps, so a
point with barycentric weights `(b, c) / 64` is a lattice point, and a ray is
started a whole number of direction lengths before it. The exact distance and
weights are therefore known without computing them.

The golden vectors in `TRACE-TP-009` and `TRACE-TP-010` are hand computed; the
comments beside each case give the exact value the floor is taken of.
Regenerating them is a contract change.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| TRACE-TP-001 | Constructed hits | TRACE-REQ-004, TRACE-REQ-007, TRACE-REQ-008 | Rays built through a known lattice point of a triangle at a whole-unit distance must return that exact distance, exact weights and the facing from the triple product. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_001_constructed_hits_report_exact_distance_and_weights` |
| TRACE-TP-002 | Constructed misses | TRACE-REQ-004 | Rays through points just outside each edge, and rays starting past the triangle, must miss. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_002_points_outside_the_triangle_or_behind_the_ray_miss` |
| TRACE-TP-003 | Inclusive distance limits | TRACE-REQ-006, TRACE-REQ-008 | A hit at exactly 5.0 is reported for limits 5.0..=5.0 and refused for limits one raw unit away. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_003_distance_limits_are_inclusive` |
| TRACE-TP-004 | Shared-edge watertightness | TRACE-REQ-005 | A ray through the diagonal of a split quad hits both halves; a ray elsewhere hits exactly one. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_004_a_shared_edge_never_leaks` |
| TRACE-TP-005 | Extreme domain values | TRACE-REQ-009 | Triangles and rays at the corners of the domain run under overflow-checked test arithmetic without panicking, and a triangle hit implies its bounds are accepted. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_005_extreme_domain_values_do_not_overflow` |
| TRACE-TP-006 | Domain refusal | TRACE-REQ-001 | Values one raw unit past each bound return None; values at each bound are accepted; vertices round-trip. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_006_values_outside_the_trace_domain_are_refused` |
| TRACE-TP-007 | Box bound against an exact oracle | TRACE-REQ-002, TRACE-REQ-003 | An exact rational slab test over 40 000 seeded rays and boxes: every reached box is accepted, the bound is at or below the floored exact entry and within the stated slack, and many missed boxes are rejected. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_007_box_entry_never_rejects_a_box_the_ray_reaches` |
| TRACE-TP-008 | Box distance limit | TRACE-REQ-002, TRACE-REQ-006 | An entry at exactly 4.0 is accepted for t_max 4.0 and rejected one raw unit below; negative t_max, boxes behind the ray and an origin inside the box behave as stated. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_008_box_entry_respects_the_distance_limit` |
| TRACE-TP-009 | Golden triangle hits | TRACE-REQ-004, TRACE-REQ-005, TRACE-REQ-007, TRACE-REQ-008 | Hand-computed raw distance, weights and facing for four cases: an exact hit, a floored one-third distance, a back-face hit with floored one-third weights, and a corner hit. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_009_golden_triangle_hits_lock_raw_outputs` |
| TRACE-TP-010 | Golden box entries | TRACE-REQ-002, TRACE-REQ-003 | Hand-computed raw bounds for seven cases, including an inexact reciprocal, an edge graze, a corner graze whose exact exit equals its exact entry, a near miss and a face contact. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_010_golden_box_entries_lock_raw_outputs` |
| TRACE-TP-011 | Degenerate and far cases | TRACE-REQ-010 | Collinear and duplicate-vertex triangles, a ray in the plane, and a crossing 2^71 raw away all miss; control hits at the same places are reported. | `crates/bunny-query/tests/trace_tests.rs::trace_tp_011_degenerate_triangles_parallel_rays_and_far_hits_miss` |
| TRACE-TP-012 | Deliberate mutations | TRACE-REQ-002, TRACE-REQ-003, TRACE-REQ-005 | Each rounding or edge rule, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "TRACE-TP-001"
requirements = ["TRACE-REQ-004", "TRACE-REQ-007", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_001_constructed_hits_report_exact_distance_and_weights"
oracle = "Rays built through a known lattice point of a triangle at a whole-unit distance must return that exact distance, exact weights and the facing from the triple product."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-002"
requirements = ["TRACE-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_002_points_outside_the_triangle_or_behind_the_ray_miss"
oracle = "Rays through points just outside each edge, and rays starting past the triangle, must miss."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-003"
requirements = ["TRACE-REQ-006", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_003_distance_limits_are_inclusive"
oracle = "A hit at exactly 5.0 is reported for limits 5.0..=5.0 and refused for limits one raw unit away."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-004"
requirements = ["TRACE-REQ-005"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_004_a_shared_edge_never_leaks"
oracle = "A ray through the diagonal of a split quad hits both halves; a ray elsewhere hits exactly one."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-005"
requirements = ["TRACE-REQ-009"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_005_extreme_domain_values_do_not_overflow"
oracle = "Triangles and rays at the corners of the domain run under overflow-checked test arithmetic without panicking, and a triangle hit implies its bounds are accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-006"
requirements = ["TRACE-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_006_values_outside_the_trace_domain_are_refused"
oracle = "Values one raw unit past each bound return None; values at each bound are accepted; vertices round-trip."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-007"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003"]
evidence = "property"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_007_box_entry_never_rejects_a_box_the_ray_reaches"
oracle = "An exact rational slab test over 40 000 seeded rays and boxes: every reached box is accepted, the bound is at or below the floored exact entry and within the stated slack, and many missed boxes are rejected."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-008"
requirements = ["TRACE-REQ-002", "TRACE-REQ-006"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_008_box_entry_respects_the_distance_limit"
oracle = "An entry at exactly 4.0 is accepted for t_max 4.0 and rejected one raw unit below; negative t_max, boxes behind the ray and an origin inside the box behave as stated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-009"
requirements = ["TRACE-REQ-004", "TRACE-REQ-005", "TRACE-REQ-007", "TRACE-REQ-008"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_009_golden_triangle_hits_lock_raw_outputs"
oracle = "Hand-computed raw distance, weights and facing for four cases: an exact hit, a floored one-third distance, a back-face hit with floored one-third weights, and a corner hit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-010"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_010_golden_box_entries_lock_raw_outputs"
oracle = "Hand-computed raw bounds for seven cases, including an inexact reciprocal, an edge graze, a corner graze whose exact exit equals its exact entry, a near miss and a face contact."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-011"
requirements = ["TRACE-REQ-010"]
evidence = "test"
test = "crates/bunny-query/tests/trace_tests.rs::trace_tp_011_degenerate_triangles_parallel_rays_and_far_hits_miss"
oracle = "Collinear and duplicate-vertex triangles, a ray in the plane, and a crossing 2^71 raw away all miss; control hits at the same places are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-012"
requirements = ["TRACE-REQ-002", "TRACE-REQ-003", "TRACE-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each rounding or edge rule, broken on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a container copy of the source, and
the trace tests were run (`cargo test --locked -p bunny-query --test
trace_tests`). Every mutation made at least one case fail; the restored source
passed all of them.

| Mutation in `trace.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Slab lower bound rounded up: `scaled(near, reciprocal + 1) + 1` | The box bound is a lower bound | TRACE-TP-007, TRACE-TP-008, TRACE-TP-010 |
| Slab upper bound rounded down: `scaled(far, reciprocal)` | The box test never rejects a reached box | TRACE-TP-010 |
| `v < 0` made `v <= 0` | Edge rule: the `v0`-`v1` edge is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |
| `0..=det` made `1..=det` for `u` | Edge rule: the `v0`-`v2` edge is inclusive | TRACE-TP-001, TRACE-TP-004 |
| `0..=det` made `0..det` for `u` | Edge rule: the corner `v1` is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |
| `u + v > det` made `u + v >= det` | Edge rule: the `v1`-`v2` edge is inclusive | TRACE-TP-001, TRACE-TP-004, TRACE-TP-009 |

The upper-bound mutation is caught only by the corner-graze golden case in
`TRACE-TP-010`: seeded rays almost never meet a box with exact entry equal to
exact exit. That case was added for this reason.

## Determinism Obligations And Evidence

The kernels and their tests use only integer arithmetic: `i64`, `u64`, `i128`
and `u128` additions, multiplications, shifts and divisions with defined
results. There is no floating point, randomness from the host, time,
filesystem access, locale-sensitive formatting, parallel scheduling, map
iteration, stdout, stderr or logging in any oracle.

Test builds check integer overflow, so `TRACE-TP-005` turns any overflow in
the domain corners into a test failure.

## Known Failures

The current executable surface has no known failing cases.

## Edge Cases And Unusual Inputs

Current tests cover:

- origin and vertex coordinates one raw unit inside and outside the domain;
- direction components at 1.0, above 1.0, one raw unit, and zero;
- axis-parallel and nearly axis-parallel directions against boxes;
- an origin on a box face and inside a box;
- boxes behind the ray and a negative distance limit;
- a ray grazing a box edge and a box corner;
- hits on a triangle corner and on a shared edge;
- back-face hits;
- distances and weights that are not exact in Q32.32;
- collinear and duplicate-vertex (zero-area) triangles;
- a ray lying in the triangle's plane;
- a crossing whose distance does not fit `i64`.

Future traversal work must add cases for ties between equal-distance hits.

## Stress And Fuzz

No fuzz target exists today. `TRACE-TP-001`, `TRACE-TP-002`, `TRACE-TP-004`
and `TRACE-TP-007` are seeded property cases over 4 000 to 40 000 inputs each.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Closest-hit BVH traversal over `TraceTriangle` records and its tie-breaking. | Traversal API (flyingrobots/bunny#123). |
| Before/after timing of the kernels. | Benchmark cases (flyingrobots/bunny#210). |
| Canonical hit record, plane, segment, capsule and OBB intersections. | Remaining scope of flyingrobots/bunny#159. |
