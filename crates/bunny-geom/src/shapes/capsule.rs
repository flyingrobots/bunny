//! The capsule.

use bunny_num::FixedQ32_32;

use super::ShapeError;
use crate::FixedSegment3;

/// A 3D capsule: every point within `radius` of the closed segment `segment`.
///
/// The segment's endpoint order is preserved and has no geometric meaning beyond the
/// representation; it matters to `==`. Degenerate cases, all accepted by both constructors:
///
/// - a zero radius is the segment itself;
/// - a zero-length segment (`start == end`) is the sphere of `radius` about that point;
/// - both together are a single point.
///
/// A negative radius is refused by [`Self::try_new`]. Like [`crate::FixedSphere3::new`],
/// [`Self::new`] does not validate, so a capsule built with `new` may carry one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedCapsule3 {
    /// The core segment.
    pub segment: FixedSegment3,
    /// The radius.
    pub radius: FixedQ32_32,
}

impl FixedCapsule3 {
    /// Creates a capsule without validation, as [`crate::FixedSphere3::new`] does.
    #[must_use]
    pub const fn new(segment: FixedSegment3, radius: FixedQ32_32) -> Self {
        Self { segment, radius }
    }

    /// Creates a capsule with a non-negative radius.
    ///
    /// # Errors
    /// Returns [`ShapeError::NegativeCapsuleRadius`] when `radius` is below zero (raw `< 0`);
    /// a zero radius is accepted.
    pub const fn try_new(segment: FixedSegment3, radius: FixedQ32_32) -> Result<Self, ShapeError> {
        if radius.raw() >= 0 {
            Ok(Self::new(segment, radius))
        } else {
            Err(ShapeError::NegativeCapsuleRadius)
        }
    }
}
