//! Rays and axis-aligned boxes mapped by a [`FixedAffine3`]. See
//! `docs/topics/transform/README.md#rays` and `#boxes`.
//!
//! Both helpers are built on [`FixedAffine3::checked_transform_point`] and
//! [`FixedAffine3::checked_transform_vector`]: each output component is three products, each
//! rounded once to nearest, ties to even, from the exact `i128` product, summed exactly (plus the
//! translation for a point), so it is within 3/2 raw of the exact value, and it is refused when a
//! rounded product, a partial sum or the translation sum leaves the Q32.32 range.

use core::fmt;

use bunny_linalg::{FixedAffine3, FixedVec3};
use bunny_num::FixedQ32_32;

use crate::{FixedAabb3, FixedRay3, GeomError};

const ZERO: FixedVec3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);

/// Refusal of a ray or box transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformError {
    /// The box has a minimum above its maximum on some axis, the rule of
    /// [`FixedAabb3::try_new`].
    InvertedBox,
    /// A rounded product, a partial sum or the translation sum of a point or vector transform is
    /// outside the Q32.32 range.
    OutOfRange,
    /// The transformed ray direction rounds to the zero vector.
    DegenerateDirection,
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvertedBox => "box minimum exceeds its maximum",
            Self::OutOfRange => "transformed value is outside the Q32.32 range",
            Self::DegenerateDirection => "transformed ray direction is zero",
        })
    }
}

impl std::error::Error for TransformError {}

/// A ray mapped by [`FixedRay3::checked_transform`]: its direction is **not renormalised**.
///
/// The point at parameter `t` is `origin + direction * t`, the image of the original ray's
/// point at the same `t`. The direction is nonzero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransformedRay3 {
    /// The transformed origin.
    pub origin: FixedVec3,
    /// The transformed direction, of whatever length the transform gives it.
    pub direction: FixedVec3,
}

impl TransformedRay3 {
    /// Renormalises the direction into a [`FixedRay3`] with [`FixedRay3::try_new`].
    ///
    /// The parameter no longer carries over: the point at `t` on the result is at distance `t`
    /// along the unit direction, not the image of the original point at `t`.
    ///
    /// # Errors
    /// [`GeomError::InvalidRayDirection`] when `FixedRay3::try_new` cannot normalise the
    /// direction.
    pub fn try_into_ray(self) -> Result<FixedRay3, GeomError> {
        FixedRay3::try_new(self.origin, self.direction)
    }
}

impl FixedRay3 {
    /// Maps the ray by `transform`: the origin as a point, the stored unit direction as a vector.
    ///
    /// The direction is not renormalised, so the parameter `t` of a hit keeps its meaning: for
    /// every real `t`, each component of `origin' + t direction'` is within `3/2 (1 + abs(t))`
    /// raw of the exact image of `origin + t direction` (3/2 raw from the origin, `3/2 abs(t)`
    /// from the direction). [`TransformedRay3::try_into_ray`] renormalises.
    ///
    /// # Errors
    /// Checked in this order: [`TransformError::OutOfRange`] when the origin's point transform or
    /// the direction's vector transform is `None`; [`TransformError::DegenerateDirection`] when
    /// the rounded direction is the zero vector (a singular transform, or products that round to
    /// zero).
    pub fn checked_transform(
        &self,
        transform: &FixedAffine3,
    ) -> Result<TransformedRay3, TransformError> {
        let origin = transform.checked_transform_point(self.origin);
        let direction = transform.checked_transform_vector(self.direction.into_inner());
        let (Some(origin), Some(direction)) = (origin, direction) else {
            return Err(TransformError::OutOfRange);
        };
        if direction == ZERO {
            return Err(TransformError::DegenerateDirection);
        }
        Ok(TransformedRay3 { origin, direction })
    }
}

impl FixedAabb3 {
    /// The axis-aligned box that contains the box mapped by `transform`.
    ///
    /// The result is the per-axis minimum and maximum of the eight corners mapped by
    /// [`FixedAffine3::checked_transform_point`], so it contains every rounded corner exactly.
    /// Each face is within 3/2 raw of the face of the exact image's axis-aligned hull; it can lie
    /// up to 3/2 raw inside it, so a box that must contain the exact image needs 2 raw more on
    /// each face. Under a rotation the result is larger than the image (a hull, not the image).
    /// A point box gives the point transform, as a point box.
    ///
    /// # Errors
    /// [`TransformError::InvertedBox`] when a minimum is above its maximum, before any
    /// arithmetic; [`TransformError::OutOfRange`] when a corner's point transform is `None`.
    pub fn checked_transform(&self, transform: &FixedAffine3) -> Result<Self, TransformError> {
        if !(self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z) {
            return Err(TransformError::InvertedBox);
        }
        let map = |index| {
            transform.checked_transform_point(self.corner(index)).ok_or(TransformError::OutOfRange)
        };
        let first = map(0)?;
        let mut hull = Self::new(first, first);
        for index in 1..8 {
            let point = map(index)?;
            hull = Self::new(
                componentwise(hull.min, point, Ord::min),
                componentwise(hull.max, point, Ord::max),
            );
        }
        Ok(hull)
    }

    /// Corner `index`: bit 0 picks `max.x`, bit 1 `max.y`, bit 2 `max.z`.
    const fn corner(&self, index: u8) -> FixedVec3 {
        let (lo, hi) = (self.min, self.max);
        FixedVec3::new(
            if index & 1 == 0 { lo.x } else { hi.x },
            if index & 2 == 0 { lo.y } else { hi.y },
            if index & 4 == 0 { lo.z } else { hi.z },
        )
    }
}

fn componentwise(
    a: FixedVec3,
    b: FixedVec3,
    pick: fn(FixedQ32_32, FixedQ32_32) -> FixedQ32_32,
) -> FixedVec3 {
    FixedVec3::new(pick(a.x, b.x), pick(a.y, b.y), pick(a.z, b.z))
}
