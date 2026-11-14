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
#[inline]
pub fn encode(linear: i64) -> i64 {
    if linear <= 0 {
        0
    } else if linear >= ONE_RAW {
        ONE_RAW
    } else if linear < TOE_END_RAW {
        // linear < 2^24, so the product is below 2^35.
        div_round_half_even(linear * 1292, 100)
    } else {
        power_segment(linear).max(TOE_LAST)
    }
}

/// Linear interpolation between the two knots around `linear`, in `[TOE_END_RAW, 2^32)`.
///
/// The octave `e` (`linear` in `[2^(31 - e), 2^(32 - e))`) is in `0..=8` because
/// `2^23 < TOE_END_RAW`. Each octave holds `2^SUB_SEGMENT_BITS` segments of
/// `2^(24 - e)` raw units, so the fraction is below `2^24` and the knot difference below
/// `2^32`: the product stays below `2^56`.
#[inline]
fn power_segment(linear: i64) -> i64 {
    let octave = linear.leading_zeros() - 32;
    let shift = 31 - octave - SUB_SEGMENT_BITS;
    let offset = linear - (1 << (31 - octave));
    let segment = i64::from((LOWEST_OCTAVE - octave) << SUB_SEGMENT_BITS) + (offset >> shift);
    let fraction = offset & ((1 << shift) - 1);
    let knots = usize::try_from(segment).ok().and_then(|i| KNOTS.get(i..));
    match knots.and_then(<[i64]>::first_chunk::<2>) {
        Some(&[low, high]) => low + (((high - low) * fraction) >> shift),
        // Unreachable: `segment` is at most 9 * 128 - 1 = 1151, below the 1153 knots.
        None => ONE_RAW,
    }
}
