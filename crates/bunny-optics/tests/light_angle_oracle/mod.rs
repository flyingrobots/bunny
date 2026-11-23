//! An independent oracle for a light direction built from two angles: the exact direction
//! `(cos e cos a, sin e, cos e sin a)` and that direction scaled so its largest component
//! magnitude is one.
//!
//! Values are signed fixed point with 124 fractional bits in an `i128`. `pi` comes from
//! Machin's formula and sine and cosine from their Taylor series on the angle within its
//! quadrant, so the oracle shares no algorithm, constant or helper with the kernel, which reads
//! a compile-time table with cubic Hermite interpolation (`FixedAngle::sin_cos`) and rounds
//! with its own helpers. Each oracle value is within `2^-115` of the exact real number, far
//! below the margins the tests check.

// Every narrowing here is of a value proven to fit: magnitudes below 2^126 into `i128`, and
// the low 64 bits of a turn into `u128` limbs.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

/// Fractional bits of the oracle's fixed point.
const FRAC: u32 = 124;
const ONE: u128 = 1 << FRAC;
const LOW_64: u128 = (1 << 64) - 1;

/// Fractional bits of a scaled direction component: Q32.32 raw units times `2^32`.
pub(crate) const SCALED_BITS: u32 = 64;

/// `floor(n * 2^bits / d)` by binary long division, for `0 < d < 2^126`, `n < 2^126` and a
/// result below `2^128`.
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

/// The signed product of two oracle values, the magnitude floored.
fn signed_mul(a: i128, b: i128) -> i128 {
    let magnitude = mul(a.unsigned_abs(), b.unsigned_abs()) as i128;
    if (a < 0) == (b < 0) {
        magnitude
    } else {
        -magnitude
    }
}

/// `atan(1 / k)` for an integer `k >= 2` by its Taylor series.
fn atan_inverse(k: u128) -> u128 {
    let (mut power, mut positive, mut negative, mut n) = (ONE / k, 0_u128, 0_u128, 0_u128);
    while power != 0 {
        let term = power / (2 * n + 1);
        if n % 2 == 0 {
            positive += term;
        } else {
            negative += term;
        }
        power /= k * k;
        n += 1;
    }
    positive - negative
}

/// `(sin x, cos x)` for `0 <= x <= pi / 2` by their Taylor series.
fn sin_cos_series(x: u128) -> (u128, u128) {
    let x2 = mul(x, x);
    let series = |first: u128, offset: u128| {
        let (mut term, mut positive, mut negative, mut n) = (first, 0_u128, 0_u128, 0_u128);
        while term != 0 {
            if n % 2 == 0 {
                positive += term;
            } else {
                negative += term;
            }
            let k = 2 * n + offset;
            term = mul(term, x2) / ((k + 1) * (k + 2));
            n += 1;
        }
        positive - negative
    };
    (series(x, 1), series(ONE, 0))
}

/// The exact trigonometry the tests compare against.
pub(crate) struct Oracle {
    half_pi: u128,
}

impl Oracle {
    /// Derives `pi / 2` by Machin's formula, `pi = 16 atan(1/5) - 4 atan(1/239)`.
    pub(crate) fn new() -> Self {
        let pi = 16 * atan_inverse(5) - 4 * atan_inverse(239);
        Self { half_pi: pi / 2 }
    }

    /// `(sin, cos)` of the angle `raw / 2^64` turn, in units of `2^-124`.
    pub(crate) fn sin_cos(&self, raw: u64) -> (i128, i128) {
        let quadrant = raw >> 62;
        // The position within the quadrant as a fraction of it, then in radians.
        let fraction = u128::from(raw & ((1 << 62) - 1)) << (FRAC - 62);
        let (s, c) = sin_cos_series(mul(fraction, self.half_pi));
        let (s, c) = (s as i128, c as i128);
        match quadrant {
            0 => (s, c),
            1 => (c, -s),
            2 => (-s, -c),
            _ => (-c, s),
        }
    }

    /// The exact unit direction `(cos e cos a, sin e, cos e sin a)`, in units of `2^-124`.
    pub(crate) fn direction(&self, azimuth: u64, elevation: u64) -> [i128; 3] {
        let (sin_a, cos_a) = self.sin_cos(azimuth);
        let (sin_e, cos_e) = self.sin_cos(elevation);
        [signed_mul(cos_e, cos_a), sin_e, signed_mul(cos_e, sin_a)]
    }

    /// The exact direction divided by its largest component magnitude, each component in units
    /// of `2^-64` (raw Q32.32 units times `2^32`), the magnitude floored.
    pub(crate) fn scaled(&self, azimuth: u64, elevation: u64) -> [i128; 3] {
        let d = self.direction(azimuth, elevation);
        let largest = d.iter().map(|c| c.unsigned_abs()).max().unwrap_or(1);
        d.map(|c| {
            let magnitude = ratio(c.unsigned_abs(), largest, SCALED_BITS) as i128;
            if c < 0 {
                -magnitude
            } else {
                magnitude
            }
        })
    }
}
