//! Classification of segments, triangles and boxes against a fixed plane.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use super::{FixedPlane3, FixedSegment3, PlaneSide};
use crate::FixedAabb3;

/// Where a segment, triangle or box lies relative to a plane.
///
/// Every variant is decided by the point rule, [`FixedPlane3::classify`], at the
/// shape's endpoints, vertices or corners. A shape that touches the plane without
/// crossing it takes the side of its other vertices: under the closed front
/// half-space of clipping, `Front` keeps it whole and `Behind` keeps at most the
/// touching face, edge or point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneSpan {
    /// Every vertex is `Front` or `On`, and at least one is `Front`.
    Front,
    /// Every vertex is `Behind` or `On`, and at least one is `Behind`.
    Behind,
    /// At least one vertex is `Front` and at least one is `Behind`.
    Straddling,
    /// Every vertex is `On`.
    Coplanar,
}

/// Which sides a set of vertices has reached.
#[derive(Clone, Copy, Default)]
struct Reached {
    front: bool,
    behind: bool,
}

impl Reached {
    const fn with(self, side: PlaneSide) -> Self {
        match side {
            PlaneSide::Front => Self { front: true, behind: self.behind },
            PlaneSide::Behind => Self { front: self.front, behind: true },
            PlaneSide::On => self,
        }
    }

    const fn span(self) -> PlaneSpan {
        match (self.front, self.behind) {
            (false, false) => PlaneSpan::Coplanar,
            (true, false) => PlaneSpan::Front,
            (false, true) => PlaneSpan::Behind,
            (true, true) => PlaneSpan::Straddling,
        }
    }
}

/// The two ends of one box axis: the one whose rounded product with `normal` is
/// smallest, then the one whose product is largest.
///
/// `checked_mul` rounds to nearest, ties to even, which is monotone in each
/// operand, so for `normal >= 0` the larger coordinate gives the larger product
/// and for `normal < 0` the smaller one does. Ordering the ends first makes an
/// inverted axis (`min > max`) the same as its swapped axis.
fn axis_ends(normal: FixedQ32_32, a: FixedQ32_32, b: FixedQ32_32) -> (FixedQ32_32, FixedQ32_32) {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    if normal.raw() >= 0 {
        (lo, hi)
    } else {
        (hi, lo)
    }
}

impl FixedPlane3 {
    /// Classifies a segment by the point rule at its two endpoints.
    ///
    /// Returns `None` when either endpoint's signed distance overflows. A
    /// zero-length segment gets its point's side (`On` maps to `Coplanar`).
    #[must_use]
    pub fn classify_segment(self, segment: FixedSegment3) -> Option<PlaneSpan> {
        self.classify_points(&[segment.start, segment.end])
    }

    /// Classifies a triangle by the point rule at its three vertices.
    ///
    /// A degenerate triangle (collinear or coincident vertices) is classified
    /// like any other. Returns `None` when any vertex's signed distance
    /// overflows. See `docs/topics/plane-clipping/README.md`.
    #[must_use]
    pub fn classify_triangle(self, triangle: [FixedVec3; 3]) -> Option<PlaneSpan> {
        self.classify_points(&triangle)
    }

    /// Classifies a box exactly as the point rule classifies its eight corners.
    ///
    /// Only two corners are evaluated: per axis, the end with the smallest
    /// rounded product with the normal, and the end with the largest. The signed
    /// distance is a sum of one such product per axis, each partial sum checked,
    /// so every corner's distance and every partial sum lies between those of
    /// the two corners: the result, `None` included, is the eight-corner result.
    /// An inverted box (`min > max` on an axis) is the box with the same eight
    /// corners. Returns `None` when any corner's signed distance overflows.
    #[must_use]
    pub fn classify_aabb(self, aabb: FixedAabb3) -> Option<PlaneSpan> {
        let n = self.normal;
        let x = axis_ends(n.x, aabb.min.x, aabb.max.x);
        let y = axis_ends(n.y, aabb.min.y, aabb.max.y);
        let z = axis_ends(n.z, aabb.min.z, aabb.max.z);
        let least = FixedVec3::new(x.0, y.0, z.0);
        let most = FixedVec3::new(x.1, y.1, z.1);
        self.classify_points(&[least, most])
    }

    fn classify_points(self, points: &[FixedVec3]) -> Option<PlaneSpan> {
        let mut reached = Reached::default();
        for point in points {
            reached = reached.with(self.classify(*point)?);
        }
        Some(reached.span())
    }
}
