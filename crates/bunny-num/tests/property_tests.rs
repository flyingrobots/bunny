//! Deterministic property tests.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const NUMERIC_PROPERTY_SEED: u64 = 0xB002_0129_0000_0001;
const NUMERIC_PROPERTY_CASES: usize = 128;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn raw_i64(state: &mut u64) -> i64 {
    i64::from_ne_bytes(next_seed(state).to_ne_bytes())
}

fn small_raw(state: &mut u64) -> i64 {
    let value = next_seed(state) % 257;
    let signed = i64::try_from(value).expect("property value is bounded") - 128;
    signed * (ONE_RAW / 16)
}

fn assert_case_eq<T>(actual: T, expected: T, case: usize, property: &str)
where
    T: Copy + core::fmt::Debug + PartialEq,
{
    assert_eq!(actual, expected, "{property}; seed={NUMERIC_PROPERTY_SEED:#018x}; case={case}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_fixed_q32_32_raw_round_trip_and_ordering_are_stable() {
    let mut state = NUMERIC_PROPERTY_SEED;
    let mut previous_raw = raw_i64(&mut state);
    let mut previous = FixedQ32_32::from_raw(previous_raw);

    for case in 0..NUMERIC_PROPERTY_CASES {
        let raw = raw_i64(&mut state);
        let value = FixedQ32_32::from_raw(raw);

        assert_case_eq(value.raw(), raw, case, "raw round trip");
        assert_case_eq(value.to_raw(), raw, case, "compatibility raw round trip");

        let expected_order = raw.cmp(&previous_raw);
        let actual_order = value.cmp(&previous);
        assert_case_eq(actual_order, expected_order, case, "ordering follows raw values");

        previous_raw = raw;
        previous = value;
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_checked_arithmetic_round_trips_for_bounded_values() {
    let mut state = NUMERIC_PROPERTY_SEED ^ 0xA11C_EADD_5AFE_0001;

    for case in 0..NUMERIC_PROPERTY_CASES {
        let lhs = FixedQ32_32::from_raw(small_raw(&mut state));
        let rhs = FixedQ32_32::from_raw(small_raw(&mut state));

        let sum = lhs.checked_add(rhs).expect("bounded addition should fit");
        assert_case_eq(sum.checked_sub(rhs), Some(lhs), case, "checked addition round trip");

        let diff = lhs.checked_sub(rhs).expect("bounded subtraction should fit");
        assert_case_eq(diff.checked_add(rhs), Some(lhs), case, "checked subtraction round trip");

        assert_case_eq(lhs.checked_mul(FixedQ32_32::ONE), Some(lhs), case, "multiply by one");
        assert_case_eq(
            rhs.checked_mul(FixedQ32_32::ZERO),
            Some(FixedQ32_32::ZERO),
            case,
            "multiply by zero",
        );
    }
}
