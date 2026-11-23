//! Checked clamp, inverse lerp and remap in Q32.32 (`bunny-lerp/1`). Not yet implemented.

use crate::fixed_q32_32::FixedQ32_32;

impl FixedQ32_32 {
    /// `self` limited to `[lo, hi]`. Not yet implemented: always `None`.
    #[must_use]
    pub const fn checked_clamp(self, lo: Self, hi: Self) -> Option<Self> {
        let _ = (self, lo, hi);
        None
    }

    /// The parameter `t` of `x` on the line from `a` to `b`. Not yet implemented.
    #[must_use]
    pub const fn inverse_lerp(a: Self, b: Self, x: Self) -> Option<Self> {
        Self::remap(x, [a, b], [Self::ZERO, Self::ONE])
    }

    /// `x` mapped from `from` to `to`. Not yet implemented: always `None`.
    #[must_use]
    pub const fn remap(x: Self, from: [Self; 2], to: [Self; 2]) -> Option<Self> {
        let _ = (x, from, to);
        None
    }
}
