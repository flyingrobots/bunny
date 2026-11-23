//! Conversions out of a binary angle: turns, degrees and radians.
//!
//! Each is the exact product of the raw angle and a scale, rounded once to Q32.32 to
//! nearest with ties to even. Turns and degrees scale by an integer, so they are correctly
//! rounded; radians scale by a rounded `2 pi`, which adds less than `2^-30` raw units.

use super::FixedAngle;
use crate::fixed_q32_32::round_shift_right_u128;
use crate::FixedQ32_32;

/// `round(2 pi * 2^61)`: `2 pi` with 61 fractional bits, the most a `u64` holds.
const TAU_Q61: u128 = 14_488_038_916_154_245_685;

impl FixedAngle {
    /// The angle in turns, in `[0, 1]`: `raw / 2^64` rounded to Q32.32, ties to even.
    ///
    /// The result is the nearest Q32.32 value to the exact turn, so it is within half a raw
    /// unit. Angles within half a raw unit of a full turn give exactly `1.0`.
    /// `to_turns(from_turns(t))` is exactly the fractional part of `t`, in `[0, 1)`.
    #[must_use]
    pub const fn to_turns(self) -> FixedQ32_32 {
        rounded(self.0 as u128, 32)
    }

    /// The angle in degrees, in `[0, 360]`: `raw * 360 / 2^64` rounded to Q32.32, ties to even.
    ///
    /// The result is the nearest Q32.32 value to the exact angle in degrees, so it is within
    /// half a raw unit. Whole multiples of a quarter turn give exactly 0, 90, 180 or 270.
    #[must_use]
    pub const fn to_degrees(self) -> FixedQ32_32 {
        // 360 / 2^32 = 45 / 2^29; the product is below 2^70.
        rounded(self.0 as u128 * 45, 29)
    }

    /// The angle in radians, in `[0, 2 pi]`: within `1/2 + 2^-30` raw units of the exact
    /// `raw * 2 pi / 2^64`.
    ///
    /// The product with `round(2 pi * 2^61)` is exact in `u128` and rounded once, to
    /// nearest with ties to even. The rounded constant differs from `2 pi * 2^61` by at most
    /// one half, which moves the product by less than `2^-30` raw units for any angle, so
    /// the result is one of the two Q32.32 values either side of the exact one.
    #[must_use]
    pub const fn to_radians(self) -> FixedQ32_32 {
        // Below 2^64 * 2^64: exact in u128.
        rounded(self.0 as u128 * TAU_Q61, 93)
    }

    /// The angle in turns read as signed, in `[-1/2, 1/2]`: the raw angle as an `i64` over
    /// `2^64`, rounded to Q32.32, ties to even.
    ///
    /// The angle is read in the half-open turn `[-1/2, 1/2)`: [`Self::HALF_TURN`] gives
    /// exactly `-0.5`, and angles within half a raw unit below it round up to `+0.5`. The
    /// result is correctly rounded. Below a half turn it equals [`Self::to_turns`]; from a
    /// half turn up it is exactly `1.0` less.
    #[must_use]
    pub const fn to_turns_signed(self) -> FixedQ32_32 {
        let (magnitude, negative) = self.signed_magnitude();
        with_sign(rounded(magnitude, 32), negative)
    }

    /// The angle in degrees read as signed, in `[-180, 180]`, rounded to Q32.32, ties to even.
    ///
    /// The angle is read in `[-1/2, 1/2)` turn: [`Self::HALF_TURN`] gives exactly `-180`, and
    /// angles within half a raw unit below it round up to `+180`. The result is correctly
    /// rounded. Below a half turn it equals [`Self::to_degrees`]; from a half turn up it is
    /// exactly `360` less.
    #[must_use]
    pub const fn to_degrees_signed(self) -> FixedQ32_32 {
        let (magnitude, negative) = self.signed_magnitude();
        with_sign(rounded(magnitude * 45, 29), negative)
    }

    /// The angle in radians read as signed, in `[-pi, pi]`: within `1/2 + 2^-30` raw units
    /// of the exact value of the raw angle as an `i64` times `2 pi / 2^64`.
    ///
    /// The magnitude is rounded exactly as [`Self::to_radians`] rounds it and the sign
    /// applied after, so below a half turn the two are equal, and
    /// `a.wrapping_neg().to_radians_signed()` is the negation of `a.to_radians_signed()`
    /// for every angle but the half turn, which reads as `-pi`.
    #[must_use]
    pub const fn to_radians_signed(self) -> FixedQ32_32 {
        let (magnitude, negative) = self.signed_magnitude();
        with_sign(rounded(magnitude * TAU_Q61, 93), negative)
    }

    /// The magnitude of the raw angle read as an `i64`, at most `2^63`, and its sign.
    #[allow(clippy::cast_possible_wrap)]
    const fn signed_magnitude(self) -> (u128, bool) {
        let signed = self.0 as i64;
        (signed.unsigned_abs() as u128, signed < 0)
    }
}

/// `value`, negated when `negative`; every caller's value is at most `180 * 2^32`.
const fn with_sign(value: FixedQ32_32, negative: bool) -> FixedQ32_32 {
    if negative {
        FixedQ32_32::from_raw(-value.raw())
    } else {
        value
    }
}

/// `value / 2^shift` rounded to nearest with ties to even, as raw Q32.32.
///
/// Every caller's result is at most `360 * 2^32`, so it fits an `i64`.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
const fn rounded(value: u128, shift: u32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(round_shift_right_u128(value, shift) as i64)
}
