//! Integration tests.

use bunny_codec::{parse_obj_text, ObjError, ObjVertex};
use bunny_mesh::Triangle32;
use wasm_bindgen_test::wasm_bindgen_test;

const OBJ_TRIANGLE: &str = "\
# canonical triangle
o triangle
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vn 0.0 0.0 1.0
f 1/1/1 2/1/1 3/1/1
";

#[wasm_bindgen_test(unsupported = test)]
fn parses_obj_triangle_as_borrowed_view() {
    let mesh = parse_obj_text(OBJ_TRIANGLE).expect("canonical OBJ should parse");

    assert_eq!(mesh.source().as_ptr(), OBJ_TRIANGLE.as_ptr());
    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.vertex(2), Ok(ObjVertex { x: 0.0, y: 1.0, z: 0.0 }));
    assert_eq!(mesh.triangle(0), Ok(Triangle32::new(0, 1, 2)));
}

#[wasm_bindgen_test(unsupported = test)]
fn iterates_obj_records_in_source_order() {
    let mesh = parse_obj_text(OBJ_TRIANGLE).expect("canonical OBJ should parse");

    let vertices: Result<Vec<_>, _> = mesh.vertices().collect();
    assert_eq!(
        vertices,
        Ok(vec![
            ObjVertex { x: 0.0, y: 0.0, z: 0.0 },
            ObjVertex { x: 1.0, y: 0.0, z: 0.0 },
            ObjVertex { x: 0.0, y: 1.0, z: 0.0 },
        ])
    );

    let triangles: Result<Vec<_>, _> = mesh.triangles().collect();
    assert_eq!(triangles, Ok(vec![Triangle32::new(0, 1, 2)]));
}

#[wasm_bindgen_test(unsupported = test)]
fn obj_accessors_reject_out_of_range_indices() {
    let mesh = parse_obj_text(OBJ_TRIANGLE).expect("canonical OBJ should parse");

    assert_eq!(mesh.vertex(mesh.vertex_count()), Err(ObjError::IndexOutOfBounds));
    assert_eq!(mesh.triangle(mesh.face_count()), Err(ObjError::IndexOutOfBounds));
}

#[wasm_bindgen_test(unsupported = test)]
fn parses_obj_records_with_inline_comments() {
    let obj = "\
v 0.0 0.0 0.0 # origin
v 1.0 0.0 0.0 # x
v 0.0 1.0 0.0 # y
f 1 2 3 # cap
";
    let mesh = parse_obj_text(obj).expect("inline comments should be ignored");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.triangle(0), Ok(Triangle32::new(0, 1, 2)));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_non_triangular_obj_faces() {
    let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
f 1 2 3 4
";
    assert_eq!(parse_obj_text(obj), Err(ObjError::NonTriangularFace));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_out_of_bounds_obj_indices() {
    let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 4
";
    assert_eq!(parse_obj_text(obj), Err(ObjError::IndexOutOfBounds));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_non_finite_obj_vertices() {
    let nan = "\
v NaN 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
    assert_eq!(parse_obj_text(nan), Err(ObjError::NonFiniteVertex));

    let infinity = "\
v 0.0 inf 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
    assert_eq!(parse_obj_text(infinity), Err(ObjError::NonFiniteVertex));
}

#[wasm_bindgen_test(unsupported = test)]
fn handles_extreme_obj_float_exponents_without_panicking() {
    let underflow = "\
v 1.23e-9999999999 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
    let mesh = parse_obj_text(underflow).expect("extreme negative exponent should underflow");

    assert_eq!(mesh.vertex(0), Ok(ObjVertex { x: 0.0, y: 0.0, z: 0.0 }));

    let overflow = "\
v 1e9999999999 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
    assert_eq!(parse_obj_text(overflow), Err(ObjError::NonFiniteVertex));
}

#[wasm_bindgen_test(unsupported = test)]
fn parses_large_finite_obj_mantissas() {
    let coordinate = format!("1{}e-400", "0".repeat(400));
    let obj = format!(
        "\
v {coordinate} 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"
    );
    let mesh = parse_obj_text(&obj).expect("scaled large mantissa should parse");

    assert_eq!(mesh.vertex(0), Ok(ObjVertex { x: 1.0, y: 0.0, z: 0.0 }));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_relative_or_zero_obj_indices() {
    let negative = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 -1
";
    assert_eq!(parse_obj_text(negative), Err(ObjError::InvalidIndex));

    let zero = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 0
";
    assert_eq!(parse_obj_text(zero), Err(ObjError::InvalidIndex));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_malformed_obj_slash_face_tokens() {
    let malformed_faces = [
        "f 1/not-a-number/not-a-number 2/1/1 3/1/1",
        "f 1/2/3/4 2/1/1 3/1/1",
        "f 1/ 2/1/1 3/1/1",
        "f 1// 2/1/1 3/1/1",
    ];

    for face in malformed_faces {
        let obj = format!(
            "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
{face}
"
        );

        assert_eq!(parse_obj_text(&obj), Err(ObjError::InvalidIndex));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_unsupported_obj_statements() {
    let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
curv 0.0 1.0 1 2
f 1 2 3
";
    assert_eq!(parse_obj_text(obj), Err(ObjError::UnsupportedStatement));
}
