//! The smooth boundary between two dielectrics at a surface hit: `bunny-dielectric/1`.

use core::fmt;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};

use super::hit::{within, SurfaceHit};
use super::light::Albedo;
use super::refusal::SecondaryRayError;
use super::snell::Incidence;
use super::specular::SpecularSample;
use super::wide::{dot, narrow, wide};

/// The versioned name of [`SurfaceHit::dielectric_reflectance`],
/// [`SurfaceHit::dielectric_scatter`] and [`SurfaceHit::try_dielectric_scatter`].
/// `bunny-specular-reflection/1` does not change.
pub const DIELECTRIC_ID: &str = "bunny-dielectric/1";

/// The largest accepted refractive index, raw Q32.32: 8.0. The smallest is 1.0.
pub const MAX_REFRACTIVE_INDEX_RAW: i64 = 8 << 32;

/// The refractive indices of the media on the two sides of a surface, each in
/// `1.0..=8.0` ([`MAX_REFRACTIVE_INDEX_RAW`]).
///
/// `outside` is the medium on the side of the triangle's front face (the side its vertices
/// appear counter-clockwise from) and `inside` the medium behind it. A ray that hits the front
/// face goes from `outside` into `inside`; a ray that hits the back face, the reverse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DielectricBoundary {
    outside: FixedQ32_32,
    inside: FixedQ32_32,
}

impl DielectricBoundary {
    /// A boundary between the index `outside`, on the front-face side, and `inside`.
    ///
    /// # Errors
    ///
    /// [`RefractiveIndexError`], carrying the first index (`outside`, then `inside`) that is
    /// below 1.0 or above [`MAX_REFRACTIVE_INDEX_RAW`].
    pub fn new(outside: FixedQ32_32, inside: FixedQ32_32) -> Result<Self, RefractiveIndexError> {
        let refused = [outside, inside]
            .into_iter()
            .find(|index| !(ONE_RAW..=MAX_REFRACTIVE_INDEX_RAW).contains(&index.raw()));
        refused.map_or(Ok(Self { outside, inside }), |index| Err(RefractiveIndexError { index }))
    }

    /// The index on the front-face side.
    #[must_use]
    pub const fn outside(&self) -> FixedQ32_32 {
        self.outside
    }

    /// The index behind the front face.
    #[must_use]
    pub const fn inside(&self) -> FixedQ32_32 {
        self.inside
    }

    /// The raw indices `[incident, transmitted]` for a ray that arrives on the front face
    /// (`true`) or the back face.
    const fn ordered(&self, front_face: bool) -> [i64; 2] {
        if front_face {
            [self.outside.raw(), self.inside.raw()]
        } else {
            [self.inside.raw(), self.outside.raw()]
        }
    }
}

/// A refractive index outside `1.0..=8.0`, refused by [`DielectricBoundary::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefractiveIndexError {
    index: FixedQ32_32,
}

impl RefractiveIndexError {
    /// The refused index, as given.
    #[must_use]
    pub const fn index(&self) -> FixedQ32_32 {
        self.index
    }
}

impl fmt::Display for RefractiveIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.index.raw();
        write!(f, "refractive index raw {raw} is outside {ONE_RAW}..={MAX_REFRACTIVE_INDEX_RAW}")
    }
}

impl std::error::Error for RefractiveIndexError {}

/// How much of a ray a dielectric boundary reflects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DielectricReflectance {
    /// The unpolarized Fresnel reflectance, in `0..=1.0`: the probability that
    /// [`SurfaceHit::dielectric_scatter`] reflects. Below 1.0 a refracted ray exists and
    /// some choice value takes it. At exactly 1.0 no choice value refracts: either the exact
    /// reflectance rounds to 1.0 just short of the critical angle, or the ray arrives along
    /// or behind the plane of the rounded normal (`d . n >= 0`) while still arriving against
    /// the exact face, and no refraction is computed.
    Partial(FixedQ32_32),
    /// Total internal reflection: Snell's law has no refracted ray, decided exactly.
    Total,
}

impl DielectricReflectance {
    /// The probability of reflection: the reflectance, or 1.0 for total internal reflection.
    #[must_use]
    pub const fn probability(&self) -> FixedQ32_32 {
        match self {
            Self::Partial(reflectance) => *reflectance,
            Self::Total => FixedQ32_32::ONE,
        }
    }
}

/// The branch a dielectric boundary took for one choice value, with its continuation ray.
///
/// Either way the sample's throughput is exactly 1.0 in every channel: the branch is chosen
/// with probability equal to its share of the energy, so the share and the probability cancel.
/// That is a weight for power. A renderer that carries radiance along a path from the camera
/// owes one more factor on the [`Self::Refracted`] branch, stated there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum DielectricScatter {
    /// The ray was reflected: the sample of [`SurfaceHit::try_specular_reflection`] with a
    /// base reflectance of 1.0, which stays in the medium the ray arrived through.
    Reflected(SpecularSample),
    /// The ray was refracted: it continues through the surface, into the other medium.
    ///
    /// **The radiance factor is not applied.** The throughput of 1.0 conserves power. Radiance
    /// also changes with the index: a path traced from the camera that refracts from the
    /// medium of index `a` into the medium of index `b` must multiply its throughput by
    /// `(a / b)^2` to carry radiance. From air into glass (`a = 1.0`, `b = 1.5`) that is
    /// `(1 / 1.5)^2 = 4 / 9`; from glass back into air, `9 / 4`. Over a whole path the
    /// factors multiply to `(first index / last index)^2`, so they cancel on a path that
    /// enters a medium and leaves it again, and a renderer needs the factor only when a path's
    /// two ends lie in media of different index. (For light, which travels the other way, the
    /// factor is the reciprocal.)
    Refracted(SpecularSample),
}

impl DielectricScatter {
    /// The continuation ray and its throughput, whichever branch was taken.
    #[must_use]
    pub const fn sample(&self) -> &SpecularSample {
        match self {
            Self::Reflected(sample) | Self::Refracted(sample) => sample,
        }
    }
}

impl SurfaceHit {
    /// The reflectance of `boundary` at this hit for the ray `incoming` that reached it.
    ///
    /// With `d` the incoming direction, `n` the facing frame normal ([`Self::frame`]), `a` the
    /// index of the medium the ray arrives through and `b` the other ([`DielectricBoundary`],
    /// by [`Self::front_face`]), all as raw integers:
    ///
    /// 1. **Grazing.** When `m = -(d . n) <= 0` the ray arrives along or behind the plane of
    ///    the rounded normal: `Partial(1.0)`.
    /// 2. **Total internal reflection.** `K = b^2 (d.d)(n.n) - a^2 ((d.d)(n.n) - (d.n)^2)`,
    ///    exact in 256 bits; `Total` exactly when `K <= 0`, which is
    ///    `(a / b)^2 sin^2 >= 1` for the angle between `d` and the line of `n`. No rounded
    ///    value takes part in the decision.
    /// 3. **Reflectance.** With `q = floor(sqrt(K))`, the exact unpolarized Fresnel formula
    ///    `(r_s^2 + r_p^2) / 2`, `r_s = (a m - q) / (a m + q)` and
    ///    `r_p = (b^2 m - a q) / (b^2 m + a q)`. Each ratio is held to 62 fraction bits and the
    ///    result rounded once to nearest, ties to even: within one raw unit of the exact
    ///    formula (half a unit plus at most `2^-8`). Equal indices give exactly zero.
    ///
    /// # Errors
    ///
    /// [`SecondaryRayError::ArrivesBelowSurface`] when `d . N >= 0`, exact, with `N` the exact
    /// normal of the face the ray arrived on.
    pub fn dielectric_reflectance(
        &self,
        incoming: &TraceRay,
        boundary: &DielectricBoundary,
    ) -> Result<DielectricReflectance, SecondaryRayError> {
        self.interface(incoming, boundary).map(|(reflectance, _)| reflectance)
    }

    /// The continuation ray of `boundary` at this hit for the ray `incoming` that reached it
    /// and the choice value `choice` in `[0, 1)`. Returns `None` exactly when
    /// [`Self::try_dielectric_scatter`] returns an error; this is the fast path.
    ///
    /// The sample's throughput is exactly 1.0, a weight for power: the radiance factor
    /// `(a / b)^2` of a refraction is not applied (see [`DielectricScatter::Refracted`]).
    #[must_use]
    pub fn dielectric_scatter(
        &self,
        incoming: &TraceRay,
        boundary: &DielectricBoundary,
        choice: FixedQ32_32,
    ) -> Option<DielectricScatter> {
        self.try_dielectric_scatter(incoming, boundary, choice).ok()
    }

    /// The diagnostic path of [`Self::dielectric_scatter`].
    ///
    /// With `R` the raw [`DielectricReflectance::probability`] of
    /// [`Self::dielectric_reflectance`], the ray is reflected when `choice.raw() < R` and
    /// refracted otherwise. `R` of the `2^32` raw choice values reflect and the other
    /// `2^32 - R` refract, so over a uniform choice the reflected share is exactly the
    /// reflectance and the two shares sum to one. Each branch's throughput is exactly 1.0 per
    /// channel, a weight for power: the radiance factor of a refraction, `(a / b)^2` for a
    /// path traced from the camera out of index `a` into index `b`, is not applied (see
    /// [`DielectricScatter::Refracted`]; it cancels on a path that enters a medium and leaves
    /// it again).
    ///
    /// **Reflected.** [`Self::try_specular_reflection`] with a base reflectance of 1.0: the
    /// same ray and the same refusals.
    ///
    /// **Refracted.** The direction is the unit vector along
    /// `T = a (d (n.n) - (d.n) n) - q n` (Snell's law about the line of `n`; exact in 256 bits
    /// but for the floored root), normalized as [`super::TriangleSurface`] normalizes a face
    /// normal with a truncating prescale, each component within one raw unit of the exact
    /// refracted unit vector, and clamped to `-1.0..=1.0`. The ray starts at the first point
    /// strictly on the far side of the triangle's exact plane, stepping one raw unit at a time
    /// from [`Self::point`] along the plane normal's dominant axis: at most five steps for a
    /// hit the trace kernel reported. Query it with a minimum distance of zero.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`SecondaryRayError::ChoiceOutsideUnitInterval`] when `choice`
    /// is outside `[0, 1)`; [`SecondaryRayError::ArrivesBelowSurface`] as for
    /// [`Self::dielectric_reflectance`]; then, when reflected, the errors of
    /// [`Self::try_specular_reflection`]; when refracted,
    /// [`SecondaryRayError::RefractionAboveSurface`] when the direction `t` has `t . N >= 0`,
    /// exact (a near-grazing refraction about the rounded normal that does not go through the
    /// exact plane), [`SecondaryRayError::StartOutsideTraceDomain`] when the far-side start
    /// point is outside the trace domain, and the defensive
    /// [`SecondaryRayError::BounceRayRefused`], which no input is known to reach.
    pub fn try_dielectric_scatter(
        &self,
        incoming: &TraceRay,
        boundary: &DielectricBoundary,
        choice: FixedQ32_32,
    ) -> Result<DielectricScatter, SecondaryRayError> {
        if !(0..ONE_RAW).contains(&choice.raw()) {
            return Err(SecondaryRayError::ChoiceOutsideUnitInterval { choice });
        }
        let (reflectance, transmission) = self.interface(incoming, boundary)?;
        match transmission.filter(|_| choice >= reflectance.probability()) {
            Some((incidence, root)) => {
                self.refracted_sample(&incidence, root).map(DielectricScatter::Refracted)
            }
            None => self
                .try_specular_reflection(incoming, &Albedo::WHITE)
                .map(DielectricScatter::Reflected),
        }
    }

    /// The reflectance and, when Snell's law has a refracted ray, the incidence and the root
    /// that give it.
    fn interface(
        &self,
        incoming: &TraceRay,
        boundary: &DielectricBoundary,
    ) -> Result<(DielectricReflectance, Option<(Incidence, u128)>), SecondaryRayError> {
        let direction = incoming.direction();
        if dot(wide(direction), self.facing_normal()) >= 0 {
            return Err(SecondaryRayError::ArrivesBelowSurface { direction });
        }
        let indices = boundary.ordered(self.front_face());
        let normal = wide(self.frame().normal());
        let Some(incidence) = Incidence::new(wide(direction), normal, indices) else {
            return Ok((DielectricReflectance::Partial(FixedQ32_32::ONE), None));
        };
        Ok(incidence.root().map_or((DielectricReflectance::Total, None), |root| {
            let reflectance = DielectricReflectance::Partial(incidence.reflectance(root));
            (reflectance, Some((incidence, root)))
        }))
    }

    /// The refracted sample for an incidence and its root: the ray from the far-side start
    /// point along the refracted direction, with a throughput of exactly 1.0.
    fn refracted_sample(
        &self,
        incidence: &Incidence,
        root: u128,
    ) -> Result<SpecularSample, SecondaryRayError> {
        let direction = incidence.refracted(root);
        if dot(wide(direction), self.facing_normal()) >= 0 {
            return Err(SecondaryRayError::RefractionAboveSurface { direction });
        }
        let start = self.far_point();
        if !within(start, u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs())) {
            return Err(SecondaryRayError::StartOutsideTraceDomain { start: narrow(start) });
        }
        let origin = narrow(start);
        let ray = TraceRay::new(origin, direction)
            .ok_or(SecondaryRayError::BounceRayRefused { origin, direction })?;
        Ok(SpecularSample::new(ray, [FixedQ32_32::ONE; 3]))
    }
}
