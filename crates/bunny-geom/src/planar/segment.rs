//! Segment containment, intersection and closest point.

use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;

use super::FixedSegment2;
use crate::wide::Wide;
use crate::{orientation2, Orientation2};

impl FixedSegment2 {
    /// Creates a segment from two endpoints, in order. A zero-length segment is accepted.
    #[must_use]
    pub const fn new(start: FixedVec2, end: FixedVec2) -> Self {
        Self { start, end }
    }

    /// Whether `point` is on the closed segment: [`orientation2`]`(start, end, point)` is
    /// exactly [`Orientation2::Collinear`] and `point` is in the box of the two endpoints.
    ///
    /// Exact for every representable input; both ends are included. A point on the segment's
    /// line past an end is not on it. A zero-length segment contains only its point. The
    /// answer does not depend on endpoint order.
    #[must_use]
    pub fn contains_point(&self, point: FixedVec2) -> bool {
        orientation2(self.start, self.end, point) == Orientation2::Collinear
            && in_endpoint_box(self.start, self.end, point)
    }

    /// Whether the closed segments share at least one point.
    ///
    /// Decided from the exact signs of four [`orientation2`] tests: the segments cross when
    /// each one's endpoints are strictly on opposite sides of the other's line, and otherwise
    /// they meet exactly when an endpoint of one is on the other ([`Self::contains_point`]).
    /// Exact for every representable input, with the boundary included: an endpoint touching
    /// the other segment, a shared endpoint and a collinear overlap of a single point all
    /// count. Parallel segments on different lines never meet. Zero-length segments are
    /// points. Symmetric, and independent of endpoint order.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        let (a, b, c, d) = (self.start, self.end, other.start, other.end);
        let (abc, abd) = (orientation2(a, b, c), orientation2(a, b, d));
        let (cda, cdb) = (orientation2(c, d, a), orientation2(c, d, b));
        if opposite(abc, abd) && opposite(cda, cdb) {
            return true;
        }
        touches(abc, a, b, c)
            || touches(abd, a, b, d)
            || touches(cda, c, d, a)
            || touches(cdb, c, d, b)
    }

    /// The point of the closed segment nearest `point`, each coordinate rounded once.
    ///
    /// With `a = start`, `b = end`, the exact parameter of the nearest point is `n / d`, with
    /// `n = (point - a) . (b - a)` and `d = |b - a|^2` on raw values. When `d = 0` the result
    /// is `a`; when `n <= 0` it is `a` and when `n >= d` it is `b`, exactly. Otherwise each
    /// coordinate is the exact value `(a d + (b - a) n) / d` rounded once, to nearest with
    /// ties to even (on the coordinate, not on its offset from `a`), so it is within half a
    /// raw unit of the exact nearest point on each axis. The rounded point need not be on the
    /// segment's line.
    ///
    /// Widths: `|d|` and `|n|` are below `2^129` and each numerator below `2^194`, held in
    /// 256 bits. The exact coordinate lies between the endpoints' coordinates and rounding to
    /// nearest cannot pass an integer bound, so the result is in the endpoints' box and always
    /// fits; `None` is the refusal of a quotient outside `i64`, which no representable input
    /// reaches. The answer does not depend on endpoint order: reversing the segment gives the
    /// same exact point and so the same rounding.
    #[must_use]
    pub fn closest_point(&self, point: FixedVec2) -> Option<FixedVec2> {
        let (start, end) = (self.start, self.end);
        let step = [
            Wide::difference(end.x.raw(), start.x.raw()),
            Wide::difference(end.y.raw(), start.y.raw()),
        ];
        match Parameter::of(start, step, point) {
            Parameter::AtStart => Some(start),
            Parameter::AtEnd => Some(end),
            Parameter::Between { n, d } => {
                let divisor = d.magnitude();
                let axis = |from: i64, along: Wide| {
                    Wide::difference(from, 0).times(d).plus(along.times(n)).round_div(&divisor)
                };
                let [dx, dy] = step;
                let x = axis(start.x.raw(), dx)?;
                let y = axis(start.y.raw(), dy)?;
                Some(FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y)))
            }
        }
    }
}

/// Where the nearest point of a segment to a point lies, from the exact parameter `n / d`.
enum Parameter {
    /// `d = 0` or `n <= 0`: the start.
    AtStart,
    /// `n >= d > 0`: the end.
    AtEnd,
    /// `0 < n < d`, with both values.
    Between {
        /// `(p - a) . (b - a)`.
        n: Wide,
        /// `|b - a|^2`.
        d: Wide,
    },
}

impl Parameter {
    /// The exact `n = (point - start) . step` and `d = step . step`, classified.
    fn of(start: FixedVec2, step: [Wide; 2], point: FixedVec2) -> Self {
        let [dx, dy] = step;
        let d = dx.times(dx).plus(dy.times(dy));
        let wx = Wide::difference(point.x.raw(), start.x.raw());
        let wy = Wide::difference(point.y.raw(), start.y.raw());
        let n = wx.times(dx).plus(wy.times(dy));
        if d.is_zero() || n.is_negative() || n.is_zero() {
            Self::AtStart
        } else if !n.minus(d).is_negative() {
            Self::AtEnd
        } else {
            Self::Between { n, d }
        }
    }
}

/// Whether two orientations are strictly opposite: one counter-clockwise, one clockwise.
const fn opposite(first: Orientation2, second: Orientation2) -> bool {
    matches!(
        (first, second),
        (Orientation2::CounterClockwise, Orientation2::Clockwise)
            | (Orientation2::Clockwise, Orientation2::CounterClockwise)
    )
}

/// Whether `p`, with orientation `side` against the line `a`-`b`, is on the segment `a`-`b`.
fn touches(side: Orientation2, a: FixedVec2, b: FixedVec2, p: FixedVec2) -> bool {
    side == Orientation2::Collinear && in_endpoint_box(a, b, p)
}

/// Whether `p` is in the closed box spanned by `a` and `b`. Exact raw comparisons.
const fn in_endpoint_box(a: FixedVec2, b: FixedVec2, p: FixedVec2) -> bool {
    in_span(a.x.raw(), b.x.raw(), p.x.raw()) && in_span(a.y.raw(), b.y.raw(), p.y.raw())
}

/// Whether `value` is between `first` and `second`, both included, in either order.
const fn in_span(first: i64, second: i64, value: i64) -> bool {
    let (lo, hi) = if first <= second { (first, second) } else { (second, first) };
    lo <= value && value <= hi
}
