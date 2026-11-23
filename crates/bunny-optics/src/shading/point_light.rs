//! A point light and its sample at a surface hit: `bunny-point-light/1`.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};

use super::hit::SurfaceHit;
use super::light::{check_radiance, LightSample};
use super::wide::{div_round, dot, narrow, round_shift, sub, unit, wide, Wide, ONE};
use super::{ShadingError, FRAC_1_PI};

/// The versioned name of [`PointLight`] and [`SurfaceHit::point_light_sample`].
/// `bunny-diffuse-shading/1` does not change.
pub const POINT_LIGHT_ID: &str = "bunny-point-light/1";

/// The minimum distance from a point to a point light, raw Q32.32: `2^25` (1/128 unit). A
/// point at this distance or nearer is refused with [`PointLightError::TooNear`].
///
/// At any farther distance the irradiance of the brightest light ([`super::MAX_LIGHT_RAW`])
/// is below `2^30` units, so no step leaves Q32.32.
pub const MIN_LIGHT_DISTANCE_RAW: i64 = 1 << 25;

/// The squared minimum distance in raw units squared: `2^50`.
const MIN_DISTANCE_SQUARED: i128 = 1 << 50;

/// A light at a point with no size: hard shadows that end at the light.
///
/// `intensity` is, per channel, the radiant intensity of the light (power per unit solid
/// angle), the same in every direction. A surface at distance `d` whose normal makes the
/// angle `theta` with the direction to the light receives irradiance
/// `intensity * cos(theta) / d^2`; a [`super::DirectionalLight`] is given by that irradiance
/// itself. See `docs/topics/optics/README.md#point-light`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointLight {
    position: FixedVec3,
    intensity: [FixedQ32_32; 3],
}

impl PointLight {
    /// A point light at `position` with `intensity` per channel.
    ///
    /// # Errors
    ///
    /// [`PointLightError::PositionOutsideTraceDomain`] when a coordinate magnitude of
    /// `position` is at least `TRACE_COORD_LIMIT_RAW` (256 units); then
    /// [`PointLightError::NegativeIntensity`] when a channel is below zero, or
    /// [`PointLightError::IntensityTooLarge`] when one exceeds [`super::MAX_LIGHT_RAW`].
    pub fn new(position: FixedVec3, intensity: [FixedQ32_32; 3]) -> Result<Self, PointLightError> {
        if !in_domain(position) {
            return Err(PointLightError::PositionOutsideTraceDomain { position });
        }
        check_radiance(intensity).map_err(|e| match e {
            ShadingError::NegativeRadiance => PointLightError::NegativeIntensity,
            _ => PointLightError::IntensityTooLarge,
        })?;
        Ok(Self { position, intensity })
    }

    /// The position, as given.
    #[must_use]
    pub const fn position(&self) -> FixedVec3 {
        self.position
    }

    /// The intensity per channel, as given.
    #[must_use]
    pub const fn intensity(&self) -> [FixedQ32_32; 3] {
        self.intensity
    }

    /// The irradiance at `point` on a surface of unit normal `normal`:
    /// `intensity_c * c / d^2` per channel, with `v = position - point` (exact), `d^2 = v . v`
    /// (exact) and `c` the cosine. Each step rounds once to nearest, ties to even:
    ///
    /// 1. `c = normal . unit(v)`, with `unit(v)` normalized as [`super::DirectionalLight`]
    ///    normalizes its direction (within one raw unit per component), then clamped to
    ///    `0..=1.0`;
    /// 2. `g = c / d^2`, one division of the exact `c 2^64` by the exact `v . v`;
    /// 3. `intensity_c * g`, one Q32.32 product per channel.
    ///
    /// Proven infallible past the refusals: `g` is at most `2^14` units and each intensity at
    /// most `2^16`.
    ///
    /// # Errors
    ///
    /// [`PointLightError::PointOutsideTraceDomain`] when a coordinate magnitude of `point` is
    /// at least `TRACE_COORD_LIMIT_RAW`; then [`PointLightError::TooNear`] when the distance is
    /// at most [`MIN_LIGHT_DISTANCE_RAW`] (`v . v <= 2^50`, exact).
    pub fn irradiance(
        &self,
        point: FixedVec3,
        normal: FixedVec3,
    ) -> Result<[FixedQ32_32; 3], PointLightError> {
        let to_light = self.vector_to(point)?;
        let cosine = round_shift(dot(wide(normal), unit_toward(to_light)), 32).clamp(0, ONE);
        let falloff = div_round(cosine << 64, dot(to_light, to_light));
        let falloff = FixedQ32_32::from_raw(i64::try_from(falloff).unwrap_or(0));
        Ok(self.intensity.map(|i| i * falloff))
    }

    /// `position - point`, exact, after the refusals of [`Self::irradiance`].
    fn vector_to(&self, point: FixedVec3) -> Result<Wide, PointLightError> {
        if !in_domain(point) {
            return Err(PointLightError::PointOutsideTraceDomain { point });
        }
        let to_light = sub(wide(self.position), wide(point));
        if dot(to_light, to_light) <= MIN_DISTANCE_SQUARED {
            return Err(PointLightError::TooNear { to_light: narrow(to_light) });
        }
        Ok(to_light)
    }
}

impl SurfaceHit {
    /// The direct term of `light` at this hit and the shadow ray that decides it.
    ///
    /// With `v = light.position() - self.point()` (exact): when `v . N <= 0` (exact; `N` the
    /// exact normal of the face the ray arrived on), the light is along or behind the surface
    /// and the sample is dark. Otherwise, with `m` the largest component magnitude of `v`, the
    /// shadow ray runs from [`Self::point`] along `v 2^32 / m`, each component rounded to
    /// nearest with ties to even (the largest is exactly 1.0), and its
    /// [`LightSample::shadow_t_max`] is `m - 1` raw: the light is at distance exactly `m` along
    /// the largest axis, so an occluder counts exactly when it is nearer than the light. The
    /// radiance is `irradiance_c * FRAC_1_PI`, one Q32.32 product per channel, with
    /// [`PointLight::irradiance`] at [`Self::point`] and the frame normal.
    ///
    /// # Errors
    ///
    /// [`PointLightError::TooNear`] when the start point is at most
    /// [`MIN_LIGHT_DISTANCE_RAW`] from the light, before the side test.
    pub fn point_light_sample(&self, light: &PointLight) -> Result<LightSample, PointLightError> {
        let irradiance = light.irradiance(self.point(), self.frame().normal())?;
        let to_light = sub(wide(light.position), wide(self.point()));
        if dot(self.facing_normal(), to_light) <= 0 {
            return Ok(LightSample::DARK);
        }
        let largest = to_light.iter().map(|c| c.abs()).max().unwrap_or(ONE);
        let direction = narrow(to_light.map(|c| div_round(c * ONE, largest)));
        let t_max = FixedQ32_32::from_raw(i64::try_from(largest - 1).unwrap_or(0));
        let radiance = irradiance.map(|e| e * FRAC_1_PI);
        Ok(LightSample::bounded(TraceRay::new(self.point(), direction), t_max, radiance))
    }
}

/// Why a point light or its irradiance was refused. Each variant carries the values the check
/// saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointLightError {
    /// A coordinate of the light's position has a magnitude of at least
    /// `TRACE_COORD_LIMIT_RAW` (256 units).
    PositionOutsideTraceDomain {
        /// The position, as given.
        position: FixedVec3,
    },
    /// An intensity channel is below zero.
    NegativeIntensity,
    /// An intensity channel exceeds [`super::MAX_LIGHT_RAW`].
    IntensityTooLarge,
    /// A coordinate of the lit point has a magnitude of at least `TRACE_COORD_LIMIT_RAW`.
    PointOutsideTraceDomain {
        /// The point, as given.
        point: FixedVec3,
    },
    /// The point is at most [`MIN_LIGHT_DISTANCE_RAW`] from the light: `v . v <= 2^50`
    /// exactly, with `v` the vector from the point to the light.
    TooNear {
        /// The vector from the point to the light.
        to_light: FixedVec3,
    },
}

impl fmt::Display for PointLightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = |v: &FixedVec3| (v.x.raw(), v.y.raw(), v.z.raw());
        match self {
            Self::PositionOutsideTraceDomain { position } => {
                write!(f, "light position raw {:?} is outside the trace domain", raw(position))
            }
            Self::NegativeIntensity => f.write_str("light intensity is negative"),
            Self::IntensityTooLarge => f.write_str("light intensity exceeds the accepted maximum"),
            Self::PointOutsideTraceDomain { point } => {
                write!(f, "lit point raw {:?} is outside the trace domain", raw(point))
            }
            Self::TooNear { to_light } => {
                write!(
                    f,
                    "point is within the minimum distance of the light: raw {:?}",
                    raw(to_light)
                )
            }
        }
    }
}

impl std::error::Error for PointLightError {}

/// Whether every coordinate magnitude of `v` is below `TRACE_COORD_LIMIT_RAW`.
fn in_domain(v: FixedVec3) -> bool {
    [v.x, v.y, v.z].iter().all(|c| c.raw().unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs())
}

/// The unit vector along the nonzero `v` as raw components, or zero if it has none.
fn unit_toward(v: Wide) -> Wide {
    unit(v).map_or([0; 3], |u| wide(u.into_inner()))
}
