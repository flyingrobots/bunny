//! A directional light's direction from two angles or from any nonzero vector:
//! `bunny-light-direction/1`.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::{FixedAngle, FixedQ32_32};

use super::light::DirectionalLight;
use super::wide::{div_round, narrow, wide, ONE};
use super::ShadingError;

/// The versioned name of [`DirectionalLight::from_angles`] and
/// [`DirectionalLight::from_direction`]. `bunny-diffuse-shading/1` does not change.
pub const LIGHT_DIRECTION_ID: &str = "bunny-light-direction/1";

impl DirectionalLight {
    /// A directional light toward `(cos e cos a, sin e, cos e sin a)`, with `a` the azimuth
    /// and `e` the elevation, scaled by [`Self::from_direction`].
    ///
    /// The direction points toward the light, as [`Self::new`]'s does. Azimuth 0 and
    /// elevation 0 point along `+x`; a positive azimuth turns `+x` toward `+z` (the right-hand
    /// rule about `-y`); a positive elevation tilts toward `+y`. Every angle is accepted: an
    /// elevation past a quarter turn continues over the pole, and a light below the horizon
    /// (negative `y`) is valid.
    ///
    /// Sine and cosine come from [`FixedAngle::sin_cos`], each within one raw unit; the two
    /// products are Q32.32 multiplications rounded to nearest, ties to even. Each component of
    /// the result is within `5 sqrt(3) + 1/2 + 10^-8` (below 9.161) raw units of the exact
    /// direction divided by its largest component magnitude, and the sine of the angle between
    /// them is below `4.55 * 2^-32`. At an elevation of a quarter or three quarters of a turn
    /// the result is exactly `(0, +-1, 0)` for every azimuth.
    ///
    /// # Errors
    ///
    /// [`LightDirectionError::Light`] when an irradiance channel is below zero or exceeds
    /// [`super::MAX_LIGHT_RAW`]. The direction is never zero: `sin e` or `cos e` is at least
    /// `2^32 / sqrt(2) - 1` raw in magnitude, and so is `sin a` or `cos a`, so `y` or one of
    /// the products is nonzero.
    pub fn from_angles(
        azimuth: FixedAngle,
        elevation: FixedAngle,
        irradiance: [FixedQ32_32; 3],
    ) -> Result<Self, LightDirectionError> {
        let (sin_a, cos_a) = azimuth.sin_cos();
        let (sin_e, cos_e) = elevation.sin_cos();
        Self::from_direction(FixedVec3::new(cos_e * cos_a, sin_e, cos_e * sin_a), irradiance)
    }

    /// A directional light toward `direction`, of any nonzero length: each component times
    /// `2^32` divided by the exact largest component magnitude `m`, rounded once to nearest,
    /// ties to even. The largest component is exactly `+-1.0`, no component exceeds it, and
    /// every component is within half a raw unit of `v_i 2^32 / m`. A direction already scaled
    /// gives [`Self::new`]'s light bit for bit, and every positive integer multiple of a
    /// direction gives the same bits.
    ///
    /// # Errors
    ///
    /// [`LightDirectionError::ZeroDirection`] for the zero vector; then
    /// [`LightDirectionError::Light`] when an irradiance channel is below zero or exceeds
    /// [`super::MAX_LIGHT_RAW`].
    pub fn from_direction(
        direction: FixedVec3,
        irradiance: [FixedQ32_32; 3],
    ) -> Result<Self, LightDirectionError> {
        let v = wide(direction);
        let largest = v.iter().map(|c| c.abs()).max().unwrap_or(0);
        if largest == 0 {
            return Err(LightDirectionError::ZeroDirection);
        }
        // |v_i| <= 2^63, so v_i 2^32 fits i128 and each quotient is at most 2^32.
        let toward_light = narrow(v.map(|c| div_round(c * ONE, largest)));
        Self::new(toward_light, irradiance).map_err(LightDirectionError::Light)
    }
}

/// Why [`DirectionalLight::from_angles`] or [`DirectionalLight::from_direction`] refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LightDirectionError {
    /// The direction is the zero vector, which points nowhere.
    ZeroDirection,
    /// [`DirectionalLight::new`] refused the irradiance: [`ShadingError::NegativeRadiance`] or
    /// [`ShadingError::RadianceTooLarge`]. The scaled direction always has a largest component
    /// of exactly 1.0, so [`ShadingError::LightNotNormalized`] does not occur.
    Light(ShadingError),
}

impl fmt::Display for LightDirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDirection => f.write_str("light direction is the zero vector"),
            Self::Light(error) => write!(f, "light refused: {error}"),
        }
    }
}

impl std::error::Error for LightDirectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ZeroDirection => None,
            Self::Light(error) => Some(error),
        }
    }
}
