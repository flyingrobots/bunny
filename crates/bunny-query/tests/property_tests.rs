//! Deterministic property tests.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{closest_point_aabb, closest_points_segments};
use wasm_bindgen_test::wasm_bindgen_test;

use bunny_geom::FixedAabb3;

const QUERY_PROPERTY_SEED: u64 = 0xB002_0129_0000_0003;
const QUERY_PROPERTY_CASES: usize = 96;
const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn component(state: &mut u64) -> FixedQ32_32 {
    let value = next_seed(state) % 17;
    let signed = i64::try_from(value).expect("property component is bounded") - 8;
    FixedQ32_32::from_raw(signed * ONE_RAW)
}

fn vec3(state: &mut u64) -> FixedVec3 {
    FixedVec3::new(component(state), component(state), component(state))
}

fn sorted_pair(lhs: FixedQ32_32, rhs: FixedQ32_32) -> (FixedQ32_32, FixedQ32_32) {
    if lhs <= rhs {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    }
}

fn assert_case_eq<T>(actual: T, expected: T, case: usize, property: &str)
where
    T: Copy + core::fmt::Debug + PartialEq,
{
    assert_eq!(actual, expected, "{property}; seed={QUERY_PROPERTY_SEED:#018x}; case={case}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_closest_segment_query_is_symmetric_for_bounded_values() {
    let mut state = QUERY_PROPERTY_SEED;

    for case in 0..QUERY_PROPERTY_CASES {
        let p1 = vec3(&mut state);
        let q1 = vec3(&mut state);
        let p2 = vec3(&mut state);
        let q2 = vec3(&mut state);

        let (lhs_a, lhs_b) = closest_points_segments(p1, q1, p2, q2);
        let (rhs_b, rhs_a) = closest_points_segments(p2, q2, p1, q1);

        assert_case_eq(lhs_a, rhs_a, case, "segment closest point is symmetric for first segment");
        assert_case_eq(lhs_b, rhs_b, case, "segment closest point is symmetric for second segment");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_closest_point_aabb_stays_inside_bounds() {
    let mut state = QUERY_PROPERTY_SEED ^ 0xC105_E57A_ABB0_0001;

    for case in 0..QUERY_PROPERTY_CASES {
        let a = vec3(&mut state);
        let b = vec3(&mut state);
        let point = vec3(&mut state);
        let (min_x, max_x) = sorted_pair(a.x, b.x);
        let (min_y, max_y) = sorted_pair(a.y, b.y);
        let (min_z, max_z) = sorted_pair(a.z, b.z);
        let aabb = FixedAabb3::new(
            FixedVec3::new(min_x, min_y, min_z),
            FixedVec3::new(max_x, max_y, max_z),
        );

        let closest = closest_point_aabb(&aabb, point);
        assert!(closest.x >= min_x && closest.x <= max_x, "x bound; case={case}");
        assert!(closest.y >= min_y && closest.y <= max_y, "y bound; case={case}");
        assert!(closest.z >= min_z && closest.z <= max_z, "z bound; case={case}");
    }
}
