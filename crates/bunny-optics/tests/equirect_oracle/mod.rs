//! An independent interval oracle for `bunny-environment-equirect/1`.
//!
//! Values are unsigned fixed point with 124 fractional bits in a `u128`. `pi` comes from
//! Machin's formula, `atan` from its Taylor series after one argument reduction, and `sin` and
//! `cos` from their Taylor series after an octant reduction. The horizontal length
//! `sqrt(x^2 + z^2)` is bracketed by a bit-by-bit integer square root of `(x^2 + z^2) 4^j` in
//! big integers, with `j` chosen from the largest component (not the kernel's rule), so the
//! polar angle is returned as an interval that contains the exact one. Nothing here is shared
//! with the kernel, which calls `bunny-num`'s CORDIC `atan2` and table `sin_cos`. Each value
//! is within a few units of `2^-118` of the exact real number: far below the margins checked.

use std::cmp::Ordering;

use crate::exact::Big;

/// Fractional bits of the oracle's fixed point.
const FRAC: u32 = 124;
const ONE: u128 = 1 << FRAC;
const LOW_64: u128 = (1 << 64) - 1;

/// Angles are returned in units of `2^-TURN_BITS` turn.
pub(crate) const TURN_BITS: u32 = 96;

/// The oracle's own error in turn units, an over-estimate.
pub(crate) const SLACK: u128 = 4;

/// `floor(n * 2^bits / d)` by binary long division, for `0 < d < 2^127` and a result below
/// `2^128`.
fn ratio(n: u128, d: u128, bits: u32) -> u128 {
    let mut quotient = n / d;
    let mut remainder = n % d;
    for _ in 0..bits {
        remainder <<= 1;
        quotient <<= 1;
        if remainder >= d {
            remainder -= d;
            quotient |= 1;
        }
    }
    quotient
}

/// `floor(a * b / 2^124)` for `a, b < 2^126`, from four 64-bit partial products.
fn mul(a: u128, b: u128) -> u128 {
    let (a1, a0, b1, b0) = (a >> 64, a & LOW_64, b >> 64, b & LOW_64);
    let low = a0 * b0;
    let middle = a1 * b0 + a0 * b1 + (low >> 64);
    ((a1 * b1) << (128 - FRAC)) + (middle >> (FRAC - 64))
}

/// `atan(z)` for `0 <= z <= 1/2` by its Taylor series.
fn atan_series(z: u128) -> u128 {
    let z2 = mul(z, z);
    let (mut power, mut positive, mut negative, mut k) = (z, 0_u128, 0_u128, 0_u128);
    while power != 0 {
        let term = power / (2 * k + 1);
        if k % 2 == 0 {
            positive += term;
        } else {
            negative += term;
        }
        power = mul(power, z2);
        k += 1;
    }
    positive - negative
}

/// `(sin x, cos x)` for `0 <= x <= pi/4` by their Taylor series.
fn sin_cos_series(x: u128) -> (u128, u128) {
    let x2 = mul(x, x);
    let (mut sin_term, mut cos_term) = (x, ONE);
    let (mut sin, mut cos) = (0_i128, 0_i128);
    let mut n = 0_u128;
    while sin_term != 0 || cos_term != 0 {
        let sign = if n % 2 == 0 { 1 } else { -1 };
        sin += sign * i128::try_from(sin_term).expect("a term below 2^124");
        cos += sign * i128::try_from(cos_term).expect("a term below 2^124");
        sin_term = mul(sin_term, x2) / ((2 * n + 2) * (2 * n + 3));
        cos_term = mul(cos_term, x2) / ((2 * n + 1) * (2 * n + 2));
        n += 1;
    }
    let unsigned = |value: i128| u128::try_from(value).expect("sin and cos are positive here");
    (unsigned(sin), unsigned(cos))
}

/// The largest `r` with `r^2 <= n`, for `n < 2^250`.
fn isqrt(n: &Big) -> u128 {
    let mut root = 0_u128;
    for bit in (0..125).rev() {
        let candidate = root | (1 << bit);
        if Big::new(candidate).mul(&Big::new(candidate)).compare(n) != Ordering::Greater {
            root = candidate;
        }
    }
    root
}

/// The number of significant bits of `value`.
fn bits(value: u128) -> u32 {
    128 - value.leading_zeros()
}

/// A signed magnitude: `(negative, |value|)`.
pub(crate) type SignedMagnitude = (bool, u128);

/// Arbitrary-precision constants the oracle derives once.
pub(crate) struct Oracle {
    pi: u128,
}

impl Oracle {
    /// Derives `pi` by Machin's formula, `16 atan(1/5) - 4 atan(1/239)`.
    pub(crate) fn new() -> Self {
        let pi = 16 * atan_series(ratio(1, 5, FRAC)) - 4 * atan_series(ratio(1, 239, FRAC));
        Self { pi }
    }

    /// `atan(n / d)` for `0 <= n <= d`, `0 < d < 2^127`.
    fn atan(&self, n: u128, d: u128) -> u128 {
        let z = ratio(n, d, FRAC);
        if z <= ONE / 2 {
            return atan_series(z);
        }
        self.pi / 4 - atan_series(ratio(ONE - z, ONE + z, FRAC))
    }

    /// A radian angle below `2 pi` in turn units.
    fn turn_units(&self, angle: u128) -> u128 {
        ratio(angle, 2 * self.pi, TURN_BITS)
    }

    /// The angle of `(x, y)` counter-clockwise from `+x`, in turn units in `[0, 2^96)`, for
    /// magnitudes below `2^126`; `None` at the origin.
    pub(crate) fn atan2(&self, y: SignedMagnitude, x: SignedMagnitude) -> Option<u128> {
        let ((y_negative, ay), (x_negative, ax)) = (y, x);
        if ax == 0 && ay == 0 {
            return None;
        }
        let full = 1_u128 << TURN_BITS;
        let first_quadrant = if ay <= ax {
            self.turn_units(self.atan(ay, ax))
        } else {
            full / 4 - self.turn_units(self.atan(ax, ay))
        };
        let angle = match (x_negative && ax != 0, y_negative && ay != 0) {
            (false, false) => first_quadrant,
            (true, false) => full / 2 - first_quadrant,
            (true, true) => full / 2 + first_quadrant,
            (false, true) => (full - first_quadrant) % full,
        };
        Some(angle)
    }

    /// The longitude `atan2(-x, z)` of a direction in turn units; `None` when `x = z = 0`.
    pub(crate) fn longitude(&self, x: i64, z: i64) -> Option<u128> {
        let minus_x = (x > 0, u128::from(x.unsigned_abs()));
        self.atan2(minus_x, (z < 0, u128::from(z.unsigned_abs())))
    }

    /// An interval `[low, high]` of turn units that contains the exact polar angle
    /// `atan2(sqrt(x^2 + z^2), y)` of a nonzero direction, measured from `+y`.
    pub(crate) fn polar_interval(&self, [x, y, z]: [i64; 3]) -> (u128, u128) {
        let largest = [x, y, z].map(i64::unsigned_abs).into_iter().max().expect("three");
        let j = 120 - bits(u128::from(largest));
        let scale = Big::new(1 << j);
        let square = |c: i64| Big::new(u128::from(c.unsigned_abs()).pow(2));
        let scaled = square(x).add(&square(z)).mul(&scale).mul(&scale);
        let root = isqrt(&scaled);
        let vertical = (y < 0, u128::from(y.unsigned_abs()) << j);
        let at = |r: u128| self.atan2((false, r), vertical).expect("y or r is nonzero");
        let (a, b) = (at(root), at(root + 1));
        (a.min(b), a.max(b))
    }

    /// `(sin, cos)` of `num / 2^bits` turn, in units of `2^-124`, for `2 <= bits <= 120`.
    pub(crate) fn sin_cos_turns(&self, num: u128, bits: u32) -> (i128, i128) {
        let full = 1_u128 << bits;
        let quarter = full / 4;
        let t = num % full;
        let (q, w) = (t / quarter, t % quarter);
        let radians = |w: u128| mul(self.pi, (w << (FRAC - bits)) * 2);
        let (s, c) = if w <= quarter / 2 {
            sin_cos_series(radians(w))
        } else {
            let (s, c) = sin_cos_series(radians(quarter - w));
            (c, s)
        };
        let (s, c) = (signed(s), signed(c));
        match q {
            0 => (s, c),
            1 => (c, -s),
            2 => (-s, -c),
            _ => (-c, s),
        }
    }
}

fn signed(value: u128) -> i128 {
    i128::try_from(value).expect("at most 2^124")
}

/// `a b / 2^124` for signed values of magnitude at most `2^124`, truncated toward zero.
pub(crate) fn product(a: i128, b: i128) -> i128 {
    let magnitude = signed(mul(a.unsigned_abs(), b.unsigned_abs()));
    if (a < 0) == (b < 0) {
        magnitude
    } else {
        -magnitude
    }
}
