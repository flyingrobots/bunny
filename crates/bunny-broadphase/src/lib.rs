#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// `clippy::nursery` is cherry-picked, not denied as a group (CODE_STANDARDS.md, Rule 10):
// - `missing_const_for_fn` keeps every function that can be `const` usable in constant
//   expressions, as the crate's value constructors and accessors are;
// - `redundant_pub_crate` keeps crate-private items `pub` inside private modules, so the
//   module's own visibility is the one boundary to read;
// - `use_self` keeps `Self` in impls, so a renamed type changes one line.
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::redundant_pub_crate)]
#![deny(clippy::use_self)]
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
