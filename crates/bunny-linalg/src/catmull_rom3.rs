//! Uniform Catmull-Rom splines through points, as cubic Bezier segments (`bunny-curve/1`).
//!
//! Each segment is converted to a [`CubicBezier3`], whose two inner control points are the
//! only roundings, and evaluated through it; the rules and their bounds are proved in
//! `docs/topics/curve/README.md`.

use core::fmt;

use bunny_num::FixedQ32_32;

use crate::{CubicBezier3, FixedVec3};

/// Why [`CatmullRom3::new`] refused a point list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatmullRomError {
    /// Fewer than 2 points for an open spline, or fewer than 3 for a closed one.
    TooFewPoints,
    /// Every point is the same point.
    AllPointsEqual,
    /// An inner Bezier control point of a segment is outside the Q32.32 range.
    ControlPointOutOfRange {
        /// The first segment, in order, with such a point.
        segment: usize,
    },
}

impl fmt::Display for CatmullRomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints => formatter.write_str("too few points for a Catmull-Rom spline"),
            Self::AllPointsEqual => formatter.write_str("every Catmull-Rom point is the same"),
            Self::ControlPointOutOfRange { segment } => write!(
                formatter,
                "a control point of Catmull-Rom segment {segment} is outside the Q32.32 range"
            ),
        }
    }
}

impl core::error::Error for CatmullRomError {}

/// A uniform Catmull-Rom spline through borrowed points, open or closed (`bunny-curve/1`).
///
/// Segment `i` runs from point `i` to point `i + 1` (to point 0 after the last, when closed)
/// and is the [`CubicBezier3`] of [`Self::segment`]. An open spline has `n - 1` segments and
/// takes its missing neighbours by duplicating its end points; a closed one has `n` and wraps.
/// The spline is the parameter `u` in `[0, segment_count]`: segment `floor(u)` at its fraction,
/// with `u = segment_count` the end of the last segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatmullRom3<'a> {
    points: &'a [FixedVec3],
    closed: bool,
}

/// `(6 at + toward - from) / 6` rounded once to nearest, ties to even; `None` outside `i64`.
fn inner_axis(at: FixedQ32_32, toward: FixedQ32_32, from: FixedQ32_32) -> Option<FixedQ32_32> {
    // |6 at + toward - from| <= 2^66, exact in i128.
    let exact = 6 * i128::from(at.raw()) + i128::from(toward.raw()) - i128::from(from.raw());
    let (floor, rest) = (exact.div_euclid(6), exact.rem_euclid(6));
    let up = rest > 3 || (rest == 3 && floor.rem_euclid(2) == 1);
    i64::try_from(floor + i128::from(up)).ok().map(FixedQ32_32::from_raw)
}

/// `at + (toward - from) / 6`, each component rounded once by [`inner_axis`].
fn inner(at: FixedVec3, toward: FixedVec3, from: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        inner_axis(at.x, toward.x, from.x)?,
        inner_axis(at.y, toward.y, from.y)?,
        inner_axis(at.z, toward.z, from.z)?,
    ))
}

impl<'a> CatmullRom3<'a> {
    /// The spline through `points`, closed back to the first point when `closed` is set.
    ///
    /// # Errors
    ///
    /// [`CatmullRomError::TooFewPoints`] for fewer than 2 points (3 when closed);
    /// [`CatmullRomError::AllPointsEqual`] when every point is the same;
    /// [`CatmullRomError::ControlPointOutOfRange`] when an inner control point of
    /// [`Self::segment`] is outside the Q32.32 range, checked in that order.
    pub fn new(points: &'a [FixedVec3], closed: bool) -> Result<Self, CatmullRomError> {
        if points.len() < if closed { 3 } else { 2 } {
            return Err(CatmullRomError::TooFewPoints);
        }
        if points.iter().all(|point| Some(point) == points.first()) {
            return Err(CatmullRomError::AllPointsEqual);
        }
        let spline = Self { points, closed };
        match (0..spline.segment_count()).find(|&i| spline.segment(i).is_none()) {
            Some(segment) => Err(CatmullRomError::ControlPointOutOfRange { segment }),
            None => Ok(spline),
        }
    }

    /// The points the spline passes through.
    #[must_use]
    pub const fn points(self) -> &'a [FixedVec3] {
        self.points
    }

    /// Whether the spline closes back to its first point.
    #[must_use]
    pub const fn is_closed(self) -> bool {
        self.closed
    }

    /// The number of segments: `n` when closed, `n - 1` when open.
    #[must_use]
    pub const fn segment_count(self) -> usize {
        if self.closed {
            self.points.len()
        } else {
            self.points.len().saturating_sub(1)
        }
    }

    /// The indices of `P0` to `P3` for segment `i < segment_count`: wrapped when closed,
    /// clamped to the end points (the duplicated phantom neighbours) when open.
    const fn neighbours(self, i: usize) -> [usize; 4] {
        let n = self.points.len();
        if self.closed {
            // n is a slice length of 24-byte points, so i + n + 1 cannot overflow.
            [(i + n - 1) % n, i, (i + 1) % n, (i + 2) % n]
        } else {
            let last = n - 1;
            [i.saturating_sub(1), i, i + 1, if i + 2 < last { i + 2 } else { last }]
        }
    }

    /// Segment `index` as a cubic Bezier: `P1`, `P1 + (P2 - P0) / 6`, `P2 - (P3 - P1) / 6`,
    /// `P2` (`bunny-curve/1`).
    ///
    /// Each inner component is the exact rational `(6 P1 + P2 - P0) / 6` (or
    /// `(6 P2 + P1 - P3) / 6`) rounded once to nearest, ties to even, so it is within 1/2 raw
    /// unit of exact; the end points are the spline's points bit for bit.
    ///
    /// Returns `None` when `index >= segment_count()`.
    #[must_use]
    pub fn segment(self, index: usize) -> Option<CubicBezier3> {
        if index >= self.segment_count() {
            return None;
        }
        let [p0, p1, p2, p3] = self.neighbours(index).map(|i| self.points.get(i).copied());
        let (p0, p1, p2, p3) = (p0?, p1?, p2?, p3?);
        Some(CubicBezier3::new(p1, inner(p1, p2, p0)?, inner(p2, p1, p3)?, p2))
    }

    /// Every segment in order, `segment_count()` of them (`bunny-curve/1`).
    pub fn to_beziers(self) -> impl Iterator<Item = CubicBezier3> + 'a {
        (0..self.segment_count()).filter_map(move |i| self.segment(i))
    }

    /// The segment and local `t` of `u`: the integer part and the fraction, with
    /// `u = segment_count` the last segment at `t = 1.0`; `None` outside `[0, segment_count]`.
    fn locate(self, u: FixedQ32_32) -> Option<(usize, FixedQ32_32)> {
        let raw = u64::try_from(u.raw()).ok()?;
        let (whole, fraction) = (raw >> 32, (raw & 0xFFFF_FFFF).cast_signed());
        let count = u64::try_from(self.segment_count()).ok()?;
        if whole < count {
            Some((usize::try_from(whole).ok()?, FixedQ32_32::from_raw(fraction)))
        } else if whole == count && fraction == 0 {
            Some((self.segment_count().saturating_sub(1), FixedQ32_32::ONE))
        } else {
            None
        }
    }

    /// The point at `u`: [`CubicBezier3::at`] of its segment at its local `t`.
    ///
    /// Within 15/8 raw units per component of the exact Catmull-Rom cubic (3/8 from the
    /// rounded control points, 3/2 from the evaluation), and exactly point `k` at `u = k`.
    ///
    /// Returns `None` only when `u` is outside `[0, segment_count]`.
    #[must_use]
    pub fn at(self, u: FixedQ32_32) -> Option<FixedVec3> {
        let (index, t) = self.locate(u)?;
        self.segment(index)?.at(t)
    }

    /// The derivative with respect to `u` at `u`: [`CubicBezier3::derivative_at`] of its
    /// segment at its local `t`.
    ///
    /// Within 9/2 raw units per component of the exact derivative (3/2 from the rounded
    /// control points, 3 from the evaluation); at a point it is the derivative leaving it,
    /// within 3/2 raw units of `(P[k + 1] - P[k - 1]) / 2` and bit for bit the derivative
    /// arriving at it from the previous segment.
    ///
    /// Returns `None` when `u` is outside `[0, segment_count]`, or when
    /// [`CubicBezier3::derivative_at`] of the segment overflows.
    #[must_use]
    pub fn derivative_at(self, u: FixedQ32_32) -> Option<FixedVec3> {
        let (index, t) = self.locate(u)?;
        self.segment(index)?.derivative_at(t)
    }
}
