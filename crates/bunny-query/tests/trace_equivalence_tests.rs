//! `TraceRay::hit_triangle` against a frozen reference copy of its arithmetic
//! (`support/trace_reference.rs`): every output bit must agree, so any future exact
//! replacement of the kernel is checked against the plain version.
//!
//! The corpora concentrate where a shortcut on the distance limit would be decided closely
//! (flyingrobots/bunny#214 tried one): limits equal to the true distance and one raw unit
//! either side, zero, negative and the maximum, triangles whose `det` is below, at and just
//! past multiples of `2^32`, and values that use the trace domain's full bit budgets.

#[path = "support/trace_reference.rs"]
mod trace_reference;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const EQUIVALENCE_SEED: u64 = 0xB002_7ACE_0000_0214;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `-bound..=bound`, for `0 <= bound < 2^52`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is non-negative");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    /// A value whose magnitude is below `2^bits`, with `bits` itself drawn up to `max_bits`,
    /// so small and large magnitudes are equally common.
    fn scaled(&mut self, max_bits: u32) -> i64 {
        let bits = u32::try_from(self.next() % u64::from(max_bits)).expect("small") + 1;
        self.signed((1 << bits) - 1)
    }

    fn triple(&mut self, max_bits: u32) -> [i64; 3] {
        [self.scaled(max_bits), self.scaled(max_bits), self.scaled(max_bits)]
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

/// Compares the two implementations on one ray, triangle and pair of limits.
fn assert_same(ray: &TraceRay, triangle: &TraceTriangle, t_min: i64, t_max: i64) {
    let got = ray.hit_triangle(triangle, fixed(t_min), fixed(t_max)).map(|hit| {
        let (u, v) = hit.barycentrics();
        (hit.distance.raw(), hit.front_face, (u.raw(), v.raw()))
    });
    let want = trace_reference::hit_triangle(ray, triangle, fixed(t_min), fixed(t_max));
    assert_eq!(got, want, "ray={ray:?} triangle={triangle:?} limits={t_min}..={t_max}");
}

/// The limits where a shortcut on the distance limit would be decided closely, around a true
/// distance.
fn limits_around(distance: Option<i64>) -> Vec<i64> {
    let mut limits = vec![0, -1, 1, i64::MIN, i64::MAX, i64::MAX - 1];
    if let Some(d) = distance {
        limits.extend([d.saturating_sub(2), d.saturating_sub(1), d, d.saturating_add(1)]);
    }
    limits
}

/// Compares the implementations at every close limit, as an upper and as a lower limit.
/// Returns whether the ray crosses the triangle at a reportable distance.
fn assert_same_around(ray: &TraceRay, triangle: &TraceTriangle) -> bool {
    let distance = trace_reference::hit_triangle(ray, triangle, fixed(0), fixed(i64::MAX))
        .map(|(distance, _, _)| distance);
    for limit in limits_around(distance) {
        assert_same(ray, triangle, 0, limit);
        assert_same(ray, triangle, limit, i64::MAX);
        assert_same(ray, triangle, i64::MIN, limit);
    }
    distance.is_some()
}

/// A seeded ray aimed through a seeded point of a seeded triangle, so most cases cross it.
/// Coordinates and edges have magnitudes below `2^coord_bits` and `2^edge_bits`; direction
/// components below `2^direction_bits` (at most `2^32`).
fn seeded_case(seeds: &mut Seeds, bits: (u32, u32, u32)) -> Option<(TraceRay, TraceTriangle)> {
    let (coord_bits, edge_bits, direction_bits) = bits;
    let v0 = seeds.triple(coord_bits);
    let (e1, e2) = (seeds.triple(edge_bits), seeds.triple(edge_bits));
    let corner = |e: [i64; 3]| vec3([v0[0] + e[0], v0[1] + e[1], v0[2] + e[2]]);
    let triangle = TraceTriangle::new(vec3(v0), corner(e1), corner(e2))?;
    let direction = seeds.triple(direction_bits).map(|c| c.clamp(-(1 << 32), 1 << 32));
    // Through the point v0 + (e1 * a + e2 * b) / 8, mostly inside the triangle and sometimes
    // just past an edge, from `back` direction lengths before it (kept inside the domain).
    let (a, b) = eighths(seeds);
    let back = steps_back(seeds, direction);
    let target = |axis: usize| v0[axis] + (e1[axis] * a + e2[axis] * b) / 8;
    let origin = [0, 1, 2].map(|axis| target(axis) - direction[axis] * back);
    let ray = TraceRay::new(vec3(origin), vec3(direction))?;
    Some((ray, triangle))
}

/// Weights `(a, b)` in eighths with `a + b <= 9`: mostly inside a triangle, sometimes one
/// eighth past its long edge.
fn eighths(seeds: &mut Seeds) -> (i64, i64) {
    let a = seeds.next() % 10;
    let b = seeds.next() % (10 - a);
    (i64::try_from(a).expect("small"), i64::try_from(b).expect("small"))
}

/// A number of direction lengths to start before the target: at least one, below `2^30` so
/// the distance fits, and small enough that the origin stays inside the trace domain.
fn steps_back(seeds: &mut Seeds, direction: [i64; 3]) -> i64 {
    let longest = direction.iter().map(|c| c.unsigned_abs()).max().unwrap_or(1).max(1);
    let reach = ((1_u64 << 38) / longest).clamp(1, 1 << 30);
    i64::try_from(1 + seeds.next() % reach).expect("below 2^31")
}

/// Runs `cases` seeded cases with the given bit sizes; returns how many cross the triangle.
fn run_seeded(seeds: &mut Seeds, bits: (u32, u32, u32), cases: usize) -> usize {
    let mut crossed = 0;
    for _ in 0..cases {
        if let Some((ray, triangle)) = seeded_case(seeds, bits) {
            crossed += usize::from(assert_same_around(&ray, &triangle));
        }
    }
    crossed
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_201_seeded_limits_around_the_true_distance_agree() {
    let mut seeds = Seeds(EQUIVALENCE_SEED);
    // (coordinate, edge, direction) bit sizes: unit-scale scenes, tiny triangles with short
    // directions (`det` below 2^32), and the domain's full budgets.
    let shapes = [(34, 30, 32), (34, 8, 6), (36, 20, 32), (39, 39, 32), (39, 39, 3)];
    for shape in shapes {
        let crossed = run_seeded(&mut seeds, shape, 6_000);
        assert!(crossed > 2_000, "shape {shape:?}: only {crossed} of 6000 cross");
    }
}

/// Constructed cases whose `det` is an exact multiple of `2^32` and whose distance is a
/// whole number of raw units, so `t` equals `(t_max + 1) * ceil(det / 2^32)` or `t_max` times
/// it exactly at the limits around the hit.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_202_exact_distances_at_the_limit_agree() {
    let one = ONE_RAW;
    let triangle =
        TraceTriangle::new(vec3([-one, -one, 0]), vec3([one, -one, 0]), vec3([0, one, 0]))
            .expect("in the domain");
    let cases =
        [(5 * one, -one), (3, -1), (3, -3), (1 << 20, -1), (7, -(1 << 32)), (one, -(1 << 32))];
    for (height, direction) in cases {
        let ray = TraceRay::new(vec3([0, 0, height]), vec3([0, 0, direction])).expect("in domain");
        assert!(assert_same_around(&ray, &triangle), "height={height} direction={direction}");
    }
}

/// Raw-unit triangles hit at the corner `v0` straight down the z axis, so `det` and `t` are
/// chosen exactly: `det = e1x * e2y * |dz|` and `t = e1x * e2y * height`. `det = 2^32` makes
/// `ceil(det / 2^32) = 1` with `t` at the limit; `det = 2^32 + 1 = 641 * 6_700_417` and
/// `det = 1` sit one unit past a multiple of `2^32`, where rounding the quotient matters.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_204_determinants_at_multiples_of_2_pow_32_agree() {
    // (e1x, |dz|, height): det = e1x * |dz|, e2 = (0, 1, 0).
    let cases = [
        (1, 1 << 32, 7),
        (1, 1 << 32, ONE_RAW),
        (641, 6_700_417, 3),
        (641, 6_700_417, 1 << 20),
        (1, 1, 5),
        (1, 1, 1 << 30),
        (3, 1 << 32, 1 << 33),
    ];
    for (e1x, dz, height) in cases {
        let triangle = TraceTriangle::new(vec3([0, 0, 0]), vec3([e1x, 0, 0]), vec3([0, 1, 0]))
            .expect("in the domain");
        let ray = TraceRay::new(vec3([0, 0, height]), vec3([0, 0, -dz])).expect("in domain");
        assert!(assert_same_around(&ray, &triangle), "e1x={e1x} dz={dz} height={height}");
    }
}

/// Values at the edges of the trace domain, where `det` and `t` use their full bit budgets.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_203_extreme_domain_values_agree() {
    let limit = TRACE_COORD_LIMIT_RAW - 1;
    let unit = TRACE_DIRECTION_LIMIT_RAW;
    let mut crossed = 0;
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
            let origin = [-corner[0], -corner[1], corner[2]];
            let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("in the domain");
            crossed += usize::from(assert_same_around(&ray, &triangle));
        }
    }
    // TRACE-TP-005 counts the same three crossings with an independent oracle.
    assert_eq!(crossed, 3, "extreme cases that cross");
}
