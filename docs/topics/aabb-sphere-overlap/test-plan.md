# AABB-Sphere Overlap Test Plan

This document defines how Bunny proves [the AABB-sphere overlap query](README.md):
`aabb_intersects_sphere` in `bunny-query`. It answers flyingrobots/bunny#501.

## Scope

- `aabb_intersects_sphere` in `crates/bunny-query/src/closest.rs`: the overlap decision and
  the point it returns.

Out of scope: `closest_point_aabb` (unchanged; the returned point is its result), and the
uniform degeneracy policy of flyingrobots/bunny#116, which may later change what an inverted
box or a negative radius means.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| ASO-REQ-001 | `aabb_intersects_sphere(aabb, sphere)` returns `Some(p)`, with `p = closest_point_aabb(aabb, sphere.center)`, exactly when `\|p - c\|^2 <= r^2` on raw integers (`c` the centre, `r` the radius magnitude), touching included, and `None` otherwise, for every representable box, centre and radius. Nothing is rounded or saturated. | `README.md#the-rule` |
| ASO-REQ-002 | A box is read as given (`closest_point_aabb`'s per-axis clamp, so an inverted box is not swapped or empty) and the radius sign is ignored, as before flyingrobots/bunny#501. | `README.md#degenerate-inputs` |
| ASO-REQ-003 | For every input on which the routine before flyingrobots/bunny#501 did not saturate, and whose squared distance and squared radius differ by more than `2^33` raw units squared, the answer is unchanged. | `README.md#what-changed-in-flyingrobotsbunny501` |

```toml
[[requirement]]
id = "ASO-REQ-001"
summary = "Some(closest point) exactly when |p - c|^2 <= r^2 on raw integers, touching included, for every representable input; nothing rounded or saturated."
status = "active"

[[requirement]]
id = "ASO-REQ-002"
summary = "The box is read as given and the radius sign is ignored, as before #501."
status = "active"

[[requirement]]
id = "ASO-REQ-003"
summary = "Unchanged wherever the old routine did not saturate and the squares differ by more than 2^33 raw units squared."
status = "active"
```

## Oracles

`crates/bunny-query/tests/aabb_sphere_exact_tests.rs` writes its own exact reference and shares
no helper with the kernel. `oracle` clamps the centre to the box per axis, takes each gap as a
`u64` from an `i128` difference, and holds every square and sum as eight base-`2^32` limbs
(schoolbook products and carry propagation), compared limb by limb from the top. The kernel
uses native `u64` by `u64` products in `u128` and a checked sum.

`reference_before_501` is a copy of the routine as it stood before the fix (Q32.32 products,
each rounded to nearest with ties to even and saturated, and saturating sums).
`reference_saturates` recomputes that routine's steps in `i128` and `u128` and reports whether
any of them left the `i64` raw range.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| ASO-TP-001 | ASO-REQ-001 | The example of flyingrobots/bunny#501: radius `65 536` units at `(MAX, MAX, MAX)` raw against the point box at `(MIN, MIN, MIN)` is `None`; the old routine saturated and returned `Some`. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_001_issue_example_is_not_an_intersection` |
| ASO-TP-002 | ASO-REQ-001, ASO-REQ-003 | The defect's boundary: the rounded square of radius `199 032 864 766 430` raw fits `i64` and that of `199 032 864 766 431` does not; with either sign, the old routine is `None` for the far box below it and `Some` from it; the fix is `None` for both, and at that radius meets a box at distance `r` and `r - 1` but not `r + 1`. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_002_defect_boundary_is_the_saturating_radius` |
| ASO-TP-003 | ASO-REQ-001, ASO-REQ-002 | Touching at ordinary magnitudes: `(3k, -4k, 12k)` against radius `13k` for five scales is met, radius `13k - 1` and a centre one raw unit further are not, and `-13k` is met; a face contact one raw unit either side; a centre inside the box with radius 0; a zero radius on and one raw unit off the box. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_003_touching_ordinary_magnitudes` |
| ASO-TP-004 | ASO-REQ-001 | Touching at the ends of the raw range: radius `i64::MIN` (magnitude `2^63`) at gap `2^63` met and at `2^63 + 1` not; `(3k, 4k, 12k)` with `k = MAX / 13` and one raw unit either side; gaps of `2^64 - 1` on one, two and three axes and of `2^63` on two axes never met. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_004_touching_extreme_magnitudes` |
| ASO-TP-005 | ASO-REQ-001, ASO-REQ-002 | Every corner of the raw range: the eight corner point boxes, the full box, the inverted full box and a mixed box, against 343 centres from `MIN`, `MIN + 1`, `-1`, `0`, `1`, `MAX - 1`, `MAX` per axis and eight radii (30 184 cases) equal `oracle`. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_005_raw_range_corners_match_the_oracle` |
| ASO-TP-006 | ASO-REQ-001, ASO-REQ-002 | 200 000 seeded cases (seed `0x0501_A55E_0000_0003`, printed) over the whole raw range, an eighth of the draws from the special values, equal `oracle`; both answers occur. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_006_seeded_full_range_matches_the_oracle` |
| ASO-TP-007 | ASO-REQ-002, ASO-REQ-003 | 240 000 seeded cases (seed `0x0501_A55E_0000_0001`, printed) within `2^40` raw, half the boxes drawn unordered, radii of both signs: none saturates the old routine; every case equals `oracle`, and every case outside the band equals `reference_before_501`; at least 200 000 compared, 50 000 with an inverted axis, 50 000 with a negative radius and 10 000 met. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_007_ordinary_magnitudes_unchanged` |
| ASO-TP-008 | ASO-REQ-001, ASO-REQ-003 | The rounding band: a zero radius one raw unit from a point box (old `Some`, now `None`); three gaps of `1 049 600` raw against radius `1 817 961` raw (old `None`, now `Some`); 200 000 seeded cases within `2^20` raw (seed `0x0501_A55E_0000_0002`, printed) equal `oracle`, and every answer that changed lies within `2^33` raw units squared of touching. | `crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_008_rounding_band_is_exact_now` |
| ASO-TP-009 | ASO-REQ-001, ASO-REQ-002 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "ASO-TP-001"
requirements = ["ASO-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_001_issue_example_is_not_an_intersection"
oracle = "The issue's example is None; the old routine saturated and returned Some."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-002"
requirements = ["ASO-REQ-001", "ASO-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_002_defect_boundary_is_the_saturating_radius"
oracle = "Old routine None below radius 199_032_864_766_431 raw and Some from it; the fix exact on both sides."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-003"
requirements = ["ASO-REQ-001", "ASO-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_003_touching_ordinary_magnitudes"
oracle = "Hand-derived touching cases (3-4-12-13) and one raw unit either side, checked against the limb oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-004"
requirements = ["ASO-REQ-001"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_004_touching_extreme_magnitudes"
oracle = "Hand-derived touching cases at gaps up to 2^64 - 1 and radius magnitude 2^63, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-005"
requirements = ["ASO-REQ-001", "ASO-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_005_raw_range_corners_match_the_oracle"
oracle = "Independent base-2^32 limb oracle over 30 184 corner cases."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-006"
requirements = ["ASO-REQ-001", "ASO-REQ-002"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_006_seeded_full_range_matches_the_oracle"
oracle = "Independent base-2^32 limb oracle over 200 000 seeded full-range cases."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-007"
requirements = ["ASO-REQ-002", "ASO-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_007_ordinary_magnitudes_unchanged"
oracle = "Copy of the routine before #501 over 240 000 seeded ordinary cases outside the rounding band, and the limb oracle on all."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-008"
requirements = ["ASO-REQ-001", "ASO-REQ-003"]
evidence = "test"
test = "crates/bunny-query/tests/aabb_sphere_exact_tests.rs::aso_tp_008_rounding_band_is_exact_now"
oracle = "Limb oracle on 200 000 seeded small cases; every changed answer within 2^33 raw units squared of touching."
tier = "fast"
status = "implemented"

[[case]]
id = "ASO-TP-009"
requirements = ["ASO-REQ-001", "ASO-REQ-002"]
evidence = "manual-audit"
artifact = "docs/topics/aabb-sphere-overlap/test-plan.md#mutation-checks"
oracle = "Each rule of the kernel broken on purpose fails at least one case above."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-query/src/closest.rs`,
one at a time, and `cargo test --locked -p bunny-query --test aabb_sphere_exact_tests --test
closest_raw_tests --test query_tests` was run; the source was restored after each. The
unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| `<=` replaced by `<` | Touching meets | 002, 003, 004, 005, 006 |
| The sum of squares wraps (`wrapping_add`) | A sum past `u128` is `None` | 005, 006 |
| The `z` term dropped | Three axes | 003, 004, 005, 006, 007, 008 |
| A negative radius returns `None` | The radius sign is ignored | 003, 004, 005, 006, 007, 008 |
| Both squares truncated to Q32.32 (`>> 32`) before comparing | No rounding | 003, 005, 008 |
| Each gap capped at `i64::MAX` | Gaps up to `2^64 - 1` | 004, 005, 006 |
| The radius magnitude capped at `i64::MAX` | Radius magnitude up to `2^63` | 004, 005 |
| The centre returned instead of the closest point | `Some(closest_point_aabb(..))` | 002 to 008 |
| The routine before flyingrobots/bunny#501 restored | Exact for every input | 001 to 006, 008 (the first push of this branch) |
