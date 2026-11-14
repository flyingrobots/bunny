//! The pixel sampler `bunny-sobol-owen/1`: the first two dimensions of the Sobol sequence,
//! with a hash-based Owen scramble of the sample index and of each coordinate.

use bunny_num::FixedQ32_32;

use crate::hash::{absorb, mix64, pixel_state, GAMMA};
use crate::PixelSample;

/// The versioned name of [`PixelSampler::point`]. Any change to its output is a new version.
pub const SAMPLER_ID: &str = "bunny-sobol-owen/1";

/// The sampler's starting state, the ASCII bytes `SOBOL_O1`. It differs from the sample hash's
/// (zero), so the sampler and the hash never share a state for the same seed.
const SAMPLER_DOMAIN: u64 = 0x534F_424F_4C5F_4F31;

/// A stratified, decorrelated sampler of points in the unit square for one pixel.
///
/// [`Self::point`] maps a sample index and a dimension pair to a point in `[0, 1)^2`. For every
/// pair and every `k`, the points of samples `m * 2^k .. (m + 1) * 2^k` form a (0, k, 2)-net in
/// base 2. Pair 0 is the sub-pixel offset ([`Self::camera_sample`]); an integrator draws its
/// further decisions from pairs 1, 2, and so on. See `docs/topics/optics/README.md#pixel-sampler`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelSampler {
    x: u32,
    y: u32,
    state: u64,
}

impl PixelSampler {
    /// The sampler of pixel `(x, y)` under `seed`.
    #[must_use]
    pub const fn new(seed: u64, x: u32, y: u32) -> Self {
        Self { x, y, state: pixel_state(SAMPLER_DOMAIN, seed, x, y) }
    }

    /// The point `[u, v]` of `sample` in dimension pair `pair`, each coordinate in `[0, 1)`.
    #[must_use]
    pub const fn point(self, sample: u32, pair: u32) -> [FixedQ32_32; 2] {
        let first = absorb(self.state, pair as u64);
        let second = mix64(first.wrapping_add(GAMMA));
        let index = owen(sample, low(first));
        let u = owen(index.reverse_bits(), high(first));
        let v = owen(sobol_second(index), high(second));
        [fraction(u), fraction(v)]
    }

    /// Pair 0 of `sample` as the sub-pixel offset of this sampler's pixel, ready for
    /// [`crate::PinholeCamera::ray`]: `u` is `sub_x` (rightward), `v` is `sub_y` (downward).
    #[must_use]
    pub const fn camera_sample(self, sample: u32) -> PixelSample {
        let [sub_x, sub_y] = self.point(sample, 0);
        PixelSample::new(self.x, self.y, sub_x, sub_y)
    }

    /// The pixel column.
    #[must_use]
    pub const fn x(self) -> u32 {
        self.x
    }

    /// The pixel row.
    #[must_use]
    pub const fn y(self) -> u32 {
        self.y
    }
}

/// The low 32 bits.
#[allow(clippy::cast_possible_truncation)] // The mask leaves 32 bits; `try_from` is not const.
const fn low(word: u64) -> u32 {
    (word & 0xFFFF_FFFF) as u32
}

/// The high 32 bits.
#[allow(clippy::cast_possible_truncation)] // The shift leaves 32 bits; `try_from` is not const.
const fn high(word: u64) -> u32 {
    (word >> 32) as u32
}

const fn fraction(raw: u32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw as i64)
}

/// The Laine-Karras style permutation with Burley's constants. Every step leaves bit `i` equal
/// to input bit `i` xor a function of the bits below it: the add carries upward, and `x * c`
/// with `c` even depends at bit `i` only on bits below `i`.
const fn laine_karras(mut x: u32, seed: u32) -> u32 {
    x = x.wrapping_add(seed);
    x ^= x.wrapping_mul(0x6C50_B47C);
    x ^= x.wrapping_mul(0xB82F_1E52);
    x ^= x.wrapping_mul(0xC7AF_E638);
    x ^= x.wrapping_mul(0x8D22_F6E6);
    x
}

/// A nested uniform (Owen) scramble of a 32-bit fraction: each bit is flipped by a function of
/// the more significant bits only, so every dyadic interval maps onto a dyadic interval of the
/// same length.
const fn owen(value: u32, seed: u32) -> u32 {
    laine_karras(value.reverse_bits(), seed).reverse_bits()
}

/// The second Sobol dimension (primitive polynomial `x + 1`): the xor of the direction numbers
/// `V_b` for each set bit `b` of `index`, with `V_0 = 2^31` and `V_(b+1) = V_b ^ (V_b >> 1)`.
const fn sobol_second(mut index: u32) -> u32 {
    let mut result = 0;
    let mut direction = 1_u32 << 31;
    while index != 0 {
        if index & 1 == 1 {
            result ^= direction;
        }
        index >>= 1;
        direction ^= direction >> 1;
    }
    result
}
