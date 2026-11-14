//! Diffuse shading for a path tracer with at most one bounce: `bunny-diffuse-shading/1`.
//!
//! Every step a renderer takes between a trace hit and a radiance sample is one pure integer
//! function here: the triangle's normal and frame ([`TriangleSurface`]), the facing rule and the
//! start of secondary rays ([`SurfaceHit`]), the cosine-weighted bounce direction
//! ([`ShadingFrame::cosine_direction`]), the directional light's term and its shadow ray
//! ([`DirectionalLight`], [`LightSample`]), the sky ([`SkyLight`]) and the diffuse reflection
//! that combines them ([`Albedo::reflect`]). See `docs/topics/optics/README.md#diffuse-shading`.

mod frame;
mod hit;
mod light;
mod surface;
mod wide;

pub use frame::ShadingFrame;
pub use hit::SurfaceHit;
pub use light::{Albedo, DirectionalLight, LightSample, SkyLight};
pub use surface::TriangleSurface;

use core::fmt;

use bunny_num::FixedQ32_32;

/// The versioned name of this shading model. Any change to an output bit of a function it
/// names is a new version.
pub const SHADING_ID: &str = "bunny-diffuse-shading/1";

/// Pi in Q32.32: raw `13 493 037 705`, the nearest raw value to `pi * 2^32`
/// (`13 493 037 704.522...`), 0.478 raw units above pi.
pub const PI: FixedQ32_32 = FixedQ32_32::from_raw(13_493_037_705);

/// One over pi in Q32.32: raw `1 367 130 551`, the nearest raw value to `2^32 / pi`
/// (`1 367 130 551.153...`), 0.153 raw units below `1 / pi`. The directional light's term
/// multiplies by it.
pub const FRAC_1_PI: FixedQ32_32 = FixedQ32_32::from_raw(1_367_130_551);

/// The largest accepted light irradiance or sky radiance per channel, raw Q32.32: `2^48`
/// (65 536.0).
pub const MAX_LIGHT_RAW: i64 = 1 << 48;

/// The distance limit for a shadow ray's any-hit query: 512 units, raw `2^41`.
///
/// A shadow ray's direction is the light's `toward_light`, whose largest component magnitude is
/// exactly 1.0, so along that axis a distance `t` moves exactly `t` units. Its origin and every
/// triangle are inside the trace domain (below 256 units on every axis), so every occluder lies
/// at a distance below 512 and every limit of at least `2^41` gives the same answer.
pub const SHADOW_T_MAX: FixedQ32_32 = FixedQ32_32::from_raw(1 << 41);

/// Every bounce direction has a dot product with its frame's normal of at least this many raw
/// units (`2^15`, about `7.6e-6`): the direction is never below or along the surface.
pub const MIN_BOUNCE_COSINE_RAW: i64 = 1 << 15;

/// Why a light or a material was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadingError {
    /// An albedo channel is below zero or above 1.0.
    AlbedoOutOfRange,
    /// A light or sky channel is below zero.
    NegativeRadiance,
    /// A light or sky channel exceeds [`MAX_LIGHT_RAW`].
    RadianceTooLarge,
    /// The light direction's largest component magnitude is not exactly 1.0.
    LightNotNormalized,
}

impl fmt::Display for ShadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlbedoOutOfRange => "albedo channel is outside 0..=1",
            Self::NegativeRadiance => "light or sky radiance is negative",
            Self::RadianceTooLarge => "light or sky radiance exceeds the accepted maximum",
            Self::LightNotNormalized => "light direction's largest component is not exactly 1",
        })
    }
}

impl std::error::Error for ShadingError {}
