//! The default `std` build: the error types are `std` errors, as before the feature existed.

use bunny_query::{
    ShapeFrameError, TraceBoxError, TraceCylinderError, TraceDiscError, TraceSphereError,
};
use wasm_bindgen_test::wasm_bindgen_test;

fn as_std_error<E: std::error::Error + 'static>(error: E) -> Box<dyn std::error::Error> {
    Box::new(error)
}

#[wasm_bindgen_test(unsupported = test)]
fn no_std_tp_015_query_error_types_are_std_errors_with_their_messages() {
    let cases: [(Box<dyn std::error::Error>, &str); 5] = [
        (as_std_error(ShapeFrameError::AxesNotOrthogonal), "frame axes are not orthogonal"),
        (as_std_error(TraceBoxError::MinimumAboveMaximum), "box minimum is above its maximum"),
        (as_std_error(TraceCylinderError::RadiusNotPositive), "cylinder radius is not positive"),
        (as_std_error(TraceDiscError::RadiusNotPositive), "disc radius is not positive"),
        (as_std_error(TraceSphereError::RadiusNotPositive), "sphere radius is not positive"),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{message}");
    }
}
