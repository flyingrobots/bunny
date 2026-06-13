//! Float conversions for the Q32.32 fixed-point representation.

use super::{round_shift_right_u128, saturate_i128_to_i64, FRAC_BITS};

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
