//! Sine eases in Q32.32 (`bunny-ease/1`, part 3), read from [`FixedAngle::sin_cos`].
//!
//! `t` in `[0, 1]` maps exactly onto an angle: `t_raw * 2^30` angle units for a quarter turn
//! (`2^62`) and `t_raw * 2^31` for a half turn (`2^63`), both at most `2^63` in a `u64`. Each
//! ease inherits `sin_cos`'s bound of one raw unit; the derivation and the measured worst
//! cases are in `docs/topics/ease/README.md#sine-eases`.

use super::{raw, unit_raw, HALF, UNIT};
use crate::fixed_angle::FixedAngle;
use crate::fixed_q32_32::FixedQ32_32;

/// `1 - cos(t pi/2)`, the "in" sine ease.
///
/// The raw result is `2^32 - cos(t_raw 2^30 angle units)`, an exact subtraction from
/// `sin_cos`'s cosine, so it is within one raw unit of the exact value. `t = 0` gives exactly
/// `0` and `t = 1.0` exactly `1.0`, because `sin_cos` is exact at multiples of a quarter turn.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub fn sine_in(t: FixedQ32_32) -> Option<FixedQ32_32> {
    unit_raw(t).map(|r| raw(in_raw(r)))
}

/// `sin(t pi/2)`, the "out" sine ease.
///
/// Computed as `1 - sine_in(1 - t)`, which is `cos((1 - t) pi/2)` from `sin_cos` and equals
/// its `sin(t pi/2)` bit for bit: `sin_cos` reads both from one quarter-wave table, cosine at
/// the complementary position. Within one raw unit; exact at both ends.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub fn sine_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    unit_raw(t).map(|r| raw(UNIT - in_raw(UNIT - r)))
}

/// `(1 - cos(t pi)) / 2`, the "in-out" sine ease.
///
/// Below `t = 1/2` the raw result is `round_half_even((2^32 - cos(t_raw 2^31 units)) / 2)`;
/// from `1/2` it is `1.0` minus that at `1 - t`, which is the same formula bit for bit. So
/// `f(1 - t) == 1 - f(t)`, `f(1/2) = 1/2` and both ends are exact. The halving adds at most
/// half a raw unit to half of `sin_cos`'s one: within one raw unit.
///
/// Returns `None` when `t` is outside `[0, 1]`.
#[must_use]
pub fn sine_in_out(t: FixedQ32_32) -> Option<FixedQ32_32> {
    unit_raw(t).map(|r| raw(if r < HALF { in_out_raw(r) } else { UNIT - in_out_raw(UNIT - r) }))
}

/// `2^32 - cos(r 2^30)` for `r` in `[0, 2^32]`: the angle is in `[0, 2^62]`, a quarter turn.
fn in_raw(r: u64) -> u64 {
    UNIT - cos_raw(r << 30)
}

/// `round_half_even((2^32 - cos(r 2^31)) / 2)` for `r` in `[0, 2^31]`: the angle is in
/// `[0, 2^62]`, and `2^32 - cos` in `[0, 2^32]`.
fn in_out_raw(r: u64) -> u64 {
    half_even(UNIT - cos_raw(r << 31))
}

/// `round_half_even(m / 2)`: an odd `m` is a tie, and moves up only from an odd half.
const fn half_even(m: u64) -> u64 {
    (m >> 1) + (m & (m >> 1) & 1)
}

/// The raw cosine of an angle in `[0, 2^62]` (at most a quarter turn), which is in
/// `[0, 2^32]`.
#[allow(clippy::cast_sign_loss)]
fn cos_raw(angle: u64) -> u64 {
    FixedAngle::from_raw(angle).cos().raw() as u64
}
