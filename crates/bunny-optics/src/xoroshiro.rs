//! `Xoroshiro128Plus`: a stateful sequential generator, bit for bit compatible with Echo's
//! `warp-math` `Prng`.

use bunny_num::FixedQ32_32;

use crate::hash::{mix64, GAMMA};

/// The state that replaces an all-zero seed, which `xoroshiro128+` would never leave.
const ZERO_SEED_REPLACEMENT: u64 = GAMMA;

/// The `xoroshiro128+` generator of Blackman and Vigna (2016 parameters `a = 55`, `b = 14`,
/// `c = 36`), seeded as Echo's `warp-math` `Prng` seeds it.
///
/// This is a sequential generator: each value depends on how many values were drawn before
/// it. It is for code that consumes random numbers in one fixed order, such as a simulation
/// timeline. Rendering, where values must not depend on evaluation order, uses
/// [`crate::PixelStream`] and [`crate::PixelSampler`]. Not cryptographically secure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Xoroshiro128Plus {
    state: [u64; 2],
}

impl Xoroshiro128Plus {
    /// A generator with state `[seed0, seed1]`; an all-zero seed becomes
    /// `[0x9E37_79B9_7F4A_7C15, 0]`.
    #[must_use]
    pub const fn from_seed(seed0: u64, seed1: u64) -> Self {
        if seed0 == 0 && seed1 == 0 {
            Self { state: [ZERO_SEED_REPLACEMENT, 0] }
        } else {
            Self { state: [seed0, seed1] }
        }
    }

    /// A generator whose state is the first two `SplitMix64` outputs for the state `seed`.
    #[must_use]
    pub const fn from_seed_u64(seed: u64) -> Self {
        let first = seed.wrapping_add(GAMMA);
        let second = first.wrapping_add(GAMMA);
        Self::from_seed(mix64(first), mix64(second))
    }

    /// The current state, for checkpointing; [`Self::from_seed`] restores any state that is not
    /// all zero.
    #[must_use]
    pub const fn state(&self) -> [u64; 2] {
        self.state
    }

    /// The next 64-bit output, `s0 + s1` (wrapping), and one state step.
    pub const fn next_u64(&mut self) -> u64 {
        let [s0, s1] = self.state;
        let result = s0.wrapping_add(s1);
        let s1 = s1 ^ s0;
        self.state = [s0.rotate_left(55) ^ s1 ^ (s1 << 14), s1.rotate_left(36)];
        result
    }

    /// The next fraction in `[0, 1)`: the top 23 bits of [`Self::next_u64`] times `2^-23`,
    /// exactly the value Echo's `Prng::next_f32` returns, as Q32.32.
    pub const fn next_unit(&mut self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(((self.next_u64() >> 41) << 9).cast_signed())
    }

    /// The next integer in `min..=max`, or `None` (drawing nothing) when `min > max`.
    ///
    /// A one-value range returns `min` without drawing. A power-of-two span keeps the low bits
    /// of one draw; any other span rejects draws at or above the largest multiple of the span
    /// below `2^64` and reduces the first accepted draw modulo the span, as Echo does.
    pub fn next_int(&mut self, min: i32, max: i32) -> Option<i32> {
        let span = u64::try_from(i64::from(max) - i64::from(min)).ok()? + 1;
        let offset = match span {
            1 => 0,
            _ if span.is_power_of_two() => self.next_u64() & (span - 1),
            _ => self.below(span),
        };
        i32::try_from(i64::from(min) + i64::try_from(offset).ok()?).ok()
    }

    /// A uniform draw in `0..span` by rejection, for a span that is not a power of two.
    const fn below(&mut self, span: u64) -> u64 {
        let bound = u64::MAX - u64::MAX % span;
        loop {
            let candidate = self.next_u64();
            if candidate < bound {
                return candidate % span;
            }
        }
    }
}
