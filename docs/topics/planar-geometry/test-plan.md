# 2D Primitives And Queries Test Plan

This document defines how Bunny proves [`bunny-planar/1`](README.md): the 2D primitives
`FixedSegment2`, `FixedRay2`, `FixedAabb2`, `FixedCircle2` and `FixedPolygon2` in
`bunny-geom`, their constructors, the exact closed-boundary containment, overlap,
segment-intersection and closest-point queries, the ray queries and point in polygon. It
answers the first two slices of flyingrobots/bunny#149.

## Scope

- the four types, their `new` and `try_new` constructors and the refusals of `try_new`;
- `FixedAabb2::is_ordered`, `contains_point`, `overlaps` and `closest_point`;
- `FixedCircle2::contains_point`, `overlaps_circle` and `overlaps_aabb`;
- `FixedSegment2::contains_point`, `intersects` and `closest_point`;
- `FixedRay2::intersects_aabb`, `intersects_circle` and `intersects_segment`, and
  `hit_aabb`, `hit_circle` and `hit_segment` with their `RayHit2`;
- `FixedPolygon2::new`, `classify_point` with its `PolygonLocation`, and `contains_point`.

Out of scope, remaining for flyingrobots/bunny#149: segment-to-segment closest points,
intersection points, closest points on a circle, circle and box closest points to a
segment, polygon queries other than point in polygon (polygon overlap, area, validation of
simplicity), and float boundary constructors.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| PLANAR-REQ-001 | `FixedAabb2::try_new` refuses `min` above `max` on either axis (raw comparison) with `GeomError::InvalidAabbBounds`; `FixedCircle2::try_new` refuses a radius below raw zero with `GeomError::NegativeSphereRadius`; `FixedRay2::try_new` refuses a direction `FixedUnitVec2::new` refuses with `GeomError::InvalidRayDirection`; flat and point boxes, zero radii and zero-length segments are accepted, and every `new` holds its input unchanged. | `README.md#types-and-constructors` |
| PLANAR-REQ-002 | `FixedAabb2::contains_point` and `overlaps` are exact raw comparisons with the boundary included (touching edges and corners overlap); an inverted box (`is_ordered` false) contains no point and overlaps no box, itself included. | `README.md#boxes` |
| PLANAR-REQ-003 | `FixedAabb2::closest_point` is the per-axis clamp of the point to `[min, max]`, exact, and `None` exactly for an inverted box. | `README.md#boxes` |
| PLANAR-REQ-004 | `FixedCircle2::contains_point` is `\|p - c\|^2 <= r^2` on raw values, exact in 256 bits for every representable input; a zero radius contains only its centre and a negative radius no point. | `README.md#circles` |
| PLANAR-REQ-005 | `overlaps_circle` is `\|c1 - c2\|^2 <= (r1 + r2)^2` and `overlaps_aabb` is `contains_point` of the box's closest point to the centre, both exact and closed; a negative radius or an inverted box gives `false`. | `README.md#circles` |
| PLANAR-REQ-006 | `FixedSegment2::contains_point` is true exactly when `orientation2(start, end, p)` is `Collinear` and `p` is in the endpoints' box, both ends included; a zero-length segment contains only its point. | `README.md#segments` |
| PLANAR-REQ-007 | `FixedSegment2::intersects` is true exactly when the closed segments share a point: a proper crossing, an endpoint on the other segment, or a collinear overlap of at least one point, decided from exact `orientation2` signs; zero-length segments included. | `README.md#segments` |
| PLANAR-REQ-008 | `FixedSegment2::closest_point` clamps the exact parameter `n / d` (`n = (p - a) . (b - a)`, `d = \|b - a\|^2`) to `[0, 1]`, returns an endpoint exactly when the clamp applies or `d = 0`, and otherwise rounds each exact coordinate `(a d + (b - a) n) / d` once, to nearest with ties to even; each coordinate is within half a raw unit of the exact nearest point and in the endpoints' box, and it is never `None` for representable input. | `README.md#closest-point-on-a-segment` |
| PLANAR-REQ-009 | Every query is independent of argument order: `overlaps`, `overlaps_circle` and `intersects` are symmetric, and reversing a segment's endpoints changes none of `contains_point`, `intersects` or `closest_point`. | `README.md#order-independence` |
| PLANAR-REQ-010 | `FixedRay2::intersects_aabb` is true exactly when some point `o + t d`, `t >= 0`, with `d` the direction's raw value as given, is in the closed box, decided by exact slab intervals with no rounding; grazing an edge or a corner and an origin on or in the box count; an inverted box is never hit. | `README.md#ray-against-a-box` |
| PLANAR-REQ-011 | `FixedRay2::intersects_circle` is true exactly when some point `o + t d`, `t >= 0`, is in the closed disc: the origin is in the disc, or `(c - o) . d > 0` and `\|d\|^2 r^2 - ((c - o) x d)^2 >= 0`, exact in 256 bits; a tangent counts; a negative radius is never hit. | `README.md#ray-against-a-circle` |
| PLANAR-REQ-012 | `FixedRay2::intersects_segment` is true exactly when the ray and the closed segment share a point, from the exact solve `t = ((c - o) x s) / (d x s)`, `u = ((c - o) x d) / (d x s)` when `d x s != 0`; when `d x s = 0` (parallel, or a zero-length segment) the ray misses unless `(c - o) x d = 0`, and a collinear segment is hit exactly when its farther end along `d` is at `t >= 0`. Endpoint contacts count; endpoint order does not matter. | `README.md#ray-against-a-segment` |
| PLANAR-REQ-013 | `hit_aabb`, `hit_circle` and `hit_segment` return `RayHit2::Miss` exactly when the matching `intersects_` is false; otherwise the least exact parameter `t >= 0` of a shared point, rounded once to nearest raw unit with ties to even (`At`, within half a raw unit), or `Beyond` exactly when that rounded value is above `i64::MAX`. The parameter is zero when the origin is in the shape. | `README.md#first-hit-parameter` |
| PLANAR-REQ-014 | `FixedPolygon2::classify_point` returns `Boundary` exactly when the point is on an edge `v[i]`-`v[(i + 1) mod n]` (`FixedSegment2::contains_point`), and otherwise `Inside` exactly when the winding number from exact `orientation2` signs is nonzero, else `Outside`, for every representable input and either winding; `contains_point` is `classify_point != Outside`. | `README.md#point-in-polygon` |
| PLANAR-REQ-015 | Degenerate and unvalidated polygons follow the same rule: no vertices is empty, one vertex a point, two vertices a segment, repeated consecutive vertices and collinear runs change nothing, a zero-area polygon has no inside, and self-intersecting input is not detected and is classified by the nonzero winding number. The result does not depend on the starting vertex or the winding. | `README.md#degenerate-and-self-intersecting-polygons` |

```toml
[[requirement]]
id = "PLANAR-REQ-001"
summary = "try_new refuses an inverted box, a negative radius and an unnormalizable ray direction with the matching GeomError; degenerate values are accepted and new holds its input."
status = "active"

[[requirement]]
id = "PLANAR-REQ-002"
summary = "Box containment and overlap are exact and closed; an inverted box contains and overlaps nothing."
status = "active"

[[requirement]]
id = "PLANAR-REQ-003"
summary = "The box closest point is the exact per-axis clamp, None exactly for an inverted box."
status = "active"

[[requirement]]
id = "PLANAR-REQ-004"
summary = "Circle containment is the exact closed |p - c|^2 <= r^2 for every input; a negative radius contains nothing."
status = "active"

[[requirement]]
id = "PLANAR-REQ-005"
summary = "Circle-circle and circle-box overlap are exact and closed; a negative radius or inverted box gives false."
status = "active"

[[requirement]]
id = "PLANAR-REQ-006"
summary = "Segment containment is an exact Collinear orientation and the endpoint box, ends included."
status = "active"

[[requirement]]
id = "PLANAR-REQ-007"
summary = "Segment intersection is exact and closed: crossing, endpoint touch or collinear overlap, zero-length segments included."
status = "active"

[[requirement]]
id = "PLANAR-REQ-008"
summary = "The segment closest point clamps the exact parameter and rounds each exact coordinate once, ties to even; within half a raw unit, never None."
status = "active"

[[requirement]]
id = "PLANAR-REQ-009"
summary = "Overlap and intersection are symmetric and segment queries do not depend on endpoint order."
status = "active"

[[requirement]]
id = "PLANAR-REQ-010"
summary = "Ray against box is the exact closed slab test on the direction as given; grazing counts and an inverted box is never hit."
status = "active"

[[requirement]]
id = "PLANAR-REQ-011"
summary = "Ray against circle is exact in 256 bits and closed; a tangent counts and a negative radius is never hit."
status = "active"

[[requirement]]
id = "PLANAR-REQ-012"
summary = "Ray against segment is exact and closed, with the parallel, collinear and zero-length cases specified."
status = "active"

[[requirement]]
id = "PLANAR-REQ-013"
summary = "The first hit parameter is Miss exactly on a miss, otherwise the exact least parameter rounded once, ties to even, or Beyond above i64::MAX."
status = "active"

[[requirement]]
id = "PLANAR-REQ-014"
summary = "Point in polygon is exact and three-way: boundary on an edge, otherwise inside exactly for a nonzero winding number."
status = "active"

[[requirement]]
id = "PLANAR-REQ-015"
summary = "Degenerate and self-intersecting polygons follow the same rule, independent of starting vertex and winding."
status = "active"
```

## Oracles

`crates/bunny-geom/tests/planar_oracle/mod.rs` answers every query on the unbounded
integers of `crates/bunny-geom/tests/predicates_oracle/mod.rs` (32-bit digits, schoolbook
arithmetic, no division), by a different formulation from the kernel's:

- segments meet when the parametric solve `a + t (b - a) = c + u (d - c)` has `t` and `u` in
  `[0, 1]` (Cramer's rule on the cross products), or, for parallel segments on one line,
  when their parameter intervals overlap; the kernel uses four orientation signs and endpoint
  boxes;
- a point is on a segment when its cross product with the segment is zero and its parameter
  `(p - a) . (b - a)` is in `[0, |b - a|^2]`; the kernel uses the endpoint box;
- a circle meets a box when its centre is in the box grown by the radius along `x` or along
  `y`, or within the radius of one of the four corners (the Minkowski sum of the box and the
  disc); the kernel clamps to the box;
- two boxes overlap when the larger minimum is at most the smaller maximum on each axis; a
  point is in a box when the box meets the point's box;
- the nearest box point is chosen per axis among `min`, `max` and the coordinate itself;
- the nearest segment point is the exact numerator over the exact denominator, and a rounded
  coordinate `r` is checked by multiplying back: `2 |n - r d| < d`, or `= d` with `r` even.

`crates/bunny-geom/tests/planar_cast_oracle/mod.rs` answers the ray queries and point in
polygon on the same unbounded integers, again by different formulations:

- a ray meets a segment where the segment crosses the ray's line, found by interpolating
  between the endpoints' signed distances from that line, and the parameter is the crossing
  point's projection on the direction; the kernel solves with Cramer's rule;
- a ray meets a box when the origin is in it or the ray meets one of its four edges, the
  first parameter being the least over the edges; the kernel intersects slabs;
- a ray meets a disc when the ray's nearest point to the centre is in it,
  `|a w - b d|^2 <= a^2 r^2`; a rounded first parameter `T` is checked with no square root,
  by the sign of the quadratic `|o + y d - c|^2 - r^2` at `y = (T -+ 1/2) 2^-32` and the
  side of its vertex `y <= b / a`; the kernel takes an integer square root;
- a point's winding number counts the edges crossing the horizontal half-line to its right,
  at their exact rational crossing abscissa, up minus down; the kernel reads orientation
  signs. A separate even-odd count confirms the pentagram case.

The constructed cases are raw values inline in the tests, each with its derivation, and each
is also checked against the oracle. A separate Python model of the closest-point goldens
(Python's unbounded integers and fractions) is posted on the pull request, not kept in the
tree.

Every test is a `wasm_bindgen_test`, so the WebAssembly job runs them as well as the native
run.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| PLANAR-TP-001 | PLANAR-REQ-001 | Constructed: boxes inverted by one raw unit on each axis refused, flat, point and full-range boxes accepted, radii `-1` and `MIN` refused and `0` and `MAX` accepted, a zero direction refused and a direction normalized as `FixedUnitVec2::new` does, `new` unchanged, and the identity string. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_001_constructors_refuse_only_their_stated_inputs` |
| PLANAR-TP-002 | PLANAR-REQ-002, PLANAR-REQ-009 | Constructed: points on each edge and corner and one raw unit outside, flat and point boxes, touching edges and corners and one raw unit apart, at the ends of the raw range, and inverted boxes straddling a point or box; both argument orders and the oracle. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_002_box_containment_and_overlap_are_closed` |
| PLANAR-TP-003 | PLANAR-REQ-003 | Constructed: inside, on the boundary, past each side and corner, from the ends of the raw range, a point box, a flat box, and inverted boxes refused; the oracle. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_003_box_closest_point_clamps_exactly` |
| PLANAR-TP-004 | PLANAR-REQ-004 | Constructed: `3^2 + 4^2 = 5^2` on the circle and one raw unit out, a moved centre, zero and negative radii, distances `2^63 - 1` and `2^63` from the ends of the range, and a distance squared of `2^128 + 581896769` that a 128-bit sum wraps below `r^2`. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_004_circle_containment_is_closed_and_exact` |
| PLANAR-TP-005 | PLANAR-REQ-005, PLANAR-REQ-009 | Constructed: circles whose radii sum to their centre distance and one raw unit less, zero and negative radii, centres `2^64 - 1` apart with radii summing to `2^64 - 2` and then one raw unit closer; circles touching a box's corner and edge and one raw unit short, inside, enclosing, and empty inputs; both argument orders and the oracle. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_005_circle_overlaps_are_closed_and_exact` |
| PLANAR-TP-006 | PLANAR-REQ-006, PLANAR-REQ-009 | Constructed: both ends, the interior, the line past each end, one raw unit off the line, axis-aligned segments, a zero-length segment, and the diagonal and anti-diagonal of the raw range; both endpoint orders and the oracle. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_006_segment_containment_is_closed_and_exact` |
| PLANAR-TP-007 | PLANAR-REQ-007, PLANAR-REQ-009 | Constructed: a crossing, an endpoint on the interior and one raw unit short, shared endpoints, collinear overlap, touch, gap and nesting, parallel segments one raw unit apart, lines crossing outside a segment, zero-length segments, and the diagonals of the raw range crossing at `(-1/2, -1/2)` on and past an end; every order and the oracle. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_007_segment_intersection_is_closed_and_exact` |
| PLANAR-TP-008 | PLANAR-REQ-008, PLANAR-REQ-009 | Hand-derived points, each checked against the Python model and the oracle: interior feet, parameters at and past each end, ties of half a raw unit going to even on both axes, the same tie from an odd start (where rounding the offset would differ), zero-length segments, the diagonal of the raw range with ties at `-1/2`, `1/2` and `3/2`, and an offset of `2^64 - 2` raw units that does not fit `i64`; both endpoint orders. | `crates/bunny-geom/tests/planar_tests.rs::planar_tp_008_segment_closest_point_rounds_once_to_even` |
| PLANAR-TP-009 | PLANAR-REQ-002, PLANAR-REQ-003, PLANAR-REQ-004, PLANAR-REQ-005, PLANAR-REQ-009 | 40 000 seeded cases (seed `0x0149_0001_0000_0009`, printed): boxes a quarter inverted, points on or one raw unit off their boundary, points `(3k, 4k)` from a centre with radius `5k` moved by one raw unit, tangent circles, and circles near a box, at every magnitude; every answer equals the oracle and the outcome counts are pinned. | `crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_009_seeded_box_and_circle_queries_match_the_oracle` |
| PLANAR-TP-010 | PLANAR-REQ-006, PLANAR-REQ-007, PLANAR-REQ-009 | 60 000 seeded quadruples (seed `0x0149_0001_0000_000A`, printed): on one lattice line, the same with one raw unit moved, small values with shared and repeated points, any magnitude and the ends of the range; intersection in every order and containment of each endpoint equal the oracle, and the outcome counts are pinned. | `crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_010_seeded_segment_predicates_match_the_oracle` |
| PLANAR-TP-011 | PLANAR-REQ-008, PLANAR-REQ-009 | 40 000 seeded segment and point pairs from the same generator (seed `0x0149_0001_0000_000B`, printed): each coordinate is the oracle's exact value rounded once, ties to even, the result is in the endpoints' box and the same for both endpoint orders, and the outcome counts (ties included) are pinned. | `crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_011_seeded_segment_closest_points_match_the_oracle` |
| PLANAR-TP-012 | PLANAR-REQ-001, PLANAR-REQ-002, PLANAR-REQ-003, PLANAR-REQ-004, PLANAR-REQ-005, PLANAR-REQ-006, PLANAR-REQ-007, PLANAR-REQ-008 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |
| PLANAR-TP-013 | PLANAR-REQ-010, PLANAR-REQ-013 | Constructed: boxes crossed, grazed along an edge and one raw unit past it, behind, around and touching the origin, along every axis direction, flat, point and inverted boxes, a corner the shallow direction `(1, 3 2^-32)` passes under by `2^-32` (which rounded slab bounds report as a hit) and one raw unit further, and parameters `2^63 - 1` and `2^63` from the ends of the range; the oracle. | `crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_013_ray_box_hits_are_closed_and_exact` |
| PLANAR-TP-014 | PLANAR-REQ-011, PLANAR-REQ-013 | Constructed: chords and tangents from `3^2 + 4^2 = 5^2` and one raw unit short, circles behind, through and around the origin, zero and negative radii, irrational first roots along the shallow direction `(1, 2^-32)` including a near-tangent, parameters `2^63 - 1` and `2^63` at the ends of the range, and a first root `2^-64` raw units ahead of an origin just outside a circle of radius `2^63 - 1`; the oracle's quadratic check. | `crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_014_ray_circle_hits_are_closed_and_exact` |
| PLANAR-TP-015 | PLANAR-REQ-012, PLANAR-REQ-013 | Constructed: crossings, endpoint contacts and one raw unit short, behind and through the origin, parallel off the line, collinear ahead, around and behind, zero-length segments, ties at `9/2` and `11/2` raw units, a shallow direction parallel one raw unit off, collinear and crossing, parameters `2^63 - 1` and `2^63`, and the anti-diagonal of the raw range; both endpoint orders and the oracle. | `crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_015_ray_segment_hits_are_closed_and_exact` |
| PLANAR-TP-016 | PLANAR-REQ-014, PLANAR-REQ-015 | Constructed: a square, a diamond whose vertices are level with test points, a polygon with a notch and a spike vertex, the whole raw square and its corner triangle on `x + y = -1`; points inside, on edges and vertices and one raw unit outside; every starting vertex, both windings and the oracle. | `crates/bunny-geom/tests/planar_polygon_tests.rs::planar_tp_016_point_in_polygon_is_exact_and_closed` |
| PLANAR-TP-017 | PLANAR-REQ-014, PLANAR-REQ-015 | Constructed: zero, one and two vertices, repeated consecutive vertices, a collinear run, one repeated point, zero-area polygons doubling back, a bow tie, a pentagram whose centre has winding number 2 (inside; the oracle's even-odd count says outside), and a square traced twice; every starting vertex, both windings and the oracle. | `crates/bunny-geom/tests/planar_polygon_tests.rs::planar_tp_017_polygon_degenerate_and_self_intersecting_inputs` |
| PLANAR-TP-018 | PLANAR-REQ-010, PLANAR-REQ-011, PLANAR-REQ-012, PLANAR-REQ-013 | 30 000 seeded rays (seed `0x0149_0002_0000_0012`, printed) with axis, shallow and random directions, each against a box with a corner by a point on the ray, a circle near tangency to the ray's line, and a segment across, along or ending on that line, each moved by a raw unit; every hit decision and parameter equals the oracle, `intersects_` agrees with `hit_`, segment endpoint order does not matter, and the outcome counts are pinned. | `crates/bunny-geom/tests/planar_cast_sweep_tests.rs::planar_tp_018_seeded_ray_queries_match_the_oracle` |
| PLANAR-TP-019 | PLANAR-REQ-014, PLANAR-REQ-015 | 30 000 seeded polygons of zero to eight vertices (seed `0x0149_0002_0000_0013`, printed) on a small lattice, of any magnitude and from the ends of the range, with points on and a raw unit off their vertices and edge midpoints; every classification equals the oracle's winding number, is unchanged by reversal and rotation, and the outcome counts are pinned. | `crates/bunny-geom/tests/planar_cast_sweep_tests.rs::planar_tp_019_seeded_point_in_polygon_matches_the_oracle` |
| PLANAR-TP-020 | PLANAR-REQ-010, PLANAR-REQ-011, PLANAR-REQ-012, PLANAR-REQ-013, PLANAR-REQ-014, PLANAR-REQ-015 | Each rule of the second slice broken on purpose fails at least one case above. | [Mutation checks of the second slice](#mutation-checks-of-the-second-slice) |

```toml
[[case]]
id = "PLANAR-TP-001"
requirements = ["PLANAR-REQ-001"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_001_constructors_refuse_only_their_stated_inputs"
oracle = "Constructed inputs on and one raw unit either side of each refusal."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-002"
requirements = ["PLANAR-REQ-002", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_002_box_containment_and_overlap_are_closed"
oracle = "Constructed raw boxes and points, each also checked against the interval oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-003"
requirements = ["PLANAR-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_003_box_closest_point_clamps_exactly"
oracle = "Constructed raw points, each also checked against the candidate oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-004"
requirements = ["PLANAR-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_004_circle_containment_is_closed_and_exact"
oracle = "Pythagorean triples on and one raw unit off the circle, each also checked against the big-integer oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-005"
requirements = ["PLANAR-REQ-005", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_005_circle_overlaps_are_closed_and_exact"
oracle = "Tangent and one-raw-unit-short constructions, each also checked against the Minkowski-sum oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-006"
requirements = ["PLANAR-REQ-006", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_006_segment_containment_is_closed_and_exact"
oracle = "Constructed raw points, each also checked against the parametric oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-007"
requirements = ["PLANAR-REQ-007", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_007_segment_intersection_is_closed_and_exact"
oracle = "Constructed raw segments, each also checked against the parametric-solve oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-008"
requirements = ["PLANAR-REQ-008", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_tests.rs::planar_tp_008_segment_closest_point_rounds_once_to_even"
oracle = "Hand-derived raw points from the Python model, each also checked against the big-integer oracle by multiplying back."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-009"
requirements = ["PLANAR-REQ-002", "PLANAR-REQ-003", "PLANAR-REQ-004", "PLANAR-REQ-005", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_009_seeded_box_and_circle_queries_match_the_oracle"
oracle = "Independent big-integer and interval oracles over 40 000 seeded boundary-hugging cases."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-010"
requirements = ["PLANAR-REQ-006", "PLANAR-REQ-007", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_010_seeded_segment_predicates_match_the_oracle"
oracle = "Independent parametric-solve oracle over 60 000 seeded, deliberately collinear and touching quadruples."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-011"
requirements = ["PLANAR-REQ-008", "PLANAR-REQ-009"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_sweep_tests.rs::planar_tp_011_seeded_segment_closest_points_match_the_oracle"
oracle = "Independent big-integer oracle over 40 000 seeded segment and point pairs, checked by multiplying back."
tier = "fast"
status = "implemented"

[[case]]
id = "PLANAR-TP-012"
requirements = ["PLANAR-REQ-001", "PLANAR-REQ-002", "PLANAR-REQ-003", "PLANAR-REQ-004", "PLANAR-REQ-005", "PLANAR-REQ-006", "PLANAR-REQ-007", "PLANAR-REQ-008"]
evidence = "manual-audit"
artifact = "docs/topics/planar-geometry/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "PLANAR-TP-013"
requirements = ["PLANAR-REQ-010", "PLANAR-REQ-013"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_013_ray_box_hits_are_closed_and_exact"
oracle = "Constructed raw rays and boxes on and one raw unit either side of each contact, each also checked against the edge-by-edge oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-014"
requirements = ["PLANAR-REQ-011", "PLANAR-REQ-013"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_014_ray_circle_hits_are_closed_and_exact"
oracle = "Pythagorean chords and tangents and irrational roots, each also checked against the nearest-point and square-root-free quadratic oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-015"
requirements = ["PLANAR-REQ-012", "PLANAR-REQ-013"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_ray_tests.rs::planar_tp_015_ray_segment_hits_are_closed_and_exact"
oracle = "Constructed raw rays and segments, parallel and collinear included, each also checked against the line-side interpolation oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-016"
requirements = ["PLANAR-REQ-014", "PLANAR-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_polygon_tests.rs::planar_tp_016_point_in_polygon_is_exact_and_closed"
oracle = "Constructed raw polygons and points on and one raw unit either side of edges and vertices, each also checked against the crossing-abscissa winding oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-017"
requirements = ["PLANAR-REQ-014", "PLANAR-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_polygon_tests.rs::planar_tp_017_polygon_degenerate_and_self_intersecting_inputs"
oracle = "Constructed degenerate and self-intersecting polygons, each also checked against the crossing-abscissa winding oracle."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-018"
requirements = ["PLANAR-REQ-010", "PLANAR-REQ-011", "PLANAR-REQ-012", "PLANAR-REQ-013"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_cast_sweep_tests.rs::planar_tp_018_seeded_ray_queries_match_the_oracle"
oracle = "Independent big-integer oracles over 30 000 seeded rays with contact-hugging boxes, circles and segments."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-019"
requirements = ["PLANAR-REQ-014", "PLANAR-REQ-015"]
evidence = "test"
test = "crates/bunny-geom/tests/planar_cast_sweep_tests.rs::planar_tp_019_seeded_point_in_polygon_matches_the_oracle"
oracle = "Independent crossing-abscissa winding oracle over 30 000 seeded polygons and boundary-hugging points."
tier = "fast"
status = "planned"

[[case]]
id = "PLANAR-TP-020"
requirements = ["PLANAR-REQ-010", "PLANAR-REQ-011", "PLANAR-REQ-012", "PLANAR-REQ-013", "PLANAR-REQ-014", "PLANAR-REQ-015"]
evidence = "manual-audit"
artifact = "docs/topics/planar-geometry/test-plan.md#mutation-checks-of-the-second-slice"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-geom/src/planar.rs`,
`crates/bunny-geom/src/planar/segment.rs` or `crates/bunny-geom/src/wide.rs`, and `cargo
test --locked --no-fail-fast -p bunny-geom --test planar_tests --test planar_sweep_tests`
was run; the source was restored after each. The unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| `is_ordered` uses `<` (refuses flat boxes) | Flat and point boxes are valid | 001, 002, 003, 005, 009 |
| `FixedCircle2::try_new` refuses a zero radius | Zero radius accepted | 001 |
| Box containment excludes the boundary | Closed box | 002, 009, 011 |
| `overlaps` skips the ordered check | Inverted box is empty | 002, 009 |
| `overlaps` uses `<` on one `x` bound | Touching boxes overlap | 002, 009 |
| `overlaps` drops one `y` comparison | Intervals intersect on both axes | 002, 009 |
| `closest_point` clamps an inverted box | `None` for an inverted box | 003, 005, 009 |
| The clamp keeps a value above the maximum | Per-axis clamp | 003, 005, 009 |
| Circle containment excludes the circle | Closed disc | 004, 005, 009 |
| A negative radius contains points | Negative radius is empty | 004, 005, 009 |
| `overlaps_circle` uses `r1 - r2` | Radius sum | 005, 009 |
| `overlaps_circle` skips the radius sign check | Negative radius is empty | 005, 009 |
| `overlaps_aabb` tests only the centre | Closest point in the disc | 005, 009 |
| The squared distance drops `dy` | Exact squared distance | 004, 005, 009 |
| Segment containment skips the endpoint box | Past an end is not on it | 006, 010 |
| Segment containment skips the orientation | Off the line is not on it | 006, 010 |
| The endpoint box checks `x` only | Both axes | 006, 007, 010 |
| `intersects` reads proper crossings only | Touching counts | 007, 010 |
| `intersects` skips the second segment's endpoints on the first | Every endpoint tested | 007, 010 |
| `intersects` reads an `f64` orientation | Exact sign | 007 (the diagonals of the raw range), 010 |
| `closest_point` rounds the offset from `a` | Round the coordinate | 008, 011 |
| `closest_point` does not clamp below zero | Parameter clamped to `[0, 1]` | 008, 011 |
| `closest_point` does not clamp above one | Parameter clamped to `[0, 1]` | 008, 011 |
| Ties go away from zero | Ties to even | 008, 011 |
| Ties go toward zero | Ties to even | 008, 011 |
| Ties go to the odd neighbour | Ties to even | 008, 011 |
| The quotient is truncated, not rounded | Round to nearest | 008, 011 |

One mutation survived and is equivalent: replacing the strict "one `CounterClockwise`, one
`Clockwise`" test of `intersects` by "the two orientations differ". The difference is a pair
with one `Collinear`. Say `c` is exactly on the line of `a`-`b` and `d` is not. If `c` is on
the segment `a`-`b`, the segments meet and both versions say so through the endpoint test.
If `c` is past an end, `a` and `b` lie on one side of `c` along their line. Then `a` and `b`
are on the same side of the line `c`-`d`, or both on it, and that pair is not "different"
either. The two tests therefore agree on every input. The strict form is kept because it
states the crossing rule directly.

The `wide.rs` mutations also change the shared rounding of `bunny-predicates/1`, whose
own mutation table records what they break there.

## Mutation Checks Of The Second Slice

Planned: each rule of the ray queries and point in polygon broken on purpose in a local
copy of the kernel, with the cases recorded here.
