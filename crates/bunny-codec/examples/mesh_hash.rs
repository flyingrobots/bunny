//! Parse one mesh from OBJ, ASCII PLY and binary PLY, and compute its Bunny mesh hash.
//!
//! Run with `cargo run -p bunny-codec --example mesh_hash`. It needs the `mesh-hash`
//! feature, which is on by default.
//!
//! The three readers hand back positions in two forms: OBJ and binary PLY as `f32`, which
//! must cross the float boundary through a fallible conversion, and ASCII PLY as exact
//! Q32.32 already. The hash is over the quantized mesh, so the three files of the same
//! tetrahedron give the same hash only because every coordinate here converts exactly.

use std::error::Error;
use std::fmt::Write as _;

use bunny_codec::{parse_ascii_ply, parse_binary_ply, parse_obj_text};
use bunny_linalg::{FixedVec3, Vec3};
use bunny_mesh::{
    compute_mesh_hash, mesh_bounds, quantize_vertex, IndexBufferLayout, QuantizedVertex, Triangle32,
};

/// The OBJ reader converts its one-based face indices to zero-based ones.
const OBJ: &str = "\
o tetrahedron
v 0 0 0
v 1.5 0 0
v 0 1.5 0
v 0.5 0.5 -0.25
f 1 3 2
f 1 2 4
f 2 3 4
f 3 1 4
";

const PLY_HEADER: &str = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\n\
property float y\nproperty float z\nelement face 4\nproperty list uchar int vertex_indices\n\
end_header\n";
const PLY_BODY: &str =
    "0 0 0\n1.5 0 0\n0 1.5 0\n0.5 0.5 -0.25\n3 0 2 1\n3 0 1 3\n3 1 2 3\n3 2 0 3\n";

/// The same positions and faces, written into the binary PLY below.
const POSITIONS: [[f32; 3]; 4] =
    [[0.0, 0.0, 0.0], [1.5, 0.0, 0.0], [0.0, 1.5, 0.0], [0.5, 0.5, -0.25]];
const FACES: [[i32; 3]; 4] = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];

/// A mesh in Q32.32, as every reader's output is turned into before hashing.
struct FixedMesh {
    positions: Vec<FixedVec3>,
    triangles: Vec<Triangle32>,
}

/// Canonical binary little-endian PLY: three `f32` per vertex, then a `u8` count and three
/// `i32` per face.
fn binary_ply() -> Vec<u8> {
    let header = PLY_HEADER.replace("ascii", "binary_little_endian");
    let mut bytes = header.into_bytes();
    for value in POSITIONS.iter().flatten() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for face in FACES {
        bytes.push(3);
        for index in face {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
    }
    bytes
}

/// `try_from_float` refuses NaN, infinities and values outside Q32.32; the readers already
/// refuse non-finite input, so here only the range check can still fail.
fn from_float(x: f32, y: f32, z: f32) -> Result<FixedVec3, Box<dyn Error>> {
    Ok(FixedVec3::try_from_float(Vec3::new(x, y, z))?)
}

fn read_obj(text: &str) -> Result<FixedMesh, Box<dyn Error>> {
    let mesh = parse_obj_text(text)?;
    let positions = mesh
        .vertices()
        .map(|vertex| {
            let v = vertex?;
            from_float(v.x, v.y, v.z)
        })
        .collect::<Result<_, _>>()?;
    let triangles = mesh.triangles().collect::<Result<_, _>>()?;
    Ok(FixedMesh { positions, triangles })
}

fn read_ascii_ply(text: &str) -> Result<FixedMesh, Box<dyn Error>> {
    let mesh = parse_ascii_ply(text.as_bytes())?;
    let positions = mesh
        .vertices()
        .map(|vertex| vertex.map(|v| FixedVec3::new(v.x, v.y, v.z)))
        .collect::<Result<_, _>>()?;
    let triangles = mesh.triangles().collect::<Result<_, _>>()?;
    Ok(FixedMesh { positions, triangles })
}

fn read_binary_ply(bytes: &[u8]) -> Result<FixedMesh, Box<dyn Error>> {
    let mesh = parse_binary_ply(bytes)?;
    let mut positions = Vec::with_capacity(mesh.vertex_count());
    for index in 0..mesh.vertex_count() {
        let v = mesh.vertex(index)?;
        positions.push(from_float(v.x, v.y, v.z)?);
    }
    let triangles = (0..mesh.face_count()).map(|i| mesh.triangle(i)).collect::<Result<_, _>>()?;
    Ok(FixedMesh { positions, triangles })
}

/// Quantizes against the exact bounds of the used vertices, then hashes. The bounds are part
/// of the hash input, so two meshes with the same quantized grid but different extents differ.
fn hash(mesh: &FixedMesh) -> Result<String, Box<dyn Error>> {
    let faces = IndexBufferLayout::Width32(&mesh.triangles);
    let bounds = mesh_bounds(&mesh.positions, faces)?;
    let quantized: Vec<QuantizedVertex> =
        mesh.positions.iter().map(|&p| quantize_vertex(p, &bounds)).collect();
    let digest = compute_mesh_hash(&quantized, faces, &bounds);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        write!(hex, "{byte:02x}")?;
    }
    Ok(hex)
}

/// The hash of each source, in the order OBJ, ASCII PLY, binary PLY.
fn run() -> Result<[String; 3], Box<dyn Error>> {
    let obj = read_obj(OBJ)?;
    let ascii = read_ascii_ply(&format!("{PLY_HEADER}{PLY_BODY}"))?;
    let binary = read_binary_ply(&binary_ply())?;
    Ok([hash(&obj)?, hash(&ascii)?, hash(&binary)?])
}

fn main() -> Result<(), Box<dyn Error>> {
    let [obj, ascii, binary] = run()?;
    println!("OBJ        mesh hash: {obj}");
    println!("ASCII PLY  mesh hash: {ascii}");
    println!("binary PLY mesh hash: {binary}");
    Ok(())
}

// dojo: allow panic-path -- test assertions pin the example's output to independent values
#[cfg(test)]
mod tests {
    use super::*;

    /// Computed outside Rust with Python's `hashlib` over the framing that
    /// `compute_mesh_hash` documents: `bunny-mesh:v2`, the bounds `(0, 0, -2^30)` to
    /// `(1.5 * 2^32, 1.5 * 2^32, 0)` as six `i64`, four vertices quantized by hand to
    /// `(0, 0, 65535)`, `(65535, 0, 65535)`, `(0, 65535, 65535)`, `(21845, 21845, 0)`
    /// (0.5 of a 1.5 span is 65 535 / 3 = 21 845 exactly), the `Width32` tag and four faces.
    const EXPECTED: &str = "04df7781735f6560cc8ae2415c55a2cd748b969e594a5d7c3556889b2b86a482";

    #[test]
    fn every_reader_gives_the_independently_computed_hash() -> Result<(), Box<dyn Error>> {
        assert_eq!(run()?, [EXPECTED; 3].map(str::to_owned));
        Ok(())
    }

    #[test]
    fn readers_agree_on_counts_and_faces() -> Result<(), Box<dyn Error>> {
        let obj = read_obj(OBJ)?;
        let faces = FACES.map(|[a, b, c]| [a, b, c].map(i32::cast_unsigned));
        let read: Vec<_> = obj.triangles.iter().map(|t| [t.v0, t.v1, t.v2]).collect();
        assert_eq!((obj.positions.len(), read), (4, faces.to_vec()));
        Ok(())
    }
}
