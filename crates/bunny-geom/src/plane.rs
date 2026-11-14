//! Fixed-point planes and segment clipping.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Which side of a plane a point lies on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneSide {
    /// Signed distance is positive.
    Front,
    /// Signed distance is negative.
    Behind,
    /// Signed distance is exactly zero.
    On,
}

/// A 3D line segment with deterministic fixed-point endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedSegment3 {
    /// The first endpoint.
    pub start: FixedVec3,
    /// The second endpoint.
    pub end: FixedVec3,
}

impl FixedSegment3 {
    /// Creates a segment from two endpoints.
    #[must_use]
    pub const fn new(start: FixedVec3, end: FixedVec3) -> Self {
        Self { start, end }
    }
}

/// The result of clipping a segment against a plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentClip {
    /// The part on the front of the plane or on it, in original endpoint order.
    Kept(FixedSegment3),
    /// The whole segment is behind the plane.
    Outside,
}

/// A plane `normal . p = offset` with deterministic fixed-point coefficients.
///
/// The signed distance of a point is `normal . p - offset`. The normal need not
/// be unit length; the distance is then scaled by its length. Positive distance
/// is the front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedPlane3 {
    /// The plane normal; points along it are in front.
    pub normal: FixedVec3,
    /// The value of `normal . p` on the plane.
    pub offset: FixedQ32_32,
}

impl FixedPlane3 {
    /// Creates a plane from a normal and an offset. A zero normal is accepted
    /// and gives every point the same signed distance, `-offset`.
    #[must_use]
    pub const fn new(normal: FixedVec3, offset: FixedQ32_32) -> Self {
        Self { normal, offset }
    }

    /// Computes `normal . point - offset` with checked Q32.32 arithmetic.
    ///
    /// Each product rounds ties-to-even. Returns `None` on overflow.
    #[must_use]
    pub fn signed_distance(self, point: FixedVec3) -> Option<FixedQ32_32> {
        let x = self.normal.x.checked_mul(point.x)?;
        let y = self.normal.y.checked_mul(point.y)?;
        let z = self.normal.z.checked_mul(point.z)?;
        x.checked_add(y)?.checked_add(z)?.checked_sub(self.offset)
    }

    /// Classifies a point as in front of, behind, or on the plane.
    ///
    /// Returns `None` when the signed distance overflows.
    #[must_use]
    pub fn classify(self, point: FixedVec3) -> Option<PlaneSide> {
        self.signed_distance(point).map(side_of)
    }

    /// Clips a segment to the closed front half-space.
    ///
    /// A point with signed distance zero is kept. Returns `None` on overflow.
    /// See `docs/topics/plane-clipping/README.md` for the rounding contract.
    #[must_use]
    pub fn clip_segment(self, segment: FixedSegment3) -> Option<SegmentClip> {
        let start = self.signed_distance(segment.start)?;
        let end = self.signed_distance(segment.end)?;
        match (keeps(start), keeps(end)) {
            (true, true) => Some(SegmentClip::Kept(segment)),
            (false, false) => Some(SegmentClip::Outside),
            (true, false) => {
                let cut = crossing(segment, start, end)?;
                Some(SegmentClip::Kept(FixedSegment3::new(segment.start, cut)))
            }
            (false, true) => {
                let cut = crossing(segment, start, end)?;
                Some(SegmentClip::Kept(FixedSegment3::new(cut, segment.end)))
            }
        }
    }
}

const fn side_of(distance: FixedQ32_32) -> PlaneSide {
    match distance.raw() {
        0 => PlaneSide::On,
        d if d > 0 => PlaneSide::Front,
        _ => PlaneSide::Behind,
    }
}

/// The on-plane rule: zero distance is kept, so `On` and `Front` both keep.
const fn keeps(distance: FixedQ32_32) -> bool {
    distance.raw() >= 0
}

/// Intersection of a segment whose endpoint distances have opposite keep sides.
///
/// `t = start / (start - end)` is the one checked division (ties-to-even); the
/// point is `a + (b - a) * t`, each product rounded ties-to-even.
fn crossing(segment: FixedSegment3, start: FixedQ32_32, end: FixedQ32_32) -> Option<FixedVec3> {
    let t = start.checked_div(start.checked_sub(end)?)?;
    let (a, b) = (segment.start, segment.end);
    Some(FixedVec3::new(lerp(a.x, b.x, t)?, lerp(a.y, b.y, t)?, lerp(a.z, b.z, t)?))
}

fn lerp(a: FixedQ32_32, b: FixedQ32_32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    a.checked_add(b.checked_sub(a)?.checked_mul(t)?)
}
