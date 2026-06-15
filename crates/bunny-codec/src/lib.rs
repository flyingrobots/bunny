#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Zero-copy mesh format adapters for Bunny.

mod compressed;
mod obj;
mod ply;

pub use compressed::{
    decode_compressed_mesh, CompressedIndexWidth, CompressedMesh, CompressedMeshError,
    CompressedTriangle,
};
pub use obj::{parse_obj_text, ObjError, ObjMesh, ObjTriangles, ObjVertex, ObjVertices};
pub use ply::{parse_binary_ply, PlyBinaryMesh, PlyError, PlyVertex};
