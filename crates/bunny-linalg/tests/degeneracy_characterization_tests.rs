//! CHARACTERIZATION test for flyingrobots/bunny#116, confirming flyingrobots/bunny#483.
//!
//! It pins current behaviour, not an endorsed contract: `FixedMat3::try_inverse` and
//! `FixedAffine3::try_inverse` (the inverse the `bunny-geom` transforms are built on) test the
//! ROUNDED determinant for zero, so an exactly singular matrix whose determinant rounds to a
//! nonzero raw value is inverted. Changing this is the decision flyingrobots/bunny#483 asks
//! the repository owner for; update this test with it.

use bunny_linalg::{FixedAffine3, FixedMat3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;

fn v(raw: [i64; 3]) -> FixedVec3 {
    let q = FixedQ32_32::from_raw;
    FixedVec3::new(q(raw[0]), q(raw[1]), q(raw[2]))
}

/// Known defect, flyingrobots/bunny#483. Rows `(2^32, 2^32, 2^32)`, `(2^31, 2^30, 2^29)` and
/// `(12, 10, 9)` raw: the third row is `8 / 2^32` times the sum of the first two, so the
/// matrix is exactly singular, but its determinant rounds to raw 1 and an inverse is returned.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_080_characterization_singular_matrix_is_inverted() {
    let first = [ONE; 3];
    let second = [ONE / 2, ONE / 4, ONE / 8];
    // Exact check that the third row is (8 / 2^32) (first + second).
    let third = [12, 10, 9];
    for ((a, b), c) in first.iter().zip(&second).zip(&third) {
        assert_eq!(i128::from(*c) << 32, 8 * (i128::from(*a) + i128::from(*b)));
    }
    let singular = FixedMat3::from_rows(v(first), v(second), v(third));
    assert_eq!(singular.determinant(), Some(FixedQ32_32::from_raw(1)));
    assert!(singular.try_inverse().is_some(), "flyingrobots/bunny#483: inverted though singular");
    let affine = FixedAffine3::from_parts(singular, v([0; 3]));
    assert!(affine.try_inverse().is_some(), "the affine inverse takes the same test");
}
