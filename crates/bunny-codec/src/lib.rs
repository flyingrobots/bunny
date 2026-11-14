#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Zero-copy mesh format adapters for Bunny.

mod compressed;
mod obj;
mod ply;
mod ply_ascii;

pub use compressed::{
    decode_compressed_mesh, CompressedIndexWidth, CompressedMesh, CompressedMeshError,
    CompressedTriangle,
};
pub use obj::{parse_obj_text, ObjError, ObjMesh, ObjTriangles, ObjVertex, ObjVertices};
pub use ply::{parse_binary_ply, PlyBinaryMesh, PlyError, PlyVertex};
pub use ply_ascii::{
    parse_ascii_ply, parse_ascii_ply_with_limits, parse_decimal_q32_32, AsciiPlyError,
    AsciiPlyErrorKind, AsciiPlyLimit, AsciiPlyLimitExceeded, AsciiPlyLimitedError, AsciiPlyLimits,
    AsciiPlyMesh, AsciiPlyTriangles, AsciiPlyVertex, AsciiPlyVertices, DecimalError,
    ASCII_PLY_PROFILE_ID, DECIMAL_Q32_32_ID, MAX_FRACTIONAL_SIGNIFICANT_DIGITS,
};
