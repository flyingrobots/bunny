#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Deterministic camera, ray-generation and pixel-value math for Bunny.
//!
//! A [`PinholeCamera`] turns a position, a look direction, an up hint, the tangent of half the
//! vertical field of view and an image size into one [`bunny_query::TraceRay`] per pixel
//! sample, with integer arithmetic only: no trigonometry, and no square root or division per
//! ray. Directions are not normalized; every component is at most 1.0 in magnitude, as the
//! trace kernels require. See `docs/topics/optics/README.md`.
//!
//! The pixel-value functions turn those rays' radiance samples into output codes: a
//! [`PixelAccumulator`] sums linear samples exactly and resolves their mean, a [`ToneCurve`]
//! maps linear radiance to a display value, and [`quantize_8`], [`quantize_16`] and
//! [`write_rgba8`] produce 8-bit and 16-bit codes, writing into caller-provided slices.

// The internal modules mark crate-private items `pub`, as `clippy::redundant_pub_crate`
// asks; the workspace's `unreachable_pub` lint disagrees, and is allowed on those modules.
#[allow(unreachable_pub)]
mod basis;
#[allow(clippy::module_name_repetitions)]
mod camera;
#[allow(clippy::module_name_repetitions)]
mod error;
#[allow(unreachable_pub)]
mod pixel;
#[allow(unreachable_pub)]
mod plane;
mod point;

pub use camera::{
    CameraSpec, PinholeCamera, PixelSample, MAX_IMAGE_DIMENSION, TAN_HALF_FOV_LIMIT_RAW,
};
pub use error::CameraError;
pub use pixel::{
    quantize_16, quantize_8, write_rgba8, PixelAccumulator, PixelError, ToneCurve,
    MAX_SAMPLES_PER_PIXEL, MAX_SAMPLE_RADIANCE_RAW, RGBA8_OPAQUE_IDENTITY, UNORM16_IDENTITY,
    UNORM8_IDENTITY,
};
pub use point::point_at;
