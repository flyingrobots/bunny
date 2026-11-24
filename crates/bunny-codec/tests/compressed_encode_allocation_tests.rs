//! The compressed mesh encoder's allocation contract: `encode_compressed_mesh_into` and
//! `encoded_compressed_mesh_len` allocate nothing, and `encode_compressed_mesh` allocates exactly
//! one block of exactly the stream's length on success and nothing on a refusal.

#![cfg(not(target_arch = "wasm32"))]

use bunny_codec::{
    encode_compressed_mesh, encode_compressed_mesh_into, encoded_compressed_mesh_len,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{IndexBufferLayout, QuantizedVertex, Triangle32};
use bunny_num::FixedQ32_32;

fn bounds() -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ONE, FixedQ32_32::ONE),
    )
}

/// Runs `operation` and returns its result with the number of heap blocks the calling thread
/// allocated meanwhile. Allocations by other threads, including the test harness, are not counted.
fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let mut result = None;
    let counts = allocation_counter::measure(|| result = Some(operation()));
    (result.expect("the measured operation should have run"), counts.count_total)
}

#[test]
fn cmesh_tp_012_the_encoder_allocates_only_the_returned_stream() {
    let vertices: Vec<QuantizedVertex> =
        (0_u16..500).map(|i| QuantizedVertex::new(i, i.wrapping_mul(7), 3)).collect();
    let faces: Vec<Triangle32> =
        (0_u32..400).map(|i| Triangle32::new(i, (i * 3) % 500, 499)).collect();
    let layout = IndexBufferLayout::Width32(&faces);
    let bounds = bounds();
    let required = encoded_compressed_mesh_len(&vertices, layout).expect("valid mesh");
    let mut out = vec![0_u8; required];

    let (len, blocks) = allocations_during(|| encoded_compressed_mesh_len(&vertices, layout));
    assert_eq!((len, blocks), (Ok(required), 0));

    let (written, blocks) =
        allocations_during(|| encode_compressed_mesh_into(&bounds, &vertices, layout, &mut out));
    assert_eq!((written, blocks), (Ok(required), 0));

    let (bytes, blocks) = allocations_during(|| encode_compressed_mesh(&bounds, &vertices, layout));
    let bytes = bytes.expect("valid mesh");
    assert_eq!(blocks, 1, "one allocation: the returned stream");
    assert_eq!((bytes.len(), bytes.capacity()), (required, required));
    assert_eq!(bytes, out);

    let bad = [Triangle32::new(0, 0, 500)];
    let (refused, blocks) = allocations_during(|| {
        encode_compressed_mesh(&bounds, &vertices, IndexBufferLayout::Width32(&bad))
    });
    assert!(refused.is_err());
    assert_eq!(blocks, 0, "a refusal allocates nothing");
}
