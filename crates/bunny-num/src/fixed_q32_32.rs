//! Deterministic helpers for the Q32.32 fixed-point encoding.
//!
//! The representation is an `i64` storing an integer scaled by `2^32`:
//! `real_value = raw / 2^32`.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Floating point conversion algorithms.
pub mod conversions;

pub use conversions::{from_f32, to_f32, try_from_f32, FloatConversionError};

/// Number of fractional bits in the Q32.32 fixed-point encoding.
pub const FRAC_BITS: u32 = 32;

/// The raw integer value corresponding to `1.0` in Q32.32.
pub const ONE_RAW: i64 = 1_i64 << FRAC_BITS;

/// Type-safe newtype wrapper representing a Q32.32 fixed-point value.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedQ32_32(i64);

impl FixedQ32_32 {
    /// Number of fractional bits in this fixed-point representation.
    pub const FRACTIONAL_BITS: u32 = FRAC_BITS;

    /// Scaling factor for converting whole units into raw Q32.32 values.
    pub const SCALE: i128 = 1_i128 << Self::FRACTIONAL_BITS;

    /// Minimum representable raw Q32.32 value.
    pub const MIN_RAW: i64 = i64::MIN;

    /// Maximum representable raw Q32.32 value.
    pub const MAX_RAW: i64 = i64::MAX;

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
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Retrieves the underlying raw `i64` representation.
    ///
    /// This compatibility alias is equivalent to [`Self::raw`].
    #[must_use]
    pub const fn to_raw(self) -> i64 {
        self.raw()
    }

    /// Validates and converts a native `f32` into `FixedQ32_32`.
    ///
    /// # Errors
    /// Returns `FloatConversionError::NonFinite` for `NaN` or infinity and
    /// `FloatConversionError::OutOfRange` when the finite value cannot be
    /// represented as Q32.32 after deterministic rounding.
    // dojo: allow float-boundary -- explicit validated ingress conversion into canonical Q32.32
    pub fn try_from_f32(value: f32) -> Result<Self, FloatConversionError> {
        conversions::try_from_f32(value).map(Self)
    }

    /// Converts a native `f32` into `FixedQ32_32`.
    ///
    /// This is the saturating convenience path. Use [`Self::try_from_f32`] for
    /// validating external ingress before canonical geometry/math use.
    #[must_use]
    // dojo: allow float-boundary -- explicit ingress conversion into canonical Q32.32
    pub fn from_f32(value: f32) -> Self {
        Self(from_f32(value))
    }

    /// Converts `FixedQ32_32` into a native `f32`.
    #[must_use]
    // dojo: allow float-boundary -- explicit lossy display/API egress from Q32.32
    pub fn to_f32(self) -> f32 {
        to_f32(self.0)
    }

    /// Computes the square root of `FixedQ32_32` deterministically.
    ///
    /// Returns `None` if the value is negative.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub const fn sqrt(self) -> Option<Self> {
        if self.0 < 0 {
            return None;
        }
        let val_u128 = (self.0.unsigned_abs() as u128) << FRAC_BITS;
        Some(Self(Self::sqrt_u128(val_u128) as i64))
    }

    /// Adds `self` and `rhs`.
    ///
    /// Returns `None` when the exact raw result is not representable.
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        fixed_from_i128(self.0 as i128 + rhs.0 as i128)
    }

    /// Subtracts `rhs` from `self`.
    ///
    /// Returns `None` when the exact raw result is not representable.
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        fixed_from_i128(self.0 as i128 - rhs.0 as i128)
    }

    /// Negates `self`, returning `None` when the exact raw result is not representable.
    #[must_use]
    pub const fn checked_neg(self) -> Option<Self> {
        if self.0 == i64::MIN {
            None
        } else {
            Some(Self(-self.0))
        }
    }

    /// Multiplies `self` and `rhs`.
    ///
    /// Returns `None` when the rounded Q32.32 result is not representable.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        let prod = self.0 as i128 * rhs.0 as i128;
        let rounded = round_shift_right_u128(prod.unsigned_abs(), FRAC_BITS);
        let signed_prod =
            if (self.0 < 0) ^ (rhs.0 < 0) { -(rounded as i128) } else { rounded as i128 };
        fixed_from_i128(signed_prod)
    }

    /// Computes the integer square root of a `u128` value.
    #[must_use]
    pub const fn sqrt_u128(val: u128) -> u128 {
        if val == 0 {
            return 0;
        }

        let mut op = val;
        let mut res = 0_u128;
        let mut one = 1_u128 << 126;

        one >>= even_leading_shift(val);

        while one != 0 {
            if op >= res + one {
                op -= res + one;
                res = (res >> 1) + one;
            } else {
                res >>= 1;
            }
            one >>= 2;
        }

        res
    }

    /// Divides `self` by `rhs`, returning `None` on zero divisor or overflow.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }

        let numer = (self.0 as i128) << FRAC_BITS;
        let denom = rhs.0 as i128;
        let abs_numer = if numer < 0 { -numer } else { numer };
        let abs_denom = if denom < 0 { -denom } else { denom };

        let final_abs_q = rounded_abs_quotient(abs_numer, abs_denom);
        let signed_q = signed_quotient(self.0, rhs.0, final_abs_q);
        fixed_from_i128(signed_q)
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
        let signed_prod =
            if (self.0 < 0) ^ (rhs.0 < 0) { -(rounded as i128) } else { rounded as i128 };
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
            return if self.0.is_negative() { Self(i64::MIN) } else { Self(i64::MAX) };
        }

        self.checked_div(rhs).unwrap_or(if (self.0 < 0) ^ (rhs.0 < 0) {
            Self(i64::MIN)
        } else {
            Self(i64::MAX)
        })
    }
}

impl DivAssign for FixedQ32_32 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

// dojo: allow float-boundary -- standard validated f32 ingress trait for Q32.32
impl TryFrom<f32> for FixedQ32_32 {
    type Error = FloatConversionError;

    // dojo: allow float-boundary -- standard validated f32 ingress trait for Q32.32
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::try_from_f32(value)
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

const fn even_leading_shift(value: u128) -> u32 {
    value.leading_zeros() & !1
}

const fn rounded_abs_quotient(abs_numer: i128, abs_denom: i128) -> i128 {
    let abs_q = abs_numer / abs_denom;
    let abs_r = abs_numer % abs_denom;
    round_tie_to_even(abs_q, abs_r, abs_denom)
}

const fn round_tie_to_even(abs_q: i128, abs_r: i128, abs_denom: i128) -> i128 {
    let double_r = abs_r << 1;
    if double_r > abs_denom || (double_r == abs_denom && (abs_q & 1) == 1) {
        abs_q + 1
    } else {
        abs_q
    }
}

const fn signed_quotient(lhs: i64, rhs: i64, abs_q: i128) -> i128 {
    if (lhs < 0) ^ (rhs < 0) {
        -abs_q
    } else {
        abs_q
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn fixed_from_i128(value: i128) -> Option<FixedQ32_32> {
    if value < i64::MIN as i128 || value > i64::MAX as i128 {
        None
    } else {
        Some(FixedQ32_32(value as i64))
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
