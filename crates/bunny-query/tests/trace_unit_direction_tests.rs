//! `TraceRay::from_ray` accepts every unit direction (flyingrobots/bunny#292).
//!
//! `FixedUnitVec3::try_from_unit` allows the length to be off by one raw unit, so a unit
//! direction can have a component of `2^32 + 1`. `from_ray` used to return `None` for it. The
//! tests build that vector on each axis and with both signs, check that `new` keeps its own
//! bound, compare the kernels with independent oracles for such directions, and count what
//! `FixedUnitVec3::new` can produce (it never exceeds `2^32`).

#[path = "support/volume_oracle.rs"]
#[allow(dead_code, reason = "these tests read only the oracle's hit, not its classification")]
mod volume_oracle;

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{
    TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW,
    TRACE_UNIT_DIRECTION_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB002_7ACE_0292_0001;
const ONE: i64 = 1 << 32;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> usize {
        usize::try_from(self.next() % bound).expect("small bound")
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
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn unit_ray(origin: [i64; 3], direction: [i64; 3]) -> FixedRay3 {
    let unit = FixedUnitVec3::try_from_unit(vec3(direction))
        .unwrap_or_else(|| panic!("{direction:?} is within the unit-length tolerance"));
    FixedRay3::new(vec3(origin), unit)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_408_from_ray_accepts_the_unit_vector_of_the_issue_on_every_axis() {
    assert_eq!(TRACE_UNIT_DIRECTION_LIMIT_RAW, ONE + 1);
    assert_eq!(TRACE_DIRECTION_LIMIT_RAW, ONE, "the bound of `new` is unchanged");
    for axis in 0..3 {
        for sign in [1, -1] {
            let mut direction = [0; 3];
            direction[axis] = sign * (ONE + 1);
            let ray = unit_ray([0; 3], direction);
            let prepared = TraceRay::from_ray(&ray)
                .unwrap_or_else(|| panic!("from_ray refused {direction:?} with origin 0"));
            assert_eq!(raw3(prepared.direction()), direction);
            assert_eq!(raw3(prepared.origin()), [0; 3]);
            assert_eq!(
                TraceRay::new(vec3([0; 3]), vec3(direction)),
                None,
                "`new` keeps its bound of 1.0: {direction:?}"
            );
            // One raw unit more is not a unit vector, and `new` refuses it as well.
            direction[axis] = sign * (ONE + 2);
            assert!(FixedUnitVec3::try_from_unit(vec3(direction)).is_none());
            assert_eq!(TraceRay::new(vec3([0; 3]), vec3(direction)), None);
        }
    }
    // The origin bound still applies to `from_ray`.
    let mut origin = [0; 3];
    origin[1] = TRACE_COORD_LIMIT_RAW;
    assert_eq!(TraceRay::from_ray(&unit_ray(origin, [ONE + 1, 0, 0])), None);
    origin[1] = TRACE_COORD_LIMIT_RAW - 1;
    assert!(TraceRay::from_ray(&unit_ray(origin, [ONE + 1, 0, 0])).is_some());
}

/// The most that `FixedUnitVec3::new` can give is `2^32` (TRACE-REQ-001's argument: the
/// floored root is at least the largest component). Counted over generated vectors of every
/// scale, with axis-aligned, near-axis and tiny ones; none exceeds `2^32`, and every result
/// `from_ray` accepts.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_409_normalisation_never_yields_a_component_above_one() {
    let mut seeds = Seeds(SEED ^ 1);
    let (mut produced, mut refused, mut exactly_one, mut largest) = (0_u32, 0_u32, 0_u32, 0_i64);
    for case in 0..200_000_u32 {
        let scale = match seeds.below(6) {
            0 => 3,
            1 => 1 << 16,
            2 => ONE,
            3 => 1 << 40,
            4 => 1 << 52,
            _ => 1 << 61,
        };
        let mut raw = seeds.triple(scale);
        match seeds.below(8) {
            0 => raw[1] = 0,
            1 => raw[2] = seeds.signed(3),
            2 => (raw[1], raw[2]) = (seeds.signed(3), seeds.signed(3)),
            _ => {}
        }
        let Some(unit) = FixedUnitVec3::new(vec3(raw)) else {
            refused += 1;
            continue;
        };
        produced += 1;
        let direction = raw3(unit.into_inner());
        let peak = direction.iter().map(|c| c.abs()).max().expect("three components");
        assert!(peak <= ONE, "case {case}: normalizing {raw:?} gave {direction:?}");
        largest = largest.max(peak);
        exactly_one += u32::from(peak == ONE);
        assert!(
            TraceRay::from_ray(&FixedRay3::new(vec3([0; 3]), unit)).is_some(),
            "case {case}: {direction:?}"
        );
    }
    // Minimum counts, so the claim is about a real population.
    let counts = format!("{produced} normalized, {refused} refused, {exactly_one} at 1.0");
    // Measured: 137 673 normalized, 62 327 refused (nearly all raw-unit and 2^16-scale
    // vectors, whose normalization loses more than a raw unit of length), 19 241 at 1.0.
    assert!(produced >= 130_000, "{counts}");
    assert!(refused >= 50_000, "{counts}");
    assert!(exactly_one >= 15_000, "{counts}");
    assert_eq!(produced + refused, 200_000, "{counts}");
    assert_eq!(largest, ONE, "the largest component produced");
}

/// Exact entry distance of a ray into a box as a fraction of raw distance, or `None` when the
/// ray does not reach it (the oracle of `trace_tests.rs`, restated).
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

/// A unit direction with a component of `2^32 + 1` on a random axis and sign, the others in
/// `-2^16..=2^16` or zero (their squares sum to at most `2^33`).
fn long_direction(seeds: &mut Seeds) -> [i64; 3] {
    let axis = seeds.below(3);
    let mut direction = [0; 3];
    for (i, component) in direction.iter_mut().enumerate() {
        *component = if i == axis {
            if seeds.next() & 1 == 1 {
                ONE + 1
            } else {
                -(ONE + 1)
            }
        } else if seeds.below(3) == 0 {
            0
        } else {
            seeds.signed(1 << 16)
        };
    }
    direction
}

/// `hit_triangle` and `box_entry` for rays prepared by `from_ray` from the vector of the
/// issue and its neighbours, against the signed-volume oracle (floor distance, facing and
/// exact numerators) and the exact rational box entry, with the contract slack.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_410_kernels_agree_with_the_oracles_for_long_unit_directions() {
    let mut seeds = Seeds(SEED ^ 2);
    let (mut hits, mut misses, mut reached, mut rejected) = (0_u32, 0_u32, 0_u32, 0_u32);
    for case in 0..20_000_u32 {
        let direction = long_direction(&mut seeds);
        let v0 = seeds.triple(4 * ONE);
        let step = |seeds: &mut Seeds| seeds.triple(4 * ONE / 64).map(|c| c * 64);
        let (e1, e2) = (step(&mut seeds), step(&mut seeds));
        let (b, c) = (seeds.signed(40) + 40, seeds.signed(40) + 40);
        let at =
            |k: i64| [0, 1, 2].map(|i| v0[i] + (e1[i] * b + e2[i] * c) / 64 - direction[i] * k);
        let origin = at(i64::try_from(seeds.below(4)).expect("small"));
        let vertices = [v0, [0, 1, 2].map(|i| v0[i] + e1[i]), [0, 1, 2].map(|i| v0[i] + e2[i])];
        let ray = TraceRay::from_ray(&unit_ray(origin, direction)).expect("origin in the domain");
        let triangle = TraceTriangle::new(vec3(vertices[0]), vec3(vertices[1]), vec3(vertices[2]))
            .expect("triangle in the domain");
        let got = ray.hit_triangle(&triangle, FixedQ32_32::from_raw(i64::MIN), FAR).map(|hit| {
            let n = hit.barycentric_numerators();
            volume_oracle::Expected {
                distance: hit.distance.raw(),
                front_face: hit.front_face,
                u: n.u(),
                v: n.v(),
                det: n.det(),
            }
        });
        let expected = volume_oracle::verdict(origin, direction, vertices).hit;
        assert_eq!(got, expected, "case {case}: {origin:?} {direction:?} {vertices:?}");
        if expected.is_some() {
            hits += 1;
        } else {
            misses += 1;
        }

        let (p, q) = (seeds.triple(4 * ONE), seeds.triple(4 * ONE));
        let min = [0, 1, 2].map(|i| p[i].min(q[i]));
        let max = [0, 1, 2].map(|i| p[i].max(q[i]));
        let bound = ray.box_entry(&FixedAabb3::new(vec3(min), vec3(max)), FAR);
        match exact_entry(origin, direction, min, max) {
            None => rejected += u32::from(bound.is_none()),
            Some((numerator, denominator)) => {
                let exact = (numerator << 32) / denominator;
                if exact > i128::from(i64::MAX) {
                    continue;
                }
                let bound = i128::from(
                    bound
                        .unwrap_or_else(|| panic!("case {case}: a reached box was rejected"))
                        .raw(),
                );
                assert!(bound <= exact, "case {case}: bound {bound} past the entry {exact}");
                // The derivation in the chapter: short by less than `T / 2^31 + 2`.
                assert!(exact - bound <= 2 + (exact >> 31), "case {case}: slack");
                reached += 1;
            }
        }
    }
    assert!(hits >= 4_000 && misses >= 4_000, "{hits} hits, {misses} misses");
    assert!(reached >= 1_000, "{reached} reached boxes");
    assert!(rejected >= 1_000, "{rejected} boxes certainly missed");
}

/// A constructed hit with a known answer: a triangle in the plane `x = 2.0` and the ray along
/// `+x` with direction `2^32 + 1`. The point is at `2 * 2^32 / (1 + 2^-32)` raw distance,
/// `floor(2^65 / (2^32 + 1)) = 8 589 934 590` (remainder 2), computed with Python integers.
#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_411_a_long_direction_hit_has_the_exactly_floored_distance() {
    let plane = 2 * ONE;
    let triangle = TraceTriangle::new(
        vec3([plane, -ONE, -ONE]),
        vec3([plane, ONE, -ONE]),
        vec3([plane, 0, ONE]),
    )
    .expect("in the domain");
    // The normal is +x, so the ray travelling along -x meets the front face.
    for (sign, origin_x, front) in [(1, 0, false), (-1, 4 * ONE, true)] {
        let ray = TraceRay::from_ray(&unit_ray([origin_x, 0, 0], [sign * (ONE + 1), 0, 0]))
            .expect("some");
        let hit = ray
            .hit_triangle(&triangle, FixedQ32_32::from_raw(0), FAR)
            .unwrap_or_else(|| panic!("sign {sign}: the ray crosses the triangle"));
        assert_eq!(hit.distance.raw(), 8_589_934_590, "sign {sign}");
        assert_eq!(hit.front_face, front, "sign {sign}");
    }
}
