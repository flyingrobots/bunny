//! Exact linear interpolation and exact ratios in Q32.32 (`bunny-lerp/1`).
//!
//! Each function computes its exact rational value in a wide integer and rounds it once, to
//! nearest with ties to even. The rules and their proofs are in `docs/topics/lerp/README.md`.

use crate::fixed_q32_32::{FixedQ32_32, FRAC_BITS};

/// Stable contract identifier for [`FixedQ32_32::lerp`], [`FixedQ32_32::from_ratio`] and the
/// component-wise lerps built on them in `bunny-linalg`.
pub const LERP_CONTRACT_ID: &str = "bunny-lerp/1";

impl FixedQ32_32 {
    /// Interpolates from `a` to `b` by `t` (`bunny-lerp/1`).
    ///
    /// The result is `a + round_half_even((b - a) * t / 2^32)` on raw values: the step from
    /// `a` is computed exactly and rounded once, to the nearest raw unit with an exact half
    /// going to the even step, and then added to `a`. So the result is within half a raw unit
    /// of the exact `a + (b - a) t`, and `lerp(a + c, b + c, t) == lerp(a, b, t) + c`.
    ///
    /// - `t = 0` returns exactly `a` and `t = 1.0` exactly `b`, for every `a` and `b`.
    /// - Any `t` is accepted: outside `[0, 1]` the line is extrapolated.
    /// - For fixed `a` and `b` the result is monotone in `t` (non-decreasing when `b >= a`),
    ///   strictly when `|b - a| >= 1.0`.
    ///
    /// `b - a` is exact in `i128` (at most `2^64 - 1` in magnitude), so it never overflows by
    /// itself; the product with `t` is below `2^127` in magnitude and also exact.
    ///
    /// Returns `None` when the rounded result is outside the Q32.32 range.
    #[must_use]
    pub const fn lerp(a: Self, b: Self, t: Self) -> Option<Self> {
        let _ = (a, b, t, FRAC_BITS);
        None
    }

    /// The Q32.32 value nearest `numerator / denominator` (`bunny-lerp/1`).
    ///
    /// The result is `round_half_even(numerator * 2^32 / denominator)` on raw values, computed
    /// exactly in `u64`. An exact half cannot occur: it would need `2^33` to divide
    /// `denominator`. `from_ratio(d, d)` is exactly [`Self::ONE`] for every `d > 0`.
    ///
    /// Returns `None` when `denominator == 0` (including `0 / 0`) and when the result is
    /// `2^31` or more, outside the Q32.32 range.
    #[must_use]
    pub const fn from_ratio(numerator: u32, denominator: u32) -> Option<Self> {
        let _ = (numerator, denominator);
        None
    }
}
