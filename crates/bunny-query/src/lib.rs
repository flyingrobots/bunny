#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Ray-casting and spatial query math solvers for Bunny.

mod closest;
mod ray;

pub use closest::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
};
pub use ray::{ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle};
