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
    assert_eq!(
        mesh.vertex(2),
        Ok(ObjVertex {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        })
    );
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
