//! Exact division without a division instruction, for dividends no larger than the divisor.
//!
//! Normalizing a vector divides each component by the vector's length, and no component is
//! longer than the length. [`UnitDivisor`] serves that case: it finds an approximate
//! reciprocal of the divisor with a lookup table and three Newton steps (multiplications
//! only), estimates each quotient by multiplying, and then corrects the estimate with the
//! exact remainder. The results equal [`FixedQ32_32::checked_div`] bit for bit.

use crate::FixedQ32_32;

/// Reciprocal seeds for normalized divisors, indexed by the 8 bits below the leading one:
/// `round(2^15 * 1024 / (513 + 2i))`, about `2^15 * 2 / (1 + (i + 0.5) / 256)`.
static RECIPROCAL_SEEDS: [u16; 256] = reciprocal_seeds();

/// A nonzero divisor prepared for exact, division-free division of dividends whose magnitude
/// does not exceed its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitDivisor {
    magnitude: u64,
    /// The left shift that puts the magnitude's leading one in bit 63.
    shift: u32,
    /// About `2^127 / (magnitude << shift)`, within a few units.
    reciprocal: u64,
    negative: bool,
}

impl UnitDivisor {
    /// Prepares `divisor` with multiplications only, or returns `None` when it is zero.
    #[must_use]
    pub fn new(divisor: FixedQ32_32) -> Option<Self> {
        let magnitude = divisor.raw().unsigned_abs();
        if magnitude == 0 {
            return None;
        }
        let shift = magnitude.leading_zeros();
        let reciprocal = reciprocal(magnitude << shift);
        Some(Self { magnitude, shift, reciprocal, negative: divisor.raw() < 0 })
    }

    /// Divides `dividend` by this divisor, rounding ties to even.
    ///
    /// Returns exactly what `dividend.checked_div(divisor)` returns when `|dividend| <=
    /// |divisor|`, and `None` when the dividend is larger.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn checked_div(self, dividend: FixedQ32_32) -> Option<FixedQ32_32> {
        let numerator_magnitude = dividend.raw().unsigned_abs();
        if numerator_magnitude > self.magnitude {
            return None;
        }
        // The quotient is at most 2^32, so it fits an i64 with room to spare.
        let quotient = self.rounded_quotient(numerator_magnitude) as i64;
        let negative = (dividend.raw() < 0) ^ self.negative;
        Some(FixedQ32_32::from_raw(if negative { -quotient } else { quotient }))
    }

    /// `round_ties_to_even(magnitude * 2^32 / divisor)` for `magnitude <= divisor`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn rounded_quotient(self, magnitude: u64) -> i128 {
        // magnitude * reciprocal < 2^(128 - shift), and the quotient is that over 2^(95 - shift),
        // so the estimate is below 2^33 and fits a u64.
        let estimate =
            ((u128::from(magnitude) * u128::from(self.reciprocal)) >> (95 - self.shift)) as u64;
        let divisor = i128::from(self.magnitude);
        // One widening 64x64 multiply; the exact remainder may be slightly negative or too large.
        let product = (u128::from(estimate) * u128::from(self.magnitude)) as i128;
        let mut remainder = (i128::from(magnitude) << 32) - product;
        let mut quotient = i128::from(estimate);
        // The estimate is off by at most a unit or two; the exact remainder settles it.
        while remainder < 0 {
            quotient -= 1;
            remainder += divisor;
        }
        while remainder >= divisor {
            quotient += 1;
            remainder -= divisor;
        }
        let twice = remainder << 1;
        if twice > divisor || (twice == divisor && quotient & 1 == 1) {
            quotient + 1
        } else {
            quotient
        }
    }
}

/// About `2^127 / normalized` for `normalized` in `[2^63, 2^64)`: a table seed (about 9 good
/// bits) refined by three Newton steps `y <- y * (2 - x * y)`, each doubling the good bits.
#[allow(clippy::cast_possible_truncation)]
fn reciprocal(normalized: u64) -> u64 {
    let index = ((normalized >> 55) & 0xFF) as usize;
    let seed = RECIPROCAL_SEEDS.get(index).copied().unwrap_or(u16::MAX);
    let mut estimate = u64::from(seed) << 48;
    for _ in 0..3 {
        // x * y with 63 fractional bits, then (2 - x * y) with 63 fractional bits.
        let product = ((u128::from(normalized) * u128::from(estimate)) >> 64) as u64;
        let correction = (1_u128 << 64) - u128::from(product);
        let next = (u128::from(estimate) * correction) >> 63;
        estimate = u64::try_from(next).unwrap_or(u64::MAX);
    }
    estimate
}

/// Builds [`RECIPROCAL_SEEDS`] at compile time.
#[allow(clippy::cast_possible_truncation)]
const fn reciprocal_seeds() -> [u16; 256] {
    let mut seeds = [0_u16; 256];
    let mut rest: &mut [u16] = &mut seeds;
    let mut i = 0_u32;
    while let Some((slot, tail)) = rest.split_first_mut() {
        let denominator = 513 + 2 * i;
        let seed = ((1_u32 << 25) + denominator / 2) / denominator;
        *slot = if seed > u16::MAX as u32 { u16::MAX } else { seed as u16 };
        rest = tail;
        i += 1;
    }
    seeds
}
