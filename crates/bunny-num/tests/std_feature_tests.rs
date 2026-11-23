//! The default `std` build: the error types are `std` errors, as before the feature existed.

use bunny_num::{DivisionByZero, EllipseDomainError, FloatConversionError};
use wasm_bindgen_test::wasm_bindgen_test;

fn as_std_error<E: std::error::Error + 'static>(error: E) -> Box<dyn std::error::Error> {
    Box::new(error)
}

#[wasm_bindgen_test(unsupported = test)]
fn no_std_tp_006_error_types_are_std_errors_with_their_messages() {
    let cases: [(Box<dyn std::error::Error>, &str); 4] = [
        (as_std_error(FloatConversionError::NonFinite), "float value is not finite"),
        (as_std_error(FloatConversionError::OutOfRange), "float value is outside the Q32.32 range"),
        (as_std_error(DivisionByZero), "division by zero in bunny-u32-div-round-even/1"),
        (
            as_std_error(EllipseDomainError),
            "operand exceeds 46,340 maximum in bunny-u32-ellipse-classify/1",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{message}");
    }
}
