//! Why a surface hit, a bounce direction or a bounce ray could not be built.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Why [`super::SurfaceHit::try_new`], [`super::SurfaceHit::try_bounce_ray`] or
/// [`super::ShadingFrame::try_cosine_direction`] refused its input. Each variant carries the
/// values the check saw.
///
/// The `Option` forms ([`super::SurfaceHit::new`], [`super::SurfaceHit::bounce_ray`],
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
    /// `TraceRay::new` refused the bounce ray. This is a defensive refusal: the start point of
    /// a [`super::SurfaceHit`] is inside the trace domain and every bounce direction is nonzero
    /// with components in `-1.0..=1.0`, so no input is known to reach it.
    BounceRayRefused {
        /// The ray's origin, the hit's start point.
        origin: FixedVec3,
        /// The cosine-weighted bounce direction.
        direction: FixedVec3,
    },
}

impl fmt::Display for SecondaryRayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HitPointUnrepresentable { distance } => write!(
                f,
                "hit point at distance raw {} is not representable in Q32.32",
                distance.raw()
            ),
            Self::HitPointOutsideTraceDomain { point } => {
                write!(f, "hit point raw {} is outside the trace domain", Raw(*point))
            }
            Self::NotOnArrivalSide { point } => write!(
                f,
                "hit point raw {} is not within two raw-unit steps of the arrival side",
                Raw(*point)
            ),
            Self::StartOutsideTraceDomain { start } => {
                write!(f, "start point raw {} is outside the trace domain", Raw(*start))
            }
            Self::SampleOutsideUnitInterval { sample: [u, v] } => {
                write!(f, "sample raw ({}, {}) is outside [0, 1)", u.raw(), v.raw())
            }
            Self::BounceRayRefused { origin, direction } => write!(
                f,
                "bounce ray from raw {} along raw {} is not a valid trace ray",
                Raw(*origin),
                Raw(*direction)
            ),
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
