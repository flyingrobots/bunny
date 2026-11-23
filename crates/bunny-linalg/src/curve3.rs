//! The parametric curve interface that [`crate::ArcLengthTable`] samples (`bunny-curve/1`).

use bunny_num::FixedQ32_32;

use crate::{CubicBezier3, FixedVec3};

/// A curve in Q32.32 made of `span_count()` spans, with its parameter `u` running over
/// `[0, span_count]` (`bunny-curve/1`).
///
/// An implementation must be deterministic: the same `u` gives the same point, bit for bit.
/// [`crate::ArcLengthTable`] samples it only at parameters in `[0, span_count]`; `at` returns
/// `None` outside that range, and may return `None` inside it, which the table reports as
/// [`crate::ArcLengthError::CurveRefused`].
pub trait Curve3 {
    /// The number of spans; the parameter runs over `[0, span_count]`.
    fn span_count(&self) -> u32;

    /// The point at parameter `u`, or `None` when `u` is outside `[0, span_count]`.
    fn at(&self, u: FixedQ32_32) -> Option<FixedVec3>;
}

/// One span, the parameter of [`CubicBezier3::at`]: `u` in `[0, 1]`.
impl Curve3 for CubicBezier3 {
    fn span_count(&self) -> u32 {
        1
    }

    fn at(&self, u: FixedQ32_32) -> Option<FixedVec3> {
        Self::at(*self, u)
    }
}
