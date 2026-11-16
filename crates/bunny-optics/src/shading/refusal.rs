//! Why a surface hit, a bounce direction, a bounce ray, a mirror reflection or a dielectric
//! sample could not be built.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Why [`super::SurfaceHit::try_new`], [`super::SurfaceHit::try_bounce_ray`],
/// [`super::SurfaceHit::try_specular_reflection`],
/// [`super::SurfaceHit::try_dielectric_scatter`] or
/// [`super::ShadingFrame::try_cosine_direction`] refused its input. Each variant carries the
/// values the check saw.
///
/// The `Option` forms ([`super::SurfaceHit::new`], [`super::SurfaceHit::bounce_ray`],
/// [`super::SurfaceHit::specular_reflection`], [`super::SurfaceHit::dielectric_scatter`],
/// [`super::ShadingFrame::cosine_direction`]) return `None` exactly when these return an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecondaryRayError {
    /// `point_at(ray, hit.distance)` is not representable in Q32.32: a product or a sum of a
    /// coordinate overflows. A distance the trace kernel reported for a ray inside the trace
    /// domain never does this; a distance changed after the query can.
    HitPointUnrepresentable {
        /// The hit distance, in multiples of the ray direction.
        distance: FixedQ32_32,
    },
    /// A coordinate of the hit point `point_at(ray, hit.distance)` has a magnitude of at least
    /// `TRACE_COORD_LIMIT_RAW + 2` raw units: more than two raw units outside the trace domain,
    /// so no step can bring the start point inside. Like the next two, it means the hit is not
    /// one the trace kernel reported for this ray on this surface's triangle.
    HitPointOutsideTraceDomain {
        /// The hit point, before any step.
        point: FixedVec3,
    },
    /// No point among `point`, one and two raw units from it along the plane normal's dominant
    /// axis is strictly on the arrival side of the triangle's plane. The hit is not one the
    /// trace kernel reported for this ray on this surface's triangle, with its face: for such a
    /// hit two steps always suffice.
    NotOnArrivalSide {
        /// The hit point `point_at(ray, hit.distance)`, before any step.
        point: FixedVec3,
    },
    /// The start point, after its step, is outside the trace domain (a coordinate magnitude of
    /// at least `TRACE_COORD_LIMIT_RAW`, 256 units). A hit the trace kernel reported reaches
    /// this only on the domain's boundary.
    StartOutsideTraceDomain {
        /// The stepped start point.
        start: FixedVec3,
    },
    /// A coordinate of the unit-square sample is outside `[0, 1)`.
    SampleOutsideUnitInterval {
        /// The sample `[u, v]`, as given.
        sample: [FixedQ32_32; 2],
    },
    /// `TraceRay::new` refused the bounce ray, the reflected ray or the refracted ray. This is
    /// a defensive refusal: the start point is inside the trace domain and every bounce,
    /// reflected and refracted direction is nonzero with components in `-1.0..=1.0`, so no
    /// input is known to reach it.
    BounceRayRefused {
        /// The ray's origin, the hit's start point.
        origin: FixedVec3,
        /// The cosine-weighted bounce direction, the reflected or the refracted direction.
        direction: FixedVec3,
    },
    /// The incoming direction `d` is not strictly against the face the hit faces: `d . N >= 0`,
    /// exact, with `N` the exact normal of the face the ray arrived on. The ray arrives from
    /// below the surface (or along it), so it is not the ray the hit was built from.
    ArrivesBelowSurface {
        /// The incoming direction, as given.
        direction: FixedVec3,
    },
    /// The reflected direction is along or below the exact plane (`r . N <= 0`, exact): a
    /// grazing ray whose reflection about the rounded unit normal does not leave the surface.
    ReflectionBelowSurface {
        /// The normalized reflected direction.
        direction: FixedVec3,
    },
    /// The choice value of a dielectric sample is outside `[0, 1)`.
    ChoiceOutsideUnitInterval {
        /// The choice value, as given.
        choice: FixedQ32_32,
    },
    /// The refracted direction is along or above the exact plane (`t . N >= 0`, exact): a
    /// near-grazing refraction about the rounded unit normal that does not go through the
    /// surface.
    RefractionAboveSurface {
        /// The normalized refracted direction.
        direction: FixedVec3,
    },
}

impl fmt::Display for SecondaryRayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HitPointUnrepresentable { distance } => {
                let distance = distance.raw();
                write!(f, "hit point at distance raw {distance} is not representable in Q32.32")
            }
            Self::SampleOutsideUnitInterval { sample: [u, v] } => {
                write!(f, "sample raw ({}, {}) is outside [0, 1)", u.raw(), v.raw())
            }
            Self::ChoiceOutsideUnitInterval { choice } => {
                write!(f, "choice raw {} is outside [0, 1)", choice.raw())
            }
            Self::BounceRayRefused { origin, direction } => {
                let (origin, direction) = (Raw(*origin), Raw(*direction));
                write!(f, "bounce ray from raw {origin} along raw {direction} is not a trace ray")
            }
            Self::HitPointOutsideTraceDomain { point: v }
            | Self::NotOnArrivalSide { point: v }
            | Self::StartOutsideTraceDomain { start: v }
            | Self::ArrivesBelowSurface { direction: v }
            | Self::ReflectionBelowSurface { direction: v }
            | Self::RefractionAboveSurface { direction: v } => {
                let (subject, reason) = self.vector_message();
                write!(f, "{subject} raw {} {reason}", Raw(*v))
            }
        }
    }
}

impl SecondaryRayError {
    /// The subject and the reason of a refusal that carries one vector; empty for the others,
    /// whose messages [`fmt::Display`] writes itself.
    const fn vector_message(&self) -> (&'static str, &'static str) {
        match self {
            Self::HitPointOutsideTraceDomain { .. } => ("hit point", "is outside the trace domain"),
            Self::NotOnArrivalSide { .. } => {
                ("hit point", "is not within two steps of the arrival side")
            }
            Self::StartOutsideTraceDomain { .. } => ("start point", "is outside the trace domain"),
            Self::ArrivesBelowSurface { .. } => {
                ("incoming direction", "arrives from below the surface")
            }
            Self::ReflectionBelowSurface { .. } => {
                ("reflected direction", "does not leave the surface")
            }
            Self::RefractionAboveSurface { .. } => {
                ("refracted direction", "does not go through the surface")
            }
            _ => ("", ""),
        }
    }
}

impl std::error::Error for SecondaryRayError {}

/// A vector shown as its raw Q32.32 components, `(x, y, z)`.
struct Raw(FixedVec3);

impl fmt::Display for Raw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(v) = self;
        write!(f, "({}, {}, {})", v.x.raw(), v.y.raw(), v.z.raw())
    }
}
