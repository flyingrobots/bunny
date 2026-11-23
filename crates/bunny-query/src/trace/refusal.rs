//! The diagnostic paths of [`TraceRay::new`] and [`TraceTriangle::new`]: the same values, with
//! the reason for a refusal (flyingrobots/bunny#445).

use core::fmt;

use super::{in_trace_domain, narrow_sub3, raw3, reciprocal, TraceRay, TraceTriangle};
use crate::TRACE_DIRECTION_LIMIT_RAW;
use bunny_linalg::FixedVec3;

/// Why [`TraceRay::try_new`] refused a ray. Each variant carries the values the check saw.
///
/// [`TraceRay::new`] returns `None` exactly when `try_new` returns an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TraceRayError {
    /// An origin coordinate has a magnitude of at least [`crate::TRACE_COORD_LIMIT_RAW`]
    /// (`2^40` raw, 256 units): it is outside the trace domain.
    OriginOutsideTraceDomain {
        /// The origin, as given.
        origin: FixedVec3,
    },
    /// A direction component has a magnitude above [`crate::TRACE_DIRECTION_LIMIT_RAW`]
    /// (`2^32` raw, 1.0).
    DirectionComponentTooLarge {
        /// The direction, as given.
        direction: FixedVec3,
    },
    /// Every direction component is zero, so the ray has no line.
    ZeroDirection,
}

/// Why [`TraceTriangle::try_new`] refused a triangle.
///
/// [`TraceTriangle::new`] returns `None` exactly when `try_new` returns an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TraceTriangleError {
    /// A coordinate of a vertex has a magnitude of at least [`crate::TRACE_COORD_LIMIT_RAW`]
    /// (`2^40` raw, 256 units): it is outside the trace domain.
    VertexOutsideTraceDomain {
        /// Which vertex, `0`, `1` or `2` in argument order: the first one outside.
        index: usize,
        /// That vertex, as given.
        vertex: FixedVec3,
    },
}

impl TraceRay {
    /// The diagnostic path of [`Self::new`]: the same ray, with the reason for a refusal. Call
    /// it when `new` returns `None`; `new` is the fast path. `TRACE-TP-725` checks over
    /// generated inputs that `new` returns `Some(r)` exactly when this returns `Ok(r)`.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`TraceRayError::OriginOutsideTraceDomain`] when an origin
    /// coordinate's raw magnitude is `2^40` or more; [`TraceRayError::DirectionComponentTooLarge`]
    /// when a direction component's raw magnitude is above `2^32`;
    /// [`TraceRayError::ZeroDirection`] when the direction is `(0, 0, 0)`.
    pub fn try_new(origin: FixedVec3, direction: FixedVec3) -> Result<Self, TraceRayError> {
        let (origin_raw, direction_raw) = (raw3(origin), raw3(direction));
        let limit = TRACE_DIRECTION_LIMIT_RAW.unsigned_abs();
        if !origin_raw.iter().all(|&c| in_trace_domain(c)) {
            Err(TraceRayError::OriginOutsideTraceDomain { origin })
        } else if !direction_raw.iter().all(|&c| c.unsigned_abs() <= limit) {
            Err(TraceRayError::DirectionComponentTooLarge { direction })
        } else if direction_raw == [0; 3] {
            Err(TraceRayError::ZeroDirection)
        } else {
            let reciprocal = direction_raw.map(reciprocal);
            Ok(Self { origin: origin_raw, direction: direction_raw, reciprocal })
        }
    }
}

impl TraceTriangle {
    /// The diagnostic path of [`Self::new`]: the same triangle, with the reason for a refusal.
    /// `TRACE-TP-726` checks over generated inputs that `new` returns `Some(t)` exactly when
    /// this returns `Ok(t)`.
    ///
    /// # Errors
    ///
    /// [`TraceTriangleError::VertexOutsideTraceDomain`] naming the first vertex, in argument
    /// order, with a coordinate whose raw magnitude is `2^40` or more.
    pub fn try_new(
        v0: FixedVec3,
        v1: FixedVec3,
        v2: FixedVec3,
    ) -> Result<Self, TraceTriangleError> {
        let outside = |(_, v): &(usize, FixedVec3)| !raw3(*v).iter().all(|&c| in_trace_domain(c));
        if let Some((index, vertex)) = [v0, v1, v2].into_iter().enumerate().find(outside) {
            return Err(TraceTriangleError::VertexOutsideTraceDomain { index, vertex });
        }
        let (v0, v1, v2) = (raw3(v0), raw3(v1), raw3(v2));
        Ok(Self { v0, edge1: narrow_sub3(v1, v0), edge2: narrow_sub3(v2, v0) })
    }
}

impl fmt::Display for TraceRayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OriginOutsideTraceDomain { origin } => {
                write!(f, "ray origin raw {} is outside the trace domain", Raw(*origin))
            }
            Self::DirectionComponentTooLarge { direction } => write!(
                f,
                "ray direction raw {} has a component above 1.0 in magnitude",
                Raw(*direction)
            ),
            Self::ZeroDirection => f.write_str("ray direction is zero"),
        }
    }
}

impl core::error::Error for TraceRayError {}

impl fmt::Display for TraceTriangleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::VertexOutsideTraceDomain { index, vertex } = self;
        write!(f, "triangle vertex {index} at raw {} is outside the trace domain", Raw(*vertex))
    }
}

impl core::error::Error for TraceTriangleError {}

/// A vector shown as its raw Q32.32 components, `(x, y, z)`.
struct Raw(FixedVec3);

impl fmt::Display for Raw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(v) = self;
        write!(f, "({}, {}, {})", v.x.raw(), v.y.raw(), v.z.raw())
    }
}
