use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{dequantize_vertex, quantize_vertex, QuantizedVertex};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

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
