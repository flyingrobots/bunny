use bunny_codec::{parse_binary_ply, PlyError, PlyVertex};
use bunny_mesh::Triangle32;
use wasm_bindgen_test::wasm_bindgen_test;

const HEADER: &str = concat!(
    "ply\n",
    "format binary_little_endian 1.0\n",
    "element vertex 3\n",
    "property float x\n",
    "property float y\n",
    "property float z\n",
    "element face 1\n",
    "property list uchar int vertex_indices\n",
    "end_header\n",
);
const VERTEX_BYTES: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    128, 63, 0, 0, 0, 0,
];
const FACE_BYTES: &[u8] = &[3, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0];

fn canonical_triangle_ply() -> Vec<u8> {
    let mut bytes = Vec::from(HEADER.as_bytes());
    bytes.extend_from_slice(VERTEX_BYTES);
    bytes.extend_from_slice(FACE_BYTES);
    bytes
}

fn triangle_ply_with_header(header: &str) -> Vec<u8> {
    let mut bytes = Vec::from(header.as_bytes());
    bytes.extend_from_slice(VERTEX_BYTES);
    bytes.extend_from_slice(FACE_BYTES);
    bytes
}

#[wasm_bindgen_test(unsupported = test)]
fn parses_binary_triangle_ply_as_borrowed_view() {
    let bytes = canonical_triangle_ply();
    let mesh = parse_binary_ply(&bytes).expect("canonical binary PLY should parse");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.vertex_bytes(), VERTEX_BYTES);
    assert_eq!(mesh.face_bytes(), FACE_BYTES);
    assert_eq!(
        mesh.vertex(1),
        Ok(PlyVertex {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        })
    );
    assert_eq!(mesh.triangle(0), Ok(Triangle32::new(0, 1, 2)));

    let vertex_ptr = bytes.as_ptr().wrapping_add(HEADER.len());
    assert_eq!(mesh.vertex_bytes().as_ptr(), vertex_ptr);
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_non_binary_little_endian_ply() {
    let bytes = b"ply\nformat ascii 1.0\nend_header\n";
    assert_eq!(parse_binary_ply(bytes), Err(PlyError::UnsupportedFormat));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_duplicate_or_late_ply_format_directives() {
    let duplicate_format = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "format binary_little_endian 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    assert_eq!(
        parse_binary_ply(&triangle_ply_with_header(duplicate_format)),
        Err(PlyError::UnsupportedFormat)
    );

    let late_format = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "element vertex 3\n",
        "format binary_little_endian 1.0\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    assert_eq!(
        parse_binary_ply(&triangle_ply_with_header(late_format)),
        Err(PlyError::UnsupportedFormat)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn matches_end_header_only_as_header_line() {
    let header = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "comment contains end_header\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    let bytes = triangle_ply_with_header(header);

    assert_eq!(
        parse_binary_ply(&bytes)
            .expect("commented terminator text should not split header")
            .face_count(),
        1
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn skips_properties_for_empty_auxiliary_elements() {
    let header = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 0\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    let bytes = triangle_ply_with_header(header);

    assert_eq!(
        parse_binary_ply(&bytes)
            .expect("zero-count auxiliary element should not require payload")
            .face_count(),
        1
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn accepts_bare_comment_and_obj_info_header_lines() {
    let header = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "comment\n",
        "obj_info\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    let bytes = triangle_ply_with_header(header);

    assert_eq!(
        parse_binary_ply(&bytes)
            .expect("bare comment metadata should be ignored")
            .face_count(),
        1
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_out_of_order_or_duplicate_ply_elements() {
    let face_before_vertex = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "end_header\n",
    );
    assert_eq!(
        parse_binary_ply(&triangle_ply_with_header(face_before_vertex)),
        Err(PlyError::MissingVertexElement)
    );

    let duplicate_vertex = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    assert_eq!(
        parse_binary_ply(&triangle_ply_with_header(duplicate_vertex)),
        Err(PlyError::UnsupportedElement)
    );

    let duplicate_face = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    );
    assert_eq!(
        parse_binary_ply(&triangle_ply_with_header(duplicate_face)),
        Err(PlyError::UnsupportedElement)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_non_triangular_faces() {
    let mut bytes = canonical_triangle_ply();
    let face_start = HEADER.len() + VERTEX_BYTES.len();
    bytes[face_start] = 4;

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::NonTriangularFace));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_polygon_ply_payloads_as_non_triangular_faces() {
    let mut bytes = canonical_triangle_ply();
    let face_start = HEADER.len() + VERTEX_BYTES.len();
    bytes[face_start] = 4;
    bytes.extend_from_slice(&3_i32.to_le_bytes());

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::NonTriangularFace));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_negative_face_indices() {
    let mut bytes = canonical_triangle_ply();
    let first_index_start = HEADER.len() + VERTEX_BYTES.len() + 1;
    bytes[first_index_start..first_index_start + 4].copy_from_slice(&(-1_i32).to_le_bytes());

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::NegativeIndex));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_out_of_bounds_face_indices() {
    let mut bytes = canonical_triangle_ply();
    let third_index_start = HEADER.len() + VERTEX_BYTES.len() + 9;
    bytes[third_index_start..third_index_start + 4].copy_from_slice(&3_i32.to_le_bytes());

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::IndexOutOfBounds));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_non_finite_vertices() {
    let mut nan_bytes = canonical_triangle_ply();
    let vertex_start = HEADER.len();
    nan_bytes[vertex_start..vertex_start + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(parse_binary_ply(&nan_bytes), Err(PlyError::NonFiniteVertex));

    let mut infinity_bytes = canonical_triangle_ply();
    infinity_bytes[vertex_start + 4..vertex_start + 8]
        .copy_from_slice(&f32::INFINITY.to_le_bytes());
    assert_eq!(
        parse_binary_ply(&infinity_bytes),
        Err(PlyError::NonFiniteVertex)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_trailing_payload_bytes() {
    let mut bytes = canonical_triangle_ply();
    bytes.push(0);

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::TrailingData));
}
