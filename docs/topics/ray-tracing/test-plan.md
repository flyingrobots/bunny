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
- overflow freedom at the edges of the domain;
- the overflow behaviour of the general-purpose `ray_intersects_aabb` and its
  effect on `bunny_broadphase::intersect_ray` (flyingrobots/bunny#211).

The other solvers in `crates/bunny-query/src/ray.rs` keep their own evidence
and are out of scope. BVH closest-hit traversal and timing are out of scope
until those APIs exist.

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
| TRACE-REQ-011 | ray_intersects_aabb returns every hit it reported before flyingrobots/bunny#211 with the same bits, and every miss not caused by overflow stays a miss. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-012 | When a coordinate difference or slab distance overflows, ray_intersects_aabb reports a hit exactly when an exact rational slab test does. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-013 | Reported ray_intersects_aabb bounds are the exact slab bounds rounded to nearest (ties to even) and saturated to the Q32.32 range, with t_enter <= t_exit; coordinate differences are widened, not saturated. | `README.md#general-purpose-ray-aabb-solver` |
| TRACE-REQ-014 | bunny_broadphase::intersect_ray visits a leaf whose box a near-axis ray passes through. | `README.md#general-purpose-ray-aabb-solver` |

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

[[requirement]]
id = "TRACE-REQ-011"
summary = "ray_intersects_aabb returns every hit it reported before flyingrobots/bunny#211 with the same bits, and every miss not caused by overflow stays a miss."
status = "active"

[[requirement]]
id = "TRACE-REQ-012"
summary = "When a coordinate difference or slab distance overflows, ray_intersects_aabb reports a hit exactly when an exact rational slab test does."
status = "active"

[[requirement]]
id = "TRACE-REQ-013"
summary = "Reported ray_intersects_aabb bounds are the exact slab bounds rounded to nearest (ties to even) and saturated to the Q32.32 range, with t_enter <= t_exit; coordinate differences are widened, not saturated."
status = "active"

[[requirement]]
id = "TRACE-REQ-014"
summary = "bunny_broadphase::intersect_ray visits a leaf whose box a near-axis ray passes through."
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
| TRACE-TP-013 | Differential against the old function | TRACE-REQ-011, TRACE-REQ-013 | Over 200 000 seeded ordinary, axis-parallel, tiny-component, grazing and origin-inside cases, every hit of the retained pre-#211 implementation is returned with the same bits, every miss it reports without overflow stays a miss, and every bound equals the exact bound rounded and saturated. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::every_previous_hit_keeps_its_bits_on_the_ordinary_corpus` |
| TRACE-TP-014 | Exact rational oracle on the overflow region | TRACE-REQ-011, TRACE-REQ-012, TRACE-REQ-013 | Over 200 000 seeded cases with coordinates near and past 2^62, i64 extremes and components of 1 to 4 000 raw units, every case where the old function overflowed agrees in hit or miss with an i128 cross-multiplication slab test, and every bound equals the exact bound rounded and saturated. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::overflow_cases_agree_with_the_exact_rational_oracle` |
| TRACE-TP-015 | Oracle rounding pinned to checked_div | TRACE-REQ-013 | The test oracle's floor-based ties-to-even rounding equals FixedQ32_32::checked_div on 50 000 seeded representable quotients and on exact ties. | `crates/bunny-query/tests/ray_aabb_oracle_tests.rs::rounding_oracle_matches_checked_div_on_representable_quotients` |
| TRACE-TP-016 | Golden one-raw-unit component | TRACE-REQ-012, TRACE-REQ-013 | Origin (-5, 0, 0), direction (1, 2^-32, 0), box [-1, 1]^3 returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::one_raw_unit_component_hits_the_box_it_passes_through` |
| TRACE-TP-017 | Golden two-raw-unit component | TRACE-REQ-012, TRACE-REQ-013 | The same ray with a component of 2 raw units returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::two_raw_unit_component_hits_the_box_it_passes_through` |
| TRACE-TP-018 | Golden negative tiny component | TRACE-REQ-012, TRACE-REQ-013 | The same ray with a component of -1 raw unit returns exactly (4, 6). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::negative_tiny_component_hits_too` |
| TRACE-TP-019 | Sweep across the old threshold | TRACE-REQ-011, TRACE-REQ-012 | For k = 1..=64 and components d = 1..=2k+3 raw units, box [-k, k]^3 from (-100, 0, 0) returns exactly (100 - k, 100 + k) on both sides of the old d <= 2k miss boundary. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::hits_across_the_old_overflow_threshold_of_twice_the_slab_half_width` |
| TRACE-TP-020 | Golden unrepresentable exit | TRACE-REQ-013 | From x = -1.5e9 into x in [5e8, 7e8] returns (2e9, i64::MAX raw). | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::origin_far_from_box_saturates_the_unrepresentable_exit` |
| TRACE-TP-021 | Widened coordinate difference | TRACE-REQ-013 | A difference of 2^63 + 2^30 raw with direction 1 + 2^-32 returns the exactly computed raw exit 9 223 372 035 781 033 984, not the 9 223 372 034 707 292 160 that saturating the difference first gives. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::coordinate_difference_beyond_i64_is_widened_not_saturated` |
| TRACE-TP-022 | Saturated entries decided exactly | TRACE-REQ-012 | Two slabs whose entries both saturate to the maximum, and whose rounded bounds would tie, are a miss because the exact entry exceeds the exact exit. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_entries_that_tie_when_rounded_are_decided_exactly` |
| TRACE-TP-023 | Hit beyond the representable distance | TRACE-REQ-012, TRACE-REQ-013 | A box first reached more than 2^31 units from the origin is a hit with both bounds at i64::MAX raw. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::a_hit_beyond_the_representable_distance_saturates_both_bounds` |
| TRACE-TP-024 | No false hit from saturation | TRACE-REQ-012 | Near-axis rays with components of 0 to 4 raw units still miss a box beside their path. | `crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_bounds_never_create_a_false_hit` |
| TRACE-TP-025 | Face contact at the edge of the range | TRACE-REQ-012, TRACE-REQ-013 | An origin at i64::MAX raw on the box's +x face returns (i64::MIN raw, 0). | `crates/bunny-query/tests/query_tests.rs::ray_aabb_saturates_an_overflowing_slab_distance_instead_of_missing` |
| TRACE-TP-026 | Broadphase leaf visit | TRACE-REQ-014 | A one-leaf BVH walked by intersect_ray with direction (1, 2^-32, 0) visits primitive 0. | `crates/bunny-broadphase/tests/bvh_tests.rs::issue_211_intersect_ray_visits_a_leaf_a_near_axis_ray_passes_through` |
| TRACE-TP-027 | Deliberate mutations of the overflow path | TRACE-REQ-012, TRACE-REQ-013 | Each rule of the overflow path, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |

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

[[case]]
id = "TRACE-TP-013"
requirements = ["TRACE-REQ-011", "TRACE-REQ-013"]
evidence = "property"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::every_previous_hit_keeps_its_bits_on_the_ordinary_corpus"
oracle = "Over 200 000 seeded ordinary, axis-parallel, tiny-component, grazing and origin-inside cases, every hit of the retained pre-#211 implementation is returned with the same bits, every miss it reports without overflow stays a miss, and every bound equals the exact bound rounded and saturated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-014"
requirements = ["TRACE-REQ-011", "TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "property"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::overflow_cases_agree_with_the_exact_rational_oracle"
oracle = "Over 200 000 seeded cases with coordinates near and past 2^62, i64 extremes and components of 1 to 4 000 raw units, every case where the old function overflowed agrees in hit or miss with an i128 cross-multiplication slab test, and every bound equals the exact bound rounded and saturated."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-015"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_oracle_tests.rs::rounding_oracle_matches_checked_div_on_representable_quotients"
oracle = "The test oracle's floor-based ties-to-even rounding equals FixedQ32_32::checked_div on 50 000 seeded representable quotients and on exact ties."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-016"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::one_raw_unit_component_hits_the_box_it_passes_through"
oracle = "Origin (-5, 0, 0), direction (1, 2^-32, 0), box [-1, 1]^3 returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-017"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::two_raw_unit_component_hits_the_box_it_passes_through"
oracle = "The same ray with a component of 2 raw units returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-018"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::negative_tiny_component_hits_too"
oracle = "The same ray with a component of -1 raw unit returns exactly (4, 6)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-019"
requirements = ["TRACE-REQ-011", "TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::hits_across_the_old_overflow_threshold_of_twice_the_slab_half_width"
oracle = "For k = 1..=64 and components d = 1..=2k+3 raw units, box [-k, k]^3 from (-100, 0, 0) returns exactly (100 - k, 100 + k) on both sides of the old d <= 2k miss boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-020"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::origin_far_from_box_saturates_the_unrepresentable_exit"
oracle = "From x = -1.5e9 into x in [5e8, 7e8] returns (2e9, i64::MAX raw)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-021"
requirements = ["TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::coordinate_difference_beyond_i64_is_widened_not_saturated"
oracle = "A difference of 2^63 + 2^30 raw with direction 1 + 2^-32 returns the exactly computed raw exit 9 223 372 035 781 033 984, not the 9 223 372 034 707 292 160 that saturating the difference first gives."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-022"
requirements = ["TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_entries_that_tie_when_rounded_are_decided_exactly"
oracle = "Two slabs whose entries both saturate to the maximum, and whose rounded bounds would tie, are a miss because the exact entry exceeds the exact exit."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-023"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::a_hit_beyond_the_representable_distance_saturates_both_bounds"
oracle = "A box first reached more than 2^31 units from the origin is a hit with both bounds at i64::MAX raw."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-024"
requirements = ["TRACE-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/ray_aabb_overflow_tests.rs::saturated_bounds_never_create_a_false_hit"
oracle = "Near-axis rays with components of 0 to 4 raw units still miss a box beside their path."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-025"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/query_tests.rs::ray_aabb_saturates_an_overflowing_slab_distance_instead_of_missing"
oracle = "An origin at i64::MAX raw on the box's +x face returns (i64::MIN raw, 0)."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-026"
requirements = ["TRACE-REQ-014"]
evidence = "test"
test = "crates/bunny-broadphase/tests/bvh_tests.rs::issue_211_intersect_ray_visits_a_leaf_a_near_axis_ray_passes_through"
oracle = "A one-leaf BVH walked by intersect_ray with direction (1, 2^-32, 0) visits primitive 0."
tier = "fast"
status = "implemented"

[[case]]
id = "TRACE-TP-027"
requirements = ["TRACE-REQ-012", "TRACE-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/ray-tracing/test-plan.md#mutation-checks"
oracle = "Each rule of the ray_intersects_aabb overflow path, broken on purpose, makes at least one implemented case fail."
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

The overflow path of `ray_intersects_aabb` (`crates/bunny-query/src/ray/aabb_wide.rs`)
was checked the same way, running `ray_aabb_overflow_tests`, `ray_aabb_oracle_tests`
and `query_tests`:

| Mutation in `ray/aabb_wide.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Magnitude truncated instead of rounded to nearest | Bounds are correctly rounded | TRACE-TP-013, TRACE-TP-014 |
| Hit decided from the rounded, saturated bounds instead of exactly | Saturation never creates a hit | TRACE-TP-013, TRACE-TP-014, TRACE-TP-022 |
| Coordinate difference saturated to `i64` before the division | Differences are widened | TRACE-TP-014, TRACE-TP-021 |
| `t_exit >= 0` check dropped | Boxes behind the origin miss | TRACE-TP-013, TRACE-TP-014 |

Ties to even versus ties away from zero is not observable through the public
API: a quotient `diff * 2^32 / d` is an exact half only when `d` is a multiple
of `2^33`, and a unit direction component is at most `2^32 + 1` raw. Replacing
the tie rule therefore passes every case; `TRACE-TP-015` pins the oracle's tie
rule to `checked_div` directly instead.

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
- a crossing whose distance does not fit `i64`;
- for `ray_intersects_aabb`: direction components of 1 to 4 000 raw units,
  coordinate differences past `i64`, slab distances past Q32.32 on one or both
  sides, two saturated entries, an origin on a face at the edge of the range,
  inverted boxes (`min > max` on an axis) and grazing contacts whose rounded
  bounds tie.

Future traversal work must add cases for ties between equal-distance hits.

## Stress And Fuzz

No fuzz target exists today. `TRACE-TP-001`, `TRACE-TP-002`, `TRACE-TP-004`
and `TRACE-TP-007` are seeded property cases over 4 000 to 40 000 inputs each.
`TRACE-TP-013` and `TRACE-TP-014` are seeded property cases over 200 000 inputs
each (SplitMix64, seeds `0xB002_0211_0000_0001` and `0xB002_0211_0000_0002`);
their failure messages name the seed and the case index.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| Closest-hit BVH traversal over `TraceTriangle` records and its tie-breaking. | Traversal API (flyingrobots/bunny#123). |
| Before/after timing of the kernels. | Benchmark cases (flyingrobots/bunny#210). |
| Canonical hit record, plane, segment, capsule and OBB intersections. | Remaining scope of flyingrobots/bunny#159. |
