//! The rounding of the Smith roots and of the masking ratio, pinned on values derived by hand
//! in each case. Oracle: the arithmetic written in the comments.

use wasm_bindgen_test::wasm_bindgen_test;

use super::{masking_ratio, smith_root};
use crate::shading::wide::ONE;

const HALF: i128 = 1 << 31;

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_158_the_smith_root_is_floored_in_units_of_two_to_the_minus_sixty() {
    // Roughness 1.0 or cosine 1.0: the radicand is 1, so the root is 2^60.
    assert_eq!([0, 1, HALF, ONE].map(|c| smith_root(ONE, c)), [1 << 60; 4]);
    assert_eq!([1 << 16, HALF, ONE].map(|alpha| smith_root(alpha, ONE)), [1 << 60; 3]);
    // Cosine 0: the root is the roughness itself, exactly.
    assert_eq!(smith_root(HALF, 0), 1 << 59);
    assert_eq!(smith_root(1 << 16, 0), 1 << 44);
    // Roughness 1/2, cosine 1/2: 1/4 + (3/4)(1/4) = 7/16, so the root is 2^58 sqrt(7) =
    // 762 585 895 592 031 451.78...: the floor is the one integer r with
    // r^2 <= 7 * 2^116 < (r + 1)^2; to nearest it would be r + 1.
    let root = smith_root(HALF, HALF);
    assert_eq!(root, 762_585_895_592_031_451);
    let (square, radicand) = (root.unsigned_abs().pow(2), 7_u128 << 116);
    assert!(square <= radicand && radicand - square <= 2 * root.unsigned_abs());
    assert!(2 * (radicand - square) > 2 * root.unsigned_abs() + 1, "its fraction is above 1/2");
    // Out-of-range arguments are clamped into range first.
    assert_eq!(smith_root(0, 0), 1 << 44);
    assert_eq!(smith_root(2 * ONE, -1), 1 << 60);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_159_the_masking_ratio_is_rounded_to_nearest_ties_to_even() {
    // At roughness 1.0 both roots are 1.0 and the ratio is c_l (c_v + 1) / (c_l + c_v); with
    // raw cosines that is c_l (c_v + 2^32) / (c_l + c_v) raw units.
    // c_v = 1.0, c_l = 1/2: 2/3, raw 2 863 311 530.67 (a floor gives ...530).
    assert_eq!(masking_ratio(ONE, ONE, HALF), 2_863_311_531);
    // c_v = c_l = 1 raw unit: (2^32 + 1) / 2 = 2 147 483 648.5, a tie: to even, ...648 (half
    // up gives ...649).
    assert_eq!(masking_ratio(ONE, 1, 1), 2_147_483_648);
    // c_v = c_l = 3 raw units: (2^32 + 3) / 2 = 2 147 483 649.5, a tie: to even, ...650 (a
    // floor, or half down, gives ...649).
    assert_eq!(masking_ratio(ONE, 3, 3), 2_147_483_650);
    // c_v = 3, c_l = 1: (2^32 + 3) / 4 = 1 073 741 824.75.
    assert_eq!(masking_ratio(ONE, 3, 1), 1_073_741_825);
    // Equal directions at normal incidence: exactly 1.0, for any roughness.
    assert_eq!([1 << 16, HALF, ONE].map(|alpha| masking_ratio(alpha, ONE, ONE)), [ONE; 3]);
    // Roughness 1/2, c_v = 1.0, c_l = 1/2: s(c_v) = 1 and s(c_l) = sqrt(7) / 4, so the ratio
    // is 1 / (1/2 + sqrt(7)/4) = 4 (sqrt(7) - 2) / 3 = 0.861 001 748..., raw
    // 3 697 974 349.83.
    assert_eq!(masking_ratio(HALF, ONE, HALF), 3_697_974_350);
    // A cosine of zero or below gives zero.
    assert_eq!(masking_ratio(HALF, ONE, 0), 0);
    assert_eq!(masking_ratio(HALF, 0, 0), 0);
    assert_eq!(masking_ratio(HALF, ONE, -5), 0);
}
