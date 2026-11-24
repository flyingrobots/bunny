# Exact Separating-Axis Tests Test Plan

This document defines how Bunny proves [`bunny-sat/1`](README.md): `sat_obb_obb`,
`sat_triangle_aabb`, `SatClass`, `ObbAxis` and `TriangleAabbAxis`, all in `bunny-geom`. It is
the first slice of flyingrobots/bunny#147.

## Scope

- the closed-set classification of two shapes into `Separated`, `Touching` and `Overlapping`;
- the candidate axes of two oriented boxes and of a triangle against an axis-aligned box, their
  order, the skipping of exactly zero axes, and the first-axis reporting rule;
- exact projections with no rounding, no normalised axis and no overflow over every accepted
  input, degenerate triangles and flat, point and inverted boxes included.

Out of scope: the minimum translation axis, penetration depth and contact points, which the
issue asks for and a later slice will add; other shape pairs.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| SAT-REQ-001 | The class is a property of the exact closed shapes: `Separated` exactly when they are disjoint, `Touching` exactly when the origin is on the boundary of their Minkowski difference, `Overlapping` exactly when it is in its interior; `intersects()` is "not `Separated`". | `README.md#classification` |
| SAT-REQ-002 | `sat_obb_obb` tests `FaceOfFirst(0..3)` (`a_1 x a_2`, `a_2 x a_0`, `a_0 x a_1`), `FaceOfSecond(0..3)`, then `EdgePair { first: i, second: j }` (`a_i x b_j`, row-major), on raw unnormalised vectors; it skips exactly zero axes and reports the first axis with a positive gap as `Separated`, else the first with a zero gap as `Touching`. | `README.md#oriented-boxes` |
| SAT-REQ-003 | `sat_obb_obb` is exact for every pair of accepted boxes at every centre (interval ends below `2^167`), never refuses or overflows, and its class does not depend on the argument order. | `README.md#widths` |
| SAT-REQ-004 | `sat_triangle_aabb` tests `BoxFace(0..3)`, `TriangleFace` (skipped for a degenerate triangle), then `EdgeCross { edge, axis }` (`e_edge x u_axis`, row-major, skipped when zero) with the same reporting rule; an inverted box is the box with the same corners. | `README.md#triangles-against-axis-aligned-boxes` |
| SAT-REQ-005 | `sat_triangle_aabb` projects through the exact predicates and is exact for every representable input, `i64::MIN` and `i64::MAX` coordinates included; its class does not depend on the vertex order. | `README.md#triangles-against-axis-aligned-boxes` |

```toml
[[requirement]]
id = "SAT-REQ-001"
summary = "Separated, Touching and Overlapping are the origin outside, on the boundary of and inside the exact Minkowski difference of the closed shapes."
status = "active"

[[requirement]]
id = "SAT-REQ-002"
summary = "sat_obb_obb tests the six face normals then the nine edge cross products in the documented order, skips zero axes, and reports the first separating, else the first touching, axis."
status = "active"

[[requirement]]
id = "SAT-REQ-003"
summary = "sat_obb_obb is exact for every accepted box pair at every centre, never refuses or overflows, and its class is symmetric in its arguments."
status = "active"

[[requirement]]
id = "SAT-REQ-004"
summary = "sat_triangle_aabb tests the box faces, the triangle face, then the edge cross products in order, skips zero axes and treats an inverted box as the box with the same corners."
status = "active"

[[requirement]]
id = "SAT-REQ-005"
summary = "sat_triangle_aabb is exact through the predicates for every representable input, and its class does not depend on vertex order."
status = "active"
```

## Oracles

`crates/bunny-geom/tests/sat_oracle/mod.rs` works only from the corners of each shape: an
oriented box's eight corners exactly on the `2^-64` grid (`centre 2^32 + sum(+-h_i a_i)` on raw
values), a triangle's vertices and a box's eight corners (as given, so an inverted box needs no
reordering) on the raw grid. It shares no helper with the kernel, which projects a centre and
sums extents in a 256-bit two's complement integer, or calls the exact predicates. It uses the
unbounded integer of `tests/predicates_oracle/mod.rs` (32-bit digits, schoolbook arithmetic).
Two answers are computed separately:

- **Corner projections** (`expected`): each candidate axis built from the raw columns or
  vertices in the chapter's order, every corner projected onto it, the gap of the two
  projections' least and greatest values, and the first separating, else first zero-gap,
  nonzero axis. It checks the reported axis, the order and the skipping.
- **Minkowski hull** (`hull`): every difference of a corner of each shape, deduplicated, and
  the origin placed against their convex hull by brute force. In three dimensions it tries the
  plane through every three difference points, keeps those with every point on one closed side
  (each recorded as a bit mask of the points on it, so a facet is scanned once), and reports
  outside if the origin is strictly beyond one, boundary if on one, interior otherwise; a hull
  in a plane, on a line or at a point is tested in that plane or on that line, and has no
  interior. It knows nothing of separating axes, so it checks that the candidate lists are
  complete and that the class means what SAT-REQ-001 says. When every difference coordinate is
  below `2^39`, the three-dimensional search runs in checked `i128` (an overflow panics), else
  in the unbounded integer.

Every test is a `wasm_bindgen_test`, so the WebAssembly job runs them as well as the native
run.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| SAT-TP-001 | SAT-REQ-001, SAT-REQ-002 | Unit cubes on the identity axes moved along each axis to a gap of one raw unit (`Separated` at `FaceOfFirst(k)`), zero (`Touching` there) and minus one (`Overlapping`), on both sides; boxes of different sizes; the identity string; both oracles and the swapped call on each. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_001_axis_aligned_boxes_on_and_one_raw_unit_off_each_face` |
| SAT-TP-002 | SAT-REQ-002, SAT-REQ-003 | Three rotations (a general axis and angle, an eighth turn about `z`, a turn about `x`): the second box is the first moved by exactly three raw columns `a_0`, so contact is exact when the half extents along `a_0` sum to `3 2^32`; one raw unit less is `Separated` at `FaceOfFirst(0)`, equal is `Touching` there, one more is `Overlapping`. A kernel that tested the column `a_k` instead of the face normal fails here. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_002_rotated_boxes_face_to_face_on_and_one_raw_unit_off` |
| SAT-TP-003 | SAT-REQ-002 | Two unit cubes, one turned an eighth about `z`, one about `y`, centred `2 sqrt(2) +- 1/16` apart on `x`: only `EdgePair { first: 2, second: 1 }` separates, every face normal overlaps; swapped, the same axis is `EdgePair { first: 1, second: 2 }`; and two rotated cubes on which five edge pairs separate and no face does, reported at `(0, 1)` in row-major order. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_003_only_an_edge_pair_separates_crossed_diamonds` |
| SAT-TP-004 | SAT-REQ-001, SAT-REQ-002 | Point boxes coincident (`Touching`) and one raw unit apart; a point box inside a cube, on a face and one raw unit out; a segment box through a cube; two coplanar flat squares (`Touching` at `FaceOfFirst(2)`); overlapping cubes on parallel axes, whose zero edge pairs must be skipped (`Overlapping`, not `Touching`). | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_004_flat_segment_and_point_boxes_and_parallel_edges` |
| SAT-TP-005 | SAT-REQ-003 | Boxes centred at `i64::MIN` and `i64::MAX` with half extents up to `i64::MAX` raw, rotated and on the identity: both oracles agree, and cubes of half extent `i64::MAX` centred at `i64::MIN` and at `i64::MAX - 1`, `- 2` and `i64::MAX` are `Touching`, `Overlapping` and `Separated`. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_005_boxes_at_the_ends_of_the_raw_range` |
| SAT-TP-006 | SAT-REQ-001, SAT-REQ-004 | A triangle in the plane `x_k = 1 + delta` raw over the unit box's face, for each `k` and `delta` in `1, 0, -1`: `Separated` and `Touching` at `BoxFace(k)`, then `Overlapping`; both oracles and the inverted box on each. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_006_triangles_on_and_one_raw_unit_off_each_box_face` |
| SAT-TP-007 | SAT-REQ-004 | The triangle on the plane `x + y + z = 3 + delta` raw against the unit box's corner `(1, 1, 1)`: `Separated` and `Touching` at `TriangleFace`, then `Overlapping`. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_007_triangle_face_on_and_one_raw_unit_off_a_box_corner` |
| SAT-TP-008 | SAT-REQ-004 | A tilted triangle whose edge 0 runs along `x + y = 2 + delta` raw past the box's `z` edge at `(1, 1)`: `Separated` and `Touching` at `EdgeCross { edge: 0, axis: 2 }`, then `Overlapping`. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_008_an_edge_cross_product_on_and_one_raw_unit_off_a_box_edge` |
| SAT-TP-009 | SAT-REQ-001, SAT-REQ-004 | Point triangles inside, on and one raw unit off a face; segment triangles through the box, along a box edge and one raw unit off it; a point box on a flat triangle and one raw unit above; a flat box crossing a vertical triangle (`Overlapping`) and one lying in its plane (`Touching`); a box inverted on one axis. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_009_degenerate_triangles_and_flat_and_inverted_boxes` |
| SAT-TP-010 | SAT-REQ-005 | Triangles and boxes with coordinates `i64::MIN`, `i64::MAX` and one raw unit inside them: the box of the whole range against its floor triangle (`Touching` at `BoxFace(2)`), a diagonal and a tilted triangle against the whole box and its far and near corner cells; both oracles agree. | `crates/bunny-geom/tests/sat_tests.rs::sat_tp_010_triangles_and_boxes_at_the_ends_of_the_raw_range` |
| SAT-TP-011 | SAT-REQ-001, SAT-REQ-002, SAT-REQ-003 | 800 seeded box pairs (seed printed): 400 small (centres and half extents within 8 raw units, so the hull runs in `i128`), half of them on signed-permutation axes placed face, edge or corner to corner or one raw unit off, half rotated (some sharing axes), each against both oracles; 400 large, on rotations and permutations, a third face to face on exact columns with the contact moved by `-1`, `0` or `1` raw, the rest at every magnitude up to the ends of the range, against the corner projections and every hundredth against the hull. Every case's swap has the same class and the swap's axis matches the oracle. Outcome counts, and counts of edge-pair witnesses, are pinned. | `crates/bunny-geom/tests/sat_sweep_tests.rs::sat_tp_011_seeded_box_pairs_match_the_oracles` |
| SAT-TP-012 | SAT-REQ-001, SAT-REQ-004, SAT-REQ-005 | 2 000 seeded triangle and box pairs (seed printed) within `2^20` raw, one in ten at the ends of the range: triangles on a box face plane or one raw unit off, with edge 0 exactly through a box corner or one raw unit off, degenerate triangles, flat boxes and boxes inverted on some axes; both oracles on each, and a rotated and a reversed vertex order and the inverted box give the same class. Outcome counts, and counts of triangle-face and edge witnesses, are pinned. | `crates/bunny-geom/tests/sat_sweep_tests.rs::sat_tp_012_seeded_triangles_and_boxes_match_the_oracles` |
| SAT-TP-013 | SAT-REQ-001, SAT-REQ-002, SAT-REQ-003, SAT-REQ-004, SAT-REQ-005 | Each rule broken on purpose, including a column used as a face normal and a dropped carry, fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "SAT-TP-001"
requirements = ["SAT-REQ-001", "SAT-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_001_axis_aligned_boxes_on_and_one_raw_unit_off_each_face"
oracle = "Constructed contacts and one raw unit either side, each also checked against the corner-projection and Minkowski-hull oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-002"
requirements = ["SAT-REQ-002", "SAT-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_002_rotated_boxes_face_to_face_on_and_one_raw_unit_off"
oracle = "Exact face contact on rotated columns and one raw unit either side, each also checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-003"
requirements = ["SAT-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_003_only_an_edge_pair_separates_crossed_diamonds"
oracle = "A hand-derived edge-edge configuration, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-004"
requirements = ["SAT-REQ-001", "SAT-REQ-002"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_004_flat_segment_and_point_boxes_and_parallel_edges"
oracle = "Constructed degenerate boxes on and one raw unit off contact, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-005"
requirements = ["SAT-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_005_boxes_at_the_ends_of_the_raw_range"
oracle = "Boxes at the ends of the raw range against both oracles, with hand-derived contacts."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-006"
requirements = ["SAT-REQ-001", "SAT-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_006_triangles_on_and_one_raw_unit_off_each_box_face"
oracle = "Constructed face contacts and one raw unit either side, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-007"
requirements = ["SAT-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_007_triangle_face_on_and_one_raw_unit_off_a_box_corner"
oracle = "A constructed corner contact and one raw unit either side, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-008"
requirements = ["SAT-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_008_an_edge_cross_product_on_and_one_raw_unit_off_a_box_edge"
oracle = "A constructed edge contact and one raw unit either side, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-009"
requirements = ["SAT-REQ-001", "SAT-REQ-004"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_009_degenerate_triangles_and_flat_and_inverted_boxes"
oracle = "Constructed degenerate shapes on and one raw unit off contact, checked against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-010"
requirements = ["SAT-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_tests.rs::sat_tp_010_triangles_and_boxes_at_the_ends_of_the_raw_range"
oracle = "Shapes at the ends of the raw range against both oracles."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-011"
requirements = ["SAT-REQ-001", "SAT-REQ-002", "SAT-REQ-003"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_sweep_tests.rs::sat_tp_011_seeded_box_pairs_match_the_oracles"
oracle = "Corner projections and the Minkowski hull over 800 seeded box pairs built to touch."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-012"
requirements = ["SAT-REQ-001", "SAT-REQ-004", "SAT-REQ-005"]
evidence = "test"
test = "crates/bunny-geom/tests/sat_sweep_tests.rs::sat_tp_012_seeded_triangles_and_boxes_match_the_oracles"
oracle = "Corner projections and the Minkowski hull over 2 000 seeded triangle and box pairs built to touch."
tier = "fast"
status = "implemented"

[[case]]
id = "SAT-TP-013"
requirements = ["SAT-REQ-001", "SAT-REQ-002", "SAT-REQ-003", "SAT-REQ-004", "SAT-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/convex-collision/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-geom/src/sat.rs` or
`crates/bunny-geom/src/sat/wide.rs`, and `cargo test --locked --no-fail-fast -p bunny-geom
--test sat_tests --test sat_sweep_tests` was run; the source was restored after each. The
unmutated source passed every case. The crate denies warnings, so each mutation is written to
compile (a rule disabled by a condition that never holds, not by deleting the code it uses).

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| A zero gap counts as separated | Touching is not separation | 001, 002, 004 to 012 |
| A zero gap counts as overlap | Touching is reported | 001, 002, 004 to 012 |
| The last zero-gap axis is reported, not the first | First touching axis | 001, 002, 004, 005, 006, 009, 010, 011, 012 |
| The gap reads only one side (`q_lo - p_hi`) | Both sides of the interval | 001 to 007, 009 to 012 |
| Zero OBB axes are not skipped | Zero axes skipped | 001, 002, 004, 005, 011 |
| Zero edge cross products are not skipped | Zero axes skipped | 006, 009, 012 |
| The degenerate triangle's zero normal is not skipped | Zero axes skipped | 009, 012 |
| An OBB face normal is the column `a_k` | Face normal `a_(k+1) x a_(k+2)` | 002, 011 |
| The OBB centre is not scaled by `2^32` | Projection on the `2^-64` grid | 001 to 005, 011 |
| Radii use the signed, not absolute, projection | `sum(h_i \|a_i . L\|)` | 001 to 005, 011 |
| Edge pairs in column-major order | Row-major edge pairs | 003 |
| The second box's faces before the first's | Axis order | 001, 002, 004, 005, 011 |
| OBB edge pairs omitted | Complete axis list | 003, 005, 011 |
| Triangle edge cross products omitted | Complete axis list | 008, 012 |
| Triangle face omitted | Complete axis list | 007, 010, 012 |
| The triangle face before the box faces | Axis order | 006, 009, 010, 012 |
| An inverted box is not reordered | Inverted box is the same box | 006 to 010, 012 |
| The triangle's edge interval assumes a positive apex | Interval from zero to the apex | 007, 008, 010, 012 |
| The edge projection keeps the dropped axis | Projection drops `axis` | 012 |
| Products drop the carry between limbs | Exact 256-bit products | 001 to 005, 011 |
| Comparison ignores the sign | Signed comparison | 001, 003 to 012 |
| Narrow values are not sign extended | Exact widening | 001 to 005, 009 to 012 |

The column-major mutation first survived: no case had two separating edge pairs whose order
differs between row-major and column-major. A seeded search over the corner-projection oracle
found two rotated unit cubes on which `(0, 1)`, `(0, 2)`, `(1, 0)`, `(1, 1)` and `(2, 2)` all
separate and no face does; it was added to SAT-TP-003 and the mutation now fails.
