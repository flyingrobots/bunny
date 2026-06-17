//! Integration tests.

use bunny_num::fixed_q32_32::{from_f32, to_f32, ONE_RAW};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn q32_32_raw_encoding_golden_vectors_are_stable() {
    assert_eq!(from_f32(0.25), ONE_RAW / 4);
    assert_eq!(from_f32(-0.25), -(ONE_RAW / 4));
    assert_eq!(from_f32(1.5), ONE_RAW + (ONE_RAW / 2));

    #[allow(clippy::float_cmp)]
    {
        assert_eq!(to_f32(ONE_RAW / 4), 0.25);
    }

    let three = FixedQ32_32::from_raw(3 * ONE_RAW);
    let two = FixedQ32_32::from_raw(2 * ONE_RAW);
    let quotient = three.checked_div(two).expect("non-zero divisor should divide");
    assert_eq!(quotient.to_raw(), ONE_RAW + (ONE_RAW / 2));
}
