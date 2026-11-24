# Closest Points Between Segments Test Plan

This document defines how Bunny proves [the closest-point chapter](README.md) for
`bunny_query::closest_points_segments`. It answers flyingrobots/bunny#506.

The IDs start at CLOSEST-REQ-010 and CLOSEST-TP-020. The lower numbers are kept free for the
triangle cases of draft flyingrobots/bunny#504, which uses CLOSEST-REQ-001 to -004 and
CLOSEST-TP-001 to -009 in the same chapter.

## Scope

- `closest_points_segments` in `bunny-query`, for segments of zero and nonzero length,
  parallel, collinear, crossing, touching and skew

Out of scope: an error bound for non-parallel segments of nonzero length, which is not
stated, and the other closest-point helpers of `bunny-query`.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| CLOSEST-REQ-010 | For two segments of nonzero length whose raw directions are not exactly parallel, the result is bit-identical to the routine as it was before flyingrobots/bunny#506. | `README.md#segments-of-nonzero-length` |
| CLOSEST-REQ-011 | A zero-length segment is a point. With `p2 == q2` the result is `(c, p2)`, where `(p2, c)` is the result of the call with the point first, `closest_points_segments(p2, q2, p1, q1)`. With `p1 == q1` the result is `p1` and its closest point on the second segment. With both zero length the result is `(p1, p2)` exactly. | `README.md#zero-length-segments` |
| CLOSEST-REQ-012 | When one segment has zero length and every coordinate is below `2^45` raw, the point is returned exactly and each component `k` of the other point is within `1 + min(\|d_k\|, ceil(\|d_k\| / 2^33) + ceil(2^35 \|d_k\| / \|d\|^2))` raw units of the exact closest point, where `d` is the raw direction of the segment of nonzero length. | `README.md#accuracy` |
| CLOSEST-REQ-013 | Two segments of nonzero length whose raw directions are exactly parallel (collinear included) take the parallel rule: `p1` and its projection when `p1` projects onto the second segment, otherwise the nearer end of the second segment and its projection onto the first. Below `2^45` raw each component of each point is within the larger of the two segments' bounds of an exact closest pair. Pairs whose rounded `a e - b^2` was zero keep their previous answer. | `README.md#parallel-and-collinear-segments` |
| CLOSEST-REQ-014 | At the ends of the raw range the saturated answer of a zero-length segment is pinned (flyingrobots/bunny#116, decision D5). | `README.md#the-ends-of-the-raw-range` |

```toml
[[requirement]]
id = "CLOSEST-REQ-010"
summary = "Non-degenerate, non-parallel segment pairs give bit-identical results to the routine before flyingrobots/bunny#506."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-011"
summary = "A zero-length segment is a point: a zero-length second segment is answered as the mirror of the call with the point first; two points give themselves."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-012"
summary = "Below 2^45 raw, with one zero-length segment, the point is exact and the other point is within the stated bound of the exact closest point."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-013"
summary = "Exactly parallel segments take the parallel rule and are within the larger segment bound of an exact closest pair; pairs with a rounded zero denominator are unchanged."
status = "active"

[[requirement]]
id = "CLOSEST-REQ-014"
summary = "Saturated answers of a zero-length segment at the ends of the raw range are pinned, pending decision D5 of flyingrobots/bunny#116."
status = "active"
```

## Oracles

`crates/bunny-query/tests/closest_segments_degenerate_tests.rs` has its own exact oracle and
shares no code with the kernel. It works in a signed integer of 1 024 bits written in the
test file. It minimizes the exact squared distance `|r + d1 s - d2 t|^2` over `[0, 1]^2`:

- the interior critical point when `a e - b^2 > 0` and it lies in the square, which is then
  the unique minimum;
- otherwise the clamped projections on the four edges `s = 0`, `s = 1`, `t = 0` and `t = 1`,
  and the least exact squared distance among them.

For parallel segments the closest pair is not unique, so the oracle also projects the
output points back onto the segments to find the exact closest pair nearest the output.
An output is accepted when some exact closest pair is within the bound in every component.
Every coordinate the oracle is given is below `2^40` raw.

The arrangement of a seeded pair is decided by the test's own exact checks, not by the
kernel: a zero-length segment by raw equality, parallel directions by the exact cross product
in wide integers, collinearity by a second cross product.

`reference_closest_points_segments` in the same file is the routine as it was before
flyingrobots/bunny#506, with its helpers inlined. It equals the kernel on every case of
CLOSEST-TP-024 and CLOSEST-TP-025 on the unfixed kernel.

The seeds are `0xB050_6C10_5E57_0001` (the arrangement sweep) and `0xB050_6C10_5E57_0002`
(the differential). Both are printed, and every failure message names the seed and the case.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| CLOSEST-TP-020 | CLOSEST-REQ-011, CLOSEST-REQ-012 | The example of flyingrobots/bunny#506: `(0,0,0)-(2,0,0)` and the point `(1,1,0)` give `(1,0,0)`. The previous routine gave `(0,0,0)`. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_020_the_issue_example_returns_the_closest_point_of_the_first_segment` |
| CLOSEST-TP-021 | CLOSEST-REQ-011, CLOSEST-REQ-012 | A zero-length second segment against both directions of a segment: points that project inside, onto each end, beyond each end and one raw unit either side of each end. Hand-derived exact answers, the oracle bound, and equality with the mirrored call. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_021_a_zero_length_second_segment_gives_the_closest_point_of_the_first` |
| CLOSEST-TP-022 | CLOSEST-REQ-010, CLOSEST-REQ-011 | A zero-length first segment gives the same hand-derived answers, unchanged from the previous routine; two points give themselves, including at `i64::MIN` and `i64::MAX`. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_022_a_zero_length_first_segment_and_two_points` |
| CLOSEST-TP-023 | CLOSEST-REQ-012 | Segments one raw unit long on each axis against a point, in both argument orders, within the oracle bound. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_023_one_raw_unit_segments_against_a_point` |
| CLOSEST-TP-024 | CLOSEST-REQ-010 | 100 000 seeded pairs of non-degenerate, non-parallel segments equal the previous routine bit for bit. Five families of 20 000: coordinates below `2^40` raw, the full raw range, whole units on a small grid, short segments, and one raw unit off every arrangement of CLOSEST-TP-025. Other draws are skipped and counted. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_024_non_degenerate_non_parallel_pairs_are_bit_identical` |
| CLOSEST-TP-025 | CLOSEST-REQ-011, CLOSEST-REQ-012, CLOSEST-REQ-013 | 36 000 seeded pairs: nine arrangements (second zero length, first zero length, both, parallel, collinear overlapping, collinear disjoint, crossing, a shared end, skew) by four direction scales by 1 000. Each arrangement with a stated bound is within it against the oracle. An answer changes only for a zero-length second segment or a parallel pair with a nonzero rounded denominator. The counts are printed. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_025_seeded_arrangements_against_the_exact_oracle` |
| CLOSEST-TP-026 | CLOSEST-REQ-013 | Hand-derived parallel and collinear cases (inside, containing, disjoint, offset, both directions) with a zero rounded denominator are unchanged; an exactly parallel pair with a rounded denominator of 10 raw, about 0.46 units off before, is within the bound. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_026_parallel_segments_take_the_parallel_rule` |
| CLOSEST-TP-027 | CLOSEST-REQ-014 | Hand-derived saturated answers for a point against the segment spanning the whole x range, in both argument orders; a short segment at the corner of the range is exact. | `crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_027_the_ends_of_the_raw_range_are_pinned` |
| CLOSEST-TP-028 | CLOSEST-REQ-010, CLOSEST-REQ-011, CLOSEST-REQ-013 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "CLOSEST-TP-020"
requirements = ["CLOSEST-REQ-011", "CLOSEST-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_020_the_issue_example_returns_the_closest_point_of_the_first_segment"
oracle = "Hand-derived raw answer, also checked against the exact oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-021"
requirements = ["CLOSEST-REQ-011", "CLOSEST-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_021_a_zero_length_second_segment_gives_the_closest_point_of_the_first"
oracle = "Hand-derived raw answers, the exact oracle bound and the mirrored call."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-022"
requirements = ["CLOSEST-REQ-010", "CLOSEST-REQ-011"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_022_a_zero_length_first_segment_and_two_points"
oracle = "Hand-derived raw answers, the exact oracle bound and the frozen previous routine."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-023"
requirements = ["CLOSEST-REQ-012"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_023_one_raw_unit_segments_against_a_point"
oracle = "The exact oracle bound."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-024"
requirements = ["CLOSEST-REQ-010"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_024_non_degenerate_non_parallel_pairs_are_bit_identical"
oracle = "The frozen previous routine, bit for bit, over 100 000 seeded pairs, seed printed."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-025"
requirements = ["CLOSEST-REQ-011", "CLOSEST-REQ-012", "CLOSEST-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_025_seeded_arrangements_against_the_exact_oracle"
oracle = "Exact big-integer oracle over 36 000 seeded pairs in nine arrangements, seed printed."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-026"
requirements = ["CLOSEST-REQ-013"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_026_parallel_segments_take_the_parallel_rule"
oracle = "Hand-derived raw answers, the frozen previous routine and the exact oracle bound."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-027"
requirements = ["CLOSEST-REQ-014"]
evidence = "test"
test = "crates/bunny-query/tests/closest_segments_degenerate_tests.rs::closest_tp_027_the_ends_of_the_raw_range_are_pinned"
oracle = "Hand-derived saturated raw answers."
tier = "fast"
status = "implemented"

[[case]]
id = "CLOSEST-TP-028"
requirements = ["CLOSEST-REQ-010", "CLOSEST-REQ-011", "CLOSEST-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/closest-point/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

The existing goldens of `closest_points_segments` are not changed:
`crates/bunny-query/tests/closest_raw_tests.rs::closest_point_raw_q32_32_outputs`,
`crates/bunny-query/tests/query_tests.rs::test_closest_points_segments` and
`crates/bunny-query/tests/property_tests.rs::prop_closest_segment_query_is_symmetric_for_bounded_values`.

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-query/src/closest.rs`.
Then `cargo test --locked --no-fail-fast -p bunny-query --test
closest_segments_degenerate_tests --test closest_raw_tests --test query_tests --test
property_tests` was run, and the source was restored after each. The unmutated source passed
every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| The zero-length branch removed (the routine before the fix) | CLOSEST-REQ-011 | 020, 021, 025, 027 |
| The mirrored pair not swapped back | CLOSEST-REQ-011 | 020, 021, 022, 023, 025, 027 |
| The first segment tested for zero length instead of the second | CLOSEST-REQ-011 | 020, 021, 022, 025, 027 |
| Zero length decided by the rounded `d2 . d2 == 0`, not raw equality | CLOSEST-REQ-010 | 024 |
| The exact parallel test removed (only the rounded `a e - b^2 == 0`) | CLOSEST-REQ-013 | 025, 026 |
| Every pair treated as parallel | CLOSEST-REQ-010 | 024, 025, `closest_point_raw_q32_32_outputs`, `test_closest_points_segments`, `prop_closest_segment_query_is_symmetric_for_bounded_values` |
| The sign of each exact product ignored | CLOSEST-REQ-010, CLOSEST-REQ-013 | 024, 025 |
| One component of the exact cross product dropped | CLOSEST-REQ-010 | 024, 025, `closest_point_raw_q32_32_outputs`, `test_closest_points_segments` |

One mutation survives: taking the parallel test on the saturated differences `q - p` instead
of the exact raw differences. A difference saturates only when a component exceeds `2^63`
raw. Then `a` or `e` saturates too, and on every pair tried the two rules gave the same
bits. CLOSEST-TP-026 pins two such pairs at the ends of the raw range, one exactly parallel
and one not, but neither tells the two tests apart.
