#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Bounding volume hierarchy (BVH) and spatial broadphase query solvers.

pub mod bvh;
pub mod sweep_and_prune;
pub mod traversal;
pub mod utils;

pub use bvh::{build_bvh, BvhNode};
pub use sweep_and_prune::sweep_and_prune;
pub use traversal::{intersect_aabb, intersect_ray};
