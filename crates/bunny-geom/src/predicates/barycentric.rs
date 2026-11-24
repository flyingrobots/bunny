//! Exact barycentric numerators of a point against a triangle, and its exact location.

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;

use super::wide::Wide;
use super::{det2, det3, ExactDeterminant};

/// A vertex of a triangle `(a, b, c)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriangleVertex {
    /// The first vertex.
    A,
    /// The second vertex.
    B,
    /// The third vertex.
    C,
}

/// An edge of a triangle `(a, b, c)`, named by its two vertices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriangleEdge {
    /// The edge from `b` to `c`, opposite `a`.
    BC,
    /// The edge from `c` to `a`, opposite `b`.
    CA,
    /// The edge from `a` to `b`, opposite `c`.
    AB,
}

/// Where a point is relative to a triangle, decided exactly from the signs of its
/// barycentric numerators.
///
/// Precedence: `Outside` when any numerator is negative (a point on an edge's line but past
/// a vertex is outside); otherwise `OnVertex` when two are zero, `OnEdge` when one is zero,
/// and `Inside` when none is. The variants partition the plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriangleLocation {
    /// Strictly inside: every numerator is positive.
    Inside,
    /// On an edge, between its two vertices, excluding them.
    OnEdge(TriangleEdge),
    /// Exactly on a vertex.
    OnVertex(TriangleVertex),
    /// Strictly outside the closed triangle.
    Outside,
}

/// The exact barycentric coordinates of a point: three integer numerators over a common
/// positive denominator, in raw squared units.
///
/// The weights of `a`, `b` and `c` are `na / d`, `nb / d` and `nc / d`, with
/// `na = D2(p, b, c)`, `nb = D2(a, p, c)`, `nc = D2(a, b, p)` and `d = D2(a, b, c)`, where
/// `D2(a, b, c) = (b - a) x (c - a)`. When `d` is negative (a clockwise triangle) all four are
/// negated, so `d > 0` and the weights are unchanged. `na + nb + nc = d` exactly. Each
/// magnitude is at most `(2^64 - 1)^2`.
///
/// `nb`, `nc` and `d` correspond to the `u`, `v` and `det` of `bunny_query`'s
/// `BarycentricNumerators`, which the ray-triangle test returns in `i128` for hits only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExactBarycentric {
    numerators: [Wide; 3],
    denominator: Wide,
}

impl ExactBarycentric {
    /// The numerators of the weights of `a`, `b` and `c`, exact.
    #[must_use]
    pub const fn numerators(&self) -> [ExactDeterminant; 3] {
        let [a, b, c] = self.numerators;
        [ExactDeterminant(a), ExactDeterminant(b), ExactDeterminant(c)]
    }

    /// The common denominator, exact and positive: the triangle's doubled area magnitude.
    #[must_use]
    pub const fn denominator(&self) -> ExactDeterminant {
        ExactDeterminant(self.denominator)
    }

    /// The exact location of the point; see [`TriangleLocation`] for the precedence.
    #[must_use]
    pub fn location(&self) -> TriangleLocation {
        if self.numerators.iter().any(Wide::is_negative) {
            return TriangleLocation::Outside;
        }
        let [a, b, c] = self.numerators.map(|n| n.is_zero());
        match (a, b, c) {
            (false, true, true) => TriangleLocation::OnVertex(TriangleVertex::A),
            (true, false, true) => TriangleLocation::OnVertex(TriangleVertex::B),
            (true, true, false) => TriangleLocation::OnVertex(TriangleVertex::C),
            (true, false, false) => TriangleLocation::OnEdge(TriangleEdge::BC),
            (false, true, false) => TriangleLocation::OnEdge(TriangleEdge::CA),
            (false, false, true) => TriangleLocation::OnEdge(TriangleEdge::AB),
            // No numerator is zero; three zeros cannot happen, since they sum to d > 0.
            _ => TriangleLocation::Inside,
        }
    }

    /// Whether the point is in the closed triangle, boundary included: not `Outside`.
    #[must_use]
    pub fn contains_closed(&self) -> bool {
        self.location() != TriangleLocation::Outside
    }

    /// Whether the point is in the open triangle, boundary excluded: `Inside`.
    #[must_use]
    pub fn contains_open(&self) -> bool {
        self.location() == TriangleLocation::Inside
    }

    /// The weights of `a`, `b` and `c` in Q32.32, each rounded once from the exact ratio:
    /// raw `round_half_even(n 2^32 / d)`. `None` when any of the three is outside `i64`.
    ///
    /// Each is within half a raw unit of the exact weight, so the three sum to `2^32` (1.0)
    /// within one raw unit; they are not adjusted to sum exactly. Decide location with
    /// [`Self::location`], not from these.
    #[must_use]
    pub fn weights_rounded(&self) -> Option<[FixedQ32_32; 3]> {
        let scale = Wide::power_of_two(32);
        let den = self.denominator.magnitude();
        let [a, b, c] = self.numerators.map(|n| n.times(scale).round_div(&den));
        Some([a?, b?, c?].map(FixedQ32_32::from_raw))
    }
}

/// The exact barycentric coordinates of `p` against the triangle `a`, `b`, `c`, or `None`
/// when the triangle has exactly zero area (collinear or repeated vertices).
///
/// Nothing is rounded; see [`ExactBarycentric`] for the numerators and
/// [`ExactBarycentric::location`] for the exact classification.
#[must_use]
pub fn barycentric2(
    p: FixedVec2,
    a: FixedVec2,
    b: FixedVec2,
    c: FixedVec2,
) -> Option<ExactBarycentric> {
    let denominator = det2(a, b, c);
    if denominator.is_zero() {
        return None;
    }
    let numerators = [det2(p, b, c), det2(a, p, c), det2(a, b, p)];
    if denominator.is_negative() {
        return Some(ExactBarycentric {
            numerators: numerators.map(Wide::negated),
            denominator: denominator.negated(),
        });
    }
    Some(ExactBarycentric { numerators, denominator })
}

/// The exact barycentric coordinates of a point `p` exactly in the plane of the 3D triangle
/// `a`, `b`, `c`; `None` when the triangle has zero area or `p` is not exactly coplanar
/// (`orientation3(a, b, c, p)` is not `Coplanar`).
///
/// They are [`barycentric2`] of the projection that drops the first axis, in the order `x`,
/// `y`, `z`, in which the normal `(b - a) x (c - a)` is nonzero, keeping `(y, z)`, `(z, x)` or
/// `(x, y)`. For a coplanar point the ratios are those of the 3D areas exactly; the
/// numerators are in the units of that projection.
#[must_use]
pub fn barycentric3_coplanar(
    p: FixedVec3,
    a: FixedVec3,
    b: FixedVec3,
    c: FixedVec3,
) -> Option<ExactBarycentric> {
    if !det3(a, b, c, p).is_zero() {
        return None;
    }
    let drop_x = |q: FixedVec3| FixedVec2::new(q.y, q.z);
    let drop_y = |q: FixedVec3| FixedVec2::new(q.z, q.x);
    let drop_z = |q: FixedVec3| FixedVec2::new(q.x, q.y);
    barycentric2(drop_x(p), drop_x(a), drop_x(b), drop_x(c))
        .or_else(|| barycentric2(drop_y(p), drop_y(a), drop_y(b), drop_y(c)))
        .or_else(|| barycentric2(drop_z(p), drop_z(a), drop_z(b), drop_z(c)))
}
