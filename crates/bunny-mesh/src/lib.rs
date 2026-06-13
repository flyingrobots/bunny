#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Quantized mesh layouts and verification hashes for Stanford Bunny.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use sha2::{Digest, Sha256};

const MESH_HASH_DOMAIN: &[u8; 13] = b"bunny-mesh:v2";
const QUANTIZATION_STEPS: u128 = u16::MAX as u128;

/// A 3D vertex quantized to 16-bit unsigned integers relative to a bounding box.
#[repr(C)]
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

const fn round_div_ties_to_even(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled_remainder = remainder * 2;
    if doubled_remainder > denominator {
        quotient + 1
    } else if doubled_remainder < denominator || (quotient & 1) == 0 {
        quotient
    } else {
        quotient + 1
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn round_scaled_ratio_to_u16(numerator: u128, denominator: u128) -> u16 {
    let scaled = numerator * QUANTIZATION_STEPS;
    round_div_ties_to_even(scaled, denominator) as u16
}

#[allow(clippy::cast_possible_truncation)]
fn clamp_i128_to_i64(value: i128) -> i64 {
    if value < i128::from(i64::MIN) {
        i64::MIN
    } else if value > i128::from(i64::MAX) {
        i64::MAX
    } else {
        value as i64
    }
}

/// Quantizes a single scalar value relative to min and max boundaries.
#[must_use]
pub fn quantize_scalar(val: FixedQ32_32, min: FixedQ32_32, max: FixedQ32_32) -> u16 {
    let min_raw = i128::from(min.to_raw());
    let max_raw = i128::from(max.to_raw());
    let val_raw = i128::from(val.to_raw());

    if max_raw <= min_raw || val_raw <= min_raw {
        return 0;
    }
    if val_raw >= max_raw {
        return u16::MAX;
    }

    let numerator = (val_raw - min_raw).unsigned_abs();
    let denominator = (max_raw - min_raw).unsigned_abs();
    round_scaled_ratio_to_u16(numerator, denominator)
}

/// Dequantizes a single 16-bit scalar value back to fixed-point relative to min and max boundaries.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn dequantize_scalar(q: u16, min: FixedQ32_32, max: FixedQ32_32) -> FixedQ32_32 {
    let min_raw = i128::from(min.to_raw());
    let max_raw = i128::from(max.to_raw());

    if max_raw <= min_raw || q == 0 {
        return min;
    }
    if q == u16::MAX {
        return max;
    }

    let span = (max_raw - min_raw).unsigned_abs();
    let scaled = span * u128::from(q);
    let offset = round_div_ties_to_even(scaled, QUANTIZATION_STEPS) as i128;
    FixedQ32_32::from_raw(clamp_i128_to_i64(min_raw + offset))
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
#[repr(C)]
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
#[repr(C)]
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

fn u32_index_is_valid(index: u32, vertex_count: usize) -> bool {
    usize::try_from(index).is_ok_and(|index| index < vertex_count)
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
                    if !u32_index_is_valid(face.v0, vertex_count)
                        || !u32_index_is_valid(face.v1, vertex_count)
                        || !u32_index_is_valid(face.v2, vertex_count)
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

#[allow(clippy::cast_possible_truncation)]
fn update_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_le_bytes());
}

fn update_vertices(hasher: &mut Sha256, vertices: &[QuantizedVertex]) {
    update_len(hasher, vertices.len());
    for vertex in vertices {
        hasher.update(vertex.x.to_le_bytes());
        hasher.update(vertex.y.to_le_bytes());
        hasher.update(vertex.z.to_le_bytes());
    }
}

fn update_faces16(hasher: &mut Sha256, faces: &[Triangle16]) {
    hasher.update([0_u8]);
    update_len(hasher, faces.len());
    for face in faces {
        hasher.update(face.v0.to_le_bytes());
        hasher.update(face.v1.to_le_bytes());
        hasher.update(face.v2.to_le_bytes());
    }
}

fn update_faces32(hasher: &mut Sha256, faces: &[Triangle32]) {
    hasher.update([1_u8]);
    update_len(hasher, faces.len());
    for face in faces {
        hasher.update(face.v0.to_le_bytes());
        hasher.update(face.v1.to_le_bytes());
        hasher.update(face.v2.to_le_bytes());
    }
}

fn update_bounds(hasher: &mut Sha256, bounds: &FixedAabb3) {
    hasher.update(bounds.min.x.to_raw().to_le_bytes());
    hasher.update(bounds.min.y.to_raw().to_le_bytes());
    hasher.update(bounds.min.z.to_raw().to_le_bytes());
    hasher.update(bounds.max.x.to_raw().to_le_bytes());
    hasher.update(bounds.max.y.to_raw().to_le_bytes());
    hasher.update(bounds.max.z.to_raw().to_le_bytes());
}

/// Computes the cryptographic SHA-256 hash of the quantized mesh vertex and index data.
///
/// This function is zero-allocation and operates on slices of vertices and the index buffer layout.
/// The byte representation is processed sequentially:
/// 1. A domain marker and six little-endian Q32.32 bound coordinates.
/// 2. A little-endian `u64` vertex count.
/// 3. The vertices as little-endian `u16` values `[x, y, z]`.
/// 4. The index layout tag (`0x00` for `Width16`, `0x01` for `Width32`).
/// 5. A little-endian `u64` face count, then each face's little-endian indices.
#[must_use]
pub fn compute_mesh_hash(
    vertices: &[QuantizedVertex],
    indices: IndexBufferLayout,
    bounds: &FixedAabb3,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(MESH_HASH_DOMAIN);
    update_bounds(&mut hasher, bounds);
    update_vertices(&mut hasher, vertices);
    match indices {
        IndexBufferLayout::Width16(faces) => update_faces16(&mut hasher, faces),
        IndexBufferLayout::Width32(faces) => update_faces32(&mut hasher, faces),
    }

    let result = hasher.finalize();
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&result);
    hash
}
