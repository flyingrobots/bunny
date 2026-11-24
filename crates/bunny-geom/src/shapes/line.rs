//! The infinite line.

use bunny_linalg::{FixedUnitVec3, FixedVec3};

use super::ShapeError;

/// An infinite 3D line through `point` along the unit `direction`, in fixed-point coordinates.
///
/// The line is every `point + t * direction` for every real `t`: a ray without the `t >= 0`
/// restriction. The direction is a [`FixedUnitVec3`], the same type and the same
/// normalization rule as [`crate::FixedRay3`], so its length is within the unit band that type
/// guarantees, not exactly one.
///
/// Equality is representation equality: two lines with the same geometry (a different
/// `point` on it, or the opposite direction) are not `==`. Nothing is canonicalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedLine3 {
    /// A point on the line.
    pub point: FixedVec3,
    /// The unit direction.
    pub direction: FixedUnitVec3,
}

impl FixedLine3 {
    /// Creates a line from a point and an already normalized direction. Every input is a line.
    #[must_use]
    pub const fn new(point: FixedVec3, direction: FixedUnitVec3) -> Self {
        Self { point, direction }
    }

    /// Creates a line by normalizing `direction` with `FixedUnitVec3::new`, exactly as
    /// [`crate::FixedRay3::try_new`] does.
    ///
    /// # Errors
    /// Returns [`ShapeError::InvalidLineDirection`] exactly when `FixedUnitVec3::new(direction)`
    /// is `None`: the zero vector, a vector whose raw length exceeds `i64::MAX`, or one whose
    /// normalized length falls outside the unit band (for example `(1, 1, 0)` raw, whose floor
    /// square-root length is 1 raw unit). `(1, 0, 0)` raw is accepted.
    pub fn try_new(point: FixedVec3, direction: FixedVec3) -> Result<Self, ShapeError> {
        FixedUnitVec3::new(direction)
            .map(|direction| Self::new(point, direction))
            .ok_or(ShapeError::InvalidLineDirection)
    }
}
