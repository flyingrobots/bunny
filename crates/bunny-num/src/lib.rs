#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Deterministic numeric profiles for Bunny graphics primitives.

pub mod fixed_q32_32;

pub use fixed_q32_32::{FixedQ32_32, FloatConversionError};

/// Bunny's first scalar representation.
///
/// This is intentionally tiny while the deterministic profile is still being
/// designed. Public APIs should name `Scalar` instead of spelling a primitive
/// float directly.
// dojo: allow float-boundary -- scalar DTO boundary; canonical math uses FixedQ32_32
pub type Scalar = f32;

/// Returns true when the scalar can participate in canonical Bunny contracts.
#[must_use]
pub const fn is_finite(value: Scalar) -> bool {
    value.is_finite()
}

// Exact integer square root of a `u128` without 128-bit division.
//
// The result equals `u128::isqrt`: the floor of the square root. The top 64 bits go
// through `u64::isqrt`, whose steps divide in hardware, and one Karatsuba square-root
// step (Zimmermann, INRIA RR-3805) finishes the low half. Core's `u128` version ends that
// step with a `u128 / u128` division, which aarch64 and wasm32 run in software; here the
// divisor is below `2^33`, so the division splits into two `u64` divisions.

/// Floor of the square root of `n`.
#[allow(clippy::cast_possible_truncation)]
#[inline]
pub(crate) const fn isqrt_u128(n: u128) -> u128 {
    if n <= u64::MAX as u128 {
        return (n as u64).isqrt() as u128;
    }
    // An even shift keeps the root exact: sqrt(n << 2k) == sqrt(n) << k. After it, one of
    // the two top bits is set, which the Karatsuba step requires.
    let shift = n.leading_zeros() & !1;
    normalized_root(n << shift) >> (shift / 2)
}

/// Floor of the square root of `n`, where one of the two top bits of `n` is set.
#[allow(clippy::cast_possible_truncation)]
#[inline]
const fn normalized_root(n: u128) -> u128 {
    let high = (n >> 64) as u64;
    // high >= 2^62, so 2^31 <= root < 2^32 and remainder <= 2 * root < 2^33.
    let root = high.isqrt();
    let remainder = high - root * root;
    let next_digits = (n as u64) >> 32;
    let quotient = divide_by_double_root(remainder, next_digits, root << 1);
    let candidate = ((root as u128) << 32) + quotient as u128;
    // The quotient can overshoot by one; correct exactly as core does.
    match candidate.checked_mul(candidate) {
        Some(square) if square <= n => candidate,
        _ => candidate - 1,
    }
}

/// `(remainder * 2^32 + digits) / divisor` for `remainder < 2^33`, `digits < 2^32` and
/// `2^32 <= divisor < 2^33`, by long division in 31-bit digits so every step fits a `u64`.
#[inline]
const fn divide_by_double_root(remainder: u64, digits: u64, divisor: u64) -> u64 {
    const DIGIT_BITS: u32 = 31;
    const DIGIT_MASK: u64 = (1 << DIGIT_BITS) - 1;
    // Upper digits: remainder * 2 + the top bit of `digits`, below 2^34.
    let upper = (remainder << 1) | (digits >> DIGIT_BITS);
    let high_quotient = upper / divisor;
    // The partial remainder is below the divisor (< 2^33), so this is below 2^64.
    let lower = ((upper % divisor) << DIGIT_BITS) | (digits & DIGIT_MASK);
    (high_quotient << DIGIT_BITS) + lower / divisor
}
