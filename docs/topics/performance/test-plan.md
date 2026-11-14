# Performance Test Plan

This document defines how Bunny proves its performance tooling and the fidelity rule
described in the [performance chapter](README.md).

## Scope

This plan covers:

- parsing, aggregation and comparison of `bunny-perf` output in `cargo xtask perf`
- the regression threshold used by CI
- building the harness against a revision that lacks newer feature groups
- the fidelity rule that every optimization must satisfy

Timing numbers themselves are measurements, not test oracles: no test asserts that a
kernel runs in a given number of nanoseconds.

## Test Goals

- Prove the report takes medians across rounds and computes change and speedup correctly.
- Prove the regression gate fails exactly when a case slows past the threshold.
- Prove cases present only in the candidate are reported as new, not as regressions.
- Prove the generated harness manifest points at the requested revision's crates.

## Non-Goals

- Asserting absolute timings or speedups.
- Testing GitHub Actions itself; the workflow is exercised by running it.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| PERF-REQ-001 | An optimization either preserves every output bit or ships as a new API with a stated error bound. | `README.md#the-fidelity-rule` |
| PERF-REQ-002 | Comparisons report the median of each percentile and the best p50 across interleaved rounds. | `README.md#measuring` |
| PERF-REQ-003 | The regression gate fails only when a case's best p50 slows past the threshold. | `README.md#continuous-integration` |
| PERF-REQ-004 | Cases the base revision cannot build are reported as new. | `README.md#measuring` |
| PERF-REQ-005 | The harness is built from the same source against each revision's own crates. | `README.md#measuring` |
| PERF-REQ-006 | The whole-scene closest-hit cases time comparable work: both APIs choose the same primitive on every benchmark ray. | `README.md#adding-a-case` |
| PERF-REQ-007 | The traversal counters come from a harness replica that tests the same primitives, in the same order with the same limits, as `closest_hit` and `any_hit` on every benchmark ray; the core crates carry no counters. | `README.md#profile-of-closest_hit-on-the-benchmark-scene` |
| PERF-REQ-009 | The SAH-tree trace cases time comparable work: closest_hit and any_hit give the same answer on every benchmark ray in the build_bvh tree and the build_bvh_sah tree. | `README.md#adding-a-case` |

```toml
[[requirement]]
id = "PERF-REQ-001"
summary = "An optimization either preserves every output bit or ships as a new API with a stated error bound."
status = "active"

[[requirement]]
id = "PERF-REQ-002"
summary = "Comparisons report the median of each percentile and the best p50 across interleaved rounds."
status = "active"

[[requirement]]
id = "PERF-REQ-003"
summary = "The regression gate fails only when a case's best p50 slows past the threshold."
status = "active"

[[requirement]]
id = "PERF-REQ-004"
summary = "Cases the base revision cannot build are reported as new."
status = "active"

[[requirement]]
id = "PERF-REQ-005"
summary = "The harness is built from the same source against each revision's own crates."
status = "active"

[[requirement]]
id = "PERF-REQ-006"
summary = "The whole-scene closest-hit cases time comparable work: both APIs choose the same primitive on every benchmark ray."
status = "active"

[[requirement]]
id = "PERF-REQ-007"
summary = "The traversal counters come from a harness replica that tests the same primitives, in the same order with the same limits, as closest_hit and any_hit on every benchmark ray; the core crates carry no counters."
status = "active"

[[requirement]]
id = "PERF-REQ-009"
summary = "The SAH-tree trace cases time comparable work: closest_hit and any_hit give the same answer on every benchmark ray in the build_bvh tree and the build_bvh_sah tree."
status = "active"
```

Each optimization adds its equivalence or error-bound cases under PERF-REQ-001.

## Fixtures

No external fixtures. Tests build synthetic `bunny-perf` output and manifests in memory.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| PERF-TP-001 | Parsing | PERF-REQ-002 | Exact parsed timings; malformed lines are errors. | `xtask/src/perf_report.rs::tests::runs_parse_tab_separated_lines` |
| PERF-TP-002 | Aggregation | PERF-REQ-002 | Median of odd and even round counts. | `xtask/src/perf_report.rs::tests::median_damps_one_noisy_round` |
| PERF-TP-003 | Comparison | PERF-REQ-002, PERF-REQ-004 | Change, speedup and `new` rows in the rendered table. | `xtask/src/perf_report.rs::tests::comparison_reports_change_and_new_cases` |
| PERF-TP-004 | Gate | PERF-REQ-003 | Only cases past the threshold are reported. | `xtask/src/perf_report.rs::tests::regressions_respect_the_threshold` |
| PERF-TP-005 | Build | PERF-REQ-004 | Feature groups exclude `default` and comments. | `xtask/src/perf.rs::tests::feature_groups_skip_default_and_comments` |
| PERF-TP-007 | Gate | PERF-REQ-003 | A bimodal case whose medians land in different modes does not fail the gate. | `xtask/src/perf_report.rs::tests::the_gate_ignores_a_bimodal_case_whose_median_lands_in_different_modes` |
| PERF-TP-008 | Exact replacement | PERF-REQ-001 | The new square root equals the retained bit-by-bit reference on boundaries and perfect squares. | `crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_u128_matches_reference_on_boundaries` |
| PERF-TP-009 | Exact replacement | PERF-REQ-001 | The new square root equals the reference on 20 000 seeded inputs across all magnitudes. | `crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_u128_matches_reference_on_seeded_inputs` |
| PERF-TP-010 | Exact replacement | PERF-REQ-001 | `FixedQ32_32::sqrt` equals the reference for every raw value 0..=65 536. | `crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_matches_reference_for_every_small_raw_value` |
| PERF-TP-011 | Error bound | PERF-REQ-001 | `sin` and `cos` stay within one raw unit of `f64` trigonometry. | `crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004_error_is_within_one_raw_unit` |
| PERF-TP-012 | Exact replacement | PERF-REQ-001 | Bunny's two-divide square root (retained, not selected on any target) equals core's `u128::isqrt` on 200 000 seeded inputs. | `crates/bunny-num/src/lib.rs::tests::two_divides_matches_core_isqrt_on_many_seeded_inputs` |
| PERF-TP-013 | Exact replacement | PERF-REQ-001 | Bunny's two-divide square root equals core's `u128::isqrt` at the 64-bit split and around quotient corrections. | `crates/bunny-num/src/lib.rs::tests::two_divides_matches_core_isqrt_around_the_split_and_correction_points` |
| PERF-TP-014 | Dispatch | PERF-REQ-001 | The implementation chosen for the target equals the retained two-divide implementation on 20 000 seeded inputs. | `crates/bunny-num/src/lib.rs::tests::the_chosen_implementation_matches_the_other_one` |
| PERF-TP-020 | Build | PERF-REQ-005 | Harness binary names follow the target (`.wasm`, `.exe`, none). | `xtask/src/perf.rs::tests::binary_names_follow_the_target` |
| PERF-TP-021 | Build | PERF-REQ-005 | Manifest paths use forward slashes and escape quotes, so Windows and unusual paths produce valid TOML. | `xtask/src/perf.rs::tests::toml_paths_use_forward_slashes_and_escape_quotes` |
| PERF-TP-022 | Gate | PERF-REQ-003 | A run that measured no case on both sides fails instead of passing. | `xtask/src/perf.rs::tests::the_gate_refuses_a_run_that_measured_nothing` |
| PERF-TP-023 | Gate | PERF-REQ-003 | The failure message reports the best-p50 change the gate tested. | `xtask/src/perf.rs::tests::the_gate_message_reports_the_best_p50_change_it_tested` |
| PERF-TP-024 | Build | PERF-REQ-005 | The generated manifest lists every crate the cases use, including `bunny-broadphase`. | `xtask/src/perf.rs::tests::harness_manifest_includes_every_crate_the_cases_use` |
| PERF-TP-025 | Comparability | PERF-REQ-006 | `scene/closest_visitor` and `trace/closest_hit` choose the same primitive (or both miss) on all 1 024 camera rays. | `crates/bunny-perf/src/scene.rs::tests::both_paths_choose_the_same_primitive_on_every_ray` |
| PERF-TP-026 | Comparability | PERF-REQ-006 | The benchmark scene has 27 650 triangles. | `crates/bunny-perf/src/scene.rs::tests::the_scene_has_the_stated_size` |
| PERF-TP-027 | Comparability | PERF-REQ-006 | More than 600 of the 1 024 rays hit the mesh, so the cases do traversal work, not only misses. | `crates/bunny-perf/src/scene.rs::tests::most_rays_hit_something` |
| PERF-TP-028 | Profile | PERF-REQ-007 | The the replica `closest_hit` search tests the same primitives with the same limits in the same order, and finds the same hit, as `closest_hit` on all 1 024 rays; its stage classification never disagrees with `hit_triangle`. | `crates/bunny-perf/src/profile.rs::tests::the_replica_closest_search_agrees_with_closest_hit_on_every_ray` |
| PERF-TP-029 | Profile | PERF-REQ-007 | The the replica `any_hit` search tests the same primitives in the same order and gives the same answer as `any_hit` on all 1 024 rays. | `crates/bunny-perf/src/profile.rs::tests::the_replica_any_search_agrees_with_any_hit_on_every_ray` |
| PERF-TP-030 | Profile | PERF-REQ-007 | The the counters obey their identities (popped = skipped + visited, box calls = 1 + 2 interior visits, triangle stages sum to tests, no overflow). | `crates/bunny-perf/src/profile.rs::tests::counters_obey_their_identities` |
| PERF-TP-031 | Profile | PERF-REQ-007 | The the tree statistics account for every primitive, nodes = 2 leaves - 1, depth below the stack capacity. | `crates/bunny-perf/src/profile.rs::tests::the_tree_statistics_account_for_every_primitive` |
| PERF-TP-032 | Profile | PERF-REQ-007 | The the three ray classes cover all 1 024 rays and the sphere class is not empty. | `crates/bunny-perf/src/profile.rs::tests::rays_split_into_the_three_classes` |
| PERF-TP-033 | Profile | PERF-REQ-007 | The the stage replica classifies hand-built rays as divided, behind, parallel, first-barycentric and second-barycentric failures. | `crates/bunny-perf/src/profile.rs::tests::the_stage_replica_matches_hit_triangle_on_a_hand_built_case` |
| PERF-TP-034 | Exact new API | PERF-REQ-001 | FixedDivisor equals checked_div on boundary values, signs and overflow. | `crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_boundaries` |
| PERF-TP-035 | Exact new API | PERF-REQ-001 | FixedDivisor equals checked_div for every power-of-two divisor and both signs. | `crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_for_every_power_of_two_divisor` |
| PERF-TP-036 | Exact new API | PERF-REQ-001 | FixedDivisor equals checked_div on 100 000 seeded pairs. | `crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_seeded_pairs` |
| PERF-TP-037 | Exact new API | PERF-REQ-001 | FixedDivisor equals checked_div on 10 000 constructed exact ties. | `crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_exact_ties` |
| PERF-TP-038 | Exact new API | PERF-REQ-001 | Prepared ray-box results equal ray_intersects_aabb on 160 000 seeded pairs. | `crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_seeded_rays_and_boxes` |
| PERF-TP-039 | Exact new API | PERF-REQ-001 | Prepared ray-box results equal ray_intersects_aabb for axis-parallel rays. | `crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_for_axis_rays_inside_and_outside_boxes` |
| PERF-TP-040 | Exact new API | PERF-REQ-001 | Prepared ray-box results equal ray_intersects_aabb, bit for bit, on an overflow-heavy seeded corpus (coordinates near the i64 extremes, direction components of a few raw units) that hits, misses and saturates bounds. | `crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_the_overflow_corpus` |
| PERF-TP-041 | Exact new API | PERF-REQ-001 | Prepared ray-box results equal ray_intersects_aabb on a constructed case whose slab distances overflow and whose result comes from the exact wide path. | `crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_constructed_overflow_cases` |
| PERF-TP-047 | Comparability | PERF-REQ-009 | `closest_hit` returns the same hit and `any_hit` the same answer on all 1 024 camera rays through both trees. | `crates/bunny-perf/src/scene.rs::tests::the_sah_tree_and_the_quartile_tree_agree_on_every_ray` |
| PERF-TP-048 | Quality | PERF-REQ-009 | The SAH tree's exact SAH cost on the benchmark scene is below the quartile tree's; shapes and triangle-test counts are printed for the ledger. | `crates/bunny-perf/src/scene.rs::tests::the_sah_tree_costs_less_than_the_quartile_tree` |
| PERF-TP-006 | Build | PERF-REQ-005 | The generated manifest is standalone and points at the given crates. | `xtask/src/perf.rs::tests::harness_manifest_points_at_the_given_crates` |

```toml
[[case]]
id = "PERF-TP-001"
requirements = ["PERF-REQ-002"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::runs_parse_tab_separated_lines"
oracle = "Exact parsed timings; malformed lines are errors."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-002"
requirements = ["PERF-REQ-002"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::median_damps_one_noisy_round"
oracle = "Median of odd and even round counts."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-003"
requirements = ["PERF-REQ-002", "PERF-REQ-004"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::comparison_reports_change_and_new_cases"
oracle = "Change, speedup and new rows in the rendered table."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-004"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::regressions_respect_the_threshold"
oracle = "Only cases past the threshold are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-005"
requirements = ["PERF-REQ-004"]
evidence = "test"
test = "xtask/src/perf.rs::tests::feature_groups_skip_default_and_comments"
oracle = "Feature groups exclude default and comments."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-006"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::harness_manifest_points_at_the_given_crates"
oracle = "The generated manifest is standalone and points at the given crates."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-007"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::the_gate_ignores_a_bimodal_case_whose_median_lands_in_different_modes"
oracle = "A bimodal case whose medians land in different modes does not fail the gate."
tier = "fast"
status = "implemented"
[[case]]
id = "PERF-TP-008"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_u128_matches_reference_on_boundaries"
oracle = "The new square root equals the retained bit-by-bit reference on boundaries and perfect squares."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-009"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_u128_matches_reference_on_seeded_inputs"
oracle = "The new square root equals the reference on 20 000 seeded inputs across all magnitudes."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-010"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/sqrt_equivalence_tests.rs::sqrt_matches_reference_for_every_small_raw_value"
oracle = "FixedQ32_32::sqrt equals the reference for every raw value 0..=65 536."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-011"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_angle_tests.rs::at_tp_004_error_is_within_one_raw_unit"
oracle = "sin and cos stay within one raw unit of f64 trigonometry."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-012"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/src/lib.rs::tests::two_divides_matches_core_isqrt_on_many_seeded_inputs"
oracle = "Bunny's two-divide square root (retained, not selected on any target) equals core's u128::isqrt on 200 000 seeded inputs."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-013"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/src/lib.rs::tests::two_divides_matches_core_isqrt_around_the_split_and_correction_points"
oracle = "Bunny's two-divide square root equals core's u128::isqrt at the 64-bit split and around quotient corrections."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-014"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/src/lib.rs::tests::the_chosen_implementation_matches_the_other_one"
oracle = "The implementation chosen for the target equals the retained two-divide implementation on 20 000 seeded inputs."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-020"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::binary_names_follow_the_target"
oracle = "Harness binary names follow the target (.wasm, .exe, none)."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-021"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::toml_paths_use_forward_slashes_and_escape_quotes"
oracle = "Manifest paths use forward slashes and escape quotes, so Windows and unusual paths produce valid TOML."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-022"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf.rs::tests::the_gate_refuses_a_run_that_measured_nothing"
oracle = "A run that measured no case on both sides fails instead of passing."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-023"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf.rs::tests::the_gate_message_reports_the_best_p50_change_it_tested"
oracle = "The failure message reports the best-p50 change the gate tested."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-024"
requirements = ["PERF-REQ-005"]
evidence = "test"
test = "xtask/src/perf.rs::tests::harness_manifest_includes_every_crate_the_cases_use"
oracle = "The generated manifest lists every crate the cases use."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-025"
requirements = ["PERF-REQ-006"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::both_paths_choose_the_same_primitive_on_every_ray"
oracle = "Both APIs choose the same primitive on every benchmark ray."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-026"
requirements = ["PERF-REQ-006"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::the_scene_has_the_stated_size"
oracle = "The scene has 27 650 triangles."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-027"
requirements = ["PERF-REQ-006"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::most_rays_hit_something"
oracle = "Most benchmark rays hit the mesh."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-028"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_replica_closest_search_agrees_with_closest_hit_on_every_ray"
oracle = "The replica `closest_hit` search tests the same primitives with the same limits in the same order, and finds the same hit, as `closest_hit` on all 1 024 rays; its stage classification never disagrees with `hit_triangle`."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-029"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_replica_any_search_agrees_with_any_hit_on_every_ray"
oracle = "The replica `any_hit` search tests the same primitives in the same order and gives the same answer as `any_hit` on all 1 024 rays."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-030"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::counters_obey_their_identities"
oracle = "The counters obey their identities (popped = skipped + visited, box calls = 1 + 2 interior visits, triangle stages sum to tests, no overflow)."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-031"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_tree_statistics_account_for_every_primitive"
oracle = "The tree statistics account for every primitive, nodes = 2 leaves - 1, depth below the stack capacity."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-032"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::rays_split_into_the_three_classes"
oracle = "The three ray classes cover all 1 024 rays and the sphere class is not empty."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-033"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_stage_replica_matches_hit_triangle_on_a_hand_built_case"
oracle = "The stage replica classifies hand-built rays as divided, behind, parallel, first-barycentric and second-barycentric failures."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-034"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_boundaries"
oracle = "FixedDivisor equals checked_div on boundary values, signs and overflow."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-035"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_for_every_power_of_two_divisor"
oracle = "FixedDivisor equals checked_div for every power-of-two divisor and both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-036"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_seeded_pairs"
oracle = "FixedDivisor equals checked_div on 100 000 seeded pairs."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-037"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/fixed_divisor_tests.rs::divisor_matches_checked_div_on_exact_ties"
oracle = "FixedDivisor equals checked_div on 10 000 constructed exact ties."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-038"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_seeded_rays_and_boxes"
oracle = "Prepared ray-box results equal ray_intersects_aabb on 160 000 seeded pairs."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-039"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_for_axis_rays_inside_and_outside_boxes"
oracle = "Prepared ray-box results equal ray_intersects_aabb for axis-parallel rays."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-040"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_the_overflow_corpus"
oracle = "Prepared ray-box results equal ray_intersects_aabb, bit for bit, on an overflow-heavy seeded corpus (coordinates near the i64 extremes, direction components of a few raw units) that hits, misses and saturates bounds."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-041"
requirements = ["PERF-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/prepared_ray_tests.rs::prepared_ray_matches_unprepared_on_constructed_overflow_cases"
oracle = "Prepared ray-box results equal ray_intersects_aabb on a constructed case whose slab distances overflow and whose result comes from the exact wide path."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-047"
requirements = ["PERF-REQ-009"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::the_sah_tree_and_the_quartile_tree_agree_on_every_ray"
oracle = "Both trees give the same closest_hit and any_hit answers on every benchmark ray."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-048"
requirements = ["PERF-REQ-009"]
evidence = "test"
test = "crates/bunny-perf/src/scene.rs::tests::the_sah_tree_costs_less_than_the_quartile_tree"
oracle = "The exact SAH cost of both trees over the benchmark scene."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The report logic is pure: it parses text, sorts with `f64::total_cmp` and renders
Markdown, with no clock, randomness or filesystem access in the tested functions. Timing
runs are deliberately outside the test oracle.

## Gaps

- The interleaving, worktree handling, cross-target runs and CI workflow are exercised by
  running `cargo xtask perf`, not by automated tests. The platform matrix has not run on
  GitHub yet.
