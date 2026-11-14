//! The sRGB transfer function on raw Q32.32 values, with integer arithmetic only.

use bunny_num::fixed_q32_32::ONE_RAW;

use super::div_round_half_even;
use super::srgb_knots::{KNOTS, SUB_SEGMENT_BITS};

/// The first raw input of the power segment: `ceil(0.0031308 * 2^32)`. Inputs below it use
/// the linear toe.
pub const TOE_END_RAW: i64 = 13_446_684;

/// The toe's value at the last toe input, `round(12.92 * (TOE_END_RAW - 1))`. The standard
/// curve's power segment starts about 115 raw units below it, so the power segment is held at
/// this value until it rises past it (23 inputs), which keeps the curve monotone.
const TOE_LAST: i64 = 173_731_144;

/// The octave of the lowest knot: knots start at `2^(31 - LOWEST_OCTAVE)` raw, `2^-9`.
const LOWEST_OCTAVE: u32 = 8;

/// Display-referred sRGB value of a linear value, both raw Q32.32.
///
/// At or below zero: 0. At or above 1.0: 1.0. Below [`TOE_END_RAW`]: `12.92 * x` rounded to
/// nearest. Otherwise the power segment, interpolated from [`KNOTS`] and floored, but never
/// below [`TOE_LAST`].
pub fn encode(linear: i64) -> i64 {
    // Stub: failing evidence for OPTICS-TP-014 to 018.
    let _ = (linear, TOE_END_RAW, TOE_LAST, LOWEST_OCTAVE, KNOTS, SUB_SEGMENT_BITS, ONE_RAW);
    let _ = div_round_half_even(0, 1);
    0
}
