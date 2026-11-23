//! Exact power eases and steps in Q32.32 (`bunny-ease/1`, part 1).
//!
//! Each power ease is a polynomial in `t` with rational coefficients. Its exact value is a
//! rational with a power-of-two denominator, computed exactly in a 256-bit integer and rounded
//! once, to nearest with ties to even. The rules and their proofs are in
//! `docs/topics/ease/README.md`.

use crate::fixed_q32_32::{FixedQ32_32, FRAC_BITS};

/// Stable contract identifier for the eases in this module.
pub const EASE_CONTRACT_ID: &str = "bunny-ease/1";

/// Raw `1.0`.
const UNIT: u64 = 1 << FRAC_BITS;
/// Raw `1/2`.
const HALF: u64 = UNIT >> 1;

/// `t^n`, the "in" power ease, for `n` in `2..=5` (GSAP's power1 to power4).
///
/// The result is `round_half_even(t_raw^n / 2^(32(n-1)))`: the exact value rounded once,
/// within half a raw unit. `t = 0` gives exactly `0` and `t = 1.0` exactly `1.0`, and the
/// result is non-decreasing in `t`.
///
/// Returns `None` when `t` is outside `[0, 1]` or `n` is outside `2..=5`.
#[must_use]
pub const fn power_in(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    match power_raw(n, t) {
        Some(r) => Some(raw(scaled_power(r, n, FRAC_BITS * (n - 1)))),
        None => None,
    }
}

/// `1 - (1 - t)^n`, the "out" power ease, for `n` in `2..=5`.
///
/// Computed as `1 - power_in(n, 1 - t)`, which is exactly the rounded `1 - (1 - t)^n`: `1.0`
/// is an even whole number of raw units, so subtracting from it maps a tie to even onto a tie
/// to even. Within half a raw unit; exact at both ends; non-decreasing.
///
/// Returns `None` when `t` is outside `[0, 1]` or `n` is outside `2..=5`.
#[must_use]
pub const fn power_out(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    match power_raw(n, t) {
        Some(r) => Some(raw(UNIT - scaled_power(UNIT - r, n, FRAC_BITS * (n - 1)))),
        None => None,
    }
}

/// `2^(n-1) t^n` below `t = 1/2` and `1 - 2^(n-1) (1 - t)^n` from it, for `n` in `2..=5`.
///
/// The lower half is `round_half_even(t_raw^n / 2^(31(n-1)))`, and the upper half is `1.0`
/// minus the lower half at `1 - t`, which is exactly the rounded upper formula (see
/// [`power_out`]). So `f(1 - t) == 1 - f(t)` bit for bit, `f(1/2) = 1/2` exactly, and the
/// result is within half a raw unit and non-decreasing.
///
/// Returns `None` when `t` is outside `[0, 1]` or `n` is outside `2..=5`.
#[must_use]
pub const fn power_in_out(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    match power_raw(n, t) {
        Some(r) if r < HALF => Some(raw(scaled_power(r, n, (FRAC_BITS - 1) * (n - 1)))),
        Some(r) => Some(raw(UNIT - scaled_power(UNIT - r, n, (FRAC_BITS - 1) * (n - 1)))),
        None => None,
    }
}

/// `k / n` for `t` in `[k/n, (k+1)/n)`, and `1.0` at `t = 1`.
///
/// `k = floor(n t_raw / 2^32)` is exact in `u64`, and `k / n` is rounded once to nearest by
/// [`FixedQ32_32::from_ratio`], where a tie cannot occur.
///
/// Returns `None` when `t` is outside `[0, 1]` or `n == 0`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub const fn steps(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    if n == 0 {
        return None;
    }
    match unit_raw(t) {
        // n t_raw <= (2^32 - 1) 2^32 < 2^64, and k <= n fits a u32.
        Some(r) => FixedQ32_32::from_ratio(((n as u64 * r) >> FRAC_BITS) as u32, n),
        None => None,
    }
}

/// The raw value of `t` when `t` is in `[0, 1]`.
#[allow(clippy::cast_sign_loss)]
const fn unit_raw(t: FixedQ32_32) -> Option<u64> {
    let r = t.raw();
    if r < 0 || r > 1 << FRAC_BITS {
        None
    } else {
        Some(r as u64)
    }
}

/// The raw value of `t` when `t` is in `[0, 1]` and `n` in `2..=5`.
const fn power_raw(n: u32, t: FixedQ32_32) -> Option<u64> {
    if n < 2 || n > 5 {
        None
    } else {
        unit_raw(t)
    }
}

/// A raw value in `[0, 2^32]`, which fits an `i64`.
#[allow(clippy::cast_possible_wrap)]
const fn raw(value: u64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value as i64)
}

/// `round_half_even(base^n / 2^shift)` for `base <= 2^32`, `n <= 5` and a result `<= 2^32`.
///
/// `base^n <= 2^160` is held exactly as `upper 2^64 + lower` while it is built. Before the
/// last factor `upper <= 2^64`, so `upper * base <= 2^96` and the `u128` never overflows. The
/// result is then split into the high and low 128 bits of a 256-bit value for the shift.
#[allow(clippy::cast_possible_truncation)]
const fn scaled_power(base: u64, n: u32, shift: u32) -> u64 {
    let mut upper: u128 = 0;
    let mut lower: u64 = 1;
    let mut i = 0;
    while i < n {
        let low_product = lower as u128 * base as u128;
        upper = upper * base as u128 + (low_product >> 64);
        lower = low_product as u64;
        i += 1;
    }
    round_shift(upper >> 64, (upper << 64) | lower as u128, shift)
}

/// `round_half_even((high 2^128 + low) / 2^shift)` for `1 <= shift <= 128` and a result
/// below `2^64`: the quotient by a shift, the remainder by a mask.
#[allow(clippy::cast_possible_truncation)]
const fn round_shift(high: u128, low: u128, shift: u32) -> u64 {
    let (quotient, remainder) = if shift == 128 {
        (high, low)
    } else {
        ((high << (128 - shift)) | (low >> shift), low & ((1 << shift) - 1))
    };
    let half = 1 << (shift - 1);
    let up = remainder > half || (remainder == half && quotient & 1 == 1);
    (quotient + up as u128) as u64
}
