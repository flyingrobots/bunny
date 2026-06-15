use bunny_geom::FixedAabb3;
use bunny_mesh::{QuantizedVertex, Triangle16, Triangle32};

use super::error::CompressedMeshError;
use super::read::{read_triangle, read_vertex, take_record};
use super::{TRIANGLE16_STRIDE, TRIANGLE32_STRIDE, VERTEX_STRIDE};

/// Triangle index width used by a Bunny compressed mesh payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedIndexWidth {
    /// Three little-endian `u16` values per triangle.
    Width16,
    /// Three little-endian `u32` values per triangle.
    Width32,
}

impl CompressedIndexWidth {
    pub(super) const fn stride(self) -> usize {
        match self {
            Self::Width16 => TRIANGLE16_STRIDE,
            Self::Width32 => TRIANGLE32_STRIDE,
        }
    }
}

/// Decoded triangle from a Bunny compressed mesh payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedTriangle {
    /// A 16-bit triangle index record.
    Width16(Triangle16),
    /// A 32-bit triangle index record.
    Width32(Triangle32),
}

/// A zero-allocation borrowed view over a validated Bunny compressed mesh payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompressedMesh<'a> {
    bounds: FixedAabb3,
    vertex_bytes: &'a [u8],
    triangle_bytes: &'a [u8],
    vertex_count: usize,
    triangle_count: usize,
    index_width: CompressedIndexWidth,
}

impl<'a> CompressedMesh<'a> {
    pub(super) const fn new(
        bounds: FixedAabb3,
        vertex_bytes: &'a [u8],
        triangle_bytes: &'a [u8],
        vertex_count: usize,
        triangle_count: usize,
        index_width: CompressedIndexWidth,
    ) -> Self {
        Self {
            bounds,
            vertex_bytes,
            triangle_bytes,
            vertex_count,
            triangle_count,
            index_width,
        }
    }

    /// Returns the decoded Q32.32 quantization bounds.
    #[must_use]
    pub const fn bounds(self) -> FixedAabb3 {
        self.bounds
    }

    /// Returns the decoded vertex count.
    #[must_use]
    pub const fn vertex_count(self) -> usize {
        self.vertex_count
    }

    /// Returns the decoded triangle count.
    #[must_use]
    pub const fn triangle_count(self) -> usize {
        self.triangle_count
    }

    /// Returns the decoded triangle index width.
    #[must_use]
    pub const fn index_width(self) -> CompressedIndexWidth {
        self.index_width
    }

    /// Returns the borrowed quantized vertex payload bytes.
    #[must_use]
    pub const fn vertex_bytes(self) -> &'a [u8] {
        self.vertex_bytes
    }

    /// Returns the borrowed triangle index payload bytes.
    #[must_use]
    pub const fn triangle_bytes(self) -> &'a [u8] {
        self.triangle_bytes
    }

    /// Reads a quantized vertex record.
    ///
    /// # Errors
    /// Returns `CompressedMeshError::IndexOutOfBounds` when `index` is outside
    /// the decoded vertex range.
    pub fn vertex(self, index: usize) -> Result<QuantizedVertex, CompressedMeshError> {
        if index >= self.vertex_count {
            return Err(CompressedMeshError::IndexOutOfBounds);
        }
        read_vertex(take_record(self.vertex_bytes, index, VERTEX_STRIDE)?)
    }

    /// Reads a triangle record using the width declared by the byte stream.
    ///
    /// # Errors
    /// Returns `CompressedMeshError::IndexOutOfBounds` when `index` is outside
    /// the decoded triangle range.
    pub fn triangle(self, index: usize) -> Result<CompressedTriangle, CompressedMeshError> {
        if index >= self.triangle_count {
            return Err(CompressedMeshError::IndexOutOfBounds);
        }
        let bytes = take_record(self.triangle_bytes, index, self.index_width.stride())?;
        read_triangle(bytes, self.index_width)
    }
}
