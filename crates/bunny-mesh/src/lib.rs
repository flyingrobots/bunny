#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Quantized mesh layouts and verification hashes for Stanford Bunny.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// A 3D vertex quantized to 16-bit unsigned integers relative to a bounding box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuantizedVertex {
    /// Quantized X coordinate.
    pub x: u16,
    /// Quantized Y coordinate.
    pub y: u16,
    /// Quantized Z coordinate.
    pub z: u16,
}

impl QuantizedVertex {
    /// Creates a new `QuantizedVertex`.
    #[must_use]
    pub const fn new(x: u16, y: u16, z: u16) -> Self {
        Self { x, y, z }
    }
}

const fn round_shift_right_u128(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        return value;
    }
    let q = value >> shift;
    let mask = (1_u128 << shift) - 1;
    let r = value & mask;
    let half = 1_u128 << (shift - 1);

    if r > half {
        q + 1
    } else if r < half {
        q
    } else if (q & 1) == 1 {
        q + 1
    } else {
        q
    }
}

/// Quantizes a single scalar value relative to min and max boundaries.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn quantize_scalar(val: FixedQ32_32, min: FixedQ32_32, max: FixedQ32_32) -> u16 {
    let span = max - min;
    if span == FixedQ32_32::ZERO {
        return 0;
    }
    let diff = val - min;
    let t = diff / span;
    let t_clamped = if t < FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else if t > FixedQ32_32::ONE {
        FixedQ32_32::ONE
    } else {
        t
    };

    let scale = FixedQ32_32::from_raw(65535 * FixedQ32_32::ONE.to_raw());
    let scaled = t_clamped * scale;

    let rounded = round_shift_right_u128(u128::from(scaled.to_raw().unsigned_abs()), 32);
    rounded as u16
}

/// Dequantizes a single 16-bit scalar value back to fixed-point relative to min and max boundaries.
#[must_use]
pub fn dequantize_scalar(q: u16, min: FixedQ32_32, max: FixedQ32_32) -> FixedQ32_32 {
    let span = max - min;
    let q_fixed = FixedQ32_32::from_raw(i64::from(q) << 32);
    let scale = FixedQ32_32::from_raw(65535 * FixedQ32_32::ONE.to_raw());
    let t = q_fixed / scale;

    min + t * span
}

/// Quantizes a 3D vertex position relative to a bounding box.
#[must_use]
pub fn quantize_vertex(p: FixedVec3, bounds: &FixedAabb3) -> QuantizedVertex {
    QuantizedVertex::new(
        quantize_scalar(p.x, bounds.min.x, bounds.max.x),
        quantize_scalar(p.y, bounds.min.y, bounds.max.y),
        quantize_scalar(p.z, bounds.min.z, bounds.max.z),
    )
}

/// Dequantizes a 16-bit quantized vertex back to a 3D fixed-point vector.
#[must_use]
pub fn dequantize_vertex(q: QuantizedVertex, bounds: &FixedAabb3) -> FixedVec3 {
    FixedVec3::new(
        dequantize_scalar(q.x, bounds.min.x, bounds.max.x),
        dequantize_scalar(q.y, bounds.min.y, bounds.max.y),
        dequantize_scalar(q.z, bounds.min.z, bounds.max.z),
    )
}

/// A triangulated face using 16-bit unsigned integer vertex indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle16 {
    /// Index of the first vertex.
    pub v0: u16,
    /// Index of the second vertex.
    pub v1: u16,
    /// Index of the third vertex.
    pub v2: u16,
}

impl Triangle16 {
    /// Creates a new `Triangle16`.
    #[must_use]
    pub const fn new(v0: u16, v1: u16, v2: u16) -> Self {
        Self { v0, v1, v2 }
    }
}

/// A triangulated face using 32-bit unsigned integer vertex indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle32 {
    /// Index of the first vertex.
    pub v0: u32,
    /// Index of the second vertex.
    pub v1: u32,
    /// Index of the third vertex.
    pub v2: u32,
}

impl Triangle32 {
    /// Creates a new `Triangle32`.
    #[must_use]
    pub const fn new(v0: u32, v1: u32, v2: u32) -> Self {
        Self { v0, v1, v2 }
    }
}

/// Memory-stable, zero-allocation layout options for index buffering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexBufferLayout<'a> {
    /// 16-bit index buffer layout.
    Width16(&'a [Triangle16]),
    /// 32-bit index buffer layout.
    Width32(&'a [Triangle32]),
}

impl IndexBufferLayout<'_> {
    /// Checks if all indices in the layout are valid for a given vertex buffer length.
    #[must_use]
    pub fn is_valid(self, vertex_count: usize) -> bool {
        match self {
            Self::Width16(faces) => {
                for face in faces {
                    if usize::from(face.v0) >= vertex_count
                        || usize::from(face.v1) >= vertex_count
                        || usize::from(face.v2) >= vertex_count
                    {
                        return false;
                    }
                }
            }
            Self::Width32(faces) => {
                for face in faces {
                    if face.v0 as usize >= vertex_count
                        || face.v1 as usize >= vertex_count
                        || face.v2 as usize >= vertex_count
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Gets the number of triangles in the index buffer.
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Width16(faces) => faces.len(),
            Self::Width32(faces) => faces.len(),
        }
    }

    /// Returns `true` if the index buffer has no triangles.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }
}
