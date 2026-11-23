//! Back, circ and bounce eases in Q32.32 (`bunny-ease/1`, part 2).
//!
//! Back and bounce are polynomials with rational coefficients, so each value is an exact
//! rational rounded once, to nearest with ties to even. Circ is `1 - sqrt(1 - t^2)`, whose
//! radicand is the exact integer `2^64 - t_raw^2`: its floor square root is subtracted from
//! `1.0` with nothing rounded before the root. The rules and proofs are in
//! `docs/topics/ease/README.md`.

use super::{round_shift, unit_raw, HALF, UNIT};
use crate::fixed_q32_32::FixedQ32_32;
use crate::isqrt_u128;

/// GSAP's default back overshoot `1.70158 = 85_079 / 50_000`, rounded once to nearest:
/// raw `7_308_230_452`, which is `85_079 2^32 / 50_000` (`7_308_230_451.53`) rounded up.
pub const BACK_DEFAULT_OVERSHOOT: FixedQ32_32 = FixedQ32_32::from_raw(7_308_230_452);

/// Raw `1.0` as a signed value.
#[allow(clippy::cast_possible_wrap)]
const UNIT_I: i64 = UNIT as i64;

/// `t^2 ((s + 1) t - s)` for overshoot `s`, GSAP's back.in, rounded once.
///
/// The result is `round_half_even(r^2 (s_raw (r - 2^32) + 2^32 r) / 2^96)`, within half a raw
/// unit, for `r = t_raw`. Every overshoot is accepted: the value is at most
/// `1 + 4 |s| / 27` in magnitude, below `2^31`. `t = 0` gives `0` and `t = 1` gives `1.0`.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn back_in(overshoot: FixedQ32_32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(FixedQ32_32::from_raw(back_scaled(overshoot.raw(), r, 96))),
        None => None,
    }
}

/// `1 - back_in(s, 1 - t)`, GSAP's back.out: exactly the rounded
/// `(t - 1)^2 ((s + 1)(t - 1) + s) + 1`, because `1.0` is even. `back_out(s, 1) = 1.0`.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn back_out(overshoot: FixedQ32_32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(FixedQ32_32::from_raw(UNIT_I - back_scaled(overshoot.raw(), UNIT - r, 96))),
        None => None,
    }
}

/// `back_in(s, 2t) / 2` below `t = 1/2` and `1 - back_in(s, 2 - 2t) / 2` from it, GSAP's
/// back.inOut with the same overshoot. The lower half is one rounding of
/// `(2r)^2 (s_raw (2r - 2^32) + 2^32 2r) / 2^97`; the upper half is its reflection, so
/// `f(1 - t) == 1 - f(t)` bit for bit and `f(1/2) = 1/2`.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn back_in_out(overshoot: FixedQ32_32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    let s = overshoot.raw();
    match unit_raw(t) {
        Some(r) if r < HALF => Some(FixedQ32_32::from_raw(back_scaled(s, 2 * r, 97))),
        Some(r) => Some(FixedQ32_32::from_raw(UNIT_I - back_scaled(s, 2 * (UNIT - r), 97))),
        None => None,
    }
}

/// `1 - sqrt(1 - t^2)`: `2^32 - isqrt(2^64 - r^2)`, the floor of the exact root subtracted
/// exactly. The result is at least the exact value and less than one raw unit above it
/// (exact when `2^64 - r^2` is a perfect square). `t = 0` gives `0`, `t = 1` gives `1.0`, and
/// it is non-decreasing.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn circ_in(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(unsigned(UNIT - root_of_difference(UNIT, r))),
        None => None,
    }
}

/// `sqrt(1 - (1 - t)^2)`: `1 - circ_in(1 - t)`, which is `isqrt(2^64 - (2^32 - r)^2)`. At
/// most the exact value and less than one raw unit below it; non-decreasing.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn circ_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(unsigned(root_of_difference(UNIT, UNIT - r))),
        None => None,
    }
}

/// `circ_in(2t) / 2` below `t = 1/2`, as `2^31 - isqrt(2^62 - r^2)` (the floor of half the
/// exact root), and its reflection `1 - f(1 - t)` from `1/2`. Within one raw unit, above the
/// exact value below `1/2` and below it from `1/2`; `f(1/2) = 1/2`; non-decreasing.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn circ_in_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) if r < HALF => Some(unsigned(HALF - root_of_difference(HALF, r))),
        Some(r) => Some(unsigned(HALF + root_of_difference(HALF, UNIT - r))),
        None => None,
    }
}

/// GSAP's bounce.out: `121/16 t^2` on `[0, 1/2.75)`, then `121/16 (t - 1.5/2.75)^2 + 3/4` on
/// `[1/2.75, 2/2.75)`, `121/16 (t - 2.25/2.75)^2 + 15/16` on `[2/2.75, 2.5/2.75)` and
/// `121/16 (t - 2.625/2.75)^2 + 63/64` on `[2.5/2.75, 1]`. Each breakpoint belongs to the piece
/// on its right; none is a raw value. On raw values a piece is
/// `round_half_even((22 r - p 2^32)^2 / 2^38) + q 2^26`, within half a raw unit.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn bounce_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(unsigned(bounce_scaled(r, 38))),
        None => None,
    }
}

/// `1 - bounce_out(1 - t)`, GSAP's bounce.in, which is exactly that value rounded once.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn bounce_in(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) => Some(unsigned(UNIT - bounce_scaled(UNIT - r, 38))),
        None => None,
    }
}

/// `(1 - bounce_out(1 - 2t)) / 2` below `t = 1/2` and its reflection `1 - f(1 - t)` from it,
/// GSAP's bounce.inOut. The lower half is one rounding of `1/2 - bounce_out(1 - 2t) / 2`.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub const fn bounce_in_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    match unit_raw(t) {
        Some(r) if r < HALF => Some(unsigned(HALF - bounce_scaled(UNIT - 2 * r, 39))),
        Some(r) => Some(unsigned(HALF + bounce_scaled(2 * r - UNIT, 39))),
        None => None,
    }
}

/// A raw value in `[0, 2^32]`, which fits an `i64`.
#[allow(clippy::cast_possible_wrap)]
const fn unsigned(value: u64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value as i64)
}

/// `isqrt(top^2 - r^2)` for `r <= top <= 2^32`: the radicand is exact in `u128`.
#[allow(clippy::cast_possible_truncation)]
const fn root_of_difference(top: u64, r: u64) -> u64 {
    let (top, r) = (top as u128, r as u128);
    isqrt_u128(top * top - r * r) as u64
}

/// `round_half_even(r^2 (s (r - 2^32) + 2^32 r) / 2^shift)` for `r <= 2^32` and `shift` of
/// 96 or 97, with the sign of the exact value. `|s (r - 2^32)| <= 2^95`, so the inner factor
/// fits an `i128`; its magnitude times `r` is below `2^128`, and times `r` again below `2^160`,
/// held as the high and low `u128` halves of a 256-bit value.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
const fn back_scaled(s: i64, r: u64, shift: u32) -> i64 {
    let inner = s as i128 * (r as i128 - UNIT as i128) + (UNIT as i128 * r as i128);
    let once = inner.unsigned_abs() * r as u128;
    let low_product = (once as u64) as u128 * r as u128;
    let middle = (once >> 64) * r as u128 + (low_product >> 64);
    let low = (middle << 64) | (low_product as u64) as u128;
    let magnitude = round_shift(middle >> 64, low, shift) as i64;
    if inner < 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// `round_half_even((22 u - p 2^32)^2 / 2^shift) + q 2^(64 - shift)` for the bounce piece
/// that owns `u <= 2^32`, with centre `p / 22` and level `q / 64`: `121/16 (t - p/22)^2` is
/// `(22 t - p)^2 / 64`. `|22 u - p 2^32| < 2^37`, so the square fits a `u128`. With
/// `shift = 38` it is `bounce_out`; with `39`, half of it, whose level `q 2^25` is whole.
const fn bounce_scaled(u: u64, shift: u32) -> u64 {
    let scaled = 22 * u;
    let (centre, level) = if scaled < 8 * UNIT {
        (0, 0)
    } else if scaled < 16 * UNIT {
        (12, 48)
    } else if scaled < 20 * UNIT {
        (18, 60)
    } else {
        (21, 63)
    };
    let offset = scaled.abs_diff(centre * UNIT) as u128;
    round_shift(0, offset * offset, shift) + (level << (64 - shift))
}
