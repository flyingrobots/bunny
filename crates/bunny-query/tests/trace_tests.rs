//! Oracles for the trace kernels: constructed exact hits, an exact rational box test, and
//! the limits of the trace domain.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{
    BarycentricNumerators, TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW,
    TRACE_DIRECTION_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

const TRACE_SEED: u64 = 0xB002_7ACE_0000_0001;
/// Barycentric weights in the constructed cases are multiples of `1 / WEIGHT_STEPS`.
const WEIGHT_STEPS: i64 = 64;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
/// `box_entry` may fall short of the true entry distance by this many raw units.
const ENTRY_SLACK_RAW: i128 = (1 << 11) + 2;
/// The slack the per-axis rounding derivation allows: one raw unit for the floor, one for
/// flooring the exact entry in the oracle.
const DERIVED_SLACK_RAW: i128 = 2;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn add(a: [i64; 3], b: [i64; 3], scale: i64) -> [i64; 3] {
    [a[0] + b[0] * scale, a[1] + b[1] * scale, a[2] + b[2] * scale]
}

fn triple_product(a: [i64; 3], b: [i64; 3], c: [i64; 3]) -> i128 {
    let w = |v: [i64; 3]| v.map(i128::from);
    let (a, b, c) = (w(a), w(b), w(c));
    a[0] * (b[1] * c[2] - b[2] * c[1])
        + a[1] * (b[2] * c[0] - b[0] * c[2])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

/// A triangle whose edges are multiples of `WEIGHT_STEPS`, a ray built to pass through the
/// point with weights `(b, c) / WEIGHT_STEPS` at `units` whole units of distance.
struct Constructed {
    v0: [i64; 3],
    step1: [i64; 3],
    step2: [i64; 3],
    direction: [i64; 3],
}

impl Constructed {
    fn seeded(seeds: &mut Seeds, coord_bound: i64, step_bound: i64) -> Self {
        Self {
            v0: seeds.triple(coord_bound),
            step1: seeds.triple(step_bound),
            step2: seeds.triple(step_bound),
            direction: seeds.triple(TRACE_DIRECTION_LIMIT_RAW),
        }
    }

    fn triangle(&self) -> Option<TraceTriangle> {
        TraceTriangle::new(
            vec3(self.v0),
            vec3(add(self.v0, self.step1, WEIGHT_STEPS)),
            vec3(add(self.v0, self.step2, WEIGHT_STEPS)),
        )
    }

    /// Ray through the point with weights `(b, c)`, starting `units` whole units before it.
    fn ray(&self, b: i64, c: i64, units: i64) -> Option<TraceRay> {
        let target = add(add(self.v0, self.step1, b), self.step2, c);
        TraceRay::new(vec3(add(target, self.direction, -units)), vec3(self.direction))
    }

    /// Positive when the ray meets the front face, zero when it is parallel to the plane.
    fn facing(&self) -> i128 {
        -triple_product(self.direction, self.step1, self.step2)
    }
}

fn weight(steps: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(steps * (ONE_RAW / WEIGHT_STEPS))
}

fn assert_constructed_hits(seeds: &mut Seeds, coord_bound: i64, step_bound: i64, cases: usize) {
    let mut checked = 0;
    for case in 0..cases {
        let built = Constructed::seeded(seeds, coord_bound, step_bound);
        let b = seeds.signed(WEIGHT_STEPS).abs();
        let c = seeds.signed(WEIGHT_STEPS - b).abs();
        let units = 1 + seeds.signed(3).abs();
        let (Some(triangle), Some(ray)) = (built.triangle(), built.ray(b, c, units)) else {
            continue;
        };
        if built.facing() == 0 {
            continue;
        }
        let hit = ray
            .hit_triangle(&triangle, FixedQ32_32::ZERO, FAR)
            .unwrap_or_else(|| panic!("seed={TRACE_SEED:#018x} case={case}: expected a hit"));
        assert_eq!(hit.distance.raw(), units * ONE_RAW, "distance; case={case}");
        assert_eq!(hit.barycentrics(), (weight(b), weight(c)), "weights; case={case}");
        assert_eq!(hit.front_face, built.facing() > 0, "facing; case={case}");
        checked += 1;
    }
    assert!(checked * 2 > cases, "most constructed cases must be in the trace domain");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_001_constructed_hits_report_exact_distance_and_weights() {
    let mut seeds = Seeds(TRACE_SEED);
    // Small triangles near the origin, then triangles and offsets close to the domain limit.
    assert_constructed_hits(&mut seeds, 8 * ONE_RAW, ONE_RAW / 256, 4000);
    assert_constructed_hits(&mut seeds, 1 << 38, 1 << 31, 4000);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_002_points_outside_the_triangle_or_behind_the_ray_miss() {
    let mut seeds = Seeds(TRACE_SEED ^ 2);
    for case in 0..4000 {
        let built = Constructed::seeded(&mut seeds, 8 * ONE_RAW, ONE_RAW / 256);
        let Some(triangle) = built.triangle() else { continue };
        if built.facing() == 0 {
            continue;
        }
        let inside = seeds.signed(WEIGHT_STEPS / 2).abs();
        let outside = [(-1, inside), (inside, -1), (inside, WEIGHT_STEPS + 1 - inside)];
        for (b, c) in outside {
            let ray = built.ray(b, c, 2).expect("ray is in the trace domain");
            assert_eq!(ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR), None, "case={case}");
        }
        let behind = built.ray(inside / 2, inside / 2, -2).expect("ray is in the trace domain");
        assert_eq!(behind.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR), None, "case={case}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_003_distance_limits_are_inclusive() {
    let built = Constructed {
        v0: [-ONE_RAW, -ONE_RAW, 0],
        step1: [2 * ONE_RAW / WEIGHT_STEPS, 0, 0],
        step2: [0, 2 * ONE_RAW / WEIGHT_STEPS, 0],
        direction: [0, 0, -ONE_RAW],
    };
    let triangle = built.triangle().expect("triangle is in the trace domain");
    let ray = built.ray(16, 16, 5).expect("ray is in the trace domain");
    let five = FixedQ32_32::from_raw(5 * ONE_RAW);
    let below = FixedQ32_32::from_raw(5 * ONE_RAW - 1);
    let above = FixedQ32_32::from_raw(5 * ONE_RAW + 1);
    let hit = ray.hit_triangle(&triangle, five, five).expect("both limits are inclusive");
    assert_eq!(hit.distance, five);
    assert!(hit.front_face, "a counter-clockwise triangle seen from +z shows its front");
    assert_eq!(ray.hit_triangle(&triangle, FixedQ32_32::ZERO, below), None);
    assert_eq!(ray.hit_triangle(&triangle, above, FAR), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_004_a_shared_edge_never_leaks() {
    let mut seeds = Seeds(TRACE_SEED ^ 4);
    for case in 0..4000 {
        let built = Constructed::seeded(&mut seeds, 8 * ONE_RAW, ONE_RAW / 256);
        // The quad v0, v0 + e1, v0 + e1 + e2, v0 + e2, split along the v0 diagonal.
        let corner = |b, c| vec3(add(add(built.v0, built.step1, b), built.step2, c));
        let (v0, v1, v2, v3) = (
            corner(0, 0),
            corner(WEIGHT_STEPS, 0),
            corner(WEIGHT_STEPS, WEIGHT_STEPS),
            corner(0, WEIGHT_STEPS),
        );
        let halves = [TraceTriangle::new(v0, v1, v2), TraceTriangle::new(v0, v2, v3)];
        if built.facing() == 0 {
            continue;
        }
        let on_diagonal = seeds.signed(WEIGHT_STEPS).abs();
        let anywhere = (seeds.signed(WEIGHT_STEPS).abs(), seeds.signed(WEIGHT_STEPS).abs());
        for (b, c) in [(on_diagonal, on_diagonal), anywhere] {
            let ray = built.ray(b, c, 3).expect("ray is in the trace domain");
            let hits = halves
                .iter()
                .flatten()
                .filter(|half| ray.hit_triangle(half, FixedQ32_32::ZERO, FAR).is_some())
                .count();
            let expected = if b == c { 2 } else { 1 };
            assert_eq!(hits, expected, "case={case} weights=({b}, {c})");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_005_extreme_domain_values_do_not_overflow() {
    let limit = TRACE_COORD_LIMIT_RAW - 1;
    let unit = TRACE_DIRECTION_LIMIT_RAW;
    for signs in 0..8_u8 {
        let sign = |bit: u8| if signs & (1 << bit) == 0 { 1 } else { -1 };
        let corner = [sign(0) * limit, sign(1) * limit, sign(2) * limit];
        let triangle = TraceTriangle::new(
            vec3(corner),
            vec3([-corner[0], corner[1], corner[2]]),
            vec3([corner[0], -corner[1], -corner[2]]),
        )
        .expect("corners are in the trace domain");
        for direction in [[unit, unit, unit], [-unit, unit, -unit], [unit, -unit, 1], [1, 1, -1]] {
            let ray = TraceRay::new(vec3([-corner[0], -corner[1], corner[2]]), vec3(direction))
                .expect("ray is in the trace domain");
            // The arithmetic is checked in test builds: reaching this line is the assertion.
            let hit = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR);
            let _ = hit.map(|hit| hit.barycentrics());
            assert!(ray.box_entry(&triangle.bounds(), FAR).is_some() || hit.is_none());
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_006_values_outside_the_trace_domain_are_refused() {
    let origin = vec3([0, 0, 0]);
    let down = vec3([0, -ONE_RAW, 0]);
    let outside = vec3([TRACE_COORD_LIMIT_RAW, 0, 0]);
    let inside = vec3([TRACE_COORD_LIMIT_RAW - 1, 0, 0]);
    assert!(TraceRay::new(inside, down).is_some());
    assert_eq!(TraceRay::new(outside, down), None);
    assert_eq!(TraceRay::new(origin, origin), None, "a zero direction has no line");
    assert_eq!(TraceRay::new(origin, vec3([ONE_RAW + 1, 0, 0])), None);
    assert!(TraceRay::new(origin, vec3([-ONE_RAW, ONE_RAW, 1])).is_some());
    assert!(TraceTriangle::new(inside, origin, down).is_some());
    assert_eq!(TraceTriangle::new(origin, outside, down), None);
    let triangle = TraceTriangle::new(inside, origin, down).expect("checked above");
    assert_eq!(triangle.vertices(), [inside, origin, down]);
}

/// Exact entry distance of a ray into a box as a fraction `(numerator, denominator)` of raw
/// distance, or `None` when the ray does not reach the box. Compares fractions by
/// cross-multiplying, so nothing is rounded.
fn exact_entry(
    origin: [i64; 3],
    direction: [i64; 3],
    min: [i64; 3],
    max: [i64; 3],
) -> Option<(i128, i128)> {
    let mut enter = (0_i128, 1_i128);
    let mut exit: Option<(i128, i128)> = None;
    for axis in 0..3 {
        let (o, d) = (i128::from(origin[axis]), i128::from(direction[axis]));
        let (lo, hi) = (i128::from(min[axis]), i128::from(max[axis]));
        if d == 0 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (near, far) = if d > 0 { (lo - o, hi - o) } else { (o - hi, o - lo) };
        let d = d.abs();
        if near * enter.1 > enter.0 * d {
            enter = (near, d);
        }
        if exit.is_none_or(|(n, e)| far * e < n * d) {
            exit = Some((far, d));
        }
    }
    let reaches = exit.is_none_or(|(n, e)| n >= 0 && enter.0 * e <= n * enter.1);
    reaches.then_some(enter)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_007_box_entry_never_rejects_a_box_the_ray_reaches() {
    let mut seeds = Seeds(TRACE_SEED ^ 7);
    let (mut reached, mut rejected) = (0, 0);
    for case in 0..40_000 {
        let origin = seeds.triple(4 * ONE_RAW);
        let mut direction = seeds.triple(TRACE_DIRECTION_LIMIT_RAW);
        for component in &mut direction {
            // One case in four has an axis-parallel or nearly axis-parallel component.
            match seeds.next() % 12 {
                0 => *component = 0,
                1 => *component = seeds.signed(3),
                _ => {}
            }
        }
        let (a, b) = (seeds.triple(4 * ONE_RAW), seeds.triple(4 * ONE_RAW));
        let min = [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])];
        let max = [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])];
        let Some(ray) = TraceRay::new(vec3(origin), vec3(direction)) else { continue };
        let bound = ray.box_entry(&FixedAabb3::new(vec3(min), vec3(max)), FAR);
        let Some((numerator, denominator)) = exact_entry(origin, direction, min, max) else {
            rejected += usize::from(bound.is_none());
            continue;
        };
        // Exact entry in raw distance is numerator * 2^32 / denominator.
        let exact = (numerator << 32) / denominator;
        if exact > i128::from(i64::MAX) {
            continue;
        }
        let bound = bound.unwrap_or_else(|| panic!("case={case}: reached box was rejected"));
        let bound = i128::from(bound.raw());
        assert!(bound <= exact, "case={case}: bound {bound} is past the entry {exact}");
        assert!(exact - bound <= ENTRY_SLACK_RAW + (exact >> 30), "case={case}: bound is loose");
        // The derivation in the chapter gives the tighter `entry / 2^31 + 1`; check it too.
        assert!(exact - bound <= DERIVED_SLACK_RAW + (exact >> 31), "case={case}: derivation");
        reached += 1;
    }
    assert!(reached > 2000, "the corpus must include reached boxes: {reached}");
    assert!(rejected > 2000, "the test must also reject missed boxes: {rejected}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_008_box_entry_respects_the_distance_limit() {
    let one = ONE_RAW;
    let unit_box = FixedAabb3::new(vec3([-one, -one, -one]), vec3([one, one, one]));
    let ray = TraceRay::new(vec3([0, 0, 5 * one]), vec3([0, 0, -one])).expect("in the domain");
    let four = FixedQ32_32::from_raw(4 * one);
    assert_eq!(ray.box_entry(&unit_box, four), Some(four));
    assert_eq!(ray.box_entry(&unit_box, FixedQ32_32::from_raw(4 * one - 1)), None);
    assert_eq!(ray.box_entry(&unit_box, FixedQ32_32::from_raw(-1)), None);
    let away = TraceRay::new(vec3([0, 0, 5 * one]), vec3([0, 0, one])).expect("in the domain");
    assert_eq!(away.box_entry(&unit_box, FAR), None);
    let inside = TraceRay::new(vec3([0, 0, 0]), vec3([1, 0, 0])).expect("in the domain");
    assert_eq!(inside.box_entry(&unit_box, FAR), Some(FixedQ32_32::ZERO));
}

/// One golden triangle case: vertices and ray in raw units, then the exact expected
/// `(distance, u, v, front_face)` in raw units.
struct GoldenHit {
    vertices: [[i64; 3]; 3],
    origin: [i64; 3],
    direction: [i64; 3],
    expected: (i64, i64, i64, bool),
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_009_golden_triangle_hits_lock_raw_outputs() {
    let one = ONE_RAW;
    let unit_triangle = [[0, 0, 0], [one, 0, 0], [0, one, 0]];
    let goldens = [
        // Straight down onto the quarter point from two units up: distance 2.0, weights 1/4.
        GoldenHit {
            vertices: unit_triangle,
            origin: [one / 4, one / 4, 2 * one],
            direction: [0, 0, -one],
            expected: (8_589_934_592, 1_073_741_824, 1_073_741_824, true),
        },
        // Direction of 3 raw units from 1 raw unit up: true distance 2^32 / 3 raw, floored.
        GoldenHit {
            vertices: unit_triangle,
            origin: [one / 4, one / 4, 1],
            direction: [0, 0, -3],
            expected: (1_431_655_765, 1_073_741_824, 1_073_741_824, true),
        },
        // From below, so the back face; weights 1/3, floored to 1_431_655_765 raw.
        GoldenHit {
            vertices: [[0, 0, 0], [3 * one, 0, 0], [0, 3 * one, 0]],
            origin: [one, one, -one],
            direction: [0, 0, one],
            expected: (4_294_967_296, 1_431_655_765, 1_431_655_765, false),
        },
        // Exactly on the corner v1: edges and corners are inclusive, u = 1.0, v = 0.
        GoldenHit {
            vertices: unit_triangle,
            origin: [one, 0, one],
            direction: [0, 0, -one],
            expected: (4_294_967_296, 4_294_967_296, 0, true),
        },
    ];
    for (index, golden) in goldens.iter().enumerate() {
        let [v0, v1, v2] = golden.vertices.map(vec3);
        let triangle = TraceTriangle::new(v0, v1, v2).expect("golden triangle is in the domain");
        let ray =
            TraceRay::new(vec3(golden.origin), vec3(golden.direction)).expect("in the domain");
        let hit = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR).expect("golden case hits");
        let (u, v) = hit.barycentrics();
        assert_eq!(
            (hit.distance.raw(), u.raw(), v.raw(), hit.front_face),
            golden.expected,
            "golden {index}"
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_010_golden_box_entries_lock_raw_outputs() {
    let one = ONE_RAW;
    let unit = ([-one, -one, -one], [one, one, one]);
    // A box whose x entry and y exit are the same exact distance, 2^52 raw: the ray from the
    // origin along (1, -3, 0) raw touches its edge at x = 2^20, y = -3 * 2^20. The x
    // reciprocal is exact; the y reciprocal floor(2^63 / 3) is not, so a slab exit rounded
    // down instead of up would fall one raw unit short of the entry and reject the box.
    let n = 1 << 20;
    let grazed = ([n, -3 * n, -1], [2 * n, 0, 1]);
    // (origin, direction, box, expected raw bound). The second case has true entry
    // 2^64 / 3 = 6_148_914_691_236_517_205.33 raw; the bound is one raw unit below its floor.
    let goldens = [
        ([0, 0, 5 * one], [0, 0, -one], unit, Some(4 * one)),
        ([0, 0, 2 * one], [0, 0, -3], unit, Some(6_148_914_691_236_517_204)),
        ([-3 * one, 0, 0], [one, one / 2, 0], unit, Some(2 * one)),
        ([-3 * one, 0, 0], [one, one, 0], unit, None),
        ([0, 2 * one, 0], [one, 0, 0], unit, None),
        ([0, one, 0], [one, 0, 0], unit, Some(0)),
        ([0, 0, 0], [1, -3, 0], grazed, Some(1 << 52)),
    ];
    for (index, (origin, direction, (min, max), expected)) in goldens.into_iter().enumerate() {
        let ray =
            TraceRay::new(vec3(origin), vec3(direction)).expect("golden ray is in the domain");
        let aabb = FixedAabb3::new(vec3(min), vec3(max));
        let bound = ray.box_entry(&aabb, FAR).map(FixedQ32_32::raw);
        assert_eq!(bound, expected, "golden {index}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_011_degenerate_triangles_parallel_rays_and_far_hits_miss() {
    let one = ONE_RAW;
    let triangle = |a, b, c| TraceTriangle::new(vec3(a), vec3(b), vec3(c)).expect("in the domain");
    let ray =
        |origin, direction| TraceRay::new(vec3(origin), vec3(direction)).expect("in the domain");
    let flat = triangle([0, 0, 0], [one, 0, 0], [0, one, 0]);
    let collinear = triangle([0, 0, 0], [one, one, 0], [2 * one, 2 * one, 0]);
    let duplicate = triangle([0, 0, 0], [0, 0, 0], [one, one, 0]);
    let down = ray([one / 4, one / 4, one], [0, 0, -one]);
    let onto_segment = ray([one, one, one], [0, 0, -one]);
    assert!(down.hit_triangle(&flat, FixedQ32_32::ZERO, FAR).is_some(), "control case hits");
    assert_eq!(onto_segment.hit_triangle(&collinear, FixedQ32_32::ZERO, FAR), None);
    assert_eq!(onto_segment.hit_triangle(&duplicate, FixedQ32_32::ZERO, FAR), None);
    // A ray lying in the triangle's plane and crossing its interior is parallel: a miss.
    let in_plane = ray([-one, one / 4, 0], [one, 0, 0]);
    assert_eq!(in_plane.hit_triangle(&flat, FixedQ32_32::ZERO, FAR), None);
    // 128 units away along a direction of one raw unit is 2^71 raw: too far for i64.
    let far_start = [one / 4, one / 4, 1 << 39];
    assert_eq!(ray(far_start, [0, 0, -1]).hit_triangle(&flat, FixedQ32_32::ZERO, FAR), None);
    let near_enough = ray(far_start, [0, 0, -one]).hit_triangle(&flat, FixedQ32_32::ZERO, FAR);
    assert_eq!(near_enough.map(|hit| hit.distance.raw()), Some(1 << 39));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_313_numerators_are_the_exact_weights_of_constructed_hits() {
    let mut seeds = Seeds(TRACE_SEED ^ 313);
    let mut checked = 0;
    for (case, (coord_bound, step_bound)) in [(8 * ONE_RAW, ONE_RAW / 256), (1 << 38, 1 << 31)]
        .into_iter()
        .cycle()
        .take(4000)
        .enumerate()
    {
        let built = Constructed::seeded(&mut seeds, coord_bound, step_bound);
        let b = seeds.signed(WEIGHT_STEPS).abs();
        let c = seeds.signed(WEIGHT_STEPS - b).abs();
        let (Some(triangle), Some(ray)) = (built.triangle(), built.ray(b, c, 2)) else {
            continue;
        };
        let Some(hit) = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR) else {
            continue;
        };
        // The point has weights (b, c) / 64 exactly, so u / det and v / det are those ratios,
        // and the values satisfy the triangle test's own conditions.
        let n = hit.barycentric_numerators();
        let steps = i128::from(WEIGHT_STEPS);
        assert_eq!(
            (n.u() * steps, n.v() * steps),
            (i128::from(b) * n.det(), i128::from(c) * n.det()),
            "case={case}"
        );
        assert_eq!(BarycentricNumerators::new(n.u(), n.v(), n.det()), Some(n), "case={case}");
        assert!(n.det() < 1 << 117, "case={case}");
        checked += 1;
    }
    assert!(checked > 3000, "{checked} hits checked");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_314_numerators_off_the_triangle_are_refused() {
    let new = BarycentricNumerators::new;
    assert!(new(0, 0, 1).is_some() && new(1, 0, 1).is_some() && new(0, 1, 1).is_some());
    assert!(new(3, 4, 7).is_some() && new(0, 0, i128::MAX).is_some());
    for (u, v, det) in [(0, 0, 0), (0, 0, -1), (-1, 0, 1), (0, -1, 1), (1, 1, 1), (4, 4, 7)] {
        assert_eq!(new(u, v, det), None, "({u}, {v}, {det})");
    }
    assert_eq!(new(i128::MAX, 1, i128::MAX), None, "u + v overflows");
    let n = new(3, 4, 7).expect("on the triangle");
    assert_eq!((n.u(), n.v(), n.det()), (3, 4, 7));
}
