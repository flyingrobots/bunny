# Closest Point On A Triangle Test Plan

This document defines how Bunny proves [the closest-point chapter](README.md) for
`bunny_query::closest_point_triangle`. It answers flyingrobots/bunny#500.

## Scope

- `closest_point_triangle` in `bunny-query`, for degenerate and non-degenerate triangles

Out of scope: `closest_point_aabb`, `aabb_intersects_sphere` and `closest_points_segments`
on their own, and an error bound for non-degenerate triangles, which is not stated.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| CLOSEST-REQ-001 | For a non-degenerate triangle (exact cross product of the raw edges nonzero), the result is bit-identical to the region-test routine as it was before flyingrobots/bunny#500. | `README.md#non-degenerate-triangles` |
| CLOSEST-REQ-002 | For a degenerate triangle, the result is `closest_points_segments(p, p, a, b).1`, where `(a, b)` are the least and greatest vertices, in vertex order, on the first axis where the vertices are not all equal, the lower index on a tie. Three equal vertices give `v0` exactly, over the whole raw range. | `README.md#degenerate-triangles` |
| CLOSEST-REQ-003 | For a degenerate triangle with every coordinate below `2^45` raw, each component is within `1 + min(\|d_k\|, ceil(\|d_k\| / 2^33) + ceil(2^35 \|d_k\| / \|d\|^2))` raw units of the exact closest point of the segment between the two farthest vertices, where `d = b - a`. | `README.md#accuracy-of-the-degenerate-answer` |
| CLOSEST-REQ-004 | For a degenerate triangle at the ends of the raw range, the saturated answer is pinned (flyingrobots/bunny#116, decision D5). | `README.md#the-ends-of-the-raw-range` |

```toml
[[requirement]]
id = "CLOSEST-REQ-001"
summary = "Non-degenerate triangles give bit-identical results to the routine before flyingrobots/bunny#500."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-002"
summary = "Degenerate triangles give closest_points_segments(p, p, a, b).1 for the hull ends chosen by the axis rule; three equal vertices give v0 exactly."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-003"
summary = "Below 2^45 raw, each component of a degenerate answer is within the stated bound of the exact closest point of the hull segment."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-004"
summary = "Degenerate answers at the ends of the raw range saturate and are pinned, pending decision D5 of flyingrobots/bunny#116."
status = "active"
```

## Oracles

`crates/bunny-query/tests/closest_triangle_degenerate_tests.rs` has its own exact oracle and
shares no code with the kernel. It finds the hull segment as the pair of vertices at the
greatest exact squared distance, the first such pair in the order `(0, 1)`, `(0, 2)`,
`(1, 2)`. The kernel instead takes the extremes on one axis. The oracle projects `p` as an
`i128` rational, clamped to the segment. Every coordinate it is given is below `2^40` raw,
so every product it forms is below `2^126`. Errors are compared as exact rationals: the
numerator `|r_k q - n_k|` against the bound times the denominator `q`.

The degeneracy of a seeded triangle is decided by the test's own exact cross product, with
magnitudes in `u128`, not by `FixedTriangle3::is_degenerate`.

`reference_closest_point_triangle` in the same file is the routine as it was before
flyingrobots/bunny#500, with its helpers inlined. Before the fix, it was checked to equal
the kernel on every case of CLOSEST-TP-005 and CLOSEST-TP-007, degenerate cases included.

The seeds are `0xB050_0C10_5E57_0001` (degenerate sweep) and `0xB050_0C10_5E57_0002`
(differential). Both are printed, and every failure message names the seed and the case.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| CLOSEST-TP-001 | CLOSEST-REQ-002, CLOSEST-REQ-003 | The example of flyingrobots/bunny#500: `(0,0,0), (0,0,0), (2,0,0)` and `(1,1,0)` give `(1,0,0)`. The previous routine gave `(0,0,0)`. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_001_the_issue_example_returns_the_closest_point_of_the_segment` |
| CLOSEST-TP-002 | CLOSEST-REQ-002, CLOSEST-REQ-003 | `v0 = v1`, `v1 = v2` and `v0 = v2`, with either end repeated. The points project inside the segment, onto each end, beyond each end, and one raw unit either side of each end. There are hand-derived exact answers where representable, and the oracle bound everywhere. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_002_repeated_vertices_give_the_closest_point_of_the_segment` |
| CLOSEST-TP-003 | CLOSEST-REQ-002 | Three equal vertices give that vertex exactly for 25 vertex and point pairs, including `i64::MIN` and `i64::MAX` in every component. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_003_three_equal_vertices_are_the_point` |
| CLOSEST-TP-004 | CLOSEST-REQ-002, CLOSEST-REQ-003 | Collinear distinct vertices with the middle vertex at each index, on lines along x, along y (x constant), along z and on a diagonal. The points are closest to the interior of the longest edge, to the middle vertex, to each end and beyond each end. The exact answers are hand-derived. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_004_collinear_vertices_give_the_closest_point_of_the_longest_edge` |
| CLOSEST-TP-005 | CLOSEST-REQ-002, CLOSEST-REQ-003 | 168 000 seeded degenerate triangles: 7 arrangements by 4 direction scales by 6 000 (large, raw-unit, whole-unit, along one or two axes). Each is within the bound of the oracle, and each equals `closest_points_segments` on the oracle's farthest pair in vertex order. The previous routine is outside the bound on more than 1 000 `v0 = v1` cases and more than 100 cases of each collinear arrangement, and unchanged on three equal vertices. The counts are printed. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_005_seeded_degenerate_triangles_match_the_oracle` |
| CLOSEST-TP-006 | CLOSEST-REQ-001 | The non-degenerate neighbours of the constructed degenerate cases, one vertex moved one raw unit off the line (300 cases), equal the previous routine bit for bit. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_006_neighbours_one_raw_unit_away_are_unchanged` |
| CLOSEST-TP-007 | CLOSEST-REQ-001 | 100 000 seeded non-degenerate triangles equal the previous routine bit for bit. There are four families of 25 000: coordinates below `2^40` raw, the full raw range, whole units with half-unit points, and one raw unit off a degenerate triangle. Degenerate draws are skipped and counted. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_007_non_degenerate_triangles_are_bit_identical_to_the_previous_routine` |
| CLOSEST-TP-008 | CLOSEST-REQ-004 | Hand-derived saturated answers for a segment spanning the whole x range (`-1` raw for two points whose exact answers are `0` and `MIN / 2`). A short segment at the corner of the range is exact. | `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_008_ends_of_the_raw_range_are_pinned` |
| CLOSEST-TP-009 | CLOSEST-REQ-001, CLOSEST-REQ-002, CLOSEST-REQ-003 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "CLOSEST-TP-001"
requirements = ["CLOSEST-REQ-002", "CLOSEST-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_001_the_issue_example_returns_the_closest_point_of_the_segment"
oracle = "Hand-derived raw answer, also checked against the exact i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-002"
requirements = ["CLOSEST-REQ-002", "CLOSEST-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_002_repeated_vertices_give_the_closest_point_of_the_segment"
oracle = "Hand-derived raw answers and the exact i128 oracle bound."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-003"
requirements = ["CLOSEST-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_003_three_equal_vertices_are_the_point"
oracle = "The vertex itself."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-004"
requirements = ["CLOSEST-REQ-002", "CLOSEST-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_004_collinear_vertices_give_the_closest_point_of_the_longest_edge"
oracle = "Hand-derived raw answers, also checked against the exact i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-005"
requirements = ["CLOSEST-REQ-002", "CLOSEST-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_005_seeded_degenerate_triangles_match_the_oracle"
oracle = "Exact i128 oracle over 168 000 seeded degenerate triangles, seed printed."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-006"
requirements = ["CLOSEST-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_006_neighbours_one_raw_unit_away_are_unchanged"
oracle = "The frozen previous routine, bit for bit."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-007"
requirements = ["CLOSEST-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_007_non_degenerate_triangles_are_bit_identical_to_the_previous_routine"
oracle = "The frozen previous routine, bit for bit, over 100 000 seeded triangles, seed printed."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-008"
requirements = ["CLOSEST-REQ-004"]
evidence = "test"
test = "crates/bunny-query/tests/closest_triangle_degenerate_tests.rs::closest_tp_008_ends_of_the_raw_range_are_pinned"
oracle = "Hand-derived saturated raw answers."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-009"
requirements = ["CLOSEST-REQ-001", "CLOSEST-REQ-002", "CLOSEST-REQ-003"]
evidence = "manual-audit"
artifact = "docs/topics/closest-point/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

The existing goldens of `closest_point_triangle` are not changed:
`crates/bunny-query/tests/closest_raw_tests.rs::closest_point_raw_q32_32_outputs` and
`crates/bunny-query/tests/query_tests.rs::test_closest_point_triangle`.

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-query/src/closest.rs`.
Then `cargo test --locked --no-fail-fast -p bunny-query --test
closest_triangle_degenerate_tests --test closest_raw_tests --test query_tests` was run, and
the source was restored after each. The unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| The degenerate branch removed (the routine before the fix) | CLOSEST-REQ-002 | 001, 002, 004, 005, 008 |
| The ends always `(v0, v2)` | The farthest pair | 002, 004, 005 |
| The axis always x, even when x is constant | The first non-constant axis | 004 (the line along y), 005 |
| The ends taken in reverse vertex order | Vertex order | 005 (the delegation check) |
| A tie on the axis goes to the later index | The lower index on a tie | 005 (`v0 = v2`: the ends come out reversed) |
| The greatest vertex compared against the least, not the greatest so far | The extremes | 004, 005 |
| Only repeated vertices treated as degenerate, not collinear ones | Exact degeneracy | 004, 005 |
| The degeneracy test inverted | CLOSEST-REQ-001 | 001, 002, 004 to 008, `closest_point_raw_q32_32_outputs`, `test_closest_point_triangle` (003 passes: the previous routine also returns `v0` for three equal vertices) |
