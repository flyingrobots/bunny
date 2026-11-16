//! The perfect mirror at a surface hit: `bunny-specular-reflection/1`.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use super::hit::SurfaceHit;
use super::light::Albedo;
use super::refusal::SecondaryRayError;
use super::wide::{dot, narrow, pick, round_shift, unit, wide, Wide, ONE};

/// The versioned name of [`schlick_fresnel`], [`SurfaceHit::specular_reflection`] and
/// [`SurfaceHit::try_specular_reflection`]. `bunny-diffuse-shading/1` does not change.
pub const SPECULAR_REFLECTION_ID: &str = "bunny-specular-reflection/1";

/// A continuation ray and the throughput that weights the radiance arriving along it.
///
/// For a delta (mirror-like) lobe the throughput is the whole weight of the path segment: the
/// radiance this sample contributes is `throughput_c * L_c`, with `L` the radiance the ray
/// brings back. Each throughput channel is in `0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecularSample {
    ray: TraceRay,
    throughput: [FixedQ32_32; 3],
}

impl SpecularSample {
    /// The continuation ray, from the hit's start point. Query it with a minimum distance of
    /// zero, as for [`SurfaceHit::bounce_ray`].
    #[must_use]
    pub const fn ray(&self) -> &TraceRay {
        &self.ray
    }

    /// The throughput per channel, each in `0..=1.0`.
    #[must_use]
    pub const fn throughput(&self) -> [FixedQ32_32; 3] {
        self.throughput
    }
}

/// Schlick's approximation of the Fresnel reflectance, `f0 + (1 - f0) (1 - cosine)^5`.
///
/// Both inputs are clamped to `0..=1.0` first. With `m = 1 - cosine` (exact), the weight
/// `w = m^5` is computed exactly (a 160-bit product) and rounded once to nearest raw unit; a
/// tie cannot occur, because a tie needs `m^5` to be an odd multiple of `2^127`, and `127` is
/// not a multiple of five. Then `f0 + (1 - f0) w` is exact in units of `2^-64` and rounded
/// once to nearest, ties to even. The result is in `0..=1.0` and within one raw unit of the
/// exact formula on the clamped inputs; at `cosine = 1.0` it is `f0` exactly and at
/// `cosine = 0` it is 1.0 exactly. Proven infallible: every intermediate is below `2^66`.
#[must_use]
pub fn schlick_fresnel(f0: FixedQ32_32, cosine: FixedQ32_32) -> FixedQ32_32 {
    let f0 = clamp_unit(f0);
    let m = ONE - clamp_unit(cosine);
    let weight = fifth_power(m.unsigned_abs());
    let exact = f0 * ONE + (ONE - f0) * weight;
    let raw = round_shift(exact, 32);
    FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
}

impl SurfaceHit {
    /// The perfect mirror at this hit for the ray `incoming` that reached it, with base
    /// reflectance `reflectance` (Schlick's `F0` per channel). Returns `None` exactly when
    /// [`Self::try_specular_reflection`] returns an error; this is the fast path.
    #[must_use]
    pub fn specular_reflection(
        &self,
        incoming: &TraceRay,
        reflectance: &Albedo,
    ) -> Option<SpecularSample> {
        self.try_specular_reflection(incoming, reflectance).ok()
    }

    /// The diagnostic path of [`Self::specular_reflection`].
    ///
    /// With `d` the incoming direction, `n` the facing frame normal ([`Self::frame`]) and `N`
    /// the exact normal of the face the ray arrived on:
    ///
    /// 1. **Direction.** `r = d (n . n) - 2 (d . n) n`, exact in `i128`: the exact reflection of
    ///    `d` about the line of `n`, scaled by `n . n`. It is normalized as
    ///    [`super::TriangleSurface`] normalizes a face normal (each component within one raw
    ///    unit of the exact unit vector along `r`, plus the prescale's floor, below `2^-28`
    ///    raw units) and clamped to `-1.0..=1.0`, so it is a valid trace direction.
    /// 2. **Cosine.** `c = -(u . n)`, with `u` the unit vector along `d` (normalized the same
    ///    way), exact in `i128`, rounded once to nearest (ties to even) and clamped to
    ///    `0..=1.0`, as [`super::DirectionalLight::cosine`] is.
    /// 3. **Throughput.** [`schlick_fresnel`]`(F0_c, c)` per channel. At normal incidence
    ///    (`c = 1.0`) it is the base reflectance exactly.
    ///
    /// The ray runs from [`Self::point`] along the direction of step 1.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`SecondaryRayError::ArrivesBelowSurface`] when `d . N >= 0`,
    /// exact: the ray does not arrive from the side this hit faces;
    /// [`SecondaryRayError::ReflectionBelowSurface`] when the normalized direction `r'` has
    /// `r' . N <= 0`, exact (a grazing ray whose reflection about the rounded normal is along
    /// or under the exact plane); the defensive [`SecondaryRayError::BounceRayRefused`] if
    /// `TraceRay::new` refuses the ray, which no input is known to reach.
    pub fn try_specular_reflection(
        &self,
        incoming: &TraceRay,
        reflectance: &Albedo,
    ) -> Result<SpecularSample, SecondaryRayError> {
        let d = incoming.direction();
        if dot(wide(d), self.facing_normal()) >= 0 {
            return Err(SecondaryRayError::ArrivesBelowSurface { direction: d });
        }
        let n = wide(self.frame().normal());
        let direction = mirror(wide(d), n);
        if dot(wide(direction), self.facing_normal()) <= 0 {
            return Err(SecondaryRayError::ReflectionBelowSurface { direction });
        }
        let cosine = incidence_cosine(wide(d), n);
        let throughput = reflectance.channels().map(|f0| schlick_fresnel(f0, cosine));
        let origin = self.point();
        let ray = TraceRay::new(origin, direction)
            .ok_or(SecondaryRayError::BounceRayRefused { origin, direction })?;
        Ok(SpecularSample { ray, throughput })
    }
}

/// The unit vector along `d (n . n) - 2 (d . n) n`, clamped to `-1.0..=1.0` per component;
/// zero if it cannot be normalized (never: `r = 0` needs `d = 2 d`). With `|d_i| <= 2^32` and
/// `|n_i| <= 2^32 + 1`, every term is below `2^99`.
fn mirror(d: Wide, n: Wide) -> FixedVec3 {
    let (nn, dn) = (dot(n, n), dot(d, n));
    let r = [0, 1, 2].map(|i| pick(d, i) * nn - 2 * dn * pick(n, i));
    let direction = unit(r).map_or([0; 3], |u| wide(u.into_inner()));
    narrow(direction.map(|c| c.clamp(-ONE, ONE)))
}

/// `-(unit(d) . n)`, rounded once to nearest (ties to even) and clamped to `0..=1.0`; zero
/// if `d` cannot be normalized (never: a trace direction is nonzero).
fn incidence_cosine(d: Wide, n: Wide) -> FixedQ32_32 {
    let u = unit(d).map_or([0; 3], |u| wide(u.into_inner()));
    let raw = round_shift(-dot(u, n), 32).clamp(0, ONE);
    FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
}

/// `round(m^5 / 2^128)` to nearest for `m <= 2^32`, from the exact 160-bit product
/// `m^2 * m^3`. No tie is possible (see [`schlick_fresnel`]).
fn fifth_power(m: u128) -> i128 {
    const LOW: u128 = (1 << 64) - 1;
    let (square, cube) = (m * m, m * m * m);
    // square <= 2^64 and each 64-bit half of the cube is below 2^64 (high half <= 2^32).
    let (high, low) = (square * (cube >> 64), square * (cube & LOW));
    let upper = high + (low >> 64);
    let remainder = ((upper & LOW) << 64) | (low & LOW);
    let quotient = (upper >> 64) + u128::from(remainder > 1 << 127);
    i128::try_from(quotient).unwrap_or(0)
}

/// The raw value of `value` clamped to `0..=1.0`.
fn clamp_unit(value: FixedQ32_32) -> i128 {
    i128::from(value.raw()).clamp(0, ONE)
}
