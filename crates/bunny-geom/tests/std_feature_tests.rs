//! The default `std` build: the error types are `std` errors, as before the feature existed.

use bunny_geom::{GeomError, PolygonClipError};
use wasm_bindgen_test::wasm_bindgen_test;

fn as_std_error<E: std::error::Error + 'static>(error: E) -> Box<dyn std::error::Error> {
    Box::new(error)
}

#[wasm_bindgen_test(unsupported = test)]
fn no_std_tp_015_geom_error_types_are_std_errors_with_their_messages() {
    let cases: [(Box<dyn std::error::Error>, &str); 2] = [
        (as_std_error(GeomError::InvalidAabbBounds), "AABB min boundary exceeds max boundary"),
        (as_std_error(PolygonClipError::TooFewVertices), "polygon has fewer than three vertices"),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{message}");
    }
}
