//! Cubic Bezier curves evaluated by de Casteljau over `bunny-lerp/1` (`bunny-curve/1`).

use bunny_num::FixedQ32_32;

use crate::FixedVec3;

/// Stable contract identifier for [`CubicBezier3`].
pub const CURVE_CONTRACT_ID: &str = "bunny-curve/1";

/// A cubic Bezier curve in Q32.32 with control points `p0` to `p3` (`bunny-curve/1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubicBezier3 {
    /// The start point.
    pub p0: FixedVec3,
    /// The first inner control point.
    pub p1: FixedVec3,
    /// The second inner control point.
    pub p2: FixedVec3,
    /// The end point.
    pub p3: FixedVec3,
}

impl CubicBezier3 {
    /// Creates a curve from its four control points.
    #[must_use]
    pub const fn new(p0: FixedVec3, p1: FixedVec3, p2: FixedVec3, p3: FixedVec3) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// The point at `t`.
    #[must_use]
    pub const fn at(self, t: FixedQ32_32) -> Option<FixedVec3> {
        let _ = (self, t);
        None
    }

    /// The derivative at `t`.
    #[must_use]
    pub const fn derivative_at(self, t: FixedQ32_32) -> Option<FixedVec3> {
        let _ = (self, t);
        None
    }

    /// The two halves at `t`.
    #[must_use]
    pub const fn split(self, t: FixedQ32_32) -> Option<(Self, Self)> {
        let _ = (self, t);
        None
    }
}
