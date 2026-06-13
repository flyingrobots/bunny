use bunny_num::fixed_q32_32::{from_f32, to_f32, FRAC_BITS, ONE_RAW};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn constants_and_raw_encoding_are_q32_32() {
    assert_eq!(FRAC_BITS, 32);
    assert_eq!(ONE_RAW, 1_i64 << 32);
}

#[wasm_bindgen_test(unsupported = test)]
fn from_f32_encodes_exact_values() {
    assert_eq!(from_f32(0.0), 0);
    assert_eq!(from_f32(-0.0), 0);
    assert_eq!(from_f32(1.0), ONE_RAW);
    assert_eq!(from_f32(-1.0), -ONE_RAW);
    assert_eq!(from_f32(0.5), 1_i64 << 31);
    assert_eq!(from_f32(1.5), ONE_RAW + (1_i64 << 31));
}

#[wasm_bindgen_test(unsupported = test)]
fn to_f32_roundtrips_basic_values() {
    for value in [0.0, -0.0, 1.0, -1.0, 0.5, 1.5] {
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(to_f32(from_f32(value)), value);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn non_finite_inputs_use_canonical_policy() {
    assert_eq!(from_f32(f32::NAN), 0);
    assert_eq!(from_f32(f32::INFINITY), i64::MAX);
    assert_eq!(from_f32(f32::NEG_INFINITY), i64::MIN);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_q32_32_operator_math() {
    let a = FixedQ32_32::from_f32(1.5);
    let b = FixedQ32_32::from_f32(2.5);

    let sum = a + b;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(sum.to_f32(), 4.0);
    }

    let diff = b - a;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(diff.to_f32(), 1.0);
    }

    let prod = a * b;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(prod.to_f32(), 3.75);
    }

    let quotient = b / a;
    // 2.5 / 1.5 = 1.6666666...
    // 1.6666666 is rounded to nearest.
    assert!((quotient.to_f32() - (2.5 / 1.5)).abs() < 1e-7);

    let neg_a = -a;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(neg_a.to_f32(), -1.5);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_q32_32_saturating_limits() {
    let max_val = FixedQ32_32::from_raw(i64::MAX);
    let min_val = FixedQ32_32::from_raw(i64::MIN);
    let one = FixedQ32_32::ONE;

    assert_eq!((max_val + one).to_raw(), i64::MAX);
    assert_eq!((min_val - one).to_raw(), i64::MIN);
    assert_eq!((-min_val).to_raw(), i64::MAX);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_q32_32_sqrt() {
    let a = FixedQ32_32::from_f32(4.0);
    let sqrt_a = a.sqrt().expect("4.0 has a real square root");
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(sqrt_a.to_f32(), 2.0);
    }

    let b = FixedQ32_32::from_f32(9.0);
    let sqrt_b = b.sqrt().expect("9.0 has a real square root");
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(sqrt_b.to_f32(), 3.0);
    }

    let c = FixedQ32_32::from_f32(2.0);
    let sqrt_c = c.sqrt().expect("2.0 has a real square root");
    assert!((sqrt_c.to_f32() - std::f32::consts::SQRT_2).abs() < 1e-7);

    let d = FixedQ32_32::from_f32(-1.0);
    assert!(d.sqrt().is_none());
}

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_q32_32_checked_div() {
    let a = FixedQ32_32::from_f32(10.0);
    let b = FixedQ32_32::from_f32(2.0);

    let res = a.checked_div(b).expect("10.0 / 2.0 succeeds");
    assert_eq!(res.to_f32(), 5.0);

    // Div by zero
    assert!(a.checked_div(FixedQ32_32::ZERO).is_none());

    // Overflow cases
    let max_val = FixedQ32_32::from_raw(i64::MAX);
    // Dividing max value by something less than 1.0 (e.g. 0.5) must overflow
    let half = FixedQ32_32::from_f32(0.5);
    assert!(max_val.checked_div(half).is_none());
}
