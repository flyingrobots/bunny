//! Exact tie-rounding vectors for every Q32.32 rounding site (`docs/NUMERIC_CONSTITUTION.md`).
//!
//! Oracle: each expected raw value is derived by hand from the exact rational result, written
//! next to it as `exact -> neighbours -> result`. Ties go to the neighbour whose last bit is
//! even; negative results use the same rule on the signed value. `U = 2^32` is one unit
//! (`1.0`) and `H = 2^31` is half of it. The test plan is
//! `docs/topics/fixed-point-rounding/test-plan.md`.

use bunny_num::fixed_q32_32::{from_f32, to_f32, try_from_f32};
use bunny_num::{FixedDivisor, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;

/// `lhs * rhs` gives `expected` raw through `checked_mul` and the `Mul` operator.
fn assert_product(lhs: i64, rhs: i64, expected: i64) {
    let (a, b) = (FixedQ32_32::from_raw(lhs), FixedQ32_32::from_raw(rhs));
    assert_eq!(a.checked_mul(b).map(FixedQ32_32::raw), Some(expected), "checked_mul({lhs}, {rhs})");
    assert_eq!((a * b).raw(), expected, "Mul({lhs}, {rhs})");
}

/// `dividend / divisor` gives `expected` raw through `checked_div`, `Div` and `FixedDivisor`.
fn assert_quotient(dividend: i64, divisor: i64, expected: i64) {
    let (a, b) = (FixedQ32_32::from_raw(dividend), FixedQ32_32::from_raw(divisor));
    let context = format!("{dividend} / {divisor}");
    assert_eq!(a.checked_div(b).map(FixedQ32_32::raw), Some(expected), "checked_div {context}");
    assert_eq!((a / b).raw(), expected, "Div {context}");
    let prepared = FixedDivisor::new(b).map(|d| d.checked_div(a).map(FixedQ32_32::raw));
    assert_eq!(prepared, Some(Some(expected)), "FixedDivisor {context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_001_products_below_half_keep_the_truncation() {
    assert_product(1, 2 * U + H - 1, 2); // 2 + (H-1)/U -> 2, 3 -> 2
    assert_product(U + 1, U + 1, U + 2); // U + 2 + 1/U -> U+2, U+3 -> U+2
    assert_product(1, -(2 * U + H - 1), -2); // -2 - (H-1)/U -> -3, -2 -> -2
    assert_product(-(U + 1), U + 1, -(U + 2)); // -(U + 2 + 1/U) -> -(U+3), -(U+2) -> -(U+2)
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_002_products_above_half_leave_the_truncation() {
    assert_product(1, 2 * U + H + 1, 3); // 2 + (H+1)/U -> 2, 3 -> 3
    assert_product(U - 1, U + 1, U); // (U^2 - 1)/U = U - 1/U -> U-1, U -> U
    assert_product(1, -(2 * U + H + 1), -3); // -2 - (H+1)/U -> -3, -2 -> -3
    assert_product(-(U - 1), U + 1, -U); // -(U - 1/U) -> -U, -(U-1) -> -U
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_003_product_ties_keep_an_even_truncation() {
    assert_product(H, 5, 2); // 5/2 = 2.5 -> 2 (even), 3 -> 2
    assert_product(1, 4 * U + H, 4); // 4.5 -> 4 (even), 5 -> 4
    assert_product(U + 1, H, H); // H + 1/2 -> H (even), H+1 -> H
    assert_product(H, -5, -2); // -2.5 -> -3, -2 (even) -> -2
    assert_product(-1, 4 * U + H, -4); // -4.5 -> -5, -4 (even) -> -4
    assert_product(-(U + 1), H, -H); // -(H + 1/2) -> -(H+1), -H (even) -> -H
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_004_product_ties_move_an_odd_truncation_to_even() {
    assert_product(H, 3, 2); // 3/2 = 1.5 -> 1, 2 (even) -> 2
    assert_product(1, 3 * U + H, 4); // 3.5 -> 3, 4 (even) -> 4
    assert_product(U + 1, 3 * H, 3 * H + 2); // 3H + 3/2 -> 3H+1, 3H+2 (even) -> 3H+2
    assert_product(H, -3, -2); // -1.5 -> -2 (even), -1 -> -2
    assert_product(-1, 3 * U + H, -4); // -3.5 -> -4 (even), -3 -> -4
    assert_product(-(U + 1), 3 * H, -(3 * H + 2)); // -(3H + 3/2) -> -(3H+2) (even) -> -(3H+2)
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_005_product_ties_at_the_widest_intermediates() {
    // Products near 2^95, the largest whose rounded result can still fit Q32.32.
    assert_product(i64::MAX, H, 1 << 62); // (2^63-1)/2 = 2^62 - 1/2 -> 2^62-1, 2^62 (even)
    assert_product(i64::MAX - 2, H, (1 << 62) - 2); // 2^62 - 3/2 -> 2^62-2 (even), 2^62-1
    assert_product(i64::MIN + 1, H, -(1 << 62)); // -(2^62 - 1/2) -> -2^62 (even)
    assert_product(i64::MIN + 3, H, 2 - (1 << 62)); // -(2^62 - 3/2) -> -(2^62-2) (even)

    // (2^62 + 2^30)(2^33 - 2) = 2^31 (2^64 - 1), so the exact result is 2^63 - 1/2. Its
    // neighbours are i64::MAX (odd) and 2^63 (even, not representable): checked_mul refuses
    // it and Mul saturates. Negated, the even neighbour -2^63 is i64::MIN, which fits.
    let (big, factor) =
        (FixedQ32_32::from_raw((1 << 62) + (1 << 30)), FixedQ32_32::from_raw((1 << 33) - 2));
    assert_eq!(big.checked_mul(factor), None, "2^63 - 1/2 rounds out of range");
    assert_eq!((big * factor).raw(), i64::MAX, "Mul saturates the out-of-range tie");
    assert_product(-((1 << 62) + (1 << 30)), (1 << 33) - 2, i64::MIN); // -(2^63 - 1/2) -> -2^63
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_006_quotients_below_half_keep_the_truncation() {
    // dividend * U / divisor is the exact raw quotient.
    assert_quotient(9, 4 * U, 2); // 9/4 = 2.25 -> 2, 3 -> 2
    assert_quotient(7, 3 * U, 2); // 7/3 = 2 + 1/3 -> 2, 3 -> 2
    assert_quotient(639, 1 << 40, 2); // 639/256 = 2.5 - 1/256 -> 2, 3 -> 2
    assert_quotient(-639, 1 << 40, -2); // -(2.5 - 1/256) -> -3, -2 -> -2
    assert_quotient(7, -3 * U, -2); // -(2 + 1/3) -> -3, -2 -> -2
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_007_quotients_above_half_leave_the_truncation() {
    assert_quotient(11, 4 * U, 3); // 11/4 = 2.75 -> 2, 3 -> 3
    assert_quotient(8, 3 * U, 3); // 8/3 = 2 + 2/3 -> 2, 3 -> 3
    assert_quotient(641, 1 << 40, 3); // 641/256 = 2.5 + 1/256 -> 2, 3 -> 3
    assert_quotient(-641, 1 << 40, -3); // -(2.5 + 1/256) -> -3, -2 -> -3
    assert_quotient(8, -3 * U, -3); // -(2 + 2/3) -> -3, -2 -> -3
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_008_quotient_ties_keep_an_even_truncation() {
    assert_quotient(5, 2 * U, 2); // 5/2 = 2.5 -> 2 (even), 3 -> 2
    assert_quotient(640, 1 << 40, 2); // 640/256 = 2.5 -> 2 (even)
    assert_quotient(15, 6 * U, 2); // 15/6 = 2.5 -> 2 (even)
    assert_quotient(-5, 2 * U, -2); // -2.5 -> -3, -2 (even) -> -2
    assert_quotient(15, -6 * U, -2); // -2.5 -> -2 (even)
    assert_quotient(-15, -6 * U, 2); // +2.5 -> 2 (even)
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_009_quotient_ties_move_an_odd_truncation_to_even() {
    assert_quotient(3, 2 * U, 2); // 3/2 = 1.5 -> 1, 2 (even) -> 2
    assert_quotient(21, 6 * U, 4); // 21/6 = 3.5 -> 3, 4 (even) -> 4
    assert_quotient(-3, 2 * U, -2); // -1.5 -> -2 (even), -1 -> -2
    assert_quotient(21, -6 * U, -4); // -3.5 -> -4 (even)
    assert_quotient(-21, -6 * U, 4); // +3.5 -> 4 (even)
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_010_quotient_ties_at_the_widest_operands() {
    // Dividends of magnitude near 2^63 shift to numerators near 2^95.
    assert_quotient(i64::MAX, 2 * U, 1 << 62); // 2^62 - 1/2 -> 2^62-1, 2^62 (even)
    assert_quotient(i64::MAX - 2, 2 * U, (1 << 62) - 2); // 2^62 - 3/2 -> 2^62-2 (even)
    assert_quotient(i64::MIN + 1, 2 * U, -(1 << 62)); // -(2^62 - 1/2) -> -2^62 (even)
    assert_quotient(i64::MAX, -2 * U, -(1 << 62)); // -(2^62 - 1/2) -> -2^62 (even)

    // The largest divisor magnitude, 2^63: 3 * 2^62 / -2^63 = -3/2 and 5 * 2^62 / -2^63 = -5/2.
    assert_quotient(3 << 30, i64::MIN, -2); // -1.5 -> -2 (even), -1
    assert_quotient(5 << 30, i64::MIN, -2); // -2.5 -> -3, -2 (even)
    assert_quotient(-(5 << 30), i64::MIN, 2); // +2.5 -> 2 (even)

    // An odd divisor never ties: 5 * 2^62 / (2^63 - 1) = 5/2 + 5/(2 (2^63 - 1)), just above.
    assert_quotient(5 << 30, i64::MAX, 3); // 2.5 + tiny -> 2, 3 -> 3
    assert_quotient(-(5 << 30), i64::MAX, -3); // -(2.5 + tiny) -> -3
}

/// `n * 2^-34`, exactly representable as an `f32` for these small `n`: raw `n / 4`.
fn quarter_units(n: i16) -> f32 {
    f32::from(n) / 17_179_869_184.0
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_011_float_ingress_rounds_ties_to_even() {
    // (n, raw): n/4 -> neighbours -> raw.
    let cases = [
        (5, 1),    // 1.25 below half -> 1
        (7, 2),    // 1.75 above half -> 2
        (2, 0),    // 0.5 tie -> 0 (even), 1
        (6, 2),    // 1.5 tie -> 1, 2 (even)
        (10, 2),   // 2.5 tie -> 2 (even), 3
        (-5, -1),  // -1.25 -> -1
        (-7, -2),  // -1.75 -> -2
        (-6, -2),  // -1.5 tie -> -2 (even), -1
        (-10, -2), // -2.5 tie -> -3, -2 (even)
    ];
    for (n, raw) in cases {
        assert_eq!(from_f32(quarter_units(n)), raw, "from_f32 of {n} * 2^-34");
        assert_eq!(try_from_f32(quarter_units(n)), Ok(raw), "try_from_f32 of {n} * 2^-34");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn fpr_tp_012_float_egress_rounds_ties_to_even() {
    // Raw values in [2^24, 2^26) lose one or two bits to the 24-bit significand. 2^24 raw is
    // 2^-8 (f32 bits 0x3B80_0000); 2^25 raw is 2^-7 (0x3C00_0000); the low bits count
    // significand steps of 2 and 4 raw units.
    let cases = [
        ((1 << 24) + 1, 0x3B80_0000), // step 0.5 tie -> 0 (even), 1
        ((1 << 24) + 3, 0x3B80_0002), // step 1.5 tie -> 1, 2 (even)
        ((1 << 24) + 5, 0x3B80_0002), // step 2.5 tie -> 2 (even), 3
        ((1 << 25) + 1, 0x3C00_0000), // step 0.25 below half -> 0
        ((1 << 25) + 3, 0x3C00_0001), // step 0.75 above half -> 1
    ];
    for (raw, bits) in cases {
        let raw: i64 = raw;
        assert_eq!(to_f32(raw).to_bits(), bits, "to_f32({raw})");
        assert_eq!(to_f32(-raw).to_bits(), bits | 0x8000_0000, "to_f32(-{raw})");
    }
}
