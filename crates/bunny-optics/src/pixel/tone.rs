//! Tone curves, output codes and the RGBA8 writer.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use super::{div_round_half_even, srgb, PixelError};

/// Identity of [`quantize_8`]: a display value in `[0, 1]` to `round(255 * v)`.
pub const UNORM8_IDENTITY: &str = "bunny-unorm8/1";

/// Identity of [`quantize_16`]: a display value in `[0, 1]` to `round(65535 * v)`.
pub const UNORM16_IDENTITY: &str = "bunny-unorm16/1";

/// Identity of the bytes [`write_rgba8`] writes: red, green and blue as
/// [`UNORM8_IDENTITY`] codes of the toned channels, then alpha 255.
pub const RGBA8_OPAQUE_IDENTITY: &str = "bunny-rgba8-opaque/1";

/// A tone curve: linear radiance to a display-referred value in `[0, 1]`.
///
/// Each variant is a versioned contract. Its output bits never change; a curve with any
/// different output is a new variant with a new identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToneCurve {
    /// `bunny-tone-srgb/1`: the sRGB transfer function (IEC 61966-2-1 constants) on the clamped
    /// input, from a 1 153-knot table; within `1.5 * 10^-6` of the exact curve.
    SrgbV1,
    /// `bunny-tone-linear-clamp/1`: the input clamped to `[0, 1]`, unchanged otherwise.
    LinearClampV1,
}

impl ToneCurve {
    /// The stable name and version of this curve.
    #[must_use]
    pub const fn identity(self) -> &'static str {
        match self {
            Self::SrgbV1 => "bunny-tone-srgb/1",
            Self::LinearClampV1 => "bunny-tone-linear-clamp/1",
        }
    }

    /// The display-referred value of one linear channel, in `[0, 1]` (raw `0..=2^32`).
    ///
    /// Inputs at or below zero give 0 and inputs at or above 1.0 give 1.0.
    #[must_use]
    pub fn apply(self, linear: FixedQ32_32) -> FixedQ32_32 {
        FixedQ32_32::from_raw(match self {
            Self::SrgbV1 => srgb::encode(linear.raw()),
            Self::LinearClampV1 => linear.raw().clamp(0, ONE_RAW),
        })
    }

    /// The 8-bit code of one linear channel: [`quantize_8`] of [`ToneCurve::apply`].
    #[must_use]
    pub fn encode_8(self, linear: FixedQ32_32) -> u8 {
        quantize_8(self.apply(linear))
    }

    /// The 16-bit code of one linear channel: [`quantize_16`] of [`ToneCurve::apply`].
    #[must_use]
    pub fn encode_16(self, linear: FixedQ32_32) -> u16 {
        quantize_16(self.apply(linear))
    }
}

/// The 8-bit code of a display value: `round(255 * v)`, to nearest with ties to even, after
/// clamping `v` to `[0, 1]`.
#[must_use]
pub fn quantize_8(display: FixedQ32_32) -> u8 {
    u8::try_from(quantize(display, 255)).unwrap_or(u8::MAX)
}

/// The 16-bit code of a display value: `round(65535 * v)`, to nearest with ties to even, after
/// clamping `v` to `[0, 1]`.
#[must_use]
pub fn quantize_16(display: FixedQ32_32) -> u16 {
    u16::try_from(quantize(display, 65_535)).unwrap_or(u16::MAX)
}

/// `round(max_code * v / 2^32)` for the clamped raw value `v`; at most `max_code`.
///
/// `v <= 2^32` and `max_code < 2^16`, so the product is below `2^48`.
const fn quantize(display: FixedQ32_32, max_code: i64) -> i64 {
    let raw = display.raw();
    let clamped = if raw < 0 {
        0
    } else if raw > ONE_RAW {
        ONE_RAW
    } else {
        raw
    };
    div_round_half_even(clamped * max_code, ONE_RAW)
}

/// Writes one opaque RGBA8 pixel: each linear channel (red, green, blue) through `curve` and
/// [`quantize_8`], then alpha 255 ([`RGBA8_OPAQUE_IDENTITY`]).
///
/// # Errors
///
/// [`PixelError::WrongSliceLength`] when `out` is not exactly four bytes long; `out` is then
/// unchanged.
pub fn write_rgba8(
    out: &mut [u8],
    curve: ToneCurve,
    linear: [FixedQ32_32; 3],
) -> Result<(), PixelError> {
    let actual = out.len();
    let pixel: &mut [u8; 4] =
        out.try_into().map_err(|_| PixelError::WrongSliceLength { expected: 4, actual })?;
    let [r, g, b] = linear.map(|channel| curve.encode_8(channel));
    *pixel = [r, g, b, u8::MAX];
    Ok(())
}
