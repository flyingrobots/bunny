mod error;
mod read;
mod view;

use read::{checked_add, checked_payload_len, parse_header, take, validate_triangles};

pub use error::CompressedMeshError;
pub use view::{CompressedIndexWidth, CompressedMesh, CompressedTriangle};

const HEADER_LEN: usize = 76;
const VERTEX_STRIDE: usize = 6;
const TRIANGLE16_STRIDE: usize = 6;
const TRIANGLE32_STRIDE: usize = 12;
const MAX_VERTICES: usize = 1_000_000;
const MAX_TRIANGLES: usize = 1_000_000;

/// Decodes a canonical Bunny compressed mesh byte stream.
///
/// The accepted profile is documented in `docs/goalposts/v0.4.0-gp3.md`.
/// Decoding borrows the vertex and triangle byte sections, validates all
/// triangle indices, and performs no heap allocation on the accepted path.
///
/// # Errors
/// Returns `CompressedMeshError` when any header, length, bounds, count, or
/// payload invariant is violated.
pub fn decode_compressed_mesh(input: &[u8]) -> Result<CompressedMesh<'_>, CompressedMeshError> {
    let header = parse_header(input)?;
    let vertex_len = checked_payload_len(header.vertex_count, VERTEX_STRIDE)?;
    let triangle_len = checked_payload_len(header.triangle_count, header.index_width.stride())?;
    let expected_len = checked_add(vertex_len, triangle_len)?;
    let expected_len_u64 =
        u64::try_from(expected_len).map_err(|_| CompressedMeshError::IntegerOverflow)?;
    let _declared_end = u64::try_from(HEADER_LEN)
        .map_err(|_| CompressedMeshError::IntegerOverflow)?
        .checked_add(header.payload_len)
        .ok_or(CompressedMeshError::IntegerOverflow)?;
    if header.payload_len != expected_len_u64 {
        return Err(CompressedMeshError::InvalidPayloadLength);
    }
    let payload_end = checked_add(HEADER_LEN, expected_len)?;
    if input.len() < payload_end {
        return Err(CompressedMeshError::PayloadTooShort);
    }
    if input.len() != payload_end {
        return Err(CompressedMeshError::TrailingData);
    }

    let vertex_bytes = take(input, HEADER_LEN, vertex_len)?;
    let triangle_start = checked_add(HEADER_LEN, vertex_len)?;
    let triangle_bytes = take(input, triangle_start, triangle_len)?;
    validate_triangles(
        triangle_bytes,
        header.triangle_count,
        header.vertex_count,
        header.index_width,
    )?;
    Ok(CompressedMesh::new(
        header.bounds,
        vertex_bytes,
        triangle_bytes,
        header.vertex_count,
        header.triangle_count,
        header.index_width,
    ))
}
