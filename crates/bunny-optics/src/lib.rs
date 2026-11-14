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
//!
//! Sample positions and random decisions come from pure functions of their key, so an image
//! cannot depend on the order or the number of threads that compute it: [`PixelStream`]
//! (`bunny-sample-hash/1`) gives 64 mixed bits per (seed, pixel, sample, dimension), and
//! [`PixelSampler`] (`bunny-sobol-owen/1`) gives stratified points in the unit square per
//! (seed, pixel, sample, dimension pair). [`Xoroshiro128Plus`] is a sequential generator for
//! code that draws in one fixed order.

// The internal modules mark crate-private items `pub`, as `clippy::redundant_pub_crate`
// asks; the workspace's `unreachable_pub` lint disagrees, and is allowed on those modules.
#[allow(unreachable_pub)]
mod basis;
#[allow(clippy::module_name_repetitions)]
mod camera;
#[allow(clippy::module_name_repetitions)]
mod error;
#[allow(unreachable_pub)]
mod hash;
#[allow(unreachable_pub)]
mod plane;
mod point;
mod sampler;
mod xoroshiro;

pub use camera::{
    CameraSpec, PinholeCamera, PixelSample, MAX_IMAGE_DIMENSION, TAN_HALF_FOV_LIMIT_RAW,
};
pub use error::CameraError;
pub use hash::{fraction_from_bits, PixelStream, SAMPLE_HASH_ID};
pub use point::point_at;
pub use sampler::{PixelSampler, SAMPLER_ID};
pub use xoroshiro::Xoroshiro128Plus;
