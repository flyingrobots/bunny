//! Rules of the ASCII PLY reader that its chapter states and no other test pinned
//! (flyingrobots/bunny#323): the certain-overflow and certain-underflow shortcuts of the
//! decimal conversion, the type check of skipped vertex properties, and the three places
//! where the chapter and the code disagreed (which line a missing coordinate is reported on,
//! which kind a malformed face count has, and the kind for a face element before the vertex
//! element). The chapter now states what the code does; each statement is pinned here.

use bunny_codec::{parse_ascii_ply, parse_decimal_q32_32, AsciiPlyErrorKind as Kind, DecimalError};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

/// A one-vertex, one-face mesh: line 1 `ply`, 2 `format`, 3 `element vertex`, 4 to 6 `x`, `y`,
/// `z`, 7 `element face`, 8 the list property, 9 `end_header`, 10 the vertex, 11 the face.
const MESH: &str = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\n\
property float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n\
0 0 0\n3 0 0 0\n";

fn edit(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "{from:?} must occur exactly once");
    text.replace(from, to)
}

fn refusal(text: &str) -> (Kind, usize) {
    let error = parse_ascii_ply(text.as_bytes()).expect_err("input should be refused");
    (error.kind(), error.line())
}

/// Thirty-two significant digits do not fit the working integer (`TooLong`) unless the value
/// is certainly out of range or certainly zero, which is decided from its magnitude first.
/// The boundary values sit on both sides of each shortcut.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_061_the_magnitude_shortcuts_decide_before_the_digit_count() {
    let long = |text: &str| parse_decimal_q32_32(text.as_bytes());
    // At least 1e10 (11 integer digits), 32 digits: Overflow, both signs, with an exponent too.
    for text in [
        "12345678901.234567890123456789012",
        "-12345678901.234567890123456789012",
        "10000000000.000000000000000000001",
        "1.2345678901234567890123456789012e10",
    ] {
        assert_eq!(long(text), Err(DecimalError::Overflow), "{text}");
    }
    // Just below 1e10 the value is not certainly out of range, so the digit count decides.
    assert_eq!(long("9999999999.123456789012345678901234"), Err(DecimalError::TooLong));
    // Below 1e-10 the value is certainly zero whatever its length; at 1.2e-10 it is not.
    for text in [
        "0.000000000012345678901234567890123456789",
        "-0.000000000012345678901234567890123456789",
        "1.2345678901234567890123456789012345e-11",
        "0.0000000000999999999999999999999999999999",
    ] {
        assert_eq!(long(text), Ok(FixedQ32_32::ZERO), "{text}");
    }
    assert_eq!(
        long(&format!("0.000000000{}", "12345678901234567890123456789012")),
        Err(DecimalError::TooLong)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_062_a_skipped_vertex_property_needs_a_scalar_type() {
    // Every other test declares extra properties with a known type; an unknown one is refused
    // at its line, and so is a list property, whatever the name.
    for (declaration, line) in [
        ("property foo red", 7),
        ("property uchar2 red", 7),
        ("property list uchar int red", 7),
        ("property red", 7),
    ] {
        let text = edit(MESH, "property float z\n", &format!("property float z\n{declaration}\n"));
        assert_eq!(refusal(&text), (Kind::UnsupportedProperty, line), "{declaration}");
    }
    // Each scalar type is accepted, and the column is counted, never read.
    for kind in [
        "char", "uchar", "short", "ushort", "int", "uint", "float", "double", "int8", "uint8",
        "int16", "uint16", "int32", "uint32", "float32", "float64",
    ] {
        let text =
            edit(MESH, "property float z\n", &format!("property float z\nproperty {kind} red\n"));
        let text = edit(&text, "end_header\n0 0 0\n", "end_header\n0 0 0 not-a-number\n");
        let mesh = parse_ascii_ply(text.as_bytes()).unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert_eq!(mesh.vertex_count(), 1, "{kind}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_063_a_missing_coordinate_is_reported_where_the_header_gives_it_up() {
    // The face element comes before x, y and z are all declared: the `element face` line.
    let no_z = edit(MESH, "property float z\n", "");
    assert_eq!(refusal(&no_z), (Kind::MissingVertexElement, 6));
    let no_x = edit(MESH, "property float x\n", "");
    assert_eq!(refusal(&no_x), (Kind::MissingVertexElement, 6));
    // With no face element after the incomplete vertex element, the `end_header` line.
    let no_face = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\n\
end_header\n0 0\n";
    assert_eq!(refusal(no_face), (Kind::MissingVertexElement, 6));
    // No vertex element at all, no face element: also the `end_header` line.
    let none = "ply\nformat ascii 1.0\nend_header\n";
    assert_eq!(refusal(none), (Kind::MissingVertexElement, 3));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_064_a_face_count_that_is_not_digits_is_an_invalid_index() {
    let face = |count: &str| edit(MESH, "3 0 0 0\n", &format!("{count} 0 0 0\n"));
    // Not a run of digits: the kind named for the count is `InvalidIndex`.
    for count in ["+3", "3.0", "x", "-3", "0x3", "3e0", "٣"] {
        assert_eq!(refusal(&face(count)), (Kind::InvalidIndex, 11), "{count:?}");
    }
    // A run of digits that is not the integer 3: `NonTriangularFace`, however long.
    for count in ["0", "1", "2", "4", "30", "99999999999999999999999999"] {
        assert_eq!(refusal(&face(count)), (Kind::NonTriangularFace, 11), "{count:?}");
    }
    // Leading zeros are still the decimal integer 3.
    for count in ["3", "03", "0003"] {
        assert!(parse_ascii_ply(face(count).as_bytes()).is_ok(), "{count:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_065_a_face_element_before_the_vertex_element_is_a_missing_vertex_element() {
    let reordered = "ply\nformat ascii 1.0\nelement face 1\nproperty list uchar int vertex_indices\n\
element vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n3 0 0 0\n0 0 0\n";
    assert_eq!(refusal(reordered), (Kind::MissingVertexElement, 3));
    // A repeated vertex or face element is the other ordering error, `UnsupportedElement`.
    let twice = edit(MESH, "element face 1\n", "element vertex 1\nelement face 1\n");
    assert_eq!(refusal(&twice), (Kind::UnsupportedElement, 7));
}
