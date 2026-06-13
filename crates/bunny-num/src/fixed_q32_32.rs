//! Deterministic helpers for the Q32.32 fixed-point encoding.
//!
//! The representation is an `i64` storing an integer scaled by `2^32`:
//! `real_value = raw / 2^32`.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Number of fractional bits in the Q32.32 fixed-point encoding.
pub const FRAC_BITS: u32 = 32;

/// The raw integer value corresponding to `1.0` in Q32.32.
pub const ONE_RAW: i64 = 1_i64 << FRAC_BITS;

/// Type-safe newtype wrapper representing a Q32.32 fixed-point value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedQ32_32(pub i64);

impl FixedQ32_32 {
    /// The constant representation of `0.0`.
    pub const ZERO: Self = Self(0);

    /// The constant representation of `1.0`.
    pub const ONE: Self = Self(ONE_RAW);

    /// Creates a `FixedQ32_32` from a raw `i64` bit pattern.
    #[must_use]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Retrieves the underlying raw `i64` representation.
    #[must_use]
    pub const fn to_raw(self) -> i64 {
        self.0
    }

    /// Converts a native `f32` into `FixedQ32_32`.
    #[must_use]
    pub fn from_f32(value: f32) -> Self {
        Self(from_f32(value))
    }

    /// Converts `FixedQ32_32` into a native `f32`.
    #[must_use]
    pub fn to_f32(self) -> f32 {
        to_f32(self.0)
    }
}

impl Add for FixedQ32_32 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let sum = i128::from(self.0) + i128::from(rhs.0);
        Self(saturate_i128_to_i64(sum))
    }
}

impl AddAssign for FixedQ32_32 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedQ32_32 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let diff = i128::from(self.0) - i128::from(rhs.0);
        Self(saturate_i128_to_i64(diff))
    }
}

impl SubAssign for FixedQ32_32 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for FixedQ32_32 {
    type Output = Self;

    fn neg(self) -> Self {
        if self.0 == i64::MIN {
            Self(i64::MAX)
        } else {
            Self(-self.0)
        }
    }
}

impl Mul for FixedQ32_32 {
    type Output = Self;

    #[allow(clippy::cast_possible_wrap)]
    fn mul(self, rhs: Self) -> Self {
        let prod = i128::from(self.0) * i128::from(rhs.0);
        let rounded = round_shift_right_u128(prod.unsigned_abs(), FRAC_BITS);
        let signed_prod = if (self.0 < 0) ^ (rhs.0 < 0) {
            -(rounded as i128)
        } else {
            rounded as i128
        };
        Self(saturate_i128_to_i64(signed_prod))
    }
}

impl MulAssign for FixedQ32_32 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for FixedQ32_32 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            if self.0 == 0 {
                return Self(0);
            }
            return if self.0.is_negative() {
                Self(i64::MIN)
            } else {
                Self(i64::MAX)
            };
        }

        let numer = i128::from(self.0) << FRAC_BITS;
        let denom = i128::from(rhs.0);
        let abs_numer = numer.abs();
        let abs_denom = denom.abs();
        let abs_q = abs_numer / abs_denom;
        let abs_r = abs_numer % abs_denom;

        let double_r = abs_r << 1;
        let final_abs_q = if double_r > abs_denom {
            abs_q + 1
        } else if double_r < abs_denom {
            abs_q
        } else if (abs_q & 1) == 1 {
            abs_q + 1
        } else {
            abs_q
        };

        let signed_q = if (self.0 < 0) ^ (rhs.0 < 0) {
            -final_abs_q
        } else {
            final_abs_q
        };

        Self(saturate_i128_to_i64(signed_q))
    }
}

impl DivAssign for FixedQ32_32 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

const fn round_shift_right_u128(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        return value;
    }
    if shift >= 128 {
        return 0;
    }

    let q = value >> shift;
    let mask = (1_u128 << shift) - 1;
    let r = value & mask;
    let half = 1_u128 << (shift - 1);

    if r > half {
        q + 1
    } else if r < half {
        q
    } else if (q & 1) == 1 {
        q + 1
    } else {
        q
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn saturate_i128_to_i64(value: i128) -> i64 {
    if value < i64::MIN as i128 {
        i64::MIN
    } else if value > i64::MAX as i128 {
        i64::MAX
    } else {
        value as i64
    }
}

/// Deterministically converts an `f32` to a Q32.32 raw `i64`.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub fn from_f32(value: f32) -> i64 {
    if value.is_nan() {
        return 0;
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            i64::MAX
        } else {
            i64::MIN
        };
    }

    let bits = value.to_bits();
    let sign = (bits >> 31) != 0;
    let exp_u8 = ((bits >> 23) & 0xff) as u8;
    let exp = i32::from(exp_u8);
    let mant = bits & 0x007f_ffff;

    if exp == 0 && mant == 0 {
        return 0;
    }

    let mantissa: u64 = if exp == 0 {
        u64::from(mant)
    } else {
        u64::from((1_u32 << 23) | mant)
    };

    let unbiased = if exp == 0 { -126 } else { exp - 127 };
    let frac_i32 = FRAC_BITS as i32;
    let shift = unbiased + (frac_i32 - 23);

    let abs_raw: i128 = if shift >= 0 {
        let shift_u = shift.unsigned_abs();
        if shift_u > 103 {
            i128::MAX
        } else {
            i128::from(mantissa) << shift_u
        }
    } else {
        let rshift = shift.unsigned_abs();
        #[allow(clippy::cast_possible_truncation)]
        let rounded = round_shift_right_u128(u128::from(mantissa), rshift) as u64;
        i128::from(rounded)
    };

    let signed_raw = if sign { -abs_raw } else { abs_raw };
    saturate_i128_to_i64(signed_raw)
}

/// Deterministically converts a Q32.32 raw `i64` to an `f32`.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub fn to_f32(raw: i64) -> f32 {
    if raw == 0 {
        return 0.0;
    }

    let sign = raw.is_negative();
    let abs: u64 = raw.unsigned_abs();
    if abs == 0 {
        return 0.0;
    }

    let k = 63_u32.saturating_sub(abs.leading_zeros());
    let frac_i32 = FRAC_BITS as i32;
    let mut exp = (k as i32) - frac_i32;

    let mut sig: u128 = if k > 23 {
        let rshift = k - 23;
        round_shift_right_u128(u128::from(abs), rshift)
    } else {
        let lshift = 23 - k;
        u128::from(abs) << lshift
    };

    if sig >= (1_u128 << 24) {
        sig >>= 1;
        exp = exp.saturating_add(1);
    }

    let exp_field = (exp + 127) as u32;
    let mantissa = (sig & ((1_u128 << 23) - 1)) as u32;
    let bits = (u32::from(sign) << 31) | (exp_field << 23) | mantissa;
    f32::from_bits(bits)
}
