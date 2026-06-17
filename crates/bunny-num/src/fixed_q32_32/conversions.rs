//! Float conversions for the Q32.32 fixed-point representation.

use super::{round_shift_right_u128, saturate_i128_to_i64, FRAC_BITS};

/// Deterministically converts an `f32` to a Q32.32 raw `i64`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
// dojo: allow float-boundary -- explicit ingress conversion into canonical Q32.32
pub fn from_f32(value: f32) -> i64 {
    if value.is_nan() {
        return 0;
    }
    if value.is_infinite() {
        return if value.is_sign_positive() { i64::MAX } else { i64::MIN };
    }

    let parts = F32Parts::from(value);
    if parts.is_zero() {
        return 0;
    }

    let abs_raw = shifted_abs_raw(parts.mantissa(), parts.raw_shift());
    let signed_raw = if parts.sign { -abs_raw } else { abs_raw };
    saturate_i128_to_i64(signed_raw)
}

/// Deterministically converts a Q32.32 raw `i64` to an `f32`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
// dojo: allow float-boundary -- explicit lossy display/API egress from Q32.32
pub fn to_f32(raw: i64) -> f32 {
    if raw == 0 {
        return 0.0;
    }

    let sign = raw.is_negative();
    let abs: u64 = raw.unsigned_abs();
    if abs == 0 {
        return 0.0;
    }

    f32_from_abs_raw(sign, abs)
}

struct F32Parts {
    sign: bool,
    exp: i32,
    mant: u32,
}

// dojo: allow float-boundary -- internal decomposition of ingress f32 boundary value
impl From<f32> for F32Parts {
    fn from(value: f32) -> Self {
        let bits = value.to_bits();
        let sign = (bits >> 31) != 0;
        let exp = i32::from(((bits >> 23) & 0xff) as u8);
        let mant = bits & 0x007f_ffff;
        Self { sign, exp, mant }
    }
}

impl F32Parts {
    const fn is_zero(&self) -> bool {
        self.exp == 0 && self.mant == 0
    }

    fn mantissa(&self) -> u64 {
        if self.exp == 0 {
            u64::from(self.mant)
        } else {
            u64::from((1_u32 << 23) | self.mant)
        }
    }

    const fn raw_shift(&self) -> i32 {
        let unbiased = if self.exp == 0 { -126 } else { self.exp - 127 };
        unbiased + (FRAC_BITS.cast_signed() - 23)
    }
}

fn shifted_abs_raw(mantissa: u64, shift: i32) -> i128 {
    if shift >= 0 {
        shifted_left_abs_raw(mantissa, shift.unsigned_abs())
    } else {
        rounded_right_abs_raw(mantissa, shift.unsigned_abs())
    }
}

fn shifted_left_abs_raw(mantissa: u64, shift: u32) -> i128 {
    if shift > 103 {
        i128::MAX
    } else {
        i128::from(mantissa) << shift
    }
}

#[allow(clippy::cast_possible_truncation)]
fn rounded_right_abs_raw(mantissa: u64, shift: u32) -> i128 {
    let rounded = round_shift_right_u128(u128::from(mantissa), shift) as u64;
    i128::from(rounded)
}

// dojo: allow float-boundary -- internal assembly of lossy f32 boundary output
fn f32_from_abs_raw(sign: bool, abs: u64) -> f32 {
    let k = 63_u32.saturating_sub(abs.leading_zeros());
    let mut exp = k.cast_signed() - FRAC_BITS.cast_signed();

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

    let exp_field = (exp + 127).cast_unsigned();
    let mantissa = (sig & ((1_u128 << 23) - 1)) as u32;
    let bits = (u32::from(sign) << 31) | (exp_field << 23) | mantissa;
    f32::from_bits(bits)
}
