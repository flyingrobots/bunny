#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Bounding volume hierarchy (BVH) and spatial broadphase query solvers.

#[allow(clippy::module_name_repetitions)]
pub mod bvh;
pub mod sweep_and_prune;
#[allow(clippy::module_name_repetitions)]
pub mod traversal;
pub mod utils;

pub use bvh::{build_bvh, BvhNode};
pub use sweep_and_prune::sweep_and_prune;
pub use traversal::{intersect_aabb, intersect_ray, TraversalError};
