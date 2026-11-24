use bunny_geom::FixedAabb3;
use bunny_mesh::{IndexBufferLayout, QuantizedVertex};

use super::error::CompressedMeshEncodeError;

const HEADER_LEN: u64 = 76;
const VERTEX_STRIDE: u64 = 6;
const TRIANGLE16_STRIDE: u64 = 6;
const TRIANGLE32_STRIDE: u64 = 12;
const MAX_RECORDS: u32 = 1_000_000;
const WIDTH16_VERTEX_CAPACITY: usize = 65_536;

/// The header values of a stream whose counts, width and length have been checked.
#[derive(Clone, Copy, Debug)]
pub(super) struct Plan {
    pub(super) vertex_count: u32,
    pub(super) triangle_count: u32,
    pub(super) index_width: u8,
    pub(super) payload_len: u64,
    pub(super) total_len: usize,
}

impl Plan {
    /// Checks the counts and the width-16 capacity, in that order, and computes the lengths.
    pub(super) fn new(
        vertices: &[QuantizedVertex],
        triangles: IndexBufferLayout<'_>,
    ) -> Result<Self, CompressedMeshEncodeError> {
        let vertex_count = record_count(vertices.len())
            .ok_or(CompressedMeshEncodeError::InvalidVertexCount { count: vertices.len() })?;
        let triangle_count = record_count(triangles.len())
            .ok_or(CompressedMeshEncodeError::InvalidTriangleCount { count: triangles.len() })?;
        let (index_width, stride) = match triangles {
            IndexBufferLayout::Width16(_) => (16, TRIANGLE16_STRIDE),
            IndexBufferLayout::Width32(_) => (32, TRIANGLE32_STRIDE),
        };
        if index_width == 16 && vertices.len() > WIDTH16_VERTEX_CAPACITY {
            return Err(CompressedMeshEncodeError::Width16VertexCapacity {
                vertex_count: vertices.len(),
            });
        }
        let payload_len = payload_len(vertex_count, triangle_count, stride)
            .ok_or(CompressedMeshEncodeError::LengthOverflow)?;
        let total_len = HEADER_LEN
            .checked_add(payload_len)
            .and_then(|len| usize::try_from(len).ok())
            .ok_or(CompressedMeshEncodeError::LengthOverflow)?;
        Ok(Self { vertex_count, triangle_count, index_width, payload_len, total_len })
    }

    /// Runs every check of the encoder: `new`, then the bounds, then the indices.
    pub(super) fn validated(
        bounds: &FixedAabb3,
        vertices: &[QuantizedVertex],
        triangles: IndexBufferLayout<'_>,
    ) -> Result<Self, CompressedMeshEncodeError> {
        let plan = Self::new(vertices, triangles)?;
        FixedAabb3::try_new(bounds.min, bounds.max)
            .map_err(|_| CompressedMeshEncodeError::InvalidBounds)?;
        validate_indices(vertices.len(), triangles)?;
        Ok(plan)
    }
}

/// A count of 1 to 1 000 000 as the `u32` the header stores, or `None`.
fn record_count(len: usize) -> Option<u32> {
    u32::try_from(len).ok().filter(|count| (1..=MAX_RECORDS).contains(count))
}

/// The payload length; at most 6 * 10^6 + 12 * 10^6 bytes for counts already checked.
fn payload_len(vertex_count: u32, triangle_count: u32, stride: u64) -> Option<u64> {
    let vertex_bytes = u64::from(vertex_count).checked_mul(VERTEX_STRIDE)?;
    let triangle_bytes = u64::from(triangle_count).checked_mul(stride)?;
    vertex_bytes.checked_add(triangle_bytes)
}

fn validate_indices(
    vertex_count: usize,
    triangles: IndexBufferLayout<'_>,
) -> Result<(), CompressedMeshEncodeError> {
    match triangles {
        IndexBufferLayout::Width16(faces) => first_out_of_bounds(
            faces.iter().map(|face| [face.v0, face.v1, face.v2].map(u32::from)),
            vertex_count,
        ),
        IndexBufferLayout::Width32(faces) => {
            first_out_of_bounds(faces.iter().map(|face| [face.v0, face.v1, face.v2]), vertex_count)
        }
    }
}

/// Refuses the first triangle, and within it the first corner, whose index is not below
/// `vertex_count`.
fn first_out_of_bounds(
    faces: impl Iterator<Item = [u32; 3]>,
    vertex_count: usize,
) -> Result<(), CompressedMeshEncodeError> {
    for (triangle, corners) in faces.enumerate() {
        if let Some(index) = corners.into_iter().find(|index| !is_below(*index, vertex_count)) {
            return Err(CompressedMeshEncodeError::IndexOutOfBounds { triangle, index });
        }
    }
    Ok(())
}

fn is_below(index: u32, vertex_count: usize) -> bool {
    usize::try_from(index).is_ok_and(|index| index < vertex_count)
}
