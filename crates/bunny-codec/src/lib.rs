#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Zero-copy mesh format adapters for Bunny.

mod ply;

pub use ply::{parse_binary_ply, PlyBinaryMesh, PlyError, PlyVertex};
