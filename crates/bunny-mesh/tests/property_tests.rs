//! Deterministic property tests.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{dequantize_vertex, quantize_vertex, QuantizedVertex};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const MESH_PROPERTY_SEED: u64 = 0xB002_0129_0000_0004;
const MESH_PROPERTY_CASES: usize = 96;
const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn q32_units(units: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(units * ONE_RAW)
}

fn bounded_units(state: &mut u64) -> i64 {
    let value = next_seed(state) % 65;
    i64::try_from(value).expect("property unit is bounded")
}

fn assert_case_eq<T>(actual: T, expected: T, case: usize, property: &str)
where
    T: Copy + core::fmt::Debug + PartialEq,
{
    assert_eq!(actual, expected, "{property}; seed={MESH_PROPERTY_SEED:#018x}; case={case}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_quantization_is_monotonic_inside_fixed_bounds() {
    let mut state = MESH_PROPERTY_SEED;
    let bounds = FixedAabb3::new(
        FixedVec3::new(q32_units(0), q32_units(0), q32_units(0)),
        FixedVec3::new(q32_units(64), q32_units(64), q32_units(64)),
    );

    for case in 0..MESH_PROPERTY_CASES {
        let first = bounded_units(&mut state);
        let second = bounded_units(&mut state);
        let low = first.min(second);
        let high = first.max(second);
        let low_vertex = FixedVec3::new(q32_units(low), q32_units(low), q32_units(low));
        let high_vertex = FixedVec3::new(q32_units(high), q32_units(high), q32_units(high));

        let quantized_low = quantize_vertex(low_vertex, &bounds);
        let quantized_high = quantize_vertex(high_vertex, &bounds);

        assert!(
            quantized_low.x <= quantized_high.x
                && quantized_low.y <= quantized_high.y
                && quantized_low.z <= quantized_high.z,
            "quantization monotonicity; seed={MESH_PROPERTY_SEED:#018x}; case={case}"
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn prop_quantized_round_trip_is_stable_for_representative_codes() {
    let bounds = FixedAabb3::new(
        FixedVec3::new(q32_units(-32), q32_units(-32), q32_units(-32)),
        FixedVec3::new(q32_units(32), q32_units(32), q32_units(32)),
    );
    let codes = [0, 1, 255, 1024, 32_768, 65_534, u16::MAX];

    for (case, code) in codes.into_iter().enumerate() {
        let quantized = QuantizedVertex::new(code, code, code);
        let dequantized = dequantize_vertex(quantized, &bounds);
        let requantized = quantize_vertex(dequantized, &bounds);

        assert_case_eq(requantized, quantized, case, "quantized code round trip");
    }
}
