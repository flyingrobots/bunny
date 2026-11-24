//! The triangle limit, which only inputs of at least 71 582 788 384 bytes reach.

use super::{check_layout, StlError, BINARY_STL_MAX_TRIANGLES};
use wasm_bindgen_test::wasm_bindgen_test;

/// Inputs this long cannot be built in a test, so the check is called directly with the
/// lengths an input of each count would have.
#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_017_the_triangle_limit_applies_after_the_length_rule() {
    let exact = |n: u32| 84 + 50 * u64::from(n);
    assert_eq!(exact(BINARY_STL_MAX_TRIANGLES + 1), 71_582_788_384);
    assert_eq!(check_layout(BINARY_STL_MAX_TRIANGLES, exact(BINARY_STL_MAX_TRIANGLES)), Ok(()));
    assert_eq!(check_layout(0, 84), Ok(()));
    for triangles in [BINARY_STL_MAX_TRIANGLES + 1, BINARY_STL_MAX_TRIANGLES + 2, u32::MAX] {
        let refused = check_layout(triangles, exact(triangles));
        assert_eq!(refused, Err(StlError::TooManyTriangles { triangles }));
        for actual in [exact(triangles) - 1, exact(triangles) + 1, 84] {
            let expected = exact(triangles);
            let mismatch = StlError::LengthMismatch { triangles, expected, actual };
            assert_eq!(check_layout(triangles, actual), Err(mismatch));
        }
    }
    assert_eq!(
        check_layout(BINARY_STL_MAX_TRIANGLES, u64::MAX),
        Err(StlError::LengthMismatch {
            triangles: BINARY_STL_MAX_TRIANGLES,
            expected: exact(BINARY_STL_MAX_TRIANGLES),
            actual: u64::MAX,
        })
    );
}
