//! Checked clamp, inverse lerp and remap in Q32.32 (`bunny-lerp/1`).
//!
//! `inverse_lerp` and `remap` compute their exact rational value in a wide integer and round
//! it once, to nearest with ties to even, as `lerp` does. `checked_clamp` is exact. The rules
//! and their proofs are in `docs/topics/lerp/README.md`.

use crate::fixed_q32_32::FixedQ32_32;

impl FixedQ32_32 {
    /// `self` limited to `[lo, hi]` (`bunny-lerp/1`): exact, with no rounding.
    ///
    /// Returns `lo` when `self < lo`, `hi` when `self > hi` and `self` otherwise, so
    /// `lo == hi` returns `lo`. Returns `None` when `lo > hi`, where [`Ord::clamp`] would
    /// panic; the bounds are never swapped. [`Ord::min`] and [`Ord::max`] are exact too.
    #[must_use]
    pub const fn checked_clamp(self, lo: Self, hi: Self) -> Option<Self> {
        if lo.raw() > hi.raw() {
            None
        } else if self.raw() < lo.raw() {
            Some(lo)
        } else if self.raw() > hi.raw() {
            Some(hi)
        } else {
            Some(self)
        }
    }

    /// The parameter `t` at which the line from `a` to `b` reaches `x` (`bunny-lerp/1`).
    ///
    /// The result is `round_half_even((x - a) * 2^32 / (b - a))` on raw values: the exact
    /// quotient rounded once, so within half a raw unit of the exact `(x - a) / (b - a)`. It
    /// is [`Self::remap`]`(x, [a, b], [ZERO, ONE])`, bit for bit.
    ///
    /// - `x = a` returns exactly `ZERO` and `x = b` exactly `ONE`.
    /// - Any `x` is accepted: outside the range between `a` and `b`, `t` is extrapolated.
    /// - [`Self::lerp`]`(a, b, inverse_lerp(a, b, x))` is exactly `x` when `|b - a| <= 1.0`,
    ///   and within `floor(1/2 + |b - a| / 2^33)` raw units of it otherwise.
    /// - `inverse_lerp(a, b, lerp(a, b, t))` is exactly `t` when `|b - a| >= 1.0`, and
    ///   within `floor(1/2 + 2^31 / |b - a|)` raw units of it otherwise (`|b - a|` raw).
    ///
    /// `x - a` and `b - a` are exact in `i128`; `(x - a) 2^32` is below `2^96` in magnitude.
    ///
    /// Returns `None` when `a == b` (the range is degenerate) and when the rounded result is
    /// outside the Q32.32 range.
    #[must_use]
    pub const fn inverse_lerp(a: Self, b: Self, x: Self) -> Option<Self> {
        Self::remap(x, [a, b], [Self::ZERO, Self::ONE])
    }

    /// `x` mapped linearly from the range `[in_lo, in_hi]` onto `[out_lo, out_hi]`
    /// (`bunny-lerp/1`).
    ///
    /// The result is `out_lo + round_half_even((x - in_lo) (out_hi - out_lo) / (in_hi -
    /// in_lo))` on raw values: the step from `out_lo` is computed exactly, rounded once to
    /// the nearest raw unit with an exact half going to the even step, and added to
    /// `out_lo`. So the result is within half a raw unit of the exact value whatever the
    /// widths of the two ranges.
    ///
    /// - `x = in_lo` returns exactly `out_lo` and `x = in_hi` exactly `out_hi`.
    /// - Any `x` is accepted: outside `[in_lo, in_hi]` the line is extrapolated. Either range
    ///   may be reversed, and `out_lo == out_hi` maps every `x` to `out_lo`.
    /// - `remap(t, [ZERO, ONE], [a, b])` is [`Self::lerp`]`(a, b, t)`, bit for bit.
    ///
    /// The three differences are exact in `i128` (at most `2^64 - 1` in magnitude), their
    /// product's magnitude is below `2^128`, exact in `u128`, and a rounded step above
    /// `2^64 - 1` is refused before the sum, which no `out_lo` brings back into range.
    ///
    /// Returns `None` when `in_lo == in_hi` (the input range is degenerate) and when the
    /// rounded result is outside the Q32.32 range.
    #[must_use]
    pub const fn remap(
        x: Self,
        [in_lo, in_hi]: [Self; 2],
        [out_lo, out_hi]: [Self; 2],
    ) -> Option<Self> {
        let run = difference(in_lo, in_hi);
        if run == 0 {
            return None;
        }
        let offset = difference(in_lo, x);
        let rise = difference(out_lo, out_hi);
        // Each magnitude is at most 2^64 - 1, so the product is below 2^128: exact in u128.
        let product = offset.unsigned_abs() * rise.unsigned_abs();
        let negative = (offset < 0) != ((rise < 0) != (run < 0));
        match rounded_quotient(product, run.unsigned_abs()) {
            Some(magnitude) => add_step(out_lo, magnitude, negative),
            None => None,
        }
    }
}

/// `to - from` on raw values, exact in `i128`.
const fn difference(from: FixedQ32_32, to: FixedQ32_32) -> i128 {
    to.raw() as i128 - from.raw() as i128
}

/// `round_half_even(numerator / divisor)` for `divisor > 0`, or `None` above `2^64 - 1`.
const fn rounded_quotient(numerator: u128, divisor: u128) -> Option<u128> {
    let quotient = numerator / divisor;
    let remainder = numerator % divisor;
    let complement = divisor - remainder;
    // A divisor of 1 leaves no remainder, so an increment only follows a quotient below 2^127.
    let rounded = if remainder > complement || (remainder == complement && quotient & 1 == 1) {
        quotient + 1
    } else {
        quotient
    };
    if rounded > u64::MAX as u128 {
        None
    } else {
        Some(rounded)
    }
}

/// `base` plus or minus a step of at most `2^64 - 1`, or `None` outside Q32.32.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
const fn add_step(base: FixedQ32_32, magnitude: u128, negative: bool) -> Option<FixedQ32_32> {
    let step = if negative { -(magnitude as i128) } else { magnitude as i128 };
    // |base| <= 2^63 and |step| < 2^64, so the sum is exact in i128 before the range check.
    let sum = base.raw() as i128 + step;
    if sum < i64::MIN as i128 || sum > i64::MAX as i128 {
        None
    } else {
        Some(FixedQ32_32::from_raw(sum as i64))
    }
}
