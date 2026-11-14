#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Ray-casting and spatial query math solvers for Bunny.

mod barycentric;
#[allow(clippy::module_name_repetitions)]
mod closest;
#[allow(clippy::module_name_repetitions)]
mod ray;
#[allow(clippy::module_name_repetitions)]
mod trace;

pub use barycentric::BarycentricNumerators;
pub use closest::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
};
pub use ray::{
    ray_intersects_aabb, ray_intersects_aabb_prepared, ray_intersects_sphere,
    ray_intersects_triangle, PreparedRay3,
};
pub use trace::{
    TraceRay, TraceTriangle, TriangleHit, TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW,
};
