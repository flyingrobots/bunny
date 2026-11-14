#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// `clippy::nursery` is cherry-picked, not denied as a group (CODE_STANDARDS.md, Rule 10):
// - `missing_const_for_fn` keeps the integer helpers usable in constant expressions;
// - `redundant_pub_crate` keeps crate-private items `pub` inside private modules, the
//   convention the module allowances below rely on;
// - `use_self` keeps `Self` in impls, so a renamed type changes one line.
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::redundant_pub_crate)]
#![deny(clippy::use_self)]
#![deny(missing_docs)]

//! Deterministic camera, sampling, diffuse shading and pixel-value math for Bunny.
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
//!
//! The diffuse shading steps of a path tracer with at most one bounce (`bunny-diffuse-shading/1`)
//! turn trace hits into radiance samples: a [`TriangleSurface`] holds a triangle's unit normal
//! and [`ShadingFrame`], a [`SurfaceHit`] applies the facing rule and starts secondary rays
//! strictly on the arrival side of the surface, [`ShadingFrame::cosine_direction`] maps a sampler
//! point to a cosine-weighted bounce direction, a [`DirectionalLight`] gives the direct term and
//! its shadow ray, and [`Albedo::reflect`] combines the direct and indirect radiance. The
//! `try_` forms of the surface hit, the bounce direction and the bounce ray name each refusal
//! with a [`SecondaryRayError`].
//!
//! The pixel-value functions turn the radiance samples into output codes: a
//! [`PixelAccumulator`] sums linear samples exactly and resolves their mean, a [`ToneCurve`]
//! maps linear radiance to a display value, and [`quantize_8`], [`quantize_16`] and
//! [`write_rgba8`] produce 8-bit and 16-bit codes, writing into caller-provided slices.

// The internal modules mark crate-private items `pub`, as `clippy::redundant_pub_crate`
// asks; the workspace's `unreachable_pub` lint disagrees, and is allowed on those modules.
#[allow(unreachable_pub)]
mod basis;
#[allow(
    clippy::module_name_repetitions,
    reason = "`CameraSpec` and `PinholeCamera` are the public names; the module is private"
)]
mod camera;
#[allow(
    clippy::module_name_repetitions,
    reason = "`CameraError` is the public name; the module is private"
)]
mod error;
#[allow(unreachable_pub)]
mod hash;
#[allow(unreachable_pub)]
mod pixel;
#[allow(unreachable_pub)]
mod plane;
mod point;
mod sampler;
#[allow(unreachable_pub)]
mod shading;
mod xoroshiro;

pub use camera::{
    CameraSpec, PinholeCamera, PixelSample, MAX_IMAGE_DIMENSION, TAN_HALF_FOV_LIMIT_RAW,
};
pub use error::CameraError;
pub use hash::{fraction_from_bits, PixelStream, SAMPLE_HASH_ID};
pub use pixel::{
    quantize_16, quantize_8, write_rgba8, PixelAccumulator, PixelError, ToneCurve,
    MAX_SAMPLES_PER_PIXEL, MAX_SAMPLE_RADIANCE_RAW, RGBA8_OPAQUE_IDENTITY, UNORM16_IDENTITY,
    UNORM8_IDENTITY,
};
pub use point::point_at;
pub use sampler::{PixelSampler, SAMPLER_ID};
pub use shading::{
    Albedo, DirectionalLight, LightSample, SecondaryRayError, ShadingError, ShadingFrame, SkyLight,
    SurfaceHit, TriangleSurface, FRAC_1_PI, MAX_LIGHT_RAW, MIN_BOUNCE_COSINE_RAW, PI, SHADING_ID,
    SHADOW_T_MAX,
};
pub use xoroshiro::Xoroshiro128Plus;
