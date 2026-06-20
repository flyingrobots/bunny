//! Deterministic property tests.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const LINALG_PROPERTY_SEED: u64 = 0xB002_0129_0000_0002;
const LINALG_PROPERTY_CASES: usize = 96;
const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn component(state: &mut u64) -> FixedQ32_32 {
    let value = next_seed(state) % 33;
    let signed = i64::try_from(value).expect("property component is bounded") - 16;
    FixedQ32_32::from_raw(signed * ONE_RAW)
}

fn vec3(state: &mut u64) -> FixedVec3 {
    FixedVec3::new(component(state), component(state), component(state))
}

fn zero_vec3() -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO)
}

fn assert_case_eq<T>(actual: T, expected: T, case: usize, property: &str)
where
    T: Copy + core::fmt::Debug + PartialEq,
{
    assert_eq!(actual, expected, "{property}; seed={LINALG_PROPERTY_SEED:#018x}; case={case}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_vec3_dot_cross_and_length_invariants_hold_for_bounded_values() {
    let mut state = LINALG_PROPERTY_SEED;

    for case in 0..LINALG_PROPERTY_CASES {
        let lhs = vec3(&mut state);
        let rhs = vec3(&mut state);

        assert_case_eq(lhs.dot(rhs), rhs.dot(lhs), case, "dot product commutes");
        assert_case_eq(lhs.cross(rhs), -rhs.cross(lhs), case, "cross product anti-commutes");
        assert_case_eq(lhs.length_squared(), lhs.dot(lhs), case, "length squared is self dot");
        assert_case_eq(lhs.cross(lhs), zero_vec3(), case, "self cross product is zero");
    }
}
