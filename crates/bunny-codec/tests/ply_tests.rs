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
fn rejects_non_triangular_faces() {
    let mut bytes = canonical_triangle_ply();
    let face_start = HEADER.len() + VERTEX_BYTES.len();
    bytes[face_start] = 4;

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
fn rejects_trailing_payload_bytes() {
    let mut bytes = canonical_triangle_ply();
    bytes.push(0);

    assert_eq!(parse_binary_ply(&bytes), Err(PlyError::TrailingData));
}
