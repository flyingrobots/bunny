//! The counter-based sample hash `bunny-sample-hash/1`.
//!
//! A pure function from (seed, pixel x, pixel y, sample index, dimension) to 64 bits, built
//! from the `SplitMix64` finalizer. It has no state to advance, so a value does not depend on
//! which other values were computed first, or on how many threads computed them.

use bunny_num::FixedQ32_32;

/// The versioned name of [`PixelStream::bits`]. Any change to its output is a new version.
pub const SAMPLE_HASH_ID: &str = "bunny-sample-hash/1";

/// The `SplitMix64` increment, `2^64` divided by the golden ratio, rounded to odd.
pub const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// The starting state of the sample hash. Zero makes its first absorption equal the first
/// `SplitMix64` output for the state `seed`.
const HASH_DOMAIN: u64 = 0;

/// The `SplitMix64` output function (Stafford's "Mix13" variant of the `MurmurHash3`
/// finalizer), a bijection of 64-bit words.
pub const fn mix64(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One absorption step: `mix64((state ^ word) + GAMMA)`, a bijection of `word` for a fixed
/// `state` and of `state` for a fixed `word`.
pub const fn absorb(state: u64, word: u64) -> u64 {
    mix64((state ^ word).wrapping_add(GAMMA))
}

/// Two 32-bit fields in one word: `low` in bits 0..32 and `high` in bits 32..64.
pub const fn pack(low: u32, high: u32) -> u64 {
    (low as u64) | ((high as u64) << 32)
}

/// The state after absorbing the seed and the pixel, starting from `domain`.
pub const fn pixel_state(domain: u64, seed: u64, x: u32, y: u32) -> u64 {
    absorb(absorb(domain, seed), pack(x, y))
}

/// The counter-based random stream of one pixel: [`PixelStream::bits`] for any sample index
/// and dimension, in any order.
///
/// The value for `(seed, x, y, sample, dimension)` is
///
/// ```text
/// absorb(absorb(absorb(0, seed), x | y << 32), sample | dimension << 32)
/// absorb(h, w) = mix64((h ^ w) + GAMMA)          // wrapping 64-bit arithmetic
/// ```
///
/// with `mix64` the `SplitMix64` finalizer. Holding the pixel's state saves two of the three
/// absorptions per value. See `docs/topics/optics/README.md#sample-hash`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelStream {
    state: u64,
}

impl PixelStream {
    /// The stream of pixel `(x, y)` under `seed`.
    #[must_use]
    pub const fn new(seed: u64, x: u32, y: u32) -> Self {
        Self { state: pixel_state(HASH_DOMAIN, seed, x, y) }
    }

    /// 64 mixed bits for `sample` and `dimension`.
    #[must_use]
    pub const fn bits(self, sample: u32, dimension: u32) -> u64 {
        absorb(self.state, pack(sample, dimension))
    }

    /// The top 32 bits of [`Self::bits`] as a fraction in `[0, 1)`; see [`fraction_from_bits`].
    #[must_use]
    pub const fn fraction(self, sample: u32, dimension: u32) -> FixedQ32_32 {
        fraction_from_bits(self.bits(sample, dimension))
    }
}

/// The top 32 bits of `bits` as a Q32.32 fraction: raw value `bits >> 32`.
///
/// The raw value is at most `2^32 - 1`, so the fraction is in `[0, 1 - 2^-32]` and never 1.0.
/// Every value is a multiple of `2^-32`, and each is hit by exactly `2^32` inputs.
#[must_use]
pub const fn fraction_from_bits(bits: u64) -> FixedQ32_32 {
    FixedQ32_32::from_raw((bits >> 32).cast_signed())
}
