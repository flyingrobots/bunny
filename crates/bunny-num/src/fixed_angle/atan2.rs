//! The angle of a direction, `atan2`, by CORDIC over binary-angle steps.
//!
//! The direction is folded into the first octant by exact symmetries, scaled so its larger
//! component has 124 bits, and rotated towards `+x` by the angles `atan(2^-i)` for
//! `i = 0..STEPS`. Those angles are held in units of `2^-96` turn, computed at compile time
//! by an integer Taylor series, and their signed sum, rounded once to the nearest angle
//! unit, is the result. No `pi`, division or floating point is involved at run time.

use super::{FixedAngle, QUARTER_BITS};
use crate::fixed_q32_32::round_shift_right_u128;
use crate::FixedQ32_32;

/// CORDIC steps; after the last the remaining angle is at most `atan(2^-71)` radians.
const STEPS: usize = 72;
/// Fractional bits of the step angles below one angle unit (`2^-96` turn in all).
const STEP_EXTRA_BITS: u32 = 32;
/// `round(2^127 / (2 pi))`: turns per radian with 127 fractional bits.
const TURNS_PER_RADIAN_Q127: i128 = 27_078_810_371_238_704_511_725_556_867_640_236_984;

/// `atan(2^-i) / (2 pi)` in units of `2^-96` turn, for `i` in `0..STEPS`.
static ATAN_STEPS_Q96: [i128; STEPS] = atan_steps();

impl FixedAngle {
    /// The angle of the direction `(x, y)`, counter-clockwise from `+x`.
    ///
    /// The result is within `1/2 + 2^-9` angle units (`2^-64` turn) of the exact angle, so
    /// it is the nearest angle or, when the exact angle is within `2^-9` units of a midpoint,
    /// its other neighbour. Every `(x, y)` other than the origin is accepted; the origin has
    /// no direction and returns `None`.
    ///
    /// Every angle is reachable: `+x` is [`Self::ZERO`], `+y` a quarter turn, `-x` exactly
    /// [`Self::HALF_TURN`], and just below `-x` just above a half turn. Read as a signed
    /// `i64`, the raw result is in `[-2^63, 2^63)`, a half-open turn `[-1/2, 1/2)` that puts
    /// `-x` at `-1/2`. Exact by construction: the axes and the diagonals give exact multiples
    /// of an eighth turn; `atan2(-y, x)`, `atan2(y, -x)` and `atan2(x, y)` are exactly the
    /// negation, `HALF_TURN - a` and `QUARTER_TURN - a` of `a = atan2(y, x)`, wrapping; and
    /// doubling both inputs, where both still fit, gives the same bits.
    #[must_use]
    pub fn atan2(y: FixedQ32_32, x: FixedQ32_32) -> Option<Self> {
        let (y, x) = (y.raw(), x.raw());
        if y == 0 && x == 0 {
            return None;
        }
        let (across, along) = (u128::from(y.unsigned_abs()), u128::from(x.unsigned_abs()));
        Some(Self(direction_angle(across, along, x < 0, y < 0)))
    }
}

/// The angle of the direction with magnitudes `along` (x) and `across` (y), not both zero
/// and each at most `2^64`, and the given signs, in angle units, wrapping.
///
/// `atan2` passes its raw inputs; `asin` and `acos` pass legs with 32 more bits. Both
/// magnitudes are scaled to the same fixed width before CORDIC, so any input this wide
/// keeps the bound of `atan2`.
pub(super) fn direction_angle(
    across: u128,
    along: u128,
    x_negative: bool,
    y_negative: bool,
) -> u64 {
    let swapped = across > along;
    let octant = if swapped { first_octant(along, across) } else { first_octant(across, along) };
    unfold(octant, swapped, x_negative, y_negative)
}

/// Rebuilds the full angle from its first-octant angle by exact reflections: about the
/// diagonal when the components were swapped, then about `+y` and about `+x`.
const fn unfold(octant: u64, swapped: bool, x_negative: bool, y_negative: bool) -> u64 {
    let quadrant = if swapped { (1 << QUARTER_BITS) - octant } else { octant };
    let half = if x_negative { (1 << (QUARTER_BITS + 1)) - quadrant } else { quadrant };
    if y_negative {
        half.wrapping_neg()
    } else {
        half
    }
}

/// `atan(near / far)` in angle units, in `[0, 2^61]`, for `0 <= near <= far`, `far > 0`
/// and `far < 2^124`.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn first_octant(near: u128, far: u128) -> u64 {
    // Put the top bit of `far` at bit 123; the vector then stays below 2^126 in magnitude.
    let shift = far.leading_zeros() - 4;
    let (mut x, mut y) = ((far << shift) as i128, (near << shift) as i128);
    let mut turned = 0_i128;
    for (i, &step) in (0_u32..).zip(ATAN_STEPS_Q96.iter()) {
        let (dx, dy) = (y >> i, x >> i);
        if y >= 0 {
            (x, y, turned) = (x + dx, y - dy, turned + step);
        } else {
            (x, y, turned) = (x - dx, y + dy, turned - step);
        }
    }
    // `turned` is above -2^31 (the remaining angle is far below half a unit), so a negative
    // sum rounds to zero; the octant bound keeps the result at most 2^61.
    round_shift_right_u128(turned.max(0) as u128, STEP_EXTRA_BITS) as u64
}

/// Builds the step-angle table (no floating point).
const fn atan_steps() -> [i128; STEPS] {
    let mut table = [0_i128; STEPS];
    let mut rest: &mut [i128] = &mut table;
    let mut i = 0;
    while let Some((slot, tail)) = rest.split_first_mut() {
        *slot = atan_step(i);
        rest = tail;
        i += 1;
    }
    table
}

/// `atan(2^-i) / (2 pi)` in units of `2^-96` turn, rounded half up.
///
/// `atan(1)` is exactly an eighth turn. Otherwise the Taylor series
/// `sum (-1)^k 2^-i(2k+1) / (2k+1)` is summed with 127 fractional bits of a turn, scaling
/// by `round(2^127 / (2 pi))`; each term is off by under two units of `2^-127` turn.
const fn atan_step(i: u32) -> i128 {
    if i == 0 {
        return 1 << 93;
    }
    let mut sum = 0_i128;
    let mut k = 0_i128;
    let mut power = i; // i (2k + 1)
    while power < 127 {
        let term = (TURNS_PER_RADIAN_Q127 >> power) / (2 * k + 1);
        sum += if k % 2 == 0 { term } else { -term };
        k += 1;
        power += 2 * i;
    }
    (sum + (1 << 30)) >> 31
}
