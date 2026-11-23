//! The default `std` build: the error types are `std` errors, as before the feature existed.

use bunny_linalg::{CatmullRomError, LookAtError, ProjectionError, ViewportError};
use wasm_bindgen_test::wasm_bindgen_test;

fn as_std_error<E: std::error::Error + 'static>(error: E) -> Box<dyn std::error::Error> {
    Box::new(error)
}

#[wasm_bindgen_test(unsupported = test)]
fn no_std_tp_015_linalg_error_types_are_std_errors_with_their_messages() {
    let cases: [(Box<dyn std::error::Error>, &str); 4] = [
        (as_std_error(CatmullRomError::TooFewPoints), "too few points for a Catmull-Rom spline"),
        (
            as_std_error(LookAtError::CoincidentEyeTarget),
            "look-at eye and target are the same point",
        ),
        (as_std_error(ProjectionError::NonPositiveExtent), "projection extent must be positive"),
        (as_std_error(ViewportError::EmptyExtent), "viewport width and height must be positive"),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{message}");
    }
}
