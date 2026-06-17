use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{QuantizedVertex, Triangle16, Triangle32};
use bunny_num::FixedQ32_32;

use super::error::CompressedMeshError;
use super::view::{CompressedIndexWidth, CompressedTriangle};
use super::{MAX_TRIANGLES, MAX_VERTICES};

const MAGIC: &[u8; 8] = b"BUNNYQZ!";
const VERSION: u8 = 1;
const WIDTH16_VERTEX_CAPACITY: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Header {
    pub(super) bounds: FixedAabb3,
    pub(super) vertex_count: usize,
    pub(super) triangle_count: usize,
    pub(super) payload_len: u64,
    pub(super) index_width: CompressedIndexWidth,
}

pub(super) fn parse_header(input: &[u8]) -> Result<Header, CompressedMeshError> {
    validate_magic(input)?;
    validate_version(input)?;
    validate_flags(input)?;
    let index_width = read_index_width(input)?;
    let vertex_count = read_count(input, 12, MAX_VERTICES)?;
    let triangle_count = read_count(input, 16, MAX_TRIANGLES)?;
    validate_width_capacity(index_width, vertex_count)?;
    Ok(Header {
        bounds: read_bounds(input, 28)?,
        vertex_count,
        triangle_count,
        payload_len: read_u64(input, 20)?,
        index_width,
    })
}

fn validate_magic(input: &[u8]) -> Result<(), CompressedMeshError> {
    if take(input, 0, MAGIC.len())? == MAGIC.as_slice() {
        Ok(())
    } else {
        Err(CompressedMeshError::InvalidMagic)
    }
}

fn validate_version(input: &[u8]) -> Result<(), CompressedMeshError> {
    if read_u8(input, 8)? == VERSION {
        Ok(())
    } else {
        Err(CompressedMeshError::UnsupportedVersion)
    }
}

fn validate_flags(input: &[u8]) -> Result<(), CompressedMeshError> {
    if read_u16(input, 10)? == 0 {
        Ok(())
    } else {
        Err(CompressedMeshError::UnsupportedFlags)
    }
}

fn read_index_width(input: &[u8]) -> Result<CompressedIndexWidth, CompressedMeshError> {
    parse_index_width(read_u8(input, 9)?)
}

fn read_count(input: &[u8], offset: usize, max: usize) -> Result<usize, CompressedMeshError> {
    parse_count(read_u32(input, offset)?, max)
}

pub(super) fn validate_triangles(
    bytes: &[u8],
    count: usize,
    vertex_count: usize,
    index_width: CompressedIndexWidth,
) -> Result<(), CompressedMeshError> {
    for index in 0..count {
        let triangle =
            read_triangle(take_record(bytes, index, index_width.stride())?, index_width)?;
        validate_triangle_bounds(triangle, vertex_count)?;
    }
    Ok(())
}

pub(super) fn read_triangle(
    bytes: &[u8],
    index_width: CompressedIndexWidth,
) -> Result<CompressedTriangle, CompressedMeshError> {
    match index_width {
        CompressedIndexWidth::Width16 => Ok(CompressedTriangle::Width16(Triangle16::new(
            read_u16(bytes, 0)?,
            read_u16(bytes, 2)?,
            read_u16(bytes, 4)?,
        ))),
        CompressedIndexWidth::Width32 => Ok(CompressedTriangle::Width32(Triangle32::new(
            read_u32(bytes, 0)?,
            read_u32(bytes, 4)?,
            read_u32(bytes, 8)?,
        ))),
    }
}

pub(super) fn read_vertex(bytes: &[u8]) -> Result<QuantizedVertex, CompressedMeshError> {
    Ok(QuantizedVertex::new(read_u16(bytes, 0)?, read_u16(bytes, 2)?, read_u16(bytes, 4)?))
}

pub(super) fn checked_payload_len(
    count: usize,
    stride: usize,
) -> Result<usize, CompressedMeshError> {
    count.checked_mul(stride).ok_or(CompressedMeshError::IntegerOverflow)
}

pub(super) fn checked_add(lhs: usize, rhs: usize) -> Result<usize, CompressedMeshError> {
    lhs.checked_add(rhs).ok_or(CompressedMeshError::IntegerOverflow)
}

pub(super) fn take_record(
    input: &[u8],
    index: usize,
    stride: usize,
) -> Result<&[u8], CompressedMeshError> {
    take(input, checked_payload_len(index, stride)?, stride)
}

pub(super) fn take(input: &[u8], start: usize, len: usize) -> Result<&[u8], CompressedMeshError> {
    let end = checked_add(start, len)?;
    input.get(start..end).ok_or(CompressedMeshError::PayloadTooShort)
}

const fn parse_index_width(value: u8) -> Result<CompressedIndexWidth, CompressedMeshError> {
    match value {
        16 => Ok(CompressedIndexWidth::Width16),
        32 => Ok(CompressedIndexWidth::Width32),
        _ => Err(CompressedMeshError::InvalidIndexWidth),
    }
}

fn parse_count(count: u32, max: usize) -> Result<usize, CompressedMeshError> {
    let count = usize::try_from(count).map_err(|_| CompressedMeshError::IntegerOverflow)?;
    if count == 0 || count > max {
        Err(CompressedMeshError::InvalidCount)
    } else {
        Ok(count)
    }
}

fn validate_width_capacity(
    index_width: CompressedIndexWidth,
    vertex_count: usize,
) -> Result<(), CompressedMeshError> {
    if index_width == CompressedIndexWidth::Width16 && vertex_count > WIDTH16_VERTEX_CAPACITY {
        Err(CompressedMeshError::InvalidCount)
    } else {
        Ok(())
    }
}

fn read_bounds(input: &[u8], start: usize) -> Result<FixedAabb3, CompressedMeshError> {
    let max_start = checked_add(start, 24)?;
    FixedAabb3::try_new(read_vec3(input, start)?, read_vec3(input, max_start)?)
        .map_err(|_| CompressedMeshError::InvalidBounds)
}

fn read_vec3(input: &[u8], start: usize) -> Result<FixedVec3, CompressedMeshError> {
    let y_start = checked_add(start, 8)?;
    let z_start = checked_add(start, 16)?;
    Ok(FixedVec3::new(
        FixedQ32_32::from_raw(read_i64(input, start)?),
        FixedQ32_32::from_raw(read_i64(input, y_start)?),
        FixedQ32_32::from_raw(read_i64(input, z_start)?),
    ))
}

fn validate_triangle_bounds(
    triangle: CompressedTriangle,
    vertex_count: usize,
) -> Result<(), CompressedMeshError> {
    match triangle {
        CompressedTriangle::Width16(face) => validate_indices16(face, vertex_count),
        CompressedTriangle::Width32(face) => validate_indices32(face, vertex_count),
    }
}

fn validate_indices16(face: Triangle16, vertex_count: usize) -> Result<(), CompressedMeshError> {
    if usize::from(face.v0) < vertex_count
        && usize::from(face.v1) < vertex_count
        && usize::from(face.v2) < vertex_count
    {
        Ok(())
    } else {
        Err(CompressedMeshError::IndexOutOfBounds)
    }
}

fn validate_indices32(face: Triangle32, vertex_count: usize) -> Result<(), CompressedMeshError> {
    if index_is_valid(face.v0, vertex_count)
        && index_is_valid(face.v1, vertex_count)
        && index_is_valid(face.v2, vertex_count)
    {
        Ok(())
    } else {
        Err(CompressedMeshError::IndexOutOfBounds)
    }
}

fn index_is_valid(index: u32, vertex_count: usize) -> bool {
    usize::try_from(index).is_ok_and(|index| index < vertex_count)
}

fn take_array<const N: usize>(input: &[u8], start: usize) -> Result<[u8; N], CompressedMeshError> {
    let mut bytes = [0_u8; N];
    bytes.copy_from_slice(take(input, start, N)?);
    Ok(bytes)
}

fn read_u8(input: &[u8], start: usize) -> Result<u8, CompressedMeshError> {
    take(input, start, 1)?.first().copied().ok_or(CompressedMeshError::PayloadTooShort)
}

fn read_u16(input: &[u8], start: usize) -> Result<u16, CompressedMeshError> {
    Ok(u16::from_le_bytes(take_array(input, start)?))
}

fn read_u32(input: &[u8], start: usize) -> Result<u32, CompressedMeshError> {
    Ok(u32::from_le_bytes(take_array(input, start)?))
}

fn read_u64(input: &[u8], start: usize) -> Result<u64, CompressedMeshError> {
    Ok(u64::from_le_bytes(take_array(input, start)?))
}

fn read_i64(input: &[u8], start: usize) -> Result<i64, CompressedMeshError> {
    Ok(i64::from_le_bytes(take_array(input, start)?))
}
