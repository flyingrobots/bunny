//! Exact repeated division by one Q32.32 divisor.
//!
//! [`FixedDivisor`] prepares a divisor once, at the cost of one 128-bit division to find its
//! reciprocal, and then divides by it with multiplications. It uses division by an
//! invariant integer: Möller and Granlund, "Improved Division by Invariant Integers",
//! IEEE Transactions on Computers 60(2), 2011, Algorithm 4. Every quotient, remainder and
//! rounding decision is exact, so `divisor.checked_div(a)` returns the same bits as
//! [`FixedQ32_32::checked_div`].

use crate::FixedQ32_32;

/// A nonzero Q32.32 divisor prepared for repeated exact division.
///
/// Preparing costs about as much as one ordinary division; each later division costs
/// roughly half of one. It pays off when one divisor is used several times, such as a ray
/// direction tested against many boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedDivisor {
    /// The divisor's magnitude shifted left until its top bit is set.
    normalized: u64,
    /// The left shift applied to the magnitude.
    shift: u32,
    /// `floor((2^128 - 1) / normalized) - 2^64`, the Möller–Granlund reciprocal.
    reciprocal: u64,
    negative: bool,
}

impl FixedDivisor {
    /// Prepares `divisor`, or returns `None` when it is zero.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn new(divisor: FixedQ32_32) -> Option<Self> {
        let magnitude = divisor.raw().unsigned_abs();
        if magnitude == 0 {
            return None;
        }
        let shift = magnitude.leading_zeros();
        let normalized = magnitude << shift;
        // normalized >= 2^63, so the 128-bit quotient lies in [2^64, 2^65].
        let reciprocal = (u128::MAX / normalized as u128).wrapping_sub(1 << 64) as u64;
        Some(Self { normalized, shift, reciprocal, negative: divisor.raw() < 0 })
    }

    /// The divisor this was prepared from.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn divisor(self) -> FixedQ32_32 {
        let magnitude = (self.normalized >> self.shift) as i64;
        FixedQ32_32::from_raw(if self.negative { magnitude.wrapping_neg() } else { magnitude })
    }

    /// Divides `dividend` by this divisor, rounding ties to even.
    ///
    /// Returns exactly what `dividend.checked_div(divisor)` returns, including `None` when
    /// the quotient does not fit Q32.32.
    #[must_use]
    pub const fn checked_div(self, dividend: FixedQ32_32) -> Option<FixedQ32_32> {
        let (quotient, remainder) = self.divide_magnitude(dividend.raw().unsigned_abs());
        let rounded = round_ties_to_even(quotient, remainder, self.normalized);
        signed_quotient(rounded, (dividend.raw() < 0) ^ self.negative)
    }

    /// `floor(magnitude * 2^32 / |divisor|)`, and the remainder scaled by `2^shift`.
    #[allow(clippy::cast_possible_truncation)]
    const fn divide_magnitude(self, magnitude: u64) -> (u128, u64) {
        // The numerator magnitude * 2^(32 + shift), as three 64-bit words; the top one is
        // below 2^31.
        let wide = (magnitude as u128) << 32;
        let top = if self.shift == 0 { 0 } else { (wide >> (128 - self.shift)) as u64 };
        let shifted = wide << self.shift;
        let (middle, low) = ((shifted >> 64) as u64, shifted as u64);
        if top == 0 && middle < self.normalized {
            let (quotient, remainder) = self.divide_two_words(middle, low);
            return (quotient as u128, remainder);
        }
        let (high_quotient, partial) = self.divide_two_words(top, middle);
        let (low_quotient, remainder) = self.divide_two_words(partial, low);
        (((high_quotient as u128) << 64) | low_quotient as u128, remainder)
    }

    /// Divides the two-word value `high * 2^64 + low` by `normalized`, for `high <
    /// normalized`: Möller–Granlund Algorithm 4. Returns the quotient and remainder.
    #[allow(clippy::cast_possible_truncation)]
    const fn divide_two_words(self, high: u64, low: u64) -> (u64, u64) {
        let numerator = ((high as u128) << 64) | low as u128;
        let product = (self.reciprocal as u128).wrapping_mul(high as u128).wrapping_add(numerator);
        let mut quotient = ((product >> 64) as u64).wrapping_add(1);
        let mut remainder = low.wrapping_sub(quotient.wrapping_mul(self.normalized));
        if remainder > product as u64 {
            quotient = quotient.wrapping_sub(1);
            remainder = remainder.wrapping_add(self.normalized);
        }
        if remainder >= self.normalized {
            quotient = quotient.wrapping_add(1);
            remainder -= self.normalized;
        }
        (quotient, remainder)
    }
}

/// Adds one when the remainder is more than half the divisor, or exactly half with an odd
/// quotient. Both are scaled by the same power of two, which does not change the decision.
const fn round_ties_to_even(quotient: u128, remainder: u64, divisor: u64) -> u128 {
    let rest = divisor - remainder;
    if remainder > rest || (remainder == rest && quotient & 1 == 1) {
        quotient + 1
    } else {
        quotient
    }
}

/// Applies the sign, or returns `None` when the result does not fit Q32.32.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
const fn signed_quotient(magnitude: u128, negative: bool) -> Option<FixedQ32_32> {
    const MIN_MAGNITUDE: u128 = 1 << 63;
    if negative {
        if magnitude > MIN_MAGNITUDE {
            return None;
        }
        // 2^63 becomes i64::MIN, whose negation is itself.
        return Some(FixedQ32_32::from_raw((magnitude as i64).wrapping_neg()));
    }
    if magnitude >= MIN_MAGNITUDE {
        return None;
    }
    Some(FixedQ32_32::from_raw(magnitude as i64))
}
