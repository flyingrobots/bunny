//! `PointQuery::closest_on_triangle` and `PointQuery::box_distance_bound` against an exact
//! rational oracle that shares no helper with the kernel.

#[path = "support/point_triangle_oracle.rs"]
mod oracle;

use std::cmp::Ordering;

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceTriangle, TrianglePoint, POINT_BOX_GAP_LIMIT_RAW};
use oracle::Ratio;
use wasm_bindgen_test::wasm_bindgen_test;

const POINT_SEED: u64 = 0xB002_0246_0000_0001;
const SEEDED_CASES: usize = 3_000;
const BOX_CASES: usize = 2_000;
/// The largest raw coordinate in the trace domain.
const EDGE: i64 = (1 << 40) - 1;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn raw3(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn raw_of(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn query(p: [i64; 3]) -> PointQuery {
    PointQuery::new(vec3(p)).expect("query point in the trace domain")
}

fn triangle(t: [[i64; 3]; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(t[0]), vec3(t[1]), vec3(t[2])).expect("triangle in the trace domain")
}

fn kernel(p: [i64; 3], t: [[i64; 3]; 3]) -> TrianglePoint {
    query(p).closest_on_triangle(&triangle(t))
}

/// Asserts the kernel's point and squared distance are the oracle's exact values rounded once,
/// to nearest with ties to even, and returns the oracle's squared distance.
fn assert_matches_oracle(p: [i64; 3], t: [[i64; 3]; 3], context: &str) -> Ratio {
    let found = kernel(p, t);
    let (exact_point, exact_distance) = oracle::closest(p, t);
    let reported = raw_of(found.point());
    for axis in 0..3 {
        assert!(
            oracle::rounds_to(&exact_point[axis], reported[axis]),
            "{context}: axis {axis} of {reported:?} is not the exact closest point rounded (p {p:?}, t {t:?})"
        );
    }
    let squared = found.distance_squared().raw();
    assert!(
        oracle::rounds_to(&oracle::over_two_to_32(&exact_distance), squared),
        "{context}: squared distance {squared} is not the exact one rounded (p {p:?}, t {t:?})"
    );
    exact_distance
}

const ONE: i64 = 1 << 32;

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_001_query_points_outside_the_trace_domain_are_refused() {
    for axis in 0..3 {
        for (raw, accepted) in [(EDGE, true), (-EDGE, true), (EDGE + 1, false), (-EDGE - 1, false)]
        {
            let mut p = [0; 3];
            p[axis] = raw;
            assert_eq!(PointQuery::new(vec3(p)).is_some(), accepted, "axis {axis}, raw {raw}");
        }
        let mut p = [0; 3];
        p[axis] = i64::MIN;
        assert!(PointQuery::new(vec3(p)).is_none(), "axis {axis}, i64::MIN");
    }
    let p = [EDGE, -EDGE, 7];
    assert_eq!(raw_of(query(p).point()), p);
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_002_each_region_gives_its_hand_derived_point() {
    // The right triangle (0,0,0), (4,0,0), (0,4,0) in whole units.
    let t = [[0, 0, 0], [4 * ONE, 0, 0], [0, 4 * ONE, 0]];
    let cases: [([i64; 3], [i64; 3], i64); 9] = [
        // Above the face: the projection, at the height squared.
        ([ONE, ONE, 3 * ONE], [ONE, ONE, 0], 9 * ONE),
        // On the face, on an edge and on a vertex: the point itself.
        ([ONE, ONE, 0], [ONE, ONE, 0], 0),
        ([2 * ONE, 0, 0], [2 * ONE, 0, 0], 0),
        ([4 * ONE, 0, 0], [4 * ONE, 0, 0], 0),
        // Each vertex region.
        ([-ONE, -2 * ONE, 0], [0, 0, 0], 5 * ONE),
        ([6 * ONE, -ONE, 2 * ONE], [4 * ONE, 0, 0], 9 * ONE),
        ([-ONE, 6 * ONE, 0], [0, 4 * ONE, 0], 5 * ONE),
        // Edge regions: below the x leg, and beyond the hypotenuse x + y = 4.
        ([2 * ONE, -3 * ONE, ONE], [2 * ONE, 0, 0], 10 * ONE),
        ([3 * ONE, 3 * ONE, 0], [2 * ONE, 2 * ONE, 0], 2 * ONE),
    ];
    for (p, point, squared_units) in cases {
        let found = kernel(p, t);
        assert_eq!(raw_of(found.point()), point, "p {p:?}");
        // `squared_units` is in whole units times 2^32 raw: the Q32.32 raw value.
        assert_eq!(found.distance_squared().raw(), squared_units, "p {p:?}");
        assert_matches_oracle(p, t, "hand-derived");
    }
    // The hypotenuse edge region again, beside the face's far corner.
    let found = kernel([ONE, 5 * ONE, 0], t);
    assert_eq!(raw_of(found.point()), [0, 4 * ONE, 0]);
    assert_eq!(found.distance_squared().raw(), 2 * ONE);
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_003_coordinates_and_squared_distances_round_to_nearest_ties_to_even() {
    // Segment (0,0,0)-(1,1,0) raw seen from (1,0,0): the exact point is (1/2, 1/2, 0).
    let found = kernel([1, 0, 0], [[0, 0, 0], [1, 1, 0], [1, 1, 0]]);
    assert_eq!(raw_of(found.point()), [0, 0, 0]);
    // (1,0,0)-(2,1,0) seen from (2,0,0): (3/2, 1/2, 0) rounds to (2, 0, 0).
    let found = kernel([2, 0, 0], [[1, 0, 0], [2, 1, 0], [2, 1, 0]]);
    assert_eq!(raw_of(found.point()), [2, 0, 0]);
    // Thirds: (0,0,0)-(3,3,3) seen from (1,0,0) is (1/3, 1/3, 1/3), from (2,0,0) (2/3, ...).
    let found = kernel([1, 0, 0], [[0, 0, 0], [3, 3, 3], [3, 3, 3]]);
    assert_eq!(raw_of(found.point()), [0, 0, 0]);
    let found = kernel([2, 0, 0], [[0, 0, 0], [3, 3, 3], [3, 3, 3]]);
    assert_eq!(raw_of(found.point()), [1, 1, 1]);
    // Negative halves are symmetric: (-1,0,0)-(-2,-1,0) seen from (-2,0,0) is (-3/2, -1/2, 0).
    let found = kernel([-2, 0, 0], [[-1, 0, 0], [-2, -1, 0], [-2, -1, 0]]);
    assert_eq!(raw_of(found.point()), [-2, 0, 0]);
    // Squared distances in raw units squared over 2^32: exactly 1/2 rounds to 0, 3/2 to 2,
    // just above 1/2 to 1, just below to 0.
    for (vertex, expected) in [
        ([1 << 15, 1 << 15, 0], 0),
        ([1 << 16, 1 << 15, 1 << 15], 2),
        ([1 << 15, 1 << 15, 1], 1),
        ([46_340, 0, 0], 0),
        ([46_341, 0, 0], 1),
    ] {
        let found = kernel([0; 3], [vertex; 3]);
        assert_eq!(found.distance_squared().raw(), expected, "vertex {vertex:?}");
        assert_matches_oracle([0; 3], [vertex; 3], "squared-distance ties");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_004_degenerate_triangles_are_the_segment_or_point_they_span() {
    let a = [0, 0, 0];
    let b = [8 * ONE, 0, 0];
    let cases = [
        // `c` between `a` and `b`, beyond `b`, and before `a`: the segment `a b`, `a c`, `c b`.
        [a, b, [2 * ONE, 0, 0]],
        [a, [2 * ONE, 0, 0], b],
        [[2 * ONE, 0, 0], b, [-2 * ONE, 0, 0]],
        // Two vertices equal, and all three equal.
        [a, a, b],
        [b, a, b],
        [b, b, b],
    ];
    for t in cases {
        for p in [[3 * ONE, 4 * ONE, 0], [-3 * ONE, ONE, 2], [12 * ONE, -ONE, ONE], [ONE, 0, 0]] {
            assert_matches_oracle(p, t, "degenerate");
        }
    }
    // Over the segment from 0 to 8 units, (3, 4, 0) is 4 units from (3, 0, 0).
    let found = kernel([3 * ONE, 4 * ONE, 0], [a, b, [2 * ONE, 0, 0]]);
    assert_eq!(
        (raw_of(found.point()), found.distance_squared().raw()),
        ([3 * ONE, 0, 0], 16 * ONE)
    );
    // The point triangle at `b`.
    let found = kernel([3 * ONE, 4 * ONE, 0], [b, b, b]);
    assert_eq!((raw_of(found.point()), found.distance_squared().raw()), (b, 41 * ONE));
    // A sliver: nearly collinear, with a nonzero normal of one raw unit squared.
    let sliver = [a, b, [4 * ONE, 1, 0]];
    for p in [[4 * ONE, 0, 5], [4 * ONE, -7, 0], [ONE, 9, -3]] {
        assert_matches_oracle(p, sliver, "sliver");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_005_the_widest_inputs_in_the_trace_domain_are_exact() {
    let corners = [-EDGE, EDGE];
    let mut checked = 0;
    for &x in &corners {
        for &y in &corners {
            for &z in &corners {
                let p = [x, y, z];
                for t in [
                    [[-x, -y, -z], [-x, y, -z], [x, -y, -z]],
                    [[-x, -y, -z], [x, y, -z], [-x, y, z]],
                    [[-x, -y, -z], [x, y, z], [x, -y, z]],
                    [[EDGE, EDGE, -EDGE], [-EDGE, EDGE, EDGE], [EDGE, -EDGE, EDGE]],
                    [[-x, -y, -z], [-x, -y, -z], [x, y, z]],
                ] {
                    assert_matches_oracle(p, t, "domain corners");
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 40);
    // The largest squared distance: corner to opposite corner, `3 (2^41 - 2)^2` raw squared,
    // `3 (2^41 - 2)^2 / 2^32` raw in Q32.32, rounded to nearest (its remainder is not a half).
    let found = kernel([EDGE; 3], [[-EDGE; 3]; 3]);
    let exact = 3 * (i128::from(EDGE) * 2).pow(2);
    let (floor, remainder) = (exact >> 32, exact & 0xFFFF_FFFF);
    assert_ne!(remainder, 1 << 31);
    let rounded = floor + i128::from(remainder > 1 << 31);
    assert_eq!(i128::from(found.distance_squared().raw()), rounded);
    assert_eq!(raw_of(found.point()), [-EDGE; 3]);
}

/// A triangle for a seeded case, from one of five families.
fn seeded_case(seeds: &mut Seeds) -> ([i64; 3], [[i64; 3]; 3]) {
    match seeds.below(5) {
        // Anywhere in the domain.
        0 => (seeds.raw3(EDGE), [seeds.raw3(EDGE), seeds.raw3(EDGE), seeds.raw3(EDGE)]),
        // A few raw units: half-integer points, ties and degenerate triangles are common.
        1 => (seeds.raw3(6), [seeds.raw3(4), seeds.raw3(4), seeds.raw3(4)]),
        // Collinear vertices.
        2 => {
            let (a, d) = (seeds.raw3(1 << 30), seeds.raw3(1 << 20));
            let (j, k) = (seeds.signed(64), seeds.signed(64));
            let at = |m: i64| [a[0] + m * d[0], a[1] + m * d[1], a[2] + m * d[2]];
            (seeds.raw3(1 << 31), [a, at(j), at(k)])
        }
        // The query near the triangle, at unit scale.
        3 => {
            let t = [seeds.raw3(4 * ONE), seeds.raw3(4 * ONE), seeds.raw3(4 * ONE)];
            (seeds.raw3(4 * ONE), t)
        }
        // Coordinates from the domain's extremes and zero.
        _ => {
            let mut pick =
                || [-EDGE, -1, 0, 1, EDGE][usize::try_from(seeds.below(5)).expect("small")];
            let mut point = || [pick(), pick(), pick()];
            (point(), [point(), point(), point()])
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_006_seeded_points_match_the_oracle_and_compare_exactly() {
    let mut seeds = Seeds(POINT_SEED);
    let mut previous: Option<(TrianglePoint, Ratio)> = None;
    let (mut ties, mut bounds_checked) = (0, 0);
    for case in 0..SEEDED_CASES {
        let (p, t) = seeded_case(&mut seeds);
        let context = format!("seed {POINT_SEED:#X}, case {case}");
        let exact = assert_matches_oracle(p, t, &context);
        let found = kernel(p, t);
        if let Some((other, other_exact)) = &previous {
            let expected = exact.cmp(other_exact);
            assert_eq!(found.cmp_distance(other), expected, "{context}: cmp_distance");
        }
        // The edge `t0 t1` alone, as a degenerate triangle: as near as the triangle exactly
        // when the triangle's closest point is on that edge, which is common.
        let edge = [t[0], t[1], t[0]];
        let (edge_point, edge_exact) = (kernel(p, edge), oracle::closest(p, edge).1);
        let expected = exact.cmp(&edge_exact);
        assert_ne!(expected, Ordering::Greater, "{context}: the triangle holds the edge");
        assert_eq!(found.cmp_distance(&edge_point), expected, "{context}: against the edge");
        assert_eq!(edge_point.cmp_distance(&found), expected.reverse(), "{context}: reversed");
        ties += usize::from(expected == Ordering::Equal);
        // Integer bounds around the exact squared distance, from the rounded one.
        let near = u128::try_from(found.distance_squared().raw()).expect("not negative") << 32;
        for bound in
            [near.saturating_sub(1 << 32), near.saturating_sub(1), near, near + 1, near + (1 << 32)]
        {
            let expected = exact.cmp(&Ratio::int(i128::try_from(bound).expect("fits")));
            assert_eq!(found.cmp_bound(bound), expected, "{context}: cmp_bound({bound})");
            bounds_checked += 1;
        }
        assert_eq!(found.cmp_bound(u128::MAX), Ordering::Less, "{context}");
        previous = Some((found, exact));
    }
    assert!(ties > SEEDED_CASES / 4, "the corpus should hold many exact ties; it held {ties}");
    assert_eq!(bounds_checked, 5 * SEEDED_CASES);
}

/// The exact squared distance from `p` to the box, by the closest point of the box.
fn box_distance_exact(p: [i64; 3], min: [i64; 3], max: [i64; 3]) -> u128 {
    (0..3)
        .map(|k| {
            let nearest = p[k].clamp(min[k], max[k]);
            let gap = (i128::from(p[k]) - i128::from(nearest)).unsigned_abs();
            gap * gap
        })
        .sum()
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_007_the_box_bound_is_the_exact_distance_and_never_above_it() {
    let bound = |p: [i64; 3], min: [i64; 3], max: [i64; 3]| {
        query(p).box_distance_bound(&FixedAabb3::new(vec3(min), vec3(max)))
    };
    let (min, max) = ([0, 0, 0], [10, 10, 10]);
    // Inside, on a face, on a corner, and one raw unit outside each.
    assert_eq!(bound([5, 5, 5], min, max), 0);
    assert_eq!(bound([10, 5, 5], min, max), 0);
    assert_eq!(bound([10, 10, 10], min, max), 0);
    assert_eq!(bound([11, 5, 5], min, max), 1);
    assert_eq!(bound([-1, 5, 5], min, max), 1);
    assert_eq!(bound([11, 11, 11], min, max), 3);
    assert_eq!(bound([13, -4, 5], min, max), 9 + 16);
    // Gaps at the cap and past it, from boxes outside the trace domain.
    let cap = i64::try_from(POINT_BOX_GAP_LIMIT_RAW).expect("fits");
    assert_eq!(
        bound([0; 3], [cap - 1, 0, 0], [i64::MAX, 0, 0]),
        (POINT_BOX_GAP_LIMIT_RAW - 1).pow(2)
    );
    assert_eq!(bound([0; 3], [cap, 0, 0], [i64::MAX, 0, 0]), POINT_BOX_GAP_LIMIT_RAW.pow(2));
    assert_eq!(bound([0; 3], [cap + 1, 0, 0], [i64::MAX, 0, 0]), POINT_BOX_GAP_LIMIT_RAW.pow(2));
    let far = [i64::MIN, i64::MIN, i64::MIN];
    assert_eq!(bound([EDGE; 3], far, far), 3 * POINT_BOX_GAP_LIMIT_RAW.pow(2));
    // An inverted axis contributes the larger of its two signed gaps.
    assert_eq!(bound([5, 0, 0], [6, 0, 0], [2, 0, 0]), 9);
    // Seeded boxes and points: the bound equals the exact distance while gaps are below the
    // cap, and is never above the squared distance to a seeded point of the box.
    let mut seeds = Seeds(POINT_SEED ^ 0xB0C5);
    for case in 0..BOX_CASES {
        let scale = if case % 2 == 0 { 1 << 20 } else { EDGE };
        let (p, a, b) = (seeds.raw3(scale), seeds.raw3(scale), seeds.raw3(scale));
        let min = [0, 1, 2].map(|k| a[k].min(b[k]));
        let max = [0, 1, 2].map(|k| a[k].max(b[k]));
        let found = bound(p, min, max);
        let exact = box_distance_exact(p, min, max);
        let capped = (0..3).all(|k| (min[k] - p[k]).max(p[k] - max[k]) < cap);
        if capped {
            assert_eq!(found, exact, "seed {POINT_SEED:#X} ^ 0xB0C5, case {case}");
        }
        assert!(found <= exact, "case {case}: bound above the exact distance");
        let inside =
            [0, 1, 2].map(|k| min[k] + seeds.signed(i64::MAX / 4).rem_euclid(max[k] - min[k] + 1));
        assert!(
            found <= box_distance_exact(p, inside, inside),
            "case {case}: a point of the box is nearer"
        );
    }
}
