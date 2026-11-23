//! The rounding of the weight's product, pinned on values derived by hand. Oracle: the
//! arithmetic written in the comments.

use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

use super::weight;

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_160_the_weight_is_rounded_to_nearest_ties_to_even() {
    let raw = |fresnel: i64, ratio: i128| weight(FixedQ32_32::from_raw(fresnel), ratio).raw();
    // fresnel * ratio / 2^32 with fresnel = 1/2: ratio / 2.
    // Ties: 0.5 -> 0, 1.5 -> 2, 2.5 -> 2 (half up gives 1, 2, 3; a floor 0, 1, 2).
    assert_eq!([1, 3, 5].map(|ratio| raw(1 << 31, ratio)), [0, 2, 2]);
    // Just above and just below one half.
    assert_eq!(raw((1 << 31) + 1, 1), 1);
    assert_eq!(raw((1 << 31) - 1, 1), 0);
    // 1.0 leaves the other factor unchanged.
    assert_eq!(raw(1 << 32, 2_863_311_531), 2_863_311_531);
    assert_eq!(raw(3_694_871_552, 1 << 32), 3_694_871_552);
}
