use bunny_codec::{parse_binary_ply, parse_obj_text, ObjVertex, PlyVertex};
use bunny_mesh::Triangle32;
use wasm_bindgen_test::wasm_bindgen_test;

const STANFORD_BUNNY_OBJ_SUBSET: &str = include_str!("fixtures/stanford_bunny_subset.obj");
const STANFORD_BUNNY_PLY_HEADER: &str = concat!(
    "ply\n",
    "format binary_little_endian 1.0\n",
    "comment derived from Stanford bunny/reconstruction/bun_zipper.ply\n",
    "element vertex 3\n",
    "property float x\n",
    "property float y\n",
    "property float z\n",
    "element face 1\n",
    "property list uchar int vertex_indices\n",
    "end_header\n",
);
const STANFORD_BUNNY_PLY_VERTEX_BYTES: &[u8] = &[
    85, 15, 189, 189, 110, 138, 7, 62, 185, 70, 149, 60, 249, 200, 188, 189, 60, 134, 7, 62, 237,
    24, 141, 60, 214, 252, 189, 189, 196, 34, 6, 62, 60, 40, 141, 60,
];
const STANFORD_BUNNY_PLY_FACE_BYTES: &[u8] = &[3, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0];

fn stanford_bunny_binary_ply_subset() -> Vec<u8> {
    let mut bytes = Vec::from(STANFORD_BUNNY_PLY_HEADER.as_bytes());
    bytes.extend_from_slice(STANFORD_BUNNY_PLY_VERTEX_BYTES);
    bytes.extend_from_slice(STANFORD_BUNNY_PLY_FACE_BYTES);
    bytes
}

#[wasm_bindgen_test(unsupported = test)]
fn parses_stanford_bunny_obj_subset_fixture() {
    let mesh =
        parse_obj_text(STANFORD_BUNNY_OBJ_SUBSET).expect("Stanford Bunny OBJ subset should parse");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.triangle(0), Ok(Triangle32::new(0, 1, 2)));
    assert_eq!(
        mesh.vertex(0),
        Ok(ObjVertex {
            x: -8.656_194e-2,
            y: 1.424_918_8e-1,
            z: 8.432_68e-3,
        })
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn parses_stanford_bunny_binary_ply_subset_fixture() {
    let bytes = stanford_bunny_binary_ply_subset();
    let mesh = parse_binary_ply(&bytes).expect("Stanford Bunny PLY subset should parse");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.triangle(0), Ok(Triangle32::new(0, 1, 2)));
    assert_eq!(
        mesh.vertex(0),
        Ok(PlyVertex {
            x: -0.092_314_4,
            y: 0.132_364,
            z: 0.018_222_2,
        })
    );
}
