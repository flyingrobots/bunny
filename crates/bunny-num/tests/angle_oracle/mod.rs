//! An independent high-precision oracle for the angle conversions and `atan2`.
//!
//! Values are unsigned fixed point with 124 fractional bits in a `u128`. `pi` comes from
//! Machin's formula and `atan` from its Taylor series after one argument reduction, so the
//! oracle shares no algorithm, constant or helper with the kernel, which runs a CORDIC
//! iteration over a compile-time table and multiplies by a rounded `2 pi`. Each oracle
//! value is within `2^-110` of the exact real number, far below the margins the tests check.

/// Fractional bits of the oracle's fixed point.
const FRAC: u32 = 124;
const ONE: u128 = 1 << FRAC;
const LOW_64: u128 = (1 << 64) - 1;

/// Extra fractional bits the oracle keeps below an output's raw unit.
pub(crate) const EXTRA_BITS: u32 = 32;

/// `floor(n * 2^bits / d)` by binary long division, for `0 < d < 2^127` and a result
/// below `2^128`.
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

    /// `atan(n / d)` for `0 <= n <= d`, `d > 0`: the series, or `pi/4 - atan((1-z)/(1+z))`.
    fn atan(&self, n: u128, d: u128) -> u128 {
        let z = ratio(n, d, FRAC);
        if z <= ONE / 2 {
            return atan_series(z);
        }
        self.pi / 4 - atan_series(ratio(ONE - z, ONE + z, FRAC))
    }

    /// `angle / (2 pi)` in units of `2^-(64 + EXTRA_BITS)` turn, for `angle <= pi / 4`.
    fn turn_units(&self, angle: u128) -> u128 {
        ratio(angle, 2 * self.pi, 64 + EXTRA_BITS)
    }

    /// The angle of `(x, y)` counter-clockwise from `+x`, in units of `2^-(64 + EXTRA_BITS)`
    /// turn in `[0, 2^(64 + EXTRA_BITS))`; `None` at the origin.
    pub(crate) fn atan2(&self, y: i64, x: i64) -> Option<u128> {
        if x == 0 && y == 0 {
            return None;
        }
        let (ay, ax) = (u128::from(y.unsigned_abs()), u128::from(x.unsigned_abs()));
        let full = 1_u128 << (64 + EXTRA_BITS);
        let first_quadrant = if ay <= ax {
            self.turn_units(self.atan(ay, ax))
        } else {
            full / 4 - self.turn_units(self.atan(ax, ay))
        };
        let angle = match (x < 0, y < 0) {
            (false, false) => first_quadrant,
            (true, false) => full / 2 - first_quadrant,
            (true, true) => full / 2 + first_quadrant,
            (false, true) => (full - first_quadrant) % full,
        };
        Some(angle)
    }

    /// `raw * 2 pi / 2^32`, the exact radian value of a raw angle in raw Q32.32 units, scaled
    /// by `2^EXTRA_BITS` (floor).
    pub(crate) fn radians(&self, raw: u64) -> u128 {
        let tau = 2 * self.pi;
        let raw = u128::from(raw);
        let (high, low) = (raw * (tau >> 64), raw * (tau & LOW_64));
        // raw * tau / 2^(124 + 32 - EXTRA_BITS), with the low 64 bits of the product dropped.
        (high + (low >> 64)) >> (FRAC + 32 - EXTRA_BITS - 64)
    }
}
