//! Decode a Bunny compressed mesh and read its vertices and triangles back.
//!
//! Run with `cargo run -p bunny-codec --example decode_compressed_mesh`.
//!
//! `bunny-codec` has no encoder, so the example writes the smallest useful input itself: a
//! tetrahedron in the compressed mesh profile v1 (`docs/goalposts/v0.4.0-gp3.md`). The
//! decoder borrows the byte sections and validates every index once; the accessors then read
//! one record at a time and still return a `Result`, because an index can be out of range.

use std::error::Error;

use bunny_codec::{decode_compressed_mesh, CompressedMeshError, CompressedTriangle};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::dequantize_vertex;
use bunny_num::fixed_q32_32::ONE_RAW;

/// Quantized positions. On each axis 0 decodes to the bounds' minimum, 65 535 to the maximum,
/// and a value between to `min + round(span * q / 65 535)`, ties to even.
const VERTICES: [[u16; 3]; 4] =
    [[0, 0, 0], [65_535, 0, 0], [0, 65_535, 0], [32_768, 32_768, 65_535]];
/// Vertex indices of each face.
const TRIANGLES: [[u16; 3]; 4] = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
/// The quantization bounds, `[-1, 1]` on every axis, as raw Q32.32 (`min` then `max`).
const BOUNDS_RAW: [i64; 6] = [-ONE_RAW, -ONE_RAW, -ONE_RAW, ONE_RAW, ONE_RAW, ONE_RAW];

/// What the example reads back from the decoded view.
#[derive(Debug)]
struct Report {
    byte_len: usize,
    bounds: FixedAabb3,
    positions: Vec<FixedVec3>,
    triangles: Vec<[u32; 3]>,
    truncated: CompressedMeshError,
}

/// The 76-byte header, then 6 bytes per vertex and 6 per 16-bit triangle: 124 bytes.
fn encode() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(124);
    bytes.extend_from_slice(b"BUNNYQZ!");
    bytes.extend_from_slice(&[1, 16]); // profile version 1, 16-bit indices
    bytes.extend_from_slice(&0_u16.to_le_bytes()); // flags: none are defined
    bytes.extend_from_slice(&4_u32.to_le_bytes()); // vertex count
    bytes.extend_from_slice(&4_u32.to_le_bytes()); // triangle count
    bytes.extend_from_slice(&48_u64.to_le_bytes()); // payload: 4 * 6 + 4 * 6 bytes
    for raw in BOUNDS_RAW {
        bytes.extend_from_slice(&raw.to_le_bytes());
    }
    for value in VERTICES.iter().chain(&TRIANGLES).flatten() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// A face's indices, whichever width the stream declared.
fn indices(triangle: CompressedTriangle) -> [u32; 3] {
    match triangle {
        CompressedTriangle::Width16(face) => [face.v0, face.v1, face.v2].map(u32::from),
        CompressedTriangle::Width32(face) => [face.v0, face.v1, face.v2],
    }
}

fn run() -> Result<Report, Box<dyn Error>> {
    let bytes = encode();
    let mesh = decode_compressed_mesh(&bytes)?;
    let bounds = mesh.bounds();
    let positions = (0..mesh.vertex_count())
        .map(|index| mesh.vertex(index).map(|vertex| dequantize_vertex(vertex, &bounds)))
        .collect::<Result<Vec<_>, _>>()?;
    let triangles = (0..mesh.triangle_count())
        .map(|index| mesh.triangle(index).map(indices))
        .collect::<Result<Vec<_>, _>>()?;
    // A stream one byte short is refused with a typed error; nothing is read past its end.
    let (_, short) = bytes.split_last().ok_or("the encoded mesh is empty")?;
    let truncated = decode_compressed_mesh(short).err().ok_or("a truncated stream decoded")?;
    Ok(Report { byte_len: bytes.len(), bounds, positions, triangles, truncated })
}

fn raw(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn main() -> Result<(), Box<dyn Error>> {
    let report = run()?;
    println!("decoded {} bytes: {} vertices", report.byte_len, report.positions.len());
    println!("bounds raw: min {:?} max {:?}", raw(report.bounds.min), raw(report.bounds.max));
    for (index, position) in report.positions.iter().enumerate() {
        println!("vertex {index} raw Q32.32: {:?}", raw(*position));
    }
    for (index, triangle) in report.triangles.iter().enumerate() {
        println!("triangle {index}: {triangle:?}");
    }
    println!("one byte short: refused with {:?}", report.truncated);
    Ok(())
}

// dojo: allow panic-path -- test assertions pin the example's output to independent values
#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-derived: 0 and 65 535 decode to the bounds exactly. 32 768 on `[-1, 1]` is
    /// `-2^32 + round(2^33 * 32 768 / 65 535)`; since `65 535 * 65 537 = 2^32 - 1`, that is
    /// `2^32 / 65 535 = 65 537 + 1/65 535` raw, which rounds to 65 537.
    #[test]
    fn decoded_values_match_the_hand_derivation() -> Result<(), Box<dyn Error>> {
        let report = run()?;
        let (lo, hi) = (-ONE_RAW, ONE_RAW);
        assert_eq!(report.byte_len, 76 + 4 * 6 + 4 * 6);
        assert_eq!((raw(report.bounds.min), raw(report.bounds.max)), ([lo; 3], [hi; 3]));
        let positions: Vec<_> = report.positions.into_iter().map(raw).collect();
        assert_eq!(positions, [[lo, lo, lo], [hi, lo, lo], [lo, hi, lo], [65_537, 65_537, hi]],);
        assert_eq!(report.triangles, [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]);
        assert_eq!(report.truncated, CompressedMeshError::PayloadTooShort);
        Ok(())
    }
}
