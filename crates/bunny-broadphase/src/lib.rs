#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Bounding volume hierarchy (BVH) and spatial broadphase query solvers.

#[allow(clippy::module_name_repetitions)]
pub mod bvh;
#[allow(clippy::module_name_repetitions)]
pub mod closest_hit;
#[allow(clippy::module_name_repetitions)]
pub mod compact;
#[allow(clippy::module_name_repetitions)]
pub mod compact_hit;
pub mod counts;
pub mod sweep_and_prune;
#[allow(clippy::module_name_repetitions)]
pub mod traversal;
pub mod utils;

pub use bvh::{build_bvh, build_bvh_sah, BvhNode};
pub use closest_hit::{any_hit, closest_hit, BvhView, ClosestHit};
pub use compact::{compact_bvh, CompactChild, CompactError, CompactNode, CompactRoot};
pub use compact_hit::{
    compact_any_hit, compact_any_hit_counted, compact_closest_hit, compact_closest_hit_counted,
    CompactView,
};
pub use counts::TraversalCounts;
pub use sweep_and_prune::sweep_and_prune;
pub use traversal::{intersect_aabb, intersect_ray, TraversalError};
