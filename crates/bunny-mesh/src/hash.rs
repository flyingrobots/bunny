//! The SHA-256 mesh verification hash.

use bunny_geom::FixedAabb3;
use sha2::{Digest, Sha256};

use crate::{IndexBufferLayout, QuantizedVertex, Triangle16, Triangle32};

const MESH_HASH_DOMAIN: &[u8; 13] = b"bunny-mesh:v2";

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
///
/// Available with the `mesh-hash` feature, which is on by default.
#[must_use]
pub fn compute_mesh_hash(
    vertices: &[QuantizedVertex],
    indices: IndexBufferLayout<'_>,
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
