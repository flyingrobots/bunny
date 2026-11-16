//! The GGX rough conductor at a surface hit: `bunny-ggx-conductor/1`.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use super::frame::in_unit_interval;
use super::hit::SurfaceHit;
use super::light::Albedo;
use super::refusal::SecondaryRayError;
use super::smith::{masking_ratio, MIN_ALPHA};
use super::specular::{mirror, schlick_fresnel, SpecularSample};
use super::visible_normal::{disk_point, View};
use super::wide::{dot, round_shift, wide, Wide, ONE};

/// The versioned name of [`SurfaceHit::ggx_reflection`] and
/// [`SurfaceHit::try_ggx_reflection`]. It names `bunny-specular-reflection/1` for its
/// reflection and its Fresnel term; neither that identity nor `bunny-diffuse-shading/1`
/// changes.
pub const GGX_CONDUCTOR_ID: &str = "bunny-ggx-conductor/1";

/// The smallest accepted GGX roughness, raw Q32.32: `2^16` (`2^-16`, about `1.5e-5`).
///
/// It is the smallest roughness whose square, which the masking term uses, is at least one raw
/// unit; it is also the square of `1 / 256`, so the square of every nonzero 8-bit roughness
/// texel is accepted. Below it the lobe is narrower than `2^16` raw units of direction and the
/// masking ratio differs from 1.0 by less than a raw unit except within `2^-16` of grazing:
/// the sample would be the mirror of `bunny-specular-reflection/1` with rounding noise.
pub const MIN_GGX_ROUGHNESS_RAW: i64 = 1 << 16;

/// A conductor whose microfacets follow the GGX distribution: the roughness `alpha` (the
/// distribution's width, in `2^-16..=1.0`) and the base reflectance `F0` per channel.
///
/// `alpha` is the parameter of the distribution itself. A metal-roughness map stores its
/// square root; squaring a texel is the caller's step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GgxConductor {
    roughness: FixedQ32_32,
    reflectance: Albedo,
}

impl GgxConductor {
    /// A conductor with GGX roughness `roughness` and base reflectance `reflectance`.
    ///
    /// # Errors
    ///
    /// [`GgxRoughnessError`] when `roughness` is outside
    /// [`MIN_GGX_ROUGHNESS_RAW`]`..=2^32` raw (`2^-16..=1.0`).
    pub fn new(roughness: FixedQ32_32, reflectance: Albedo) -> Result<Self, GgxRoughnessError> {
        let valid = (MIN_ALPHA..=ONE).contains(&i128::from(roughness.raw()));
        valid.then_some(Self { roughness, reflectance }).ok_or(GgxRoughnessError { roughness })
    }

    /// The GGX roughness `alpha`, as given.
    #[must_use]
    pub const fn roughness(&self) -> FixedQ32_32 {
        self.roughness
    }

    /// The base reflectance, as given.
    #[must_use]
    pub const fn reflectance(&self) -> Albedo {
        self.reflectance
    }
}

/// A GGX roughness outside `2^-16..=1.0` (raw [`MIN_GGX_ROUGHNESS_RAW`]`..=2^32`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GgxRoughnessError {
    roughness: FixedQ32_32,
}

impl GgxRoughnessError {
    /// The refused roughness, as given.
    #[must_use]
    pub const fn roughness(&self) -> FixedQ32_32 {
        self.roughness
    }
}

impl fmt::Display for GgxRoughnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.roughness.raw();
        write!(f, "roughness raw {raw} is outside {MIN_GGX_ROUGHNESS_RAW}..={ONE}")
    }
}

impl std::error::Error for GgxRoughnessError {}

/// What a GGX conductor does with one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum GgxScatter {
    /// The path continues along the sample's ray, weighted by its throughput.
    Reflected(SpecularSample),
    /// The sampled direction does not leave the surface, so the path ends here and carries no
    /// radiance. This is a result, not a refusal: a single-scattering microfacet model loses
    /// this energy, more of it as the roughness grows.
    Absorbed,
}

impl SurfaceHit {
    /// One sample of a GGX conductor at this hit for the ray `incoming` that reached it and
    /// the unit-square point `sample`. Returns `None` exactly when
    /// [`Self::try_ggx_reflection`] returns an error; this is the fast path.
    #[must_use]
    pub fn ggx_reflection(
        &self,
        incoming: &TraceRay,
        conductor: &GgxConductor,
        sample: [FixedQ32_32; 2],
    ) -> Option<GgxScatter> {
        self.try_ggx_reflection(incoming, conductor, sample).ok()
    }

    /// The diagnostic path of [`Self::ggx_reflection`].
    ///
    /// The microfacet normal `m` is drawn from the GGX distribution of normals visible from
    /// the incoming direction (Heitz, "Sampling the GGX Distribution of Visible Normals",
    /// JCGT 7(4), 2018), from the one point `sample = [u, v]`. With `d` the incoming
    /// direction, `n` the facing frame normal and `N` the exact normal of the face:
    ///
    /// 1. **View.** `c_v = -(unit(d) . n)`, rounded once and clamped to `0..=1.0`, as the
    ///    mirror's cosine is. When `c_v = 0` the sample is absorbed.
    /// 2. **Microfacet normal.** `m`, a unit vector, from `d`, `n`, the roughness and the
    ///    disk point of `sample`; `docs/topics/optics/README.md#ggx-rough-conductor` states
    ///    each square root, product and normalization. When `d . m >= 0`, exact, the sample
    ///    is absorbed.
    /// 3. **Direction.** The mirror of `bunny-specular-reflection/1` about `m`: the unit
    ///    vector along `d (m . m) - 2 (d . m) m`, exact in `i128` and normalized once.
    ///    When it is along or below the exact plane (`r . N <= 0`) or its rounded cosine
    ///    `c_l = r . n` is zero or below, the sample is absorbed.
    /// 4. **Weight.** `throughput_c = F_c G`, rounded once to nearest (ties to even), with
    ///    `F_c = `[`schlick_fresnel`]`(F0_c, -(unit(d) . m))` and `G = G2 / G1`, the
    ///    height-correlated Smith ratio
    ///    `c_l (c_v + s(c_v)) / (c_l s(c_v) + c_v s(c_l))`,
    ///    `s(c) = sqrt(alpha^2 + (1 - alpha^2) c^2)`, rounded once. Each channel is within
    ///    `2 + 2^-11` raw units of the exact `F_c G` at those rounded cosines, and in
    ///    `0..=1.0`.
    ///
    /// The ray runs from [`Self::point`] along the direction of step 3.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`SecondaryRayError::SampleOutsideUnitInterval`] when a
    /// coordinate of `sample` is outside `[0, 1)`;
    /// [`SecondaryRayError::ArrivesBelowSurface`] when `d . N >= 0`, exact; the defensive
    /// [`SecondaryRayError::BounceRayRefused`] if `TraceRay::new` refuses the ray, which no
    /// input is known to reach. A direction that does not leave the surface is not an
    /// error: it is [`GgxScatter::Absorbed`].
    pub fn try_ggx_reflection(
        &self,
        incoming: &TraceRay,
        conductor: &GgxConductor,
        sample: [FixedQ32_32; 2],
    ) -> Result<GgxScatter, SecondaryRayError> {
        if !sample.into_iter().all(in_unit_interval) {
            return Err(SecondaryRayError::SampleOutsideUnitInterval { sample });
        }
        let d = incoming.direction();
        if dot(wide(d), self.facing_normal()) >= 0 {
            return Err(SecondaryRayError::ArrivesBelowSurface { direction: d });
        }
        let Some((direction, throughput)) = self.ggx_lobe(wide(d), conductor, sample) else {
            return Ok(GgxScatter::Absorbed);
        };
        let origin = self.point();
        let ray = TraceRay::new(origin, direction)
            .ok_or(SecondaryRayError::BounceRayRefused { origin, direction })?;
        Ok(GgxScatter::Reflected(SpecularSample::new(ray, throughput)))
    }

    /// The reflected direction and the throughput of [`Self::try_ggx_reflection`], or `None`
    /// for an absorbed sample.
    fn ggx_lobe(
        &self,
        d: Wide,
        conductor: &GgxConductor,
        sample: [FixedQ32_32; 2],
    ) -> Option<(FixedVec3, [FixedQ32_32; 3])> {
        let frame = self.frame();
        let view = View::new(d, &frame)?;
        let alpha = i128::from(conductor.roughness.raw());
        let facet = view.visible_normal(alpha, disk_point(sample));
        if dot(d, facet) >= 0 {
            return None;
        }
        let direction = mirror(d, facet);
        let leaves = dot(wide(direction), self.facing_normal()) > 0;
        let light = round_shift(dot(wide(direction), wide(frame.normal())), 32).clamp(0, ONE);
        if !leaves || light == 0 {
            return None;
        }
        let ratio = masking_ratio(alpha, view.cosine(), light);
        let half = FixedQ32_32::from_raw(i64::try_from(view.cosine_against(facet)).unwrap_or(0));
        let weigh = |f0| weight(schlick_fresnel(f0, half), ratio);
        Some((direction, conductor.reflectance.channels().map(weigh)))
    }
}

/// `fresnel * ratio`, exact in units of `2^-64` and rounded once to nearest, ties to even.
fn weight(fresnel: FixedQ32_32, ratio: i128) -> FixedQ32_32 {
    let raw = round_shift(i128::from(fresnel.raw()) * ratio, 32);
    FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
}
