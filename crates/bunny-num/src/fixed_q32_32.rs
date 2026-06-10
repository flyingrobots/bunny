//! Deterministic helpers for the Q32.32 fixed-point encoding.
//!
//! The representation is an `i64` storing an integer scaled by `2^32`:
//! `real_value = raw / 2^32`.

/// Number of fractional bits in the Q32.32 fixed-point encoding.
pub const FRAC_BITS: u32 = 32;

/// The raw integer value corresponding to `1.0` in Q32.32.
pub const ONE_RAW: i64 = 1_i64 << FRAC_BITS;

fn round_shift_right_u64(value: u64, shift: u32) -> u64 {
    if shift == 0 {
        return value;
    }
    if shift >= 64 {
        return 0;
    }

    let q = value >> shift;
    let mask = (1_u64 << shift) - 1;
    let r = value & mask;
    let half = 1_u64 << (shift - 1);

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

fn round_shift_right_u128(value: u128, shift: u32) -> u128 {
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

fn saturate_i128_to_i64(value: i128) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| {
        if value.is_negative() {
            i64::MIN
        } else {
            i64::MAX
        }
    })
}

/// Deterministically converts an `f32` to a Q32.32 raw `i64`.
///
/// Semantics:
///
/// - `NaN` maps to `0` because fixed-point has no NaN representation.
/// - `+infinity` and `-infinity` saturate to `i64::MAX` and `i64::MIN`.
/// - Values are rounded to nearest with ties-to-even at the Q32.32 boundary.
#[must_use]
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
    let mant = bits & 0x7fffff;

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
        let rounded = round_shift_right_u64(mantissa, rshift);
        i128::from(rounded)
    };

    let signed_raw = if sign { -abs_raw } else { abs_raw };
    saturate_i128_to_i64(signed_raw)
}

/// Deterministically converts a Q32.32 raw `i64` to an `f32`.
///
/// Rounds to nearest with ties-to-even at the `f32` boundary.
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_raw_encoding_are_q32_32() {
        assert_eq!(FRAC_BITS, 32);
        assert_eq!(ONE_RAW, 1_i64 << 32);
    }

    #[test]
    fn from_f32_encodes_exact_values() {
        assert_eq!(from_f32(0.0), 0);
        assert_eq!(from_f32(-0.0), 0);
        assert_eq!(from_f32(1.0), ONE_RAW);
        assert_eq!(from_f32(-1.0), -ONE_RAW);
        assert_eq!(from_f32(0.5), 1_i64 << 31);
        assert_eq!(from_f32(1.5), ONE_RAW + (1_i64 << 31));
    }

    #[test]
    fn to_f32_roundtrips_basic_values() {
        for value in [0.0, -0.0, 1.0, -1.0, 0.5, 1.5] {
            assert_eq!(to_f32(from_f32(value)), value);
        }
    }

    #[test]
    fn non_finite_inputs_use_canonical_policy() {
        assert_eq!(from_f32(f32::NAN), 0);
        assert_eq!(from_f32(f32::INFINITY), i64::MAX);
        assert_eq!(from_f32(f32::NEG_INFINITY), i64::MIN);
    }
}
