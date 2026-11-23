//! The exact closest point of a triangle to a query point, and the box bound that prunes a
//! closest-point search.
//!
//! The closest point and its squared distance are rationals, decided and compared exactly in
//! 256- and 512-bit integers; each is rounded once, to nearest with ties to even, only when it
//! is read. See `docs/topics/mesh-queries/README.md`.

mod feature;
mod wide;

use core::cmp::Ordering;

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use crate::{TraceTriangle, TRACE_COORD_LIMIT_RAW};
use feature::Candidate;
use wide::{round_quotient, I256};

/// Largest box gap, per axis, that [`PointQuery::box_distance_bound`] squares exactly: `2^42`
/// raw. A larger gap is counted as `2^42`, whose square `2^84` is already beyond every squared
/// distance from a point to a triangle in the trace domain (below `3 * 2^82`).
pub const POINT_BOX_GAP_LIMIT_RAW: u128 = 1 << 42;

/// A query point prepared for closest-point tests, inside the trace domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointQuery {
    point: [i64; 3],
}

impl PointQuery {
    /// Prepares a query point.
    ///
    /// Returns `None` when a coordinate is outside the trace domain
    /// (`abs(raw) < 2^40`, [`TRACE_COORD_LIMIT_RAW`]).
    #[must_use]
    pub fn new(point: FixedVec3) -> Option<Self> {
        let point = [point.x.raw(), point.y.raw(), point.z.raw()];
        let inside = point.iter().all(|c| c.unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs());
        inside.then_some(Self { point })
    }

    /// The query point.
    #[must_use]
    pub const fn point(&self) -> FixedVec3 {
        let [x, y, z] = self.point;
        FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
    }

    /// The exact closest point of the closed triangle to the query point.
    ///
    /// The triangle is two-sided and its edges and corners belong to it. A triangle whose
    /// vertices are collinear or coincident is not refused: it is the segment or point they
    /// span, and its closest point is that set's. The result is decided exactly and rounded
    /// only by [`TrianglePoint::point`] and [`TrianglePoint::distance_squared`].
    #[must_use]
    pub fn closest_on_triangle(&self, triangle: &TraceTriangle) -> TrianglePoint {
        let raw = |v: FixedVec3| [v.x.raw(), v.y.raw(), v.z.raw()];
        TrianglePoint { exact: feature::closest(self.point, triangle.vertices().map(raw)) }
    }

    /// A lower bound on the squared distance, in raw units squared, from the query point to
    /// the box.
    ///
    /// Per axis the gap is `max(min - p, p - max, 0)`, exactly in `i128`, capped at
    /// [`POINT_BOX_GAP_LIMIT_RAW`]; the bound is the sum of the three squares. It equals the
    /// exact squared distance to a box whose gaps are below the cap and is never above it, so
    /// it is never above the squared distance to any point in the box. The corners are not
    /// checked for order: an axis whose minimum is above its maximum contributes the larger of
    /// its two signed gaps.
    #[must_use]
    pub fn box_distance_bound(&self, aabb: &FixedAabb3) -> u128 {
        let [px, py, pz] = self.point;
        let gap = |p: i64, min: FixedQ32_32, max: FixedQ32_32| {
            let below = i128::from(min.raw()) - i128::from(p);
            let above = i128::from(p) - i128::from(max.raw());
            let gap = below.max(above).max(0).unsigned_abs().min(POINT_BOX_GAP_LIMIT_RAW);
            gap * gap
        };
        let (min, max) = (aabb.min, aabb.max);
        gap(px, min.x, max.x) + gap(py, min.y, max.y) + gap(pz, min.z, max.z)
    }
}

/// The exact closest point of one triangle to a [`PointQuery`].
///
/// It holds the point and its squared distance as exact rationals. [`Self::cmp_distance`] and
/// [`Self::cmp_bound`] compare them exactly; [`Self::point`] and [`Self::distance_squared`]
/// round them once.
#[derive(Clone, Copy, Debug)]
pub struct TrianglePoint {
    exact: Candidate,
}

impl TrianglePoint {
    /// The closest point, each coordinate the exact coordinate rounded to the nearest raw
    /// unit, ties to even: within half a raw unit per axis of the exact point.
    #[must_use]
    pub fn point(&self) -> FixedVec3 {
        // Each exact coordinate lies between two vertex coordinates, below `2^40` in magnitude.
        let round = |n| FixedQ32_32::from_raw(round_quotient(n, self.exact.denominator, 41));
        let [x, y, z] = self.exact.numerators.map(round);
        FixedVec3::new(x, y, z)
    }

    /// The squared distance from the query point to the exact closest point, as a Q32.32
    /// value rounded to the nearest raw unit, ties to even: within half a raw unit (`2^-33`)
    /// of the exact squared distance. It is below `3 * 2^50` raw.
    #[must_use]
    pub fn distance_squared(&self) -> FixedQ32_32 {
        // Raw units squared over `2^32` give a raw Q32.32 value; the quotient is below `2^52`.
        let denominator = self.exact.denominator.shl(FixedQ32_32::FRACTIONAL_BITS);
        FixedQ32_32::from_raw(round_quotient(self.exact.distance, denominator, 52))
    }

    /// Compares the exact squared distances of two closest points.
    #[must_use]
    pub fn cmp_distance(&self, other: &Self) -> Ordering {
        self.exact.cmp_distance(&other.exact)
    }

    /// Compares the exact squared distance, in raw units squared, with `bound`.
    ///
    /// A `bound` of `2^127` or more is compared as `2^127 - 1`; every squared distance is
    /// below `3 * 2^82`, so the answer is unchanged.
    #[must_use]
    pub fn cmp_bound(&self, bound: u128) -> Ordering {
        let bound = I256::from_i128(i128::try_from(bound).unwrap_or(i128::MAX));
        let exact = &self.exact;
        exact.distance.cmp_products(I256::ONE, bound, exact.denominator)
    }
}
