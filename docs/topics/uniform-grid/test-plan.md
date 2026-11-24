# Uniform Grid Test Plan

This document defines how Bunny verifies the uniform grid broadphase
`uniform_grid_pairs` (`bunny-uniform-grid/1`) in `bunny-broadphase`.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design: it records requirements, executable evidence and
known gaps.

## Scope

- the cell size domain and the quantization of Q32.32 coordinates to integer
  cell coordinates, over the whole raw range;
- the cells a box occupies and the exact key count, with its overflow refusal;
- the refusals and their order;
- the pairs: exactly the overlapping pairs, each once, in the same form and
  order as `sweep_and_prune`;
- the storage contract: scratch sizes, a short pair buffer, untouched tails,
  independence from scratch contents, and no allocation.

## Test Goals

- Compare every output against an oracle that shares no code with the kernel:
  floor division in `i128` with an explicit remainder correction, a closed
  overlap test written as `max(mins) <= min(maxes)`, an all-pairs scan, exact
  `u128` key counts and a direct enumeration of every (cell, object) key.
- Put constructed coordinates on each cell boundary and one raw unit either
  side, at zero, at negative boundaries and at both ends of the raw range.
- Generate seeded scenes whose coordinates sit on or next to cell boundaries
  half of the time; log every seed and pin every scene's pair count.
- Break each rule on purpose and record which case fails.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| GRID-REQ-001 | A cell size is a positive raw value from 1 to i64::MAX, any value, power of two or not; zero and negative sizes are refused with NonPositiveCellSize naming the raw value. | `README.md#cell-size` |
| GRID-REQ-002 | The cell of a coordinate is floor(raw / size), an i64, exact for the whole raw range; cell k is the half-open interval [k size, (k + 1) size), so a coordinate on a boundary is in the upper cell and a negative one below zero. | `README.md#cell-coordinates` |
| GRID-REQ-003 | A closed box occupies every cell from the cell of its min to the cell of its max inclusive on each axis; grid_key_count returns that exact count summed over the boxes, or KeyCountOverflow naming the box at which it passes usize. | `README.md#cells-of-a-box-and-the-key-count` |
| GRID-REQ-004 | Refusals, checked in a stated order before anything is written: TooManyBoxes, then per box in index order InvertedBox and KeyCountOverflow, then KeyScratchTooSmall naming required and provided. | `README.md#refusals` |
| GRID-REQ-005 | The pairs are exactly the pairs of boxes that overlap as closed boxes (touching counts), each once, equal to an all-pairs scan. | `README.md#pairs` |
| GRID-REQ-006 | Pairs are (lower index, higher index) in ascending lexicographic order, identical to sweep_and_prune on the same input, and do not depend on scratch contents on entry, the target, or the input order beyond the indices. | `README.md#pair-order` |
| GRID-REQ-007 | A short pair buffer holds the first pairs in pair order; the report counts every pair, so is_complete is false; entries past those written, and key scratch past the required length, are untouched. | `README.md#storage-contract` |
| GRID-REQ-008 | The kernel allocates nothing. | `README.md#storage-contract` |
| GRID-REQ-009 | The report counts candidate tests: one per two boxes sharing a cell, per shared cell. | `README.md#cost` |

```toml
[[requirement]]
id = "GRID-REQ-001"
summary = "A cell size is a positive raw value from 1 to i64::MAX, any value, power of two or not; zero and negative sizes are refused with NonPositiveCellSize naming the raw value."
status = "active"

[[requirement]]
id = "GRID-REQ-002"
summary = "The cell of a coordinate is floor(raw / size), an i64, exact for the whole raw range; cell k is the half-open interval [k size, (k + 1) size), so a coordinate on a boundary is in the upper cell and a negative one below zero."
status = "active"

[[requirement]]
id = "GRID-REQ-003"
summary = "A closed box occupies every cell from the cell of its min to the cell of its max inclusive on each axis; grid_key_count returns that exact count summed over the boxes, or KeyCountOverflow naming the box at which it passes usize."
status = "active"

[[requirement]]
id = "GRID-REQ-004"
summary = "Refusals, checked in a stated order before anything is written: TooManyBoxes, then per box in index order InvertedBox and KeyCountOverflow, then KeyScratchTooSmall naming required and provided."
status = "active"

[[requirement]]
id = "GRID-REQ-005"
summary = "The pairs are exactly the pairs of boxes that overlap as closed boxes (touching counts), each once, equal to an all-pairs scan."
status = "active"

[[requirement]]
id = "GRID-REQ-006"
summary = "Pairs are (lower index, higher index) in ascending lexicographic order, identical to sweep_and_prune on the same input, and do not depend on scratch contents on entry, the target, or the input order beyond the indices."
status = "active"

[[requirement]]
id = "GRID-REQ-007"
summary = "A short pair buffer holds the first pairs in pair order; the report counts every pair, so is_complete is false; entries past those written, and key scratch past the required length, are untouched."
status = "active"

[[requirement]]
id = "GRID-REQ-008"
summary = "The kernel allocates nothing."
status = "active"

[[requirement]]
id = "GRID-REQ-009"
summary = "The report counts candidate tests: one per two boxes sharing a cell, per shared cell."
status = "active"
```


## Fixtures

No external fixtures. Inputs are constructed boxes and seeded scenes built in
`crates/bunny-broadphase/tests/uniform_grid_tests.rs` from a 64-bit linear
congruential generator; every seed is a named constant and is printed with the
scene's box and pair counts.

## Test Cases

| ID | Category | Requirements | Case | Oracle | Test |
| --- | --- | --- | --- | --- | --- |
| GRID-TP-001 | Cell size | GRID-REQ-001 | 0, -1, -1.0 and i64::MIN raw are refused naming the raw value; 1, 2, 3, ONE - 1, ONE, 3 ONE + 7 and i64::MAX are accepted. | Hand-listed values on each side of zero. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_001_cell_size_must_be_positive` |
| GRID-TP-002 | Quantization | GRID-REQ-002 | For nine sizes from 1 raw to i64::MAX, every boundary k size for k in -3..=3 and one raw unit either side, and the ends of the range, equal an i128 floor oracle; hand-derived values at zero, negative boundaries and both ends. | Truncating i128 division corrected by the remainder's sign. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_002_cell_of_is_floor_division_on_raw_values` |
| GRID-TP-003 | Occupancy | GRID-REQ-003 | Key counts of boxes with faces on, and one raw unit either side of, boundaries at zero, positive and negative cells; a max on a boundary occupies the next cell; the written keys of two such boxes. | Hand-derived cell ranges. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_003_a_box_occupies_the_cells_of_its_min_to_its_max_inclusive` |
| GRID-TP-004 | Key count overflow | GRID-REQ-003, GRID-REQ-004 | The whole raw range at 1 raw and 1.0 cells, and two boxes of 2^63 keys, are refused with the box index (target-dependent index stated); the kernel then writes nothing. | Exact u128 key count of the oracle. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_004_key_count_overflow_is_refused_with_the_box_index` |
| GRID-TP-005 | Refusals | GRID-REQ-004, GRID-REQ-007 | Inverted boxes on each axis are refused at the lowest index; the lowest failing box wins whether inverted or overflowing; key scratch of required - 1 is refused naming both sizes; required and longer scratch run and leave the tail untouched; no boxes give an empty complete report. | Hand-derived errors; untouched sentinels. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_005_inverted_boxes_and_short_key_scratch_are_refused_before_anything_is_written` |
| GRID-TP-006 | Cell edges | GRID-REQ-003, GRID-REQ-005 | Boxes touching exactly at a cell boundary, one raw unit apart and one raw unit overlapping, at a positive boundary, at zero and at negative boundaries; a zero-extent box on a boundary; corner contact on all three axes. | The all-pairs scan and the sorted oracle keys, and hand-written pair lists. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_006_boxes_with_faces_on_cell_boundaries` |
| GRID-TP-007 | Range extremes | GRID-REQ-002, GRID-REQ-005 | Boxes at i64::MIN and i64::MAX raw with cells of 1 raw, 1.0, 2^62 and i64::MAX; the lowest and highest cells are pinned. | The all-pairs scan and the oracle keys. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_007_boxes_at_the_extremes_of_the_raw_range` |
| GRID-TP-008 | Seeded equality | GRID-REQ-005, GRID-REQ-006 | Five seeded scenes (scattered, dense cluster, boxes over many cells, one box larger than the occupied region, identical boxes and points), each at two or three cell sizes including non-powers of two: keys, key count and pairs equal the oracle; pair counts pinned (70, 326, 50, 170 and 158). | The all-pairs scan and the oracle keys. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_008_seeded_scenes_equal_the_all_pairs_scan` |
| GRID-TP-009 | Seeded extremes | GRID-REQ-005 | 120 seeded boxes with corners among the ends of the raw range, zero and plus or minus 2^62, at three wide cell sizes; 3 180 pairs, pinned. | The all-pairs scan and the oracle keys. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_009_seeded_scenes_at_the_extremes_of_the_raw_range` |
| GRID-TP-010 | Same as sweep | GRID-REQ-006 | On every seeded scene of GRID-TP-008, the grid's pairs equal sweep_and_prune's output element for element. | sweep_and_prune and the all-pairs scan. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_010_pairs_equal_sweep_and_prune_on_the_same_input` |
| GRID-TP-011 | Scratch and order | GRID-REQ-006 | Key scratch zeroed, filled with seeded garbage, and holding a previous run's keys reversed, and a pair buffer of stale values, give identical pairs, keys and reports; a seeded permutation of the input gives the permuted pairs. | The all-pairs scan. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_011_output_does_not_depend_on_scratch_contents_or_input_order` |
| GRID-TP-012 | Short buffer | GRID-REQ-007 | For every pair buffer length from 0 to the pair count plus 2: written is the smaller of the two, total is exact, the written prefix equals the oracle's prefix, and the rest is untouched. | The all-pairs scan. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_012_a_short_pair_buffer_holds_the_first_pairs_and_the_total_counts_all` |
| GRID-TP-013 | Once each | GRID-REQ-005, GRID-REQ-009 | Two identical boxes over 27 cells give one pair and 27 candidate tests; ten boxes in one cell give 45 tests; boxes sharing a cell without overlapping are tested and not reported. | Hand-derived counts. | `crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_013_pairs_sharing_many_cells_are_reported_once` |
| GRID-TP-014 | Allocation | GRID-REQ-008 | A counting allocator sees zero allocations by the calling thread during a run over 300 seeded boxes, under the crate's allocation measurement rule. | Thread-scoped allocation count. | `crates/bunny-broadphase/tests/uniform_grid_allocation_tests.rs::grid_tp_014_uniform_grid_pairs_allocates_zero_times` |
| GRID-TP-015 | Mutations | GRID-REQ-002, GRID-REQ-003, GRID-REQ-005, GRID-REQ-006, GRID-REQ-007 | Each deliberate mutation of the kernel makes at least one case fail. | Each mutant's test run. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "GRID-TP-001"
requirements = ["GRID-REQ-001"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_001_cell_size_must_be_positive"
oracle = "Hand-listed values on each side of zero."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-002"
requirements = ["GRID-REQ-002"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_002_cell_of_is_floor_division_on_raw_values"
oracle = "Truncating i128 division corrected by the remainder's sign."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-003"
requirements = ["GRID-REQ-003"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_003_a_box_occupies_the_cells_of_its_min_to_its_max_inclusive"
oracle = "Hand-derived cell ranges."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-004"
requirements = ["GRID-REQ-003", "GRID-REQ-004"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_004_key_count_overflow_is_refused_with_the_box_index"
oracle = "Exact u128 key count of the oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-005"
requirements = ["GRID-REQ-004", "GRID-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_005_inverted_boxes_and_short_key_scratch_are_refused_before_anything_is_written"
oracle = "Hand-derived errors; untouched sentinels."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-006"
requirements = ["GRID-REQ-003", "GRID-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_006_boxes_with_faces_on_cell_boundaries"
oracle = "The all-pairs scan and the sorted oracle keys, and hand-written pair lists."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-007"
requirements = ["GRID-REQ-002", "GRID-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_007_boxes_at_the_extremes_of_the_raw_range"
oracle = "The all-pairs scan and the oracle keys."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-008"
requirements = ["GRID-REQ-005", "GRID-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_008_seeded_scenes_equal_the_all_pairs_scan"
oracle = "The all-pairs scan and the oracle keys."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-009"
requirements = ["GRID-REQ-005"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_009_seeded_scenes_at_the_extremes_of_the_raw_range"
oracle = "The all-pairs scan and the oracle keys."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-010"
requirements = ["GRID-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_010_pairs_equal_sweep_and_prune_on_the_same_input"
oracle = "sweep_and_prune and the all-pairs scan."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-011"
requirements = ["GRID-REQ-006"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_011_output_does_not_depend_on_scratch_contents_or_input_order"
oracle = "The all-pairs scan."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-012"
requirements = ["GRID-REQ-007"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_012_a_short_pair_buffer_holds_the_first_pairs_and_the_total_counts_all"
oracle = "The all-pairs scan."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-013"
requirements = ["GRID-REQ-005", "GRID-REQ-009"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_tests.rs::grid_tp_013_pairs_sharing_many_cells_are_reported_once"
oracle = "Hand-derived counts."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-014"
requirements = ["GRID-REQ-008"]
evidence = "test"
test = "crates/bunny-broadphase/tests/uniform_grid_allocation_tests.rs::grid_tp_014_uniform_grid_pairs_allocates_zero_times"
oracle = "Thread-scoped allocation count."
tier = "fast"
status = "implemented"

[[case]]
id = "GRID-TP-015"
requirements = ["GRID-REQ-002", "GRID-REQ-003", "GRID-REQ-005", "GRID-REQ-006", "GRID-REQ-007"]
evidence = "manual-audit"
artifact = "docs/topics/uniform-grid/test-plan.md#mutation-checks"
oracle = "Each deliberate mutation makes at least one case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each mutation below was applied to the kernel on the host (then restored),
`cargo test --locked --no-fail-fast -p bunny-broadphase --test
uniform_grid_tests --test uniform_grid_allocation_tests` was run, and the
cases that failed were recorded.

| Mutation | Cases that fail |
| --- | --- |
| Floor replaced by truncation toward zero (`div_euclid` to `/` in `cell_of`) | GRID-TP-002, 003, 005, 006, 007, 008, 009, 011. The pairs alone would not show it (any monotone quantization finds the same pairs); the cells and keys do. |
| Half-open boxes: the last cell taken from `max` one raw unit lower | GRID-TP-003, 004, 005, 006, 007, 008, 009, 010, 011, 013. |
| A box's span on an axis without the `+ 1` | GRID-TP-003, 004, 005, 006, 007, 008, 009, 010, 011, 012, 013. |
| The first-shared-cell rule removed (a pair is reported in every shared cell) | GRID-TP-006, 008, 009, 010, 011, 012, 013. |
| The exact overlap filter removed | GRID-TP-006, 008, 009, 010, 011, 012, 013. |
| The first shared cell taken as the smaller, not the larger, of the low cells | GRID-TP-005, 006, 007, 008, 009, 010, 011, 012. |
| The key sort removed | GRID-TP-005, 006, 007, 008, 009, 010, 011, 012, 013, 014. |
| A full pair buffer keeps larger pairs instead of smaller ones | GRID-TP-012. |
| The final sort of the written pairs removed | GRID-TP-005, 006, 007, 008, 009, 010, 011, 012. |
| Inverted boxes accepted | GRID-TP-005. |

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic (`i64`, `i128`, `u128`, `u64`,
`usize`) with checked or proven results, in-place sorts of distinct keys and
distinct pairs, and no floating point, maps, hashing, host randomness, time or
parallel scheduling. The seeded cases run in `cargo test --workspace` on
Linux, macOS and Windows in the `determinism-matrix` CI job and under
WebAssembly in `wasm-headless` (except the allocation case, which is not built
for `wasm32`, as the crate's other allocation tests are not), where they must
equal the same oracle and the same pinned pair counts. GRID-TP-004 states its
expected box index for both 32-bit and 64-bit `usize`.

## Gaps

- `TooManyBoxes` (more than `u32::MAX` boxes) is not exercised: it would need
  more than four billion boxes. On a 32-bit target it cannot occur.
- No timing is claimed and no performance case exists; `sweep_and_prune` has
  none either.
