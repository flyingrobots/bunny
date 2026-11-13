//! `ray_intersects_aabb_prepared` must return exactly what `ray_intersects_aabb` returns.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{ray_intersects_aabb, ray_intersects_aabb_prepared, PreparedRay3};
use wasm_bindgen_test::wasm_bindgen_test;

const PREPARED_SEED: u64 = 0xB002_9AE9_0000_0001;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A coordinate in [-range, range) Bunny units, from the generator's high bits; a quarter
/// of them are exactly zero so axis-parallel directions occur.
fn coordinate(state: &mut u64, range: i64) -> FixedQ32_32 {
    let bits = next_seed(state) >> 16;
    if bits.is_multiple_of(4) {
        return FixedQ32_32::ZERO;
    }
    let span = u64::try_from(2 * range * ONE_RAW).expect("positive span");
    FixedQ32_32::from_raw(i64::try_from(bits % span).expect("bounded") - range * ONE_RAW)
}

fn vec3(state: &mut u64, range: i64) -> FixedVec3 {
    FixedVec3::new(coordinate(state, range), coordinate(state, range), coordinate(state, range))
}

fn aabb(state: &mut u64) -> FixedAabb3 {
    let (a, b) = (vec3(state, 8), vec3(state, 8));
    let min = FixedVec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let max = FixedVec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
    FixedAabb3::try_new(min, max).expect("ordered bounds")
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_on_seeded_rays_and_boxes() {
    let mut state = PREPARED_SEED;
    let mut hits = 0;
    for case in 0..20_000 {
        let Ok(ray) = FixedRay3::try_new(vec3(&mut state, 10), vec3(&mut state, 1)) else {
            continue;
        };
        let prepared = PreparedRay3::new(ray);
        assert_eq!(prepared.ray(), &ray);
        for _ in 0..8 {
            let bounds = aabb(&mut state);
            let expected = ray_intersects_aabb(&ray, &bounds);
            hits += i32::from(expected.is_some());
            assert_eq!(ray_intersects_aabb_prepared(&prepared, &bounds), expected, "case={case}");
        }
    }
    assert!(hits > 1_000, "the inputs exercise hits as well as misses: {hits}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_for_axis_rays_inside_and_outside_boxes() {
    let one = FixedQ32_32::ONE;
    let unit_box =
        FixedAabb3::try_new(FixedVec3::new(-one, -one, -one), FixedVec3::new(one, one, one))
            .expect("ordered bounds");
    let zero = FixedQ32_32::ZERO;
    let origins = [
        FixedVec3::new(zero, zero, zero),
        FixedVec3::new(one + one, zero, zero),
        FixedVec3::new(zero, one, -one),
    ];
    let directions = [
        FixedVec3::new(one, zero, zero),
        FixedVec3::new(zero, -one, zero),
        FixedVec3::new(zero, zero, one),
    ];
    for origin in origins {
        for direction in directions {
            let ray = FixedRay3::try_new(origin, direction).expect("unit direction");
            let expected = ray_intersects_aabb(&ray, &unit_box);
            assert_eq!(ray_intersects_aabb_prepared(&PreparedRay3::new(ray), &unit_box), expected);
        }
    }
}
