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

//! Ray-casting and spatial query math solvers for Bunny.

mod barycentric;
#[allow(clippy::module_name_repetitions)]
mod closest;
#[allow(clippy::module_name_repetitions)]
mod frame;
#[allow(clippy::module_name_repetitions)]
mod point_query;
#[allow(clippy::module_name_repetitions)]
mod ray;
#[allow(clippy::module_name_repetitions)]
mod trace;

pub use barycentric::BarycentricNumerators;
pub use closest::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
};
pub use frame::{
    ObjectRay, ShapeFrame, ShapeFrameError, SHAPE_FRAME_AXIS_DOT_LIMIT_RAW, SHAPE_FRAME_ID,
    SHAPE_FRAME_MAX_DIRECTION_SHIFT,
};
pub use point_query::{PointQuery, TrianglePoint, POINT_BOX_GAP_LIMIT_RAW};
pub use ray::{
    ray_intersects_aabb, ray_intersects_aabb_prepared, ray_intersects_sphere,
    ray_intersects_triangle, PreparedRay3,
};
pub use trace::{
    BoxHit, CylinderHit, DiscHit, SphereHit, TraceBox, TraceBoxError, TraceCylinder,
    TraceCylinderError, TraceDisc, TraceDiscError, TraceRay, TraceSphere, TraceSphereError,
    TraceTriangle, TriangleHit, RAY_BOX_ID, RAY_CYLINDER_ID, RAY_DISC_ID, RAY_SPHERE_ID,
    TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW, TRACE_UNIT_DIRECTION_LIMIT_RAW,
};
