//! Per-pixel sums of linear radiance samples.

use bunny_num::FixedQ32_32;

use super::{div_round_half_even, PixelError};

/// The largest accepted sample radiance per channel, raw Q32.32: `2^52` (1 048 576.0).
pub const MAX_SAMPLE_RADIANCE_RAW: i64 = 1 << 52;

/// The largest number of samples one pixel accepts (1 024).
pub const MAX_SAMPLES_PER_PIXEL: u32 = 1 << 10;

/// The running sum of the linear radiance samples of one pixel, per channel (red, green, blue).
///
/// Each channel sum is a raw `i64` of Q32.32 units. Every accepted sample channel is in
/// `0..=MAX_SAMPLE_RADIANCE_RAW` and a pixel holds at most [`MAX_SAMPLES_PER_PIXEL`] samples,
/// so every sum is at most `2^10 * 2^52 = 2^62` raw and no addition can overflow. Integer
/// addition is exact, so the sums do not depend on the order samples arrive in or on how they
/// are split between accumulators that are later merged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelAccumulator {
    sums: [i64; 3],
    count: u32,
}

impl PixelAccumulator {
    /// An accumulator holding no samples.
    pub const EMPTY: Self = Self { sums: [0; 3], count: 0 };

    /// An accumulator holding no samples.
    #[must_use]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// Adds one sample's linear radiance (red, green, blue).
    ///
    /// # Errors
    ///
    /// [`PixelError::TooManySamples`] when the pixel already holds
    /// [`MAX_SAMPLES_PER_PIXEL`] samples, then [`PixelError::NegativeRadiance`] or
    /// [`PixelError::RadianceTooLarge`] for the first channel outside
    /// `0..=MAX_SAMPLE_RADIANCE_RAW`. On error the accumulator is unchanged.
    pub fn add_sample(&mut self, radiance: [FixedQ32_32; 3]) -> Result<(), PixelError> {
        if self.count >= MAX_SAMPLES_PER_PIXEL {
            return Err(PixelError::TooManySamples);
        }
        let [r, g, b] = radiance.map(FixedQ32_32::raw);
        check_sample(r)?;
        check_sample(g)?;
        check_sample(b)?;
        self.include([r, g, b], 1);
        Ok(())
    }

    /// Adds every sample held by `other`, as if each had been added here.
    ///
    /// # Errors
    ///
    /// [`PixelError::TooManySamples`] when the two together hold more than
    /// [`MAX_SAMPLES_PER_PIXEL`] samples. On error the accumulator is unchanged.
    pub const fn merge(&mut self, other: &Self) -> Result<(), PixelError> {
        // Both counts are at most 2^10, so the sum cannot overflow.
        if self.count + other.count > MAX_SAMPLES_PER_PIXEL {
            return Err(PixelError::TooManySamples);
        }
        self.include(other.sums, other.count);
        Ok(())
    }

    /// The number of samples added.
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// The exact per-channel sums (red, green, blue), at most `2^62` raw each.
    #[must_use]
    pub fn sums(&self) -> [FixedQ32_32; 3] {
        self.sums.map(FixedQ32_32::from_raw)
    }

    /// The mean radiance per channel: each sum divided by the sample count, rounded to
    /// nearest with ties to even.
    ///
    /// For a power-of-two count this is the sum shifted right by `log2(count)` with that same
    /// rounding. The mean of accepted samples is at most [`MAX_SAMPLE_RADIANCE_RAW`].
    ///
    /// # Errors
    ///
    /// [`PixelError::NoSamples`] when no sample has been added.
    pub fn resolve(&self) -> Result<[FixedQ32_32; 3], PixelError> {
        if self.count == 0 {
            return Err(PixelError::NoSamples);
        }
        let count = i64::from(self.count);
        Ok(self.sums.map(|sum| FixedQ32_32::from_raw(div_round_half_even(sum, count))))
    }

    /// Adds already checked sums for `count` samples.
    ///
    /// Callers have checked that the new count is at most `2^10` and that each added sum is at
    /// most `count * 2^52`, so each new sum is at most `2^62`: no overflow.
    const fn include(&mut self, sums: [i64; 3], count: u32) {
        let [r, g, b] = self.sums;
        let [dr, dg, db] = sums;
        self.sums = [r + dr, g + dg, b + db];
        self.count += count;
    }
}

const fn check_sample(raw: i64) -> Result<(), PixelError> {
    if raw < 0 {
        Err(PixelError::NegativeRadiance)
    } else if raw > MAX_SAMPLE_RADIANCE_RAW {
        Err(PixelError::RadianceTooLarge)
    } else {
        Ok(())
    }
}
