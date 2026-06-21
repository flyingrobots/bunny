//! Integration tests.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{dequantize_vertex, quantize_vertex, QuantizedVertex};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

fn mesh_hash_bounds(min: f32, max: f32) -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(min),
            FixedQ32_32::from_f32(min),
            FixedQ32_32::from_f32(min),
        ),
        FixedVec3::new(
            FixedQ32_32::from_f32(max),
            FixedQ32_32::from_f32(max),
            FixedQ32_32::from_f32(max),
        ),
    )
}

#[wasm_bindgen_test(unsupported = test)]
fn test_quantization_boundaries() {
    let bounds = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(2.0),
            FixedQ32_32::from_f32(3.0),
        ),
        FixedVec3::new(
            FixedQ32_32::from_f32(11.0),
            FixedQ32_32::from_f32(22.0),
            FixedQ32_32::from_f32(33.0),
        ),
    );

    // Quantize min point
    let min_p = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::from_f32(3.0),
    );
    let q_min = quantize_vertex(min_p, &bounds);
    assert_eq!(q_min, QuantizedVertex::new(0, 0, 0));

    // Quantize max point
    let max_p = FixedVec3::new(
        FixedQ32_32::from_f32(11.0),
        FixedQ32_32::from_f32(22.0),
        FixedQ32_32::from_f32(33.0),
    );
    let q_max = quantize_vertex(max_p, &bounds);
    assert_eq!(q_max, QuantizedVertex::new(65535, 65535, 65535));

    // Dequantize min/max back and check exactness
    assert_eq!(dequantize_vertex(q_min, &bounds), min_p);
    assert_eq!(dequantize_vertex(q_max, &bounds), max_p);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_quantization_midpoint() {
    let bounds = FixedAabb3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(
            FixedQ32_32::from_f32(2.0),
            FixedQ32_32::from_f32(2.0),
            FixedQ32_32::from_f32(2.0),
        ),
    );

    // Quantize midpoint (1.0, 1.0, 1.0).
    // The exact ratio t is 0.5. Scale is 65535.
    // 0.5 * 65535 = 32767.5. Under Banker's rounding, this rounds to even: 32768.
    let mid_p = FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ONE, FixedQ32_32::ONE);
    let q_mid = quantize_vertex(mid_p, &bounds);
    assert_eq!(q_mid, QuantizedVertex::new(32768, 32768, 32768));

    // Dequantizing back should yield a value very close to 1.0
    let deq_mid = dequantize_vertex(q_mid, &bounds);
    let error_x = (deq_mid.x - FixedQ32_32::ONE).to_raw().abs();
    let error_y = (deq_mid.y - FixedQ32_32::ONE).to_raw().abs();
    let error_z = (deq_mid.z - FixedQ32_32::ONE).to_raw().abs();

    // The error should be extremely small (within Q32.32 precision for dequantization step)
    assert!(error_x < 100_000);
    assert!(error_y < 100_000);
    assert!(error_z < 100_000);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_quantization_uses_wide_single_rounding() {
    let narrow_bounds = FixedAabb3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(
            FixedQ32_32::from_f32(10.0),
            FixedQ32_32::from_f32(10.0),
            FixedQ32_32::from_f32(10.0),
        ),
    );
    let three = FixedVec3::new(
        FixedQ32_32::from_f32(3.0),
        FixedQ32_32::from_f32(3.0),
        FixedQ32_32::from_f32(3.0),
    );

    assert_eq!(quantize_vertex(three, &narrow_bounds), QuantizedVertex::new(19660, 19660, 19660));

    let wide_bounds = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
        ),
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
        ),
    );
    let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);

    assert_eq!(quantize_vertex(zero, &wide_bounds), QuantizedVertex::new(32768, 32768, 32768));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_dequantization_uses_wide_single_rounding() {
    let wide_bounds = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
        ),
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
        ),
    );
    let decoded = dequantize_vertex(QuantizedVertex::new(32768, 32768, 32768), &wide_bounds);
    let expected = FixedVec3::new(
        FixedQ32_32::from_raw(140_739_635_871_744),
        FixedQ32_32::from_raw(140_739_635_871_744),
        FixedQ32_32::from_raw(140_739_635_871_744),
    );

    assert_eq!(decoded, expected);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_dequantization_extreme_bounds_do_not_wrap() {
    let wide_bounds = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
            FixedQ32_32::from_raw(i64::MIN),
        ),
        FixedVec3::new(
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
            FixedQ32_32::from_raw(i64::MAX),
        ),
    );

    let decoded_min = dequantize_vertex(QuantizedVertex::new(0, 0, 0), &wide_bounds);
    let decoded_near_min = dequantize_vertex(QuantizedVertex::new(1, 1, 1), &wide_bounds);
    let decoded_near_max = dequantize_vertex(
        QuantizedVertex::new(u16::MAX - 1, u16::MAX - 1, u16::MAX - 1),
        &wide_bounds,
    );
    let decoded_max =
        dequantize_vertex(QuantizedVertex::new(u16::MAX, u16::MAX, u16::MAX), &wide_bounds);

    assert_eq!(decoded_min, wide_bounds.min);
    assert_eq!(decoded_max, wide_bounds.max);
    assert!(decoded_near_min.x.raw() > i64::MIN);
    assert!(decoded_near_min.x.raw() < 0);
    assert!(decoded_near_max.x.raw() > 0);
    assert!(decoded_near_max.x.raw() < i64::MAX);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_quantization_clamping() {
    let bounds = FixedAabb3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(
            FixedQ32_32::from_f32(10.0),
            FixedQ32_32::from_f32(10.0),
            FixedQ32_32::from_f32(10.0),
        ),
    );

    // Point outside (negative coords)
    let p_neg = FixedVec3::new(
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(-5.0),
        FixedQ32_32::from_f32(-0.1),
    );
    let q_neg = quantize_vertex(p_neg, &bounds);
    assert_eq!(q_neg, QuantizedVertex::new(0, 0, 0));

    // Point outside (exceeding coords)
    let p_large = FixedVec3::new(
        FixedQ32_32::from_f32(11.0),
        FixedQ32_32::from_f32(20.0),
        FixedQ32_32::from_f32(100.0),
    );
    let q_large = quantize_vertex(p_large, &bounds);
    assert_eq!(q_large, QuantizedVertex::new(65535, 65535, 65535));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_index_buffer_layouts() {
    use bunny_mesh::{IndexBufferLayout, Triangle16, Triangle32};

    let vertex_count = 4;

    // 1. Width16 tests
    let faces16 = [Triangle16::new(0, 1, 2), Triangle16::new(2, 3, 0)];
    let layout16 = IndexBufferLayout::Width16(&faces16);
    assert!(layout16.is_valid(vertex_count));
    assert_eq!(layout16.len(), 2);
    assert!(!layout16.is_empty());

    let invalid_faces16 = [
        Triangle16::new(0, 1, 4), // Index 4 is out of bounds
    ];
    let invalid_layout16 = IndexBufferLayout::Width16(&invalid_faces16);
    assert!(!invalid_layout16.is_valid(vertex_count));

    // 2. Width32 tests
    let faces32 = [Triangle32::new(0, 1, 2), Triangle32::new(2, 3, 0)];
    let layout32 = IndexBufferLayout::Width32(&faces32);
    assert!(layout32.is_valid(vertex_count));
    assert_eq!(layout32.len(), 2);
    assert!(!layout32.is_empty());

    let invalid_faces32 = [
        Triangle32::new(0, 1, 10), // Index 10 is out of bounds
    ];
    let invalid_layout32 = IndexBufferLayout::Width32(&invalid_faces32);
    assert!(!invalid_layout32.is_valid(vertex_count));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_mesh_buffer_structs_have_stable_layout() {
    use bunny_mesh::{Triangle16, Triangle32};
    use std::mem::{align_of, size_of};

    assert_eq!(size_of::<QuantizedVertex>(), 6);
    assert_eq!(align_of::<QuantizedVertex>(), 2);
    assert_eq!(size_of::<Triangle16>(), 6);
    assert_eq!(align_of::<Triangle16>(), 2);
    assert_eq!(size_of::<Triangle32>(), 12);
    assert_eq!(align_of::<Triangle32>(), 4);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_mesh_hash() {
    use bunny_mesh::{compute_mesh_hash, IndexBufferLayout, Triangle16, Triangle32};

    let vertices = [
        QuantizedVertex::new(1, 2, 3),
        QuantizedVertex::new(10, 20, 30),
        QuantizedVertex::new(100, 200, 300),
    ];

    let faces16 = [Triangle16::new(0, 1, 2)];
    let layout16 = IndexBufferLayout::Width16(&faces16);
    let bounds = mesh_hash_bounds(0.0, 1.0);

    // Get baseline hash
    let hash_base = compute_mesh_hash(&vertices, layout16, &bounds);

    // Identical mesh must match
    let hash_same = compute_mesh_hash(&vertices, layout16, &bounds);
    assert_eq!(hash_base, hash_same);

    // Altering vertex must change the hash
    let vertices_alt = [
        QuantizedVertex::new(1, 2, 3),
        QuantizedVertex::new(10, 99, 30), // altered y
        QuantizedVertex::new(100, 200, 300),
    ];
    let hash_alt_v = compute_mesh_hash(&vertices_alt, layout16, &bounds);
    assert_ne!(hash_base, hash_alt_v);

    // Altering index value must change the hash
    let faces16_alt = [Triangle16::new(0, 2, 1)];
    let layout16_alt = IndexBufferLayout::Width16(&faces16_alt);
    let hash_alt_i = compute_mesh_hash(&vertices, layout16_alt, &bounds);
    assert_ne!(hash_base, hash_alt_i);

    // Changing layout width (Width16 vs Width32) must change the hash
    let faces32 = [Triangle32::new(0, 1, 2)];
    let layout32 = IndexBufferLayout::Width32(&faces32);
    let hash_32 = compute_mesh_hash(&vertices, layout32, &bounds);
    assert_ne!(hash_base, hash_32);

    // Empty mesh hashing is stable and valid
    let hash_empty_16 = compute_mesh_hash(&[], IndexBufferLayout::Width16(&[]), &bounds);
    let hash_empty_32 = compute_mesh_hash(&[], IndexBufferLayout::Width32(&[]), &bounds);
    assert_ne!(hash_empty_16, hash_empty_32);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_mesh_hash_frames_vertex_and_face_sections() {
    use bunny_mesh::{compute_mesh_hash, IndexBufferLayout, Triangle16};

    let faces_only = [Triangle16::new(1, 2, 3)];
    let vertices_only = [QuantizedVertex::new(256, 512, 768)];
    let bounds = mesh_hash_bounds(0.0, 1.0);

    let hash_faces_only = compute_mesh_hash(&[], IndexBufferLayout::Width16(&faces_only), &bounds);
    let hash_vertices_only =
        compute_mesh_hash(&vertices_only, IndexBufferLayout::Width16(&[]), &bounds);

    assert_ne!(hash_faces_only, hash_vertices_only);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_mesh_hash_includes_quantization_bounds() {
    use bunny_mesh::{compute_mesh_hash, IndexBufferLayout, Triangle16};

    let vertices = [QuantizedVertex::new(32768, 32768, 32768)];
    let faces = [Triangle16::new(0, 0, 0)];

    let unit_bounds = mesh_hash_bounds(0.0, 1.0);
    let wide_bounds = mesh_hash_bounds(0.0, 2.0);

    let hash_unit = compute_mesh_hash(&vertices, IndexBufferLayout::Width16(&faces), &unit_bounds);
    let hash_wide = compute_mesh_hash(&vertices, IndexBufferLayout::Width16(&faces), &wide_bounds);

    assert_ne!(hash_unit, hash_wide);
}
