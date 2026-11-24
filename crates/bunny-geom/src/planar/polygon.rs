//! Point in polygon.

use bunny_linalg::FixedVec2;

use super::FixedSegment2;
use crate::{orientation2, Orientation2};

/// A 2D polygon borrowed as a slice of fixed-point vertices, in either winding: the closed
/// region bounded by the edges `vertices[i]`-`vertices[(i + 1) % n]`, the last edge closing
/// the loop.
///
/// The vertices are not validated and nothing is allocated. A simple polygon is the intended
/// input. Fewer than three vertices, repeated consecutive vertices, collinear runs, zero area
/// and self-intersection are all accepted and classified by the one rule of
/// [`Self::classify_point`]; self-intersection is not detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedPolygon2<'a> {
    /// The vertices in order; the polygon closes from the last back to the first.
    pub vertices: &'a [FixedVec2],
}

/// Where a point lies against a polygon ([`FixedPolygon2::classify_point`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolygonLocation {
    /// Off every edge, with a nonzero winding number.
    Inside,
    /// On an edge or a vertex.
    Boundary,
    /// Off every edge, with winding number zero.
    Outside,
}

impl<'a> FixedPolygon2<'a> {
    /// Borrows `vertices` as a polygon. Any slice is accepted, the empty one included.
    #[must_use]
    pub const fn new(vertices: &'a [FixedVec2]) -> Self {
        Self { vertices }
    }

    /// Classifies `point` exactly against the closed polygon.
    ///
    /// `Boundary` when the point is on some edge ([`FixedSegment2::contains_point`]: an exact
    /// [`orientation2`] sign and the endpoints' box), including every vertex. Otherwise the
    /// winding number is counted from exact `orientation2` signs: an edge rising across the
    /// point's height (from `y <= p.y` to `y > p.y`) with the point on its left adds one, an
    /// edge falling across it with the point on its right subtracts one. `Inside` when that
    /// sum is nonzero, `Outside` when it is zero. Exact for every representable input; no
    /// value is rounded.
    ///
    /// For a simple polygon, in either winding, this is the closed polygon. No vertices is
    /// the empty set, one vertex a point and two vertices a segment (both edges, no inside).
    /// Repeated consecutive vertices and collinear runs add zero-length or overlapping edges
    /// and change nothing; a zero-area polygon has no inside. For self-intersecting input
    /// `Inside` means a nonzero winding number (the nonzero fill rule): a region wound twice
    /// is inside, where the even-odd rule would put it outside. The answer does not depend
    /// on the starting vertex or the winding.
    #[must_use]
    pub fn classify_point(&self, point: FixedVec2) -> PolygonLocation {
        let mut winding = 0_i64;
        let following = self.vertices.iter().cycle().skip(1);
        for (&a, &b) in self.vertices.iter().zip(following) {
            if FixedSegment2::new(a, b).contains_point(point) {
                return PolygonLocation::Boundary;
            }
            // At most one per edge, and a slice has fewer than `2^63` elements.
            winding += crossing(a, b, point);
        }
        if winding == 0 {
            PolygonLocation::Outside
        } else {
            PolygonLocation::Inside
        }
    }

    /// Whether `point` is in the closed polygon: [`Self::classify_point`] is not `Outside`.
    #[must_use]
    pub fn contains_point(&self, point: FixedVec2) -> bool {
        self.classify_point(point) != PolygonLocation::Outside
    }
}

/// The edge `a`-`b`'s contribution to the winding number about `p`, which is not on it.
fn crossing(a: FixedVec2, b: FixedVec2, p: FixedVec2) -> i64 {
    let (ay, by, py) = (a.y.raw(), b.y.raw(), p.y.raw());
    if ay <= py && py < by {
        i64::from(orientation2(a, b, p) == Orientation2::CounterClockwise)
    } else if by <= py && py < ay {
        -i64::from(orientation2(a, b, p) == Orientation2::Clockwise)
    } else {
        0
    }
}
