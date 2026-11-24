//! 2D primitives and their exact queries (`bunny-planar/1`).
//!
//! Points are `FixedVec2` values on the raw Q32.32 grid, so every containment, overlap and
//! intersection answer here is decided exactly, with no rounding, for every representable
//! input: comparisons are on raw `i64` values, squared distances are held in 256 bits, and
//! segment tests read the exact sign of [`crate::orientation2`]. The one rounded result is
//! [`FixedSegment2::closest_point`], each coordinate rounded once. Every boundary is closed:
//! a point on an edge, a circle or a segment end is contained, and shapes that touch overlap.
//! See `docs/topics/planar-geometry/README.md`.

use bunny_linalg::{FixedUnitVec2, FixedVec2};
use bunny_num::FixedQ32_32;

use crate::wide::Wide;
use crate::GeomError;

mod polygon;
mod ray;
mod segment;

pub use polygon::{FixedPolygon2, PolygonLocation};
pub use ray::RayHit2;

/// The identity string of the 2D primitive and query contract.
pub const PLANAR_CONTRACT_ID: &str = "bunny-planar/1";

/// A 2D line segment with fixed-point endpoints over the full Q32.32 range: every point
/// between `start` and `end`, both included.
///
/// The endpoint order is kept and matters to `==`, but no query depends on it. A zero-length
/// segment (`start == end`) is a valid value and is the single point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedSegment2 {
    /// The first endpoint.
    pub start: FixedVec2,
    /// The second endpoint.
    pub end: FixedVec2,
}

/// A 2D ray with a fixed-point origin and a unit direction, as [`crate::FixedRay3`] in 3D.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedRay2 {
    /// The ray origin.
    pub origin: FixedVec2,
    /// The ray direction.
    pub direction: FixedUnitVec2,
}

impl FixedRay2 {
    /// Creates a ray with a pre-normalized direction.
    #[must_use]
    pub const fn new(origin: FixedVec2, direction: FixedUnitVec2) -> Self {
        Self { origin, direction }
    }

    /// Creates a ray by normalizing `direction` with `FixedUnitVec2::new`.
    ///
    /// # Errors
    /// Returns [`GeomError::InvalidRayDirection`] when `FixedUnitVec2::new` refuses the
    /// direction: it is zero, its length overflows, or its normalized length is not unit.
    pub fn try_new(origin: FixedVec2, direction: FixedVec2) -> Result<Self, GeomError> {
        FixedUnitVec2::new(direction)
            .map(|direction| Self::new(origin, direction))
            .ok_or(GeomError::InvalidRayDirection)
    }
}

/// A 2D axis-aligned box with fixed-point corners: every point with `min.x <= x <= max.x`
/// and `min.y <= y <= max.y`, boundary included.
///
/// A flat box (`min == max` on one axis) is a segment and a point box a point; both are
/// valid. A box whose minimum is above its maximum on some axis ([`Self::is_ordered`] false)
/// can be made with [`Self::new`], which does not validate, and is the empty set: it contains
/// no point, overlaps no box and has no closest point. [`Self::try_new`] refuses one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedAabb2 {
    /// The minimum corner.
    pub min: FixedVec2,
    /// The maximum corner.
    pub max: FixedVec2,
}

impl FixedAabb2 {
    /// Creates a box without validation, as [`crate::FixedAabb3::new`] does.
    #[must_use]
    pub const fn new(min: FixedVec2, max: FixedVec2) -> Self {
        Self { min, max }
    }

    /// Creates a box whose minimum is at most its maximum on both axes.
    ///
    /// # Errors
    /// Returns [`GeomError::InvalidAabbBounds`] when `min.x > max.x` or `min.y > max.y` (raw
    /// comparison); equal bounds are accepted.
    pub const fn try_new(min: FixedVec2, max: FixedVec2) -> Result<Self, GeomError> {
        let aabb = Self::new(min, max);
        if aabb.is_ordered() {
            Ok(aabb)
        } else {
            Err(GeomError::InvalidAabbBounds)
        }
    }

    /// Whether `min <= max` on both axes, the condition [`Self::try_new`] checks. A box that is
    /// not ordered is the empty set to every query.
    #[must_use]
    pub const fn is_ordered(&self) -> bool {
        self.min.x.raw() <= self.max.x.raw() && self.min.y.raw() <= self.max.y.raw()
    }

    /// Whether `point` is in the closed box. Exact raw comparisons; a point on an edge or a
    /// corner is contained, and an unordered box contains nothing.
    #[must_use]
    pub const fn contains_point(&self, point: FixedVec2) -> bool {
        between(point.x.raw(), self.min.x.raw(), self.max.x.raw())
            && between(point.y.raw(), self.min.y.raw(), self.max.y.raw())
    }

    /// Whether the closed boxes share a point. Exact raw comparisons; boxes touching along an
    /// edge or at a corner overlap. `false` when either box is not ordered. Symmetric.
    #[must_use]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.is_ordered()
            && other.is_ordered()
            && self.min.x.raw() <= other.max.x.raw()
            && other.min.x.raw() <= self.max.x.raw()
            && self.min.y.raw() <= other.max.y.raw()
            && other.min.y.raw() <= self.max.y.raw()
    }

    /// The point of the closed box nearest `point`: each coordinate clamped to
    /// `[min, max]`. Exact, with no rounding; `point` itself when it is in the box.
    ///
    /// `None` exactly when the box is not ordered: the empty set has no nearest point.
    #[must_use]
    pub const fn closest_point(&self, point: FixedVec2) -> Option<FixedVec2> {
        if !self.is_ordered() {
            return None;
        }
        let x = clamp(point.x.raw(), self.min.x.raw(), self.max.x.raw());
        let y = clamp(point.y.raw(), self.min.y.raw(), self.max.y.raw());
        Some(FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y)))
    }
}

/// A 2D circle with a fixed-point centre and radius: the closed disc of every point within
/// `radius` of `center`, boundary included.
///
/// A zero radius is the single point `center`. A negative radius can be made with
/// [`Self::new`], which does not validate, and is the empty set: it contains no point and
/// overlaps nothing. [`Self::try_new`] refuses one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedCircle2 {
    /// The centre.
    pub center: FixedVec2,
    /// The radius.
    pub radius: FixedQ32_32,
}

impl FixedCircle2 {
    /// Creates a circle without validation, as [`crate::FixedSphere3::new`] does.
    #[must_use]
    pub const fn new(center: FixedVec2, radius: FixedQ32_32) -> Self {
        Self { center, radius }
    }

    /// Creates a circle with a non-negative radius.
    ///
    /// # Errors
    /// Returns [`GeomError::NegativeSphereRadius`], the error of
    /// [`crate::FixedSphere3::try_new`], when `radius` is below zero (raw `< 0`); a zero
    /// radius is accepted.
    pub const fn try_new(center: FixedVec2, radius: FixedQ32_32) -> Result<Self, GeomError> {
        if radius.raw() >= 0 {
            Ok(Self::new(center, radius))
        } else {
            Err(GeomError::NegativeSphereRadius)
        }
    }

    /// Whether `point` is in the closed disc: `|point - center|^2 <= radius^2` on raw values.
    ///
    /// Exact for every representable input: each difference has magnitude at most
    /// `2^64 - 1`, the squared distance is below `2^129` and the squared radius below
    /// `2^126`, all held in 256 bits. A point on the circle is contained; a negative radius
    /// contains nothing.
    #[must_use]
    pub fn contains_point(&self, point: FixedVec2) -> bool {
        let r = self.radius.raw();
        r >= 0 && within(distance_squared(self.center, point), Wide::difference(r, 0))
    }

    /// Whether the closed discs share a point: `|c1 - c2|^2 <= (r1 + r2)^2` on raw values.
    ///
    /// Exact: the radius sum is at most `2^64 - 2` and its square below `2^128`. Tangent
    /// circles overlap. `false` when either radius is negative. Symmetric.
    #[must_use]
    pub fn overlaps_circle(&self, other: &Self) -> bool {
        let (a, b) = (self.radius.raw(), other.radius.raw());
        // `b >= 0`, so `-b` does not overflow and `a - (-b)` is the exact sum.
        a >= 0
            && b >= 0
            && within(distance_squared(self.center, other.center), Wide::difference(a, -b))
    }

    /// Whether the closed disc and the closed box share a point: whether the box's
    /// [`FixedAabb2::closest_point`] to the centre is in the disc.
    ///
    /// Exact: the closest point is an exact clamp and the containment test exact. A disc
    /// touching an edge or a corner overlaps. `false` when the radius is negative or the box
    /// is not ordered.
    #[must_use]
    pub fn overlaps_aabb(&self, aabb: &FixedAabb2) -> bool {
        aabb.closest_point(self.center).is_some_and(|nearest| self.contains_point(nearest))
    }
}

/// `lo <= value <= hi`.
const fn between(value: i64, lo: i64, hi: i64) -> bool {
    lo <= value && value <= hi
}

/// `value` clamped to `[lo, hi]`, for `lo <= hi`.
const fn clamp(value: i64, lo: i64, hi: i64) -> i64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

/// The exact `|b - a|^2` on raw values, below `2 (2^64 - 1)^2 < 2^129`.
fn distance_squared(a: FixedVec2, b: FixedVec2) -> Wide {
    let dx = Wide::difference(b.x.raw(), a.x.raw());
    let dy = Wide::difference(b.y.raw(), a.y.raw());
    dx.times(dx).plus(dy.times(dy))
}

/// Whether `squared <= reach^2` exactly.
fn within(squared: Wide, reach: Wide) -> bool {
    let excess = squared.minus(reach.times(reach));
    excess.is_negative() || excess.is_zero()
}
