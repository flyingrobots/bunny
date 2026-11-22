//! Unsigned 32-bit division rounded to nearest integer, ties to even
//! (`bunny-u32-div-round-even/1`).
//!
//! This bounded unsigned-integer operation has its own explicit contract and
//! makes no claim of Q32.32 GPU conformance.

use core::fmt::{self, Display, Formatter};

/// Stable contract identifier for unsigned 32-bit ties-to-even division.
pub const U32_DIV_ROUND_EVEN_CONTRACT_ID: &str = "bunny-u32-div-round-even/1";

/// Canonical status word for a successful rounded quotient.
pub const U32_DIV_ROUND_EVEN_STATUS_OK: u32 = 0;

/// Canonical status word when the denominator is zero.
pub const U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO: u32 = 1;

/// Error returned by [`div_round_even_u32`] when the denominator is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DivisionByZero;

impl Display for DivisionByZero {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("division by zero in bunny-u32-div-round-even/1")
    }
}

impl core::error::Error for DivisionByZero {}

/// Divides `numerator` by `denominator`, rounding to the nearest `u32` with exact
/// halves rounded to the even integer (`bunny-u32-div-round-even/1`).
///
/// # Errors
///
/// Returns [`DivisionByZero`] when `denominator == 0`, including when `numerator == 0`.
pub const fn div_round_even_u32(numerator: u32, denominator: u32) -> Result<u32, DivisionByZero> {
    if denominator == 0 {
        return Err(DivisionByZero);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let complement = denominator - remainder;
    let increment = remainder > complement || (remainder == complement && (quotient & 1) != 0);
    if increment {
        Ok(quotient + 1)
    } else {
        Ok(quotient)
    }
}

/// Canonical two-word interchange adapter `[value, status]` for
/// `bunny-u32-div-round-even/1`.
#[must_use]
pub const fn div_round_even_u32_words(n: u32, d: u32) -> [u32; 2] {
    match div_round_even_u32(n, d) {
        Ok(value) => [value, U32_DIV_ROUND_EVEN_STATUS_OK],
        Err(DivisionByZero) => [0, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO],
    }
}

/// Canonical 8-byte little-endian serialization (`value_le4 || status_le4`) for
/// `bunny-u32-div-round-even/1`.
#[must_use]
pub const fn div_round_even_u32_bytes(n: u32, d: u32) -> [u8; 8] {
    let [value, status] = div_round_even_u32_words(n, d);
    let [v0, v1, v2, v3] = value.to_le_bytes();
    let [s0, s1, s2, s3] = status.to_le_bytes();
    [v0, v1, v2, v3, s0, s1, s2, s3]
}
