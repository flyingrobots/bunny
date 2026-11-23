//! Cubic Bezier curves evaluated by de Casteljau over `bunny-lerp/1` (`bunny-curve/1`).
//!
//! Every rounding is a [`FixedVec3::lerp`]; the rules and their bounds are proved in
//! `docs/topics/curve/README.md`.

use bunny_num::FixedQ32_32;

use crate::FixedVec3;

/// Stable contract identifier for [`CubicBezier3`].
pub const CURVE_CONTRACT_ID: &str = "bunny-curve/1";

/// A cubic Bezier curve in Q32.32 with control points `p0` to `p3` (`bunny-curve/1`).
///
/// Every function takes a parameter `t` in `[0, 1]` (raw `0..=2^32`) and returns `None`
/// outside it. Inside it, [`Self::at`] and [`Self::split`] are never refused.
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

/// The de Casteljau points at `t` that the halves of a split keep. The chapter names the
/// first level `a`, `b`, `c` (here `first_near`, `first_mid`, `first_far`), the second `d`,
/// `e` (`second_near`, `second_far`) and the third `m` (`point`); `b` is used only inside.
struct Levels {
    first_near: FixedVec3,
    first_far: FixedVec3,
    second_near: FixedVec3,
    second_far: FixedVec3,
    point: FixedVec3,
}

const fn in_unit_interval(t: FixedQ32_32) -> bool {
    t.raw() >= 0 && t.raw() <= FixedQ32_32::ONE.raw()
}

/// The three-point de Casteljau step `lerp(lerp(u, v), lerp(v, w))`, with its two inner points.
const fn quadratic(
    u: FixedVec3,
    v: FixedVec3,
    w: FixedVec3,
    t: FixedQ32_32,
) -> Option<(FixedVec3, FixedVec3, FixedVec3)> {
    let (Some(first), Some(second)) = (FixedVec3::lerp(u, v, t), FixedVec3::lerp(v, w, t)) else {
        return None;
    };
    match FixedVec3::lerp(first, second, t) {
        Some(point) => Some((first, second, point)),
        None => None,
    }
}

const fn checked_difference(to: FixedVec3, from: FixedVec3) -> Option<FixedVec3> {
    match (
        to.x.raw().checked_sub(from.x.raw()),
        to.y.raw().checked_sub(from.y.raw()),
        to.z.raw().checked_sub(from.z.raw()),
    ) {
        (Some(x), Some(y), Some(z)) => Some(FixedVec3::new(
            FixedQ32_32::from_raw(x),
            FixedQ32_32::from_raw(y),
            FixedQ32_32::from_raw(z),
        )),
        _ => None,
    }
}

const fn checked_triple(v: FixedVec3) -> Option<FixedVec3> {
    match (v.x.raw().checked_mul(3), v.y.raw().checked_mul(3), v.z.raw().checked_mul(3)) {
        (Some(x), Some(y), Some(z)) => Some(FixedVec3::new(
            FixedQ32_32::from_raw(x),
            FixedQ32_32::from_raw(y),
            FixedQ32_32::from_raw(z),
        )),
        _ => None,
    }
}

impl CubicBezier3 {
    /// Creates a curve from its four control points.
    #[must_use]
    pub const fn new(p0: FixedVec3, p1: FixedVec3, p2: FixedVec3, p3: FixedVec3) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// The de Casteljau points at `t`, each a [`FixedVec3::lerp`] from the earlier point to the
    /// later one.
    const fn levels(self, t: FixedQ32_32) -> Option<Levels> {
        if !in_unit_interval(t) {
            return None;
        }
        // A lerp with t in [0, 1] lies between its ends, so none of these is ever refused.
        let Some(first_near) = FixedVec3::lerp(self.p0, self.p1, t) else {
            return None;
        };
        let Some((first_mid, first_far, second_far)) = quadratic(self.p1, self.p2, self.p3, t)
        else {
            return None;
        };
        let Some(second_near) = FixedVec3::lerp(first_near, first_mid, t) else {
            return None;
        };
        match FixedVec3::lerp(second_near, second_far, t) {
            Some(point) => Some(Levels { first_near, first_far, second_near, second_far, point }),
            None => None,
        }
    }

    /// The point at `t` by de Casteljau (`bunny-curve/1`).
    ///
    /// Three levels of [`FixedVec3::lerp`] (`bunny-lerp/1`), six lerps per component, each
    /// rounded once to nearest with ties to even: `a = lerp(p0, p1)`, `b = lerp(p1, p2)`,
    /// `c = lerp(p2, p3)`, `d = lerp(a, b)`, `e = lerp(b, c)`, and the result `lerp(d, e)`.
    /// The result is whatever those lerps give, bit for bit. A lerp with `t` in `[0, 1]` is a
    /// convex combination, so each level adds at most half a raw unit to the error of the
    /// level below: the result is within 3/2 raw units per component of the exact cubic
    /// `(1 - t)^3 p0 + 3 t (1 - t)^2 p1 + 3 t^2 (1 - t) p2 + t^3 p3`.
    ///
    /// `at(0)` is exactly `p0` and `at(1.0)` exactly `p3`.
    ///
    /// Returns `None` only when `t` is outside `[0, 1]` (raw below 0 or above `2^32`).
    #[must_use]
    pub const fn at(self, t: FixedQ32_32) -> Option<FixedVec3> {
        match self.levels(t) {
            Some(levels) => Some(levels.point),
            None => None,
        }
    }

    /// The derivative with respect to `t` at `t` (`bunny-curve/1`).
    ///
    /// The differences `p1 - p0`, `p2 - p1` and `p3 - p2` are exact, the de Casteljau
    /// quadratic of them is three lerps per component (`lerp(lerp(d0, d1), lerp(d1, d2))`),
    /// and the result is that times 3, exact. So it is within 3 raw units per component of
    /// the exact derivative, and a multiple of 3 raw units. At `t = 0` it is exactly
    /// `3 (p1 - p0)` and at `t = 1.0` exactly `3 (p3 - p2)`.
    ///
    /// Returns `None` when `t` is outside `[0, 1]`, when any difference is outside the
    /// Q32.32 range (at every `t`, even where that difference has no weight), or when the
    /// multiplication by 3 is.
    #[must_use]
    pub const fn derivative_at(self, t: FixedQ32_32) -> Option<FixedVec3> {
        if !in_unit_interval(t) {
            return None;
        }
        let (Some(d0), Some(d1), Some(d2)) = (
            checked_difference(self.p1, self.p0),
            checked_difference(self.p2, self.p1),
            checked_difference(self.p3, self.p2),
        ) else {
            return None;
        };
        match quadratic(d0, d1, d2, t) {
            Some((_, _, slope)) => checked_triple(slope),
            None => None,
        }
    }

    /// The two halves of the curve at `t`, from the de Casteljau points of [`Self::at`]
    /// (`bunny-curve/1`).
    ///
    /// The halves are `(p0, a, d, m)` and `(m, e, c, p3)`. They share `m`, which is bit for
    /// bit `at(t)`; the first starts at exactly `p0` and the second ends at exactly `p3`. The
    /// inner points are rounded, so a half evaluated at `u` is not exactly the original at the
    /// matching parameter: it is within 3 raw units per component of the exact original at
    /// `t u` (first half) or `t + (1 - t) u` (second half).
    ///
    /// Returns `None` only when `t` is outside `[0, 1]`.
    #[must_use]
    pub const fn split(self, t: FixedQ32_32) -> Option<(Self, Self)> {
        match self.levels(t) {
            Some(Levels { first_near, first_far, second_near, second_far, point }) => Some((
                Self::new(self.p0, first_near, second_near, point),
                Self::new(point, second_far, first_far, self.p3),
            )),
            None => None,
        }
    }
}
