//! Canonical binary angles and table-based deterministic sine and cosine.
//!
//! A [`FixedAngle`] stores a fraction of one full turn in units of `2^-64` turn,
//! so `2^64` units make a turn. Wraparound is ordinary wrapping integer
//! arithmetic and range reduction is a bit mask: no `pi` constant and no
//! division is involved in evaluating `sin` or `cos`.
//!
//! Sine and cosine read a quarter-wave table of `SEGMENTS + 1` samples of
//! `sin` over `[0, pi/2]`, stored with 62 fractional bits and computed at
//! compile time with an integer Taylor series. Between samples the value is
//! cubic Hermite interpolated; the slope at each sample is `cos`, which is the
//! same table read backwards. Quadrant symmetry rebuilds the full wave.
//!
//! Positive angles turn counter-clockwise (the right-hand rule about `+Z`).

use crate::fixed_q32_32::ONE_RAW;
use crate::FixedQ32_32;

/// Quarter-wave segments in the sine table (`2^SEGMENT_BITS`).
const SEGMENT_BITS: u32 = 10;
const SEGMENTS: usize = 1 << SEGMENT_BITS;
/// Fractional bits of the table samples and the interpolation arithmetic.
const Q: u32 = 62;
const Q_ONE: i128 = 1 << Q;
/// Bits of a turn below the quadrant (the two top bits pick the quadrant).
const QUARTER_BITS: u32 = 62;
const QUARTER_MASK: u64 = (1 << QUARTER_BITS) - 1;
/// Bits of a quarter-turn position below the table index.
const SEGMENT_FRACTION_BITS: u32 = QUARTER_BITS - SEGMENT_BITS;
const SEGMENT_FRACTION_MASK: u64 = (1 << SEGMENT_FRACTION_BITS) - 1;
/// `round(pi / 2 * 2^62)`.
const HALF_PI_Q62: i128 = 7_244_019_458_077_122_842;
/// Width of one table segment in radians, with 62 fractional bits.
const SEGMENT_WIDTH_Q62: i128 = HALF_PI_Q62 / SEGMENTS as i128;
/// `round(2^96 / (2 * pi))`: turn units per radian, with 32 more fractional bits than Q32.32.
const TURN_UNITS_PER_RADIAN_Q64: u128 = 12_609_553_696_233_175_933_233_255_924;
/// `round(2^96 / 360)`: turn units per degree, with 32 more fractional bits than Q32.32.
const TURN_UNITS_PER_DEGREE_Q64: u128 = 220_078_229_206_289_826_648_733_195;

/// `sin(k * pi / 2 / SEGMENTS)` for `k` in `0..=SEGMENTS`, with 62 fractional bits.
static SIN_QUARTER_Q62: [i64; SEGMENTS + 1] = sin_quarter_table();

/// A canonical angle: a fraction of one full turn in units of `2^-64` turn.
///
/// Every `u64` is a valid angle and arithmetic wraps, so an angle never needs
/// normalizing. Positive angles turn counter-clockwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FixedAngle(u64);

impl FixedAngle {
    /// No rotation.
    pub const ZERO: Self = Self(0);
    /// A quarter turn (90 degrees).
    pub const QUARTER_TURN: Self = Self(1 << 62);
    /// A half turn (180 degrees).
    pub const HALF_TURN: Self = Self(1 << 63);

    /// Creates an angle from raw turn units (`2^64` units per turn).
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw turn units (`2^64` units per turn).
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Creates an angle from a number of turns, discarding whole turns exactly.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn from_turns(turns: FixedQ32_32) -> Self {
        // The Q32.32 fraction becomes the top 32 bits of the turn; two's complement makes
        // negative turns wrap the right way.
        Self((turns.raw() as u64) << 32)
    }

    /// Creates an angle from radians, rounded to the nearest turn unit.
    ///
    /// `from_radians(-x)` is exactly the negation of `from_radians(x)`.
    #[must_use]
    pub const fn from_radians(radians: FixedQ32_32) -> Self {
        Self(scale_to_turn_units(radians, TURN_UNITS_PER_RADIAN_Q64))
    }

    /// Creates an angle from degrees, rounded to the nearest turn unit.
    ///
    /// Whole multiples of 90 degrees convert exactly.
    #[must_use]
    pub const fn from_degrees(degrees: FixedQ32_32) -> Self {
        Self(scale_to_turn_units(degrees, TURN_UNITS_PER_DEGREE_Q64))
    }

    /// Adds two angles, wrapping at a full turn.
    #[must_use]
    pub const fn wrapping_add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }

    /// Subtracts `rhs` from `self`, wrapping at a full turn.
    #[must_use]
    pub const fn wrapping_sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }

    /// The same rotation in the opposite direction.
    #[must_use]
    pub const fn wrapping_neg(self) -> Self {
        Self(self.0.wrapping_neg())
    }

    /// Sine of the angle, within one raw Q32.32 unit of the exact value.
    #[must_use]
    pub fn sin(self) -> FixedQ32_32 {
        self.sin_cos().0
    }

    /// Cosine of the angle, within one raw Q32.32 unit of the exact value.
    #[must_use]
    pub fn cos(self) -> FixedQ32_32 {
        self.sin_cos().1
    }

    /// Sine and cosine of the angle, each within one raw Q32.32 unit of the exact value.
    ///
    /// Cardinal angles are exact (`sin` and `cos` are `0`, `1` or `-1`), and
    /// `sin(-a) == -sin(a)` and `cos(-a) == cos(a)` hold bit for bit.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn sin_cos(self) -> (FixedQ32_32, FixedQ32_32) {
        let position = self.0 & QUARTER_MASK;
        let sin = sin_quarter(position);
        let cos = sin_quarter((1 << QUARTER_BITS) - position);
        // The top two bits pick the quadrant.
        let (sin, cos) = match (self.0 >> QUARTER_BITS) as u8 {
            0 => (sin, cos),
            1 => (cos, -sin),
            2 => (-sin, -cos),
            _ => (-cos, sin),
        };
        (FixedQ32_32::from_raw(sin), FixedQ32_32::from_raw(cos))
    }
}

/// `value * factor_q64 / 2^64`, rounded half up on the magnitude, as wrapping turn units.
#[allow(clippy::cast_possible_truncation)]
const fn scale_to_turn_units(value: FixedQ32_32, factor_q64: u128) -> u64 {
    let magnitude = value.raw().unsigned_abs() as u128;
    let whole = magnitude.wrapping_mul(factor_q64 >> 64);
    let fraction = magnitude * (factor_q64 & (u64::MAX as u128));
    let units = whole.wrapping_add((fraction + (1 << 63)) >> 64) as u64;
    if value.raw() < 0 {
        units.wrapping_neg()
    } else {
        units
    }
}

/// Sine of a quarter-turn position in `[0, 2^62]` (`2^62` is a quarter turn), as raw Q32.32.
#[allow(clippy::cast_possible_truncation)]
fn sin_quarter(position: u64) -> i64 {
    let index = (position >> SEGMENT_FRACTION_BITS) as usize;
    let fraction = position & SEGMENT_FRACTION_MASK;
    let Some(&start) = SIN_QUARTER_Q62.get(index) else {
        return ONE_RAW; // position == 2^62: exactly a quarter turn
    };
    if fraction == 0 {
        return q62_to_q32(i128::from(start));
    }
    let end = SIN_QUARTER_Q62.get(index + 1).map_or(Q_ONE, |&v| i128::from(v));
    // The slope of sin is cos: the same table read from the other end.
    let start_slope = SIN_QUARTER_Q62.get(SEGMENTS - index).map_or(0, |&v| i128::from(v));
    let end_slope = SIN_QUARTER_Q62.get(SEGMENTS - index - 1).map_or(0, |&v| i128::from(v));
    let t = i128::from(fraction) << (Q - SEGMENT_FRACTION_BITS);
    let value = hermite(HermiteSegment { start, end, start_slope, end_slope }, t);
    q62_to_q32(value.clamp(0, Q_ONE))
}

/// Endpoint values and slopes (per radian) of one table segment, with 62 fractional bits.
#[derive(Clone, Copy)]
struct HermiteSegment {
    start: i64,
    end: i128,
    start_slope: i128,
    end_slope: i128,
}

/// Cubic Hermite interpolation at `t` in `[0, 1)` (62 fractional bits).
fn hermite(segment: HermiteSegment, t: i128) -> i128 {
    let t2 = mul_q62(t, t);
    let t3 = mul_q62(t2, t);
    let h00 = 2 * t3 - 3 * t2 + Q_ONE;
    let h10 = t3 - 2 * t2 + t;
    let h01 = 3 * t2 - 2 * t3;
    let h11 = t3 - t2;
    let start_tangent = mul_q62(SEGMENT_WIDTH_Q62, segment.start_slope);
    let end_tangent = mul_q62(SEGMENT_WIDTH_Q62, segment.end_slope);
    mul_q62(h00, i128::from(segment.start))
        + mul_q62(h10, start_tangent)
        + mul_q62(h01, segment.end)
        + mul_q62(h11, end_tangent)
}

/// Product of two values with 62 fractional bits (floor).
const fn mul_q62(a: i128, b: i128) -> i128 {
    (a * b) >> Q
}

/// Rounds a non-negative value with 62 fractional bits to raw Q32.32, ties to even.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn q62_to_q32(value: i128) -> i64 {
    let shift = Q - 32;
    let value = value as u128;
    let quotient = value >> shift;
    let remainder = value & ((1 << shift) - 1);
    let half = 1 << (shift - 1);
    let rounded = if remainder > half || (remainder == half && quotient & 1 == 1) {
        quotient + 1
    } else {
        quotient
    };
    rounded as i64
}

/// Builds the quarter-wave table with an integer Taylor series (no floating point).
const fn sin_quarter_table() -> [i64; SEGMENTS + 1] {
    let mut table = [0_i64; SEGMENTS + 1];
    let mut rest: &mut [i64] = &mut table;
    let mut k = 0;
    while let Some((slot, tail)) = rest.split_first_mut() {
        *slot = sin_quarter_sample(k);
        rest = tail;
        k += 1;
    }
    table
}

/// Sample `k` of the table: `sin(k * pi / 2 / SEGMENTS)` with 62 fractional bits.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
const fn sin_quarter_sample(k: usize) -> i64 {
    // Exact endpoints: sin(0) = 0 and sin(pi / 2) = 1.
    if k == 0 {
        return 0;
    }
    if k >= SEGMENTS {
        return Q_ONE as i64;
    }
    taylor_sin_q62(HALF_PI_Q62 * k as i128 / SEGMENTS as i128) as i64
}

/// `sin(x)` for `x` in `[0, pi/2]` by its Taylor series, with 62 fractional bits.
const fn taylor_sin_q62(x: i128) -> i128 {
    let x2 = mul_q62(x, x);
    let mut term = x;
    let mut sum = x;
    let mut n = 1_i128;
    while term != 0 {
        term = -mul_q62(term, x2) / ((n + 1) * (n + 2));
        sum += term;
        n += 2;
    }
    sum
}
