//! Lights, albedo and the diffuse reflection that combines them.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use super::wide::{dot, round_shift, unit, wide, Wide, ONE};
use super::{ShadingError, FRAC_1_PI, MAX_LIGHT_RAW};

const ZERO: [FixedQ32_32; 3] = [FixedQ32_32::ZERO; 3];
const ONE_RAW: i64 = 1 << 32;

/// A light at infinity with no angular size: hard shadows, one direction for every point.
///
/// `toward_light` is the direction from a surface to the light; its largest component magnitude
/// must be exactly 1.0, so it is a valid trace direction as given. `irradiance` is, per channel,
/// the irradiance the light delivers to a surface perpendicular to it. A white diffuse surface
/// lit at cosine `c` reflects radiance `irradiance * c / pi`. See
/// `docs/topics/optics/README.md#radiometric-convention`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionalLight {
    toward_light: FixedVec3,
    unit: Wide,
    irradiance: [FixedQ32_32; 3],
}

impl DirectionalLight {
    /// A directional light.
    ///
    /// # Errors
    ///
    /// [`ShadingError::LightNotNormalized`] when the largest component magnitude of
    /// `toward_light` is not exactly 1.0 (raw `2^32`); then [`ShadingError::NegativeRadiance`]
    /// when an irradiance channel is below zero, or [`ShadingError::RadianceTooLarge`] when one
    /// exceeds [`MAX_LIGHT_RAW`].
    pub fn new(
        toward_light: FixedVec3,
        irradiance: [FixedQ32_32; 3],
    ) -> Result<Self, ShadingError> {
        let [x, y, z] = wide(toward_light).map(i128::unsigned_abs);
        if x.max(y).max(z) != ONE.unsigned_abs() {
            return Err(ShadingError::LightNotNormalized);
        }
        check_radiance(irradiance)?;
        // A vector with a component of exactly 1.0 and none larger normalizes.
        let unit = unit(wide(toward_light)).ok_or(ShadingError::LightNotNormalized)?;
        Ok(Self { toward_light, unit: wide(unit.into_inner()), irradiance })
    }

    /// The direction toward the light, as given: the shadow ray's direction.
    #[must_use]
    pub const fn toward_light(&self) -> FixedVec3 {
        self.toward_light
    }

    /// The irradiance per channel, as given.
    #[must_use]
    pub const fn irradiance(&self) -> [FixedQ32_32; 3] {
        self.irradiance
    }

    /// The cosine between `normal` (a unit normal, such as [`super::ShadingFrame::normal`]) and
    /// the light: `normal . unit(toward_light)`, exact in `i128`, rounded once to nearest (ties
    /// to even) and clamped to `0..=1.0`. `unit(toward_light)` is normalized once, when the
    /// light is built, to within one raw unit per component.
    #[must_use]
    pub fn cosine(&self, normal: FixedVec3) -> FixedQ32_32 {
        let raw = round_shift(dot(wide(normal), self.unit), 32).clamp(0, ONE);
        FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
    }

    /// The radiance a white diffuse surface reflects under this light at `cosine`:
    /// `irradiance_c * (cosine * FRAC_1_PI)`, two Q32.32 multiplications per channel, each
    /// rounded to nearest with ties to even, the second per channel.
    ///
    /// `cosine` is clamped to `0..=1.0` first. Proven infallible: the weight is at most
    /// `1 / pi` and each irradiance at most `2^48` raw, so no product leaves Q32.32.
    #[must_use]
    pub fn reflected_radiance(&self, cosine: FixedQ32_32) -> [FixedQ32_32; 3] {
        let cosine = FixedQ32_32::from_raw(cosine.raw().clamp(0, ONE_RAW));
        let weight = cosine * FRAC_1_PI;
        self.irradiance.map(|e| e * weight)
    }
}

/// The constant radiance of every ray that leaves the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkyLight {
    radiance: [FixedQ32_32; 3],
}

impl SkyLight {
    /// A uniform sky of `radiance` per channel.
    ///
    /// # Errors
    ///
    /// [`ShadingError::NegativeRadiance`] when a channel is below zero, or
    /// [`ShadingError::RadianceTooLarge`] when one exceeds [`MAX_LIGHT_RAW`].
    pub fn new(radiance: [FixedQ32_32; 3]) -> Result<Self, ShadingError> {
        check_radiance(radiance)?;
        Ok(Self { radiance })
    }

    /// The radiance of a ray that hits nothing, camera or bounce ray.
    #[must_use]
    pub const fn radiance(&self) -> [FixedQ32_32; 3] {
        self.radiance
    }
}

/// The linear reflectance of a diffuse (Lambertian) surface per channel, each in `0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Albedo {
    channels: [FixedQ32_32; 3],
}

impl Albedo {
    /// An albedo.
    ///
    /// # Errors
    ///
    /// [`ShadingError::AlbedoOutOfRange`] when a channel is below zero or above 1.0.
    pub fn new(channels: [FixedQ32_32; 3]) -> Result<Self, ShadingError> {
        let valid = channels.iter().all(|c| (0..=ONE_RAW).contains(&c.raw()));
        valid.then_some(Self { channels }).ok_or(ShadingError::AlbedoOutOfRange)
    }

    /// The channels, as given.
    #[must_use]
    pub const fn channels(&self) -> [FixedQ32_32; 3] {
        self.channels
    }

    /// The radiance this surface reflects: `albedo_c * (direct_c + indirect_c)` per channel. The
    /// sum is exact; the product is one Q32.32 multiplication rounded to nearest, ties to even.
    ///
    /// `direct` is the light's term ([`LightSample::radiance`]) and `indirect` the radiance
    /// arriving along the bounce ray (zero without a bounce). With cosine-weighted bounce
    /// directions the cosine and `1 / pi` cancel against the density, so the bounce term is
    /// albedo times incoming radiance and has no `pi`.
    ///
    /// Returns `None` when a sum is not representable. Every product is at most its sum in
    /// magnitude, because albedo is at most 1.0.
    #[must_use]
    pub fn reflect(
        &self,
        direct: [FixedQ32_32; 3],
        indirect: [FixedQ32_32; 3],
    ) -> Option<[FixedQ32_32; 3]> {
        let [a, b, c] = self.channels;
        let [da, db, dc] = direct;
        let [ia, ib, ic] = indirect;
        Some([a * da.checked_add(ia)?, b * db.checked_add(ib)?, c * dc.checked_add(ic)?])
    }
}

/// A directional light's term at one surface hit, before visibility is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightSample {
    ray: Option<TraceRay>,
    radiance: [FixedQ32_32; 3],
}

impl LightSample {
    /// No shadow ray and no light: the light is behind the surface.
    pub(crate) const DARK: Self = Self { ray: None, radiance: ZERO };

    /// A sample with a shadow ray; without one (`TraceRay::new` refused it) it is dark.
    pub(crate) const fn new(ray: Option<TraceRay>, radiance: [FixedQ32_32; 3]) -> Self {
        match ray {
            Some(_) => Self { ray, radiance },
            None => Self::DARK,
        }
    }

    /// The shadow ray, or `None` when the light is not on the arrival side of the surface (and,
    /// defensively, if the ray cannot be built; a [`super::SurfaceHit`] start point is always in
    /// the trace domain, so it can).
    #[must_use]
    pub const fn shadow_ray(&self) -> Option<&TraceRay> {
        self.ray.as_ref()
    }

    /// The direct radiance: the light's reflected radiance when `visible` (the shadow ray's
    /// any-hit query found nothing) and a shadow ray exists, zero otherwise.
    #[must_use]
    pub const fn radiance(&self, visible: bool) -> [FixedQ32_32; 3] {
        if visible && self.ray.is_some() {
            self.radiance
        } else {
            ZERO
        }
    }
}

fn check_radiance(radiance: [FixedQ32_32; 3]) -> Result<(), ShadingError> {
    if radiance.iter().any(|c| c.raw() < 0) {
        Err(ShadingError::NegativeRadiance)
    } else if radiance.iter().any(|c| c.raw() > MAX_LIGHT_RAW) {
        Err(ShadingError::RadianceTooLarge)
    } else {
        Ok(())
    }
}
