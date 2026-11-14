#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Deterministic camera and ray-generation math for Bunny.
//!
//! A [`PinholeCamera`] turns a position, a look direction, an up hint, the tangent of half the
//! vertical field of view and an image size into one [`bunny_query::TraceRay`] per pixel
//! sample, with integer arithmetic only: no trigonometry, and no square root or division per
//! ray. Directions are not normalized; every component is at most 1.0 in magnitude, as the
//! trace kernels require. See `docs/topics/optics/README.md`.

// The internal modules mark crate-private items `pub`, as `clippy::redundant_pub_crate`
// asks; the workspace's `unreachable_pub` lint disagrees, and is allowed on those modules.
#[allow(unreachable_pub)]
mod basis;
#[allow(clippy::module_name_repetitions)]
mod camera;
#[allow(clippy::module_name_repetitions)]
mod error;
#[allow(unreachable_pub)]
mod plane;
mod point;

pub use camera::{
    CameraSpec, PinholeCamera, PixelSample, MAX_IMAGE_DIMENSION, TAN_HALF_FOV_LIMIT_RAW,
};
pub use error::CameraError;
pub use point::point_at;
