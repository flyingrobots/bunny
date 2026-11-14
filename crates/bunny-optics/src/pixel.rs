//! Pixel values: sample accumulation, tone curves and 8-bit and 16-bit output codes.
//!
//! Everything here is a pure integer function of its arguments. Callers own every buffer: the
//! accumulator is a value, and [`write_rgba8`] writes into a caller-provided slice. See the
//! Pixel Values sections of `docs/topics/optics/README.md`.

mod accumulate;
mod srgb;
// The knot table is `pub` inside this private module, as `clippy::redundant_pub_crate` asks;
// the workspace's `unreachable_pub` lint disagrees, and is allowed here.
#[allow(unreachable_pub)]
mod srgb_knots;
mod tone;

pub use accumulate::{PixelAccumulator, MAX_SAMPLES_PER_PIXEL, MAX_SAMPLE_RADIANCE_RAW};
pub use tone::{
    quantize_16, quantize_8, write_rgba8, ToneCurve, RGBA8_OPAQUE_IDENTITY, UNORM16_IDENTITY,
    UNORM8_IDENTITY,
};

use core::fmt;

/// Why a pixel value could not be accumulated, resolved or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelError {
    /// A sample channel is below zero.
    NegativeRadiance,
    /// A sample channel exceeds [`MAX_SAMPLE_RADIANCE_RAW`].
    RadianceTooLarge,
    /// The pixel would hold more than [`MAX_SAMPLES_PER_PIXEL`] samples.
    TooManySamples,
    /// The pixel holds no samples, so it has no mean.
    NoSamples,
    /// The output slice does not have the required length.
    WrongSliceLength {
        /// The length the function writes.
        expected: usize,
        /// The length of the slice it was given.
        actual: usize,
    },
}

impl fmt::Display for PixelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeRadiance => f.write_str("sample radiance is negative"),
            Self::RadianceTooLarge => f.write_str("sample radiance exceeds the accepted maximum"),
            Self::TooManySamples => f.write_str("pixel would exceed the accepted sample count"),
            Self::NoSamples => f.write_str("pixel holds no samples"),
            Self::WrongSliceLength { expected, actual } => {
                write!(f, "output slice has length {actual}; expected {expected}")
            }
        }
    }
}

impl std::error::Error for PixelError {}

/// `numerator / denominator` rounded to nearest, ties to even, for `numerator >= 0` and
/// `denominator > 0` (every caller passes values in that domain).
///
/// The remainder is below the denominator, and every denominator here is at most `2^32`, so
/// doubling it cannot overflow.
#[inline]
const fn div_round_half_even(numerator: i64, denominator: i64) -> i64 {
    let (quotient, remainder) = (numerator / denominator, numerator % denominator);
    let twice = remainder * 2;
    if twice > denominator || (twice == denominator && quotient & 1 == 1) {
        quotient + 1
    } else {
        quotient
    }
}
