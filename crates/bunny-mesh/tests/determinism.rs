//! Integration tests.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{compute_mesh_hash, IndexBufferLayout, QuantizedVertex, Triangle16};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const EXPECTED_HASH: [u8; 32] = [
    0x5b, 0x23, 0x53, 0xc8, 0x85, 0x1c, 0x81, 0x7a, 0x66, 0x2d, 0xc3, 0x8e, 0x8b, 0xa9, 0x4f, 0x91,
    0x2a, 0xf6, 0x38, 0x10, 0x30, 0xa2, 0x5b, 0xf3, 0xaa, 0x95, 0x99, 0xbc, 0xe8, 0x96, 0xa9, 0xfd,
];

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

#[wasm_bindgen_test(unsupported = test)]
fn mesh_hash_golden_vector_bytes_are_stable() {
    let bounds = FixedAabb3::new(vec3(0, 0, 0), vec3(1, 1, 1));
    let vertices =
        [QuantizedVertex::new(0, 32_768, u16::MAX), QuantizedVertex::new(u16::MAX, 0, 32_768)];
    let faces = [Triangle16::new(0, 1, 0)];

    let hash = compute_mesh_hash(&vertices, IndexBufferLayout::Width16(&faces), &bounds);

    assert_eq!(hash, EXPECTED_HASH);
}
