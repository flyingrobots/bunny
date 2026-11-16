//! Decoding 8-bit sRGB codes to linear light.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use super::srgb8_table::SRGB8_LINEAR;

/// Identity of [`srgb8_to_linear`]: an 8-bit sRGB code to its exact linear value, rounded to
/// nearest, from a 256-entry table.
pub const SRGB8_DECODE_IDENTITY: &str = "bunny-srgb8-decode/1";

/// The linear light of an 8-bit sRGB code, in `[0, 1]` (raw `0..=2^32`): code 0 gives 0 and
/// code 255 gives 1.0 exactly.
///
/// Each of the 256 results is the exact sRGB decode rounded to nearest to a raw Q32.32 unit
/// (`bunny-srgb8-decode/1`); the table is the contract and any different output is a new
/// version.
#[must_use]
#[inline]
pub fn srgb8_to_linear(code: u8) -> FixedQ32_32 {
    // The table has 256 entries, so every `u8` finds one; the fallback is unreachable.
    FixedQ32_32::from_raw(SRGB8_LINEAR.get(usize::from(code)).copied().unwrap_or(ONE_RAW))
}
