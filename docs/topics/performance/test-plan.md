# Performance Test Plan

This document defines how Bunny proves its performance tooling and the fidelity rule
described in the [performance chapter](README.md).

## Scope

This plan covers:

- parsing, aggregation and comparison of `bunny-perf` output in `cargo xtask perf`
- the `bunny-perf-evidence/1` file written by `--evidence`, and recomputing the report from it
- the JSON Schemas that define each evidence line kind, and the writer's agreement with them
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
- Prove an evidence file holds every round and the build identity, and that the report table
  can be rebuilt from it by code independent of the report's own.

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
| PERF-REQ-003 | The regression gate fails only when a case's floor p50 (lowest, or second-lowest from five rounds) and its median p50 both slow past the threshold; one outlier round on either side changes neither the verdict of an unchanged case nor that of a real regression. | `README.md#one-lucky-round` |
| PERF-REQ-004 | Cases the base revision cannot build are reported as new. | `README.md#measuring` |
| PERF-REQ-005 | The harness is built from the same source against each revision's own crates. | `README.md#measuring` |
| PERF-REQ-006 | The whole-scene closest-hit cases time comparable work: both APIs choose the same primitive on every benchmark ray. | `README.md#adding-a-case` |
| PERF-REQ-007 | The profile takes its node, box and leaf counts from the counted compact searches and keeps a harness replica only for the per-stage triangle counters and the recorded calls; the replica tests the same primitives, in the same order with the same limits, as `closest_hit` and `any_hit` on every benchmark ray; the core crates carry no counters beyond the counted searches. | `README.md#profile-of-closest_hit-on-the-benchmark-scene` |
| PERF-REQ-008 | Memory and layout experiments time comparable work: every replica kernel and every alternative tree or layout returns the shipped function's exact result on every benchmark call or ray. | `README.md#memory-share-of-a-box-test` |
| PERF-REQ-009 | The SAH-tree trace cases time comparable work: closest_hit and any_hit give the same answer on every benchmark ray in the build_bvh tree and the build_bvh_sah tree. | `README.md#adding-a-case` |
| PERF-REQ-010 | The harness builds with each feature group alone and with all groups, so the per-group fallback of `cargo xtask perf` compares every group the base can build. | `README.md#adding-a-case` |
| PERF-REQ-011 | With `--evidence <file>`, `cargo xtask perf` writes every round of both sides (case, side, round, p50, p95, p99) as `bunny-perf-evidence/1` JSON Lines, from which the report table and the gate's floor p50s are recomputed exactly; the file is the same for the same rounds whatever order they were recorded in, and non-finite numbers are refused. | `README.md#evidence-files` |
| PERF-REQ-012 | The evidence file names the base reference, the full base and candidate SHAs, the target triple, the flags (rounds, code alignment, `RUSTFLAGS`, harness arguments, runner, threshold) and whether the working tree was dirty by `git status --porcelain`. | `README.md#evidence-files` |
| PERF-REQ-013 | Evidence is opt-in: without `--evidence` the command prints the same table, appends the same summary and exits with the same status as before, and an existing evidence file is never overwritten. | `README.md#evidence-files` |
| PERF-REQ-014 | Each line kind of `bunny-perf-evidence/1` is defined by a JSON Schema (draft 2020-12) under `schemas/bunny/perf-evidence/v1/`: every line the writer produces validates against the schema for its kind, deliberately wrong lines do not, and the fields the writer emits are exactly the fields each schema requires, in the same order. | `README.md#evidence-files` |

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
summary = "The regression gate fails only when a case's floor p50 (lowest, or second-lowest from five rounds) and its median p50 both slow past the threshold; one outlier round on either side changes neither the verdict of an unchanged case nor that of a real regression."
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
summary = "The profile takes its node, box and leaf counts from the counted compact searches and keeps a harness replica only for the per-stage triangle counters and the recorded calls; the replica tests the same primitives, in the same order with the same limits, as closest_hit and any_hit on every benchmark ray; the core crates carry no counters beyond the counted searches."
status = "active"

[[requirement]]
id = "PERF-REQ-008"
summary = "Memory and layout experiments time comparable work: every replica kernel and every alternative tree or layout returns the shipped function's exact result on every benchmark call or ray."
status = "active"

[[requirement]]
id = "PERF-REQ-009"
summary = "The SAH-tree trace cases time comparable work: closest_hit and any_hit give the same answer on every benchmark ray in the build_bvh tree and the build_bvh_sah tree."
status = "active"

[[requirement]]
id = "PERF-REQ-010"
summary = "The harness builds with each feature group alone and with all groups, so the per-group fallback of `cargo xtask perf` compares every group the base can build."
status = "active"

[[requirement]]
id = "PERF-REQ-011"
summary = "With `--evidence <file>`, `cargo xtask perf` writes every round of both sides (case, side, round, p50, p95, p99) as `bunny-perf-evidence/1` JSON Lines, from which the report table and the gate's floor p50s are recomputed exactly; the file is the same for the same rounds whatever order they were recorded in, and non-finite numbers are refused."
status = "active"

[[requirement]]
id = "PERF-REQ-012"
summary = "The evidence file names the base reference, the full base and candidate SHAs, the target triple, the flags (rounds, code alignment, `RUSTFLAGS`, harness arguments, runner, threshold) and whether the working tree was dirty by `git status --porcelain`."
status = "active"

[[requirement]]
id = "PERF-REQ-013"
summary = "Evidence is opt-in: without `--evidence` the command prints the same table, appends the same summary and exits with the same status as before, and an existing evidence file is never overwritten."
status = "active"

[[requirement]]
id = "PERF-REQ-014"
summary = "Each line kind of `bunny-perf-evidence/1` is defined by a JSON Schema (draft 2020-12) under `schemas/bunny/perf-evidence/v1/`: every line the writer produces validates against the schema for its kind, deliberately wrong lines do not, and the fields the writer emits are exactly the fields each schema requires, in the same order."
status = "active"
```

Each optimization adds its equivalence or error-bound cases under PERF-REQ-001.

## Fixtures

No external fixtures. Tests build synthetic `bunny-perf` output and manifests in memory. The
evidence tests record constructed rounds through `Measured::record`, the same seam
`run_interleaved` uses, so no harness is built or timed; the dirty flag is tested on
constructed `git status --porcelain` text, never by dirtying the real tree. The schema tests read the two
committed schema files under `schemas/bunny/perf-evidence/v1/` and validate with a test-only
validator for the subset of JSON Schema those files use (listed at the top of
`xtask/src/perf_evidence/tests/schema.rs`), which refuses any other keyword instead of
ignoring it. Both test files parse JSON with the test-only reader in
`xtask/src/perf_evidence/json.rs`, which shares nothing with the writer.

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
| PERF-TP-023 | Gate | PERF-REQ-003 | The failure message reports the floor and median change the gate tested. | `xtask/src/perf.rs::tests::the_gate_message_reports_the_floor_and_median_change_it_tested` |
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
| PERF-TP-042 | Memory | PERF-REQ-008 | The harness replica of `box_entry` (one box, and two boxes at once) returns `box_entry`'s exact result on every box test of the 1 024 benchmark searches, and the recorded calls split into one root test plus sibling pairs. | `crates/bunny-perf/src/memory.rs::tests::the_replica_box_test_returns_box_entry_bits_on_every_recorded_call` |
| PERF-TP-043 | Memory | PERF-REQ-008 | The replica returns `box_entry`'s exact result on hand-built axis-parallel, diagonal, inside-the-box and behind-the-origin rays with zero, negative, small and unbounded limits. | `crates/bunny-perf/src/memory.rs::tests::the_replica_box_test_matches_box_entry_on_axis_parallel_and_edge_rays` |
| PERF-TP-044 | Memory | PERF-REQ-008 | The shuffled tree moves more than half of the nodes, and `closest_hit` on it returns the same `ClosestHit` as on the builder's tree on all 1 024 rays. | `crates/bunny-perf/src/memory.rs::tests::the_shuffled_tree_is_a_permutation_with_the_same_answers` |
| PERF-TP-045 | Memory | PERF-REQ-008 | On all 1 024 rays, `compact_closest_hit` and `compact_any_hit` on the compact tree and on its three reordered copies call the primitive test with the same primitives and limits in the same order, and return the same answers, as `closest_hit` and `any_hit` on the builder's tree. | `crates/bunny-perf/src/compact.rs::tests::every_compact_tree_makes_the_same_calls_and_answers_as_the_builders_tree` |
| PERF-TP-046 | Memory | PERF-REQ-008 | The scene's compact tree has one node per interior source node and an interior root. | `crates/bunny-perf/src/compact.rs::tests::the_compact_tree_has_one_node_per_interior_source_node` |
| PERF-TP-047 | Comparability | PERF-REQ-009 | `closest_hit` returns the same hit and `any_hit` the same answer on all 1 024 camera rays through both trees. | `crates/bunny-perf/src/scene.rs::tests::the_sah_tree_and_the_quartile_tree_agree_on_every_ray` |
| PERF-TP-048 | Quality | PERF-REQ-009 | The SAH tree's exact SAH cost on the benchmark scene is below the quartile tree's; shapes and triangle-test counts are printed for the ledger. | `crates/bunny-perf/src/scene.rs::tests::the_sah_tree_costs_less_than_the_quartile_tree` |
| PERF-TP-049 | Build | PERF-REQ-010 | The gate's command list holds one `cargo check -p bunny-perf --no-default-features` per declared group, in manifest order, then one with every group. | `xtask/src/feature_configs.rs::tests::perf_harness_is_checked_once_per_group_alone_and_once_with_all` |
| PERF-TP-050 | Build | PERF-REQ-010 | Every group declared in the real harness manifest has a build of its own in the gate, so a new group cannot be left out. | `xtask/src/feature_configs.rs::tests::perf_gate_covers_every_group_the_harness_manifest_declares` |
| PERF-TP-051 | Build | PERF-REQ-010 | `cargo run --locked -p xtask -- code-dojo-perf-groups` exits 0; on the unfixed harness it fails on the first group (`num`: `method vec3 is never used`). | `xtask/src/code_dojo.rs::handle_perf_groups` |
| PERF-TP-052 | Gate | PERF-REQ-003 | The pull request 252 shape (7 rounds, one base round at 80.69 ns, the rest within 0.5 %) has a best-p50 change above +20 % and no regression at the 20 % threshold. | `xtask/src/perf_report.rs::tests::one_lucky_base_round_does_not_fail_the_gate` |
| PERF-TP-053 | Gate | PERF-REQ-003 | Base rounds near 100 ns and head rounds near 130 ns (7 rounds) fail the 20 % threshold. | `xtask/src/perf_report.rs::tests::a_regression_in_every_round_still_fails_the_gate` |
| PERF-TP-054 | Gate | PERF-REQ-003 | A real +30 % regression with one head round at 95 ns (best change negative) still fails the 20 % threshold. | `xtask/src/perf_report.rs::tests::one_lucky_head_round_does_not_hide_a_regression` |
| PERF-TP-055 | Gate | PERF-REQ-003 | Three fast base rounds of seven against no fast head round, equal medians: no regression at the 20 % threshold. | `xtask/src/perf_report.rs::tests::a_floor_that_regresses_while_the_median_does_not_passes` |
| PERF-TP-056 | Gate | PERF-REQ-003 | The floor is the lowest p50 for four rounds, the second-lowest for five, and a fast mode in two rounds is kept. | `xtask/src/perf_report.rs::tests::the_floor_skips_the_lowest_round_only_from_five_rounds` |
| PERF-TP-057 | Profile | PERF-REQ-007 | On all 1 024 rays and both modes, the profile's popped, skipped_at_pop, interior_visited, leaf_visited, box_calls, box_some, box_none, stack_peak and tri_tests equal what the counted compact searches return for the ray, and the replica's recorded box and triangle calls number as many as the library's box_tests and primitive_tests. Setting leaf_visited one too high fails this case and PERF-TP-030. | `crates/bunny-perf/src/profile.rs::tests::the_profile_takes_its_counts_from_the_counted_searches` |
| PERF-TP-058 | Evidence | PERF-REQ-011 | A reader written in the test, sharing no helper with `perf_report`, parses the file and rebuilds the whole report table byte for byte, and each case's floor p50s, for 1, 2, 4, 5 and 7 rounds over constructed cases with ties, identical sides, a case on one side only, zero timings and decimals that need 17 digits; every round of every case and side is present. | `xtask/src/perf_evidence/tests.rs::an_independent_reading_of_the_evidence_recomputes_the_report_table` |
| PERF-TP-059 | Evidence | PERF-REQ-011, PERF-REQ-012 | The exact text of a one-round file: key order, line order, JSON escaping of quotes, backslashes and control characters, `null` and empty arrays, and shortest round-trip numbers. | `xtask/src/perf_evidence/tests.rs::the_evidence_format_is_pinned_line_by_line` |
| PERF-TP-060 | Evidence | PERF-REQ-011 | Rounds recorded in reverse give the same file and table; lines are sorted by case, side and round. | `xtask/src/perf_evidence/tests.rs::the_evidence_is_the_same_whatever_order_the_rounds_were_recorded_in` |
| PERF-TP-061 | Evidence | PERF-REQ-012 | Empty `git status --porcelain` output is clean; a modified, staged or untracked entry is dirty; both states reach the build line. | `xtask/src/perf_evidence/tests.rs::the_dirty_flag_follows_git_status` |
| PERF-TP-062 | Evidence | PERF-REQ-011 | Infinite or NaN timings, which the report parses, and an infinite threshold are refused rather than written. | `xtask/src/perf_evidence/tests.rs::non_finite_numbers_are_refused` |
| PERF-TP-063 | Evidence | PERF-REQ-012 | The host triple is the `host:` line of `rustc -vV`, with or without a carriage return; no line, no triple. | `xtask/src/perf_evidence/tests.rs::the_host_triple_comes_from_rustc_verbose_version` |
| PERF-TP-064 | Evidence | PERF-REQ-013 | No `--evidence`, no evidence path; the option takes a value; an existing file is refused and a missing one accepted. | `xtask/src/perf.rs::tests::the_evidence_option_is_opt_in` |
| PERF-TP-065 | Evidence | PERF-REQ-013 | Recording rounds accumulates the same runs as `add_run` did, and the report is `render` of `compare` with the heading labels `handle` used before; a malformed line is still an error. | `xtask/src/perf.rs::tests::the_report_is_rendered_as_before` |
| PERF-TP-066 | Schema | PERF-REQ-014 | Every line the writer produces for the constructed cases (1, 2, 4, 5 and 7 rounds; clean, dirty and cross-target identities with `null` runner or threshold) validates against the schema for its `kind`: 15 build lines and more than 300 round lines. | `xtask/src/perf_evidence/tests/schema.rs::every_line_the_writer_produces_validates_against_its_kind_schema` |
| PERF-TP-067 | Schema | PERF-REQ-014 | Real lines with one fault each fail with a violation naming the field: a missing field, an extra field, `side` = `head`, a string `p50`, a wrong `schema` id, a wrong `kind`, `round` 0 and 1.5, string `rounds`, a number in `harness_args`, a boolean `runner`, a string `max_regression`, a string `candidate_dirty`. | `xtask/src/perf_evidence/tests/schema.rs::deliberately_wrong_lines_fail_their_schema` |
| PERF-TP-068 | Schema | PERF-REQ-014 | Each schema file parses, names draft 2020-12, is an object schema with `additionalProperties: false`, requires every property in property order, describes every property, and fixes `schema` and `kind` with `const`; every real line of the kind has exactly the required keys, in that order. Removing a field from `required`, or renaming or adding a field in the writer, fails this case. | `xtask/src/perf_evidence/tests/schema.rs::the_schema_files_are_well_formed_and_match_the_writer_fields` |
| PERF-TP-069 | Schema | PERF-REQ-014 | The test validator refuses, rather than ignores, a keyword or form outside its stated subset: `pattern`, `format`, an unknown type, `additionalProperties: true`, a numeric `const`, `$ref`. | `xtask/src/perf_evidence/tests/schema.rs::the_validator_refuses_keywords_it_does_not_implement` |
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
test = "xtask/src/perf.rs::tests::the_gate_message_reports_the_floor_and_median_change_it_tested"
oracle = "For three rounds at 100 and 130 ns the message names `a (+30.0% floor, +30.0% median)`."
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
id = "PERF-TP-042"
requirements = ["PERF-REQ-008"]
evidence = "test"
test = "crates/bunny-perf/src/memory.rs::tests::the_replica_box_test_returns_box_entry_bits_on_every_recorded_call"
oracle = "The replica box test (one box and two boxes at once) returns box_entry's exact result on every recorded box test of the 1 024 benchmark searches."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-043"
requirements = ["PERF-REQ-008"]
evidence = "test"
test = "crates/bunny-perf/src/memory.rs::tests::the_replica_box_test_matches_box_entry_on_axis_parallel_and_edge_rays"
oracle = "The replica returns box_entry's exact result on hand-built axis-parallel, diagonal, inside and behind-the-origin rays over zero, negative, small and unbounded limits."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-044"
requirements = ["PERF-REQ-008"]
evidence = "test"
test = "crates/bunny-perf/src/memory.rs::tests::the_shuffled_tree_is_a_permutation_with_the_same_answers"
oracle = "closest_hit on the shuffled tree returns the same ClosestHit as on the builder's tree on all 1 024 rays; more than half the nodes moved."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-045"
requirements = ["PERF-REQ-008"]
evidence = "test"
test = "crates/bunny-perf/src/compact.rs::tests::every_compact_tree_makes_the_same_calls_and_answers_as_the_builders_tree"
oracle = "On all 1 024 rays, the compact searches on the compact tree and its three reordered copies make the same primitive calls in the same order and return the same answers as closest_hit and any_hit on the builder's tree."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-046"
requirements = ["PERF-REQ-008"]
evidence = "test"
test = "crates/bunny-perf/src/compact.rs::tests::the_compact_tree_has_one_node_per_interior_source_node"
oracle = "The scene's compact tree has one node per interior source node and an interior root."
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

[[case]]
id = "PERF-TP-049"
requirements = ["PERF-REQ-010"]
evidence = "test"
test = "xtask/src/feature_configs.rs::tests::perf_harness_is_checked_once_per_group_alone_and_once_with_all"
oracle = "For a manifest declaring num and angle the commands' feature lists are exactly num, angle and num,angle, each with --no-default-features and -p bunny-perf."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-050"
requirements = ["PERF-REQ-010"]
evidence = "test"
test = "xtask/src/feature_configs.rs::tests::perf_gate_covers_every_group_the_harness_manifest_declares"
oracle = "For the real harness manifest there is one command per declared group plus one for all of them, and each group appears alone."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-051"
requirements = ["PERF-REQ-010"]
evidence = "static-check"
artifact = "xtask/src/code_dojo.rs"
oracle = "`code-dojo-perf-groups` builds the harness once per group alone and once with all groups, each with warnings denied, and exits 0."
tier = "slow"
status = "implemented"

[[case]]
id = "PERF-TP-052"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::one_lucky_base_round_does_not_fail_the_gate"
oracle = "The pull request 252 shape (7 rounds, one base round at 80.69 ns, the rest within 0.5 %) has a best-p50 change above +20 % and no regression at the 20 % threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-053"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::a_regression_in_every_round_still_fails_the_gate"
oracle = "Base rounds near 100 ns and head rounds near 130 ns (7 rounds) fail the 20 % threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-054"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::one_lucky_head_round_does_not_hide_a_regression"
oracle = "A real +30 % regression with one head round at 95 ns (best change negative) still fails the 20 % threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-055"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::a_floor_that_regresses_while_the_median_does_not_passes"
oracle = "Three fast base rounds of seven against no fast head round, equal medians: no regression at the 20 % threshold."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-056"
requirements = ["PERF-REQ-003"]
evidence = "test"
test = "xtask/src/perf_report.rs::tests::the_floor_skips_the_lowest_round_only_from_five_rounds"
oracle = "The floor is the lowest p50 for four rounds, the second-lowest for five, and a fast mode in two rounds is kept."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-057"
requirements = ["PERF-REQ-007"]
evidence = "test"
test = "crates/bunny-perf/src/profile.rs::tests::the_profile_takes_its_counts_from_the_counted_searches"
oracle = "The library's own counted searches (TRACE-TP-417 ties them to the uncounted searches); the replica's recorded calls as a cross-check."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-058"
requirements = ["PERF-REQ-011"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::an_independent_reading_of_the_evidence_recomputes_the_report_table"
oracle = "A reader written in the test, sharing no helper with `perf_report`, parses the file and rebuilds the whole report table byte for byte, and each case's floor p50s, for 1, 2, 4, 5 and 7 rounds over constructed cases with ties, identical sides, a case on one side only, zero timings and decimals that need 17 digits; every round of every case and side is present."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-059"
requirements = ["PERF-REQ-011", "PERF-REQ-012"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::the_evidence_format_is_pinned_line_by_line"
oracle = "The exact text of a one-round file: key order, line order, JSON escaping of quotes, backslashes and control characters, `null` and empty arrays, and shortest round-trip numbers."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-060"
requirements = ["PERF-REQ-011"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::the_evidence_is_the_same_whatever_order_the_rounds_were_recorded_in"
oracle = "Rounds recorded in reverse give the same file and table; lines are sorted by case, side and round."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-061"
requirements = ["PERF-REQ-012"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::the_dirty_flag_follows_git_status"
oracle = "Empty `git status --porcelain` output is clean; a modified, staged or untracked entry is dirty; both states reach the build line."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-062"
requirements = ["PERF-REQ-011"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::non_finite_numbers_are_refused"
oracle = "Infinite or NaN timings, which the report parses, and an infinite threshold are refused rather than written."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-063"
requirements = ["PERF-REQ-012"]
evidence = "test"
test = "xtask/src/perf_evidence/tests.rs::the_host_triple_comes_from_rustc_verbose_version"
oracle = "The host triple is the `host:` line of `rustc -vV`, with or without a carriage return; no line, no triple."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-064"
requirements = ["PERF-REQ-013"]
evidence = "test"
test = "xtask/src/perf.rs::tests::the_evidence_option_is_opt_in"
oracle = "No `--evidence`, no evidence path; the option takes a value; an existing file is refused and a missing one accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-065"
requirements = ["PERF-REQ-013"]
evidence = "test"
test = "xtask/src/perf.rs::tests::the_report_is_rendered_as_before"
oracle = "Recording rounds accumulates the same runs as `add_run` did, and the report is `render` of `compare` with the heading labels `handle` used before; a malformed line is still an error."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-066"
requirements = ["PERF-REQ-014"]
evidence = "test"
test = "xtask/src/perf_evidence/tests/schema.rs::every_line_the_writer_produces_validates_against_its_kind_schema"
oracle = "Every line the writer produces for the constructed cases (1, 2, 4, 5 and 7 rounds; clean, dirty and cross-target identities with `null` runner or threshold) validates against the schema for its `kind`: 15 build lines and more than 300 round lines."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-067"
requirements = ["PERF-REQ-014"]
evidence = "test"
test = "xtask/src/perf_evidence/tests/schema.rs::deliberately_wrong_lines_fail_their_schema"
oracle = "Real lines with one fault each fail with a violation naming the field: a missing field, an extra field, `side` = `head`, a string `p50`, a wrong `schema` id, a wrong `kind`, `round` 0 and 1.5, string `rounds`, a number in `harness_args`, a boolean `runner`, a string `max_regression`, a string `candidate_dirty`."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-068"
requirements = ["PERF-REQ-014"]
evidence = "test"
test = "xtask/src/perf_evidence/tests/schema.rs::the_schema_files_are_well_formed_and_match_the_writer_fields"
oracle = "Each schema file parses, names draft 2020-12, is an object schema with `additionalProperties: false`, requires every property in property order, describes every property, and fixes `schema` and `kind` with `const`; every real line of the kind has exactly the required keys, in that order. Removing a field from `required`, or renaming or adding a field in the writer, fails this case."
tier = "fast"
status = "implemented"

[[case]]
id = "PERF-TP-069"
requirements = ["PERF-REQ-014"]
evidence = "test"
test = "xtask/src/perf_evidence/tests/schema.rs::the_validator_refuses_keywords_it_does_not_implement"
oracle = "The test validator refuses, rather than ignores, a keyword or form outside its stated subset: `pattern`, `format`, an unknown type, `additionalProperties: true`, a numeric `const`, `$ref`."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The report logic is pure: it parses text, sorts with `f64::total_cmp` and renders
Markdown, with no clock, randomness or filesystem access in the tested functions. Timing
runs are deliberately outside the test oracle. The evidence writer is pure in the same way:
the same rounds and identity give the same bytes whatever order the rounds were recorded in
(PERF-TP-060).

## Gaps

- The interleaving, worktree handling, cross-target runs and CI workflow are exercised by
  running `cargo xtask perf`, not by automated tests. The platform matrix has not run on
  GitHub yet.
- The git and `rustc -vV` calls that gather the build identity, and the write of the evidence
  file itself, run only in a real `cargo xtask perf --evidence` run; the tests cover the pure
  functions they feed (PERF-TP-061, PERF-TP-063) and the refusal of an existing path
  (PERF-TP-064). No evidence file is committed yet.
