//! Encoder for the Bunny compressed mesh profile v1.
//!
//! The profile is the one `decode_compressed_mesh` reads, documented in
//! `docs/goalposts/v0.4.0-gp3.md` and `docs/topics/compressed-mesh/`.

mod error;
mod plan;
mod write;

use bunny_geom::FixedAabb3;
use bunny_mesh::{IndexBufferLayout, QuantizedVertex};

pub use error::CompressedMeshEncodeError;

/// Returns the length in bytes of the stream that encodes a mesh with these records.
///
/// The length is `76 + 6 * vertices + stride * triangles`, where the stride is 6 bytes for
/// `IndexBufferLayout::Width16` and 12 for `Width32`; the largest is 18 000 076 bytes. Only the
/// counts and the width-16 capacity are checked: bounds and indices are not, so a mesh this
/// accepts can still be refused by `encode_compressed_mesh_into`. Allocates nothing.
///
/// # Errors
/// `InvalidVertexCount`, `InvalidTriangleCount` or `Width16VertexCapacity`, in that order, as
/// for `encode_compressed_mesh_into`; `LengthOverflow` where the length does not fit `usize`.
pub fn encoded_compressed_mesh_len(
    vertices: &[QuantizedVertex],
    triangles: IndexBufferLayout<'_>,
) -> Result<usize, CompressedMeshEncodeError> {
    plan::Plan::new(vertices, triangles).map(|plan| plan.total_len)
}

/// Encodes a mesh into the front of `output` and returns the number of bytes written.
///
/// The stream is the canonical profile v1 encoding of `bounds`, `vertices` and `triangles`: the
/// index width is the variant of `triangles`, the flags are zero and every field is
/// little-endian, so the bytes depend on the records alone and `decode_compressed_mesh` returns
/// them unchanged. Every check runs before the first byte is written: on an error `output` is
/// untouched, and on success bytes past the returned length are untouched. Allocates nothing.
///
/// # Errors
/// Checked in this order, the first failure returned:
/// `InvalidVertexCount` (not 1 to 1 000 000), `InvalidTriangleCount` (not 1 to 1 000 000),
/// `Width16VertexCapacity` (more than 65 536 vertices at width 16), `InvalidBounds` (a min above
/// its max on any axis), `IndexOutOfBounds` (the first triangle, and its first corner, whose index
/// is not below the vertex count), then `OutputTooSmall` (`output` shorter than the stream).
/// `LengthOverflow` where the length does not fit `usize`, which no `std` target has.
pub fn encode_compressed_mesh_into(
    bounds: &FixedAabb3,
    vertices: &[QuantizedVertex],
    triangles: IndexBufferLayout<'_>,
    output: &mut [u8],
) -> Result<usize, CompressedMeshEncodeError> {
    let plan = plan::Plan::validated(bounds, vertices, triangles)?;
    let provided = output.len();
    let stream = output
        .get_mut(..plan.total_len)
        .ok_or(CompressedMeshEncodeError::OutputTooSmall { required: plan.total_len, provided })?;
    write::write_stream(stream, &plan, write::Records { bounds, vertices, triangles });
    Ok(plan.total_len)
}

/// Encodes a mesh into a new `Vec<u8>` holding exactly the canonical profile v1 stream.
///
/// The bytes are those `encode_compressed_mesh_into` writes. Every check runs first; on success
/// the function allocates once, a block of exactly the stream's length (`len() == capacity()`),
/// and on an error it allocates nothing.
///
/// # Errors
/// As `encode_compressed_mesh_into`, except `OutputTooSmall`, which cannot occur.
pub fn encode_compressed_mesh(
    bounds: &FixedAabb3,
    vertices: &[QuantizedVertex],
    triangles: IndexBufferLayout<'_>,
) -> Result<Vec<u8>, CompressedMeshEncodeError> {
    let plan = plan::Plan::validated(bounds, vertices, triangles)?;
    let mut stream = vec![0_u8; plan.total_len];
    write::write_stream(&mut stream, &plan, write::Records { bounds, vertices, triangles });
    Ok(stream)
}
