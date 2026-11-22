//! Viewport mapping between normalized device coordinates and viewport
//! coordinates.
//!
//! The viewport origin is the top-left corner, pixel `y` runs down, and a
//! coordinate is continuous: integers fall on pixel corners, so pixel `i`
//! spans `[i, i + 1)` and its centre is `i + 1/2`. Depth passes through
//! unchanged. Choosing the pixel a coordinate belongs to is the caller's job.
//! See `docs/topics/coordinate-law/README.md`.

use std::fmt;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::FixedVec3;

/// Error returned by the viewport mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportError {
    /// The viewport width or height was zero.
    EmptyExtent,
    /// A result was outside the Q32.32 range.
    Overflow,
}

impl fmt::Display for ViewportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyExtent => "viewport width and height must be positive",
            Self::Overflow => "viewport arithmetic is outside the Q32.32 range",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ViewportError {}

/// A viewport of `width` by `height` device pixels with its origin at the
/// top-left corner and `y` running down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    width: u32,
    height: u32,
}

impl Viewport {
    /// Creates a viewport of the given size in device pixels.
    ///
    /// # Errors
    /// Returns [`ViewportError::EmptyExtent`] when `width` or `height` is zero.
    pub const fn try_new(width: u32, height: u32) -> Result<Self, ViewportError> {
        if width == 0 || height == 0 {
            Err(ViewportError::EmptyExtent)
        } else {
            Ok(Self { width, height })
        }
    }

    /// Width in device pixels.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Height in device pixels.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Maps NDC to continuous viewport coordinates.
    ///
    /// `x = (x_ndc + 1) * width / 2` and `y = (1 - y_ndc) * height / 2`, each
    /// floored in raw Q32.32 units; `z` is returned unchanged. Points outside
    /// the NDC cube map linearly and are not clipped.
    ///
    /// # Errors
    /// Returns [`ViewportError::Overflow`] when a result does not fit Q32.32.
    pub fn try_ndc_to_viewport(self, ndc: FixedVec3) -> Result<FixedVec3, ViewportError> {
        let x = scale_to_pixels(i128::from(ndc.x.raw()) + one(), self.width)?;
        let y = scale_to_pixels(one() - i128::from(ndc.y.raw()), self.height)?;
        Ok(FixedVec3::new(x, y, ndc.z))
    }

    /// Maps continuous viewport coordinates back to NDC.
    ///
    /// `x_ndc = 2 * x / width - 1` and `y_ndc = 1 - 2 * y / height`, with the
    /// quotient floored in raw Q32.32 units; `z` is returned unchanged.
    ///
    /// # Errors
    /// Returns [`ViewportError::Overflow`] when a result does not fit Q32.32.
    pub fn try_viewport_to_ndc(self, pixel: FixedVec3) -> Result<FixedVec3, ViewportError> {
        let x = to_fixed(scale_to_ndc(pixel.x, self.width) - one())?;
        let y = to_fixed(one() - scale_to_ndc(pixel.y, self.height))?;
        Ok(FixedVec3::new(x, y, pixel.z))
    }
}

fn one() -> i128 {
    i128::from(ONE_RAW)
}

/// `floor(shifted * size / 2)` where `shifted` is an NDC value moved to `[0, 2]`.
fn scale_to_pixels(shifted: i128, size: u32) -> Result<FixedQ32_32, ViewportError> {
    to_fixed((shifted * i128::from(size)) >> 1)
}

/// `floor(2 * pixel / size)`: the raw NDC offset from the low edge of the cube.
fn scale_to_ndc(pixel: FixedQ32_32, size: u32) -> i128 {
    (i128::from(pixel.raw()) * 2).div_euclid(i128::from(size))
}

fn to_fixed(raw: i128) -> Result<FixedQ32_32, ViewportError> {
    i64::try_from(raw).map(FixedQ32_32::from_raw).map_err(|_| ViewportError::Overflow)
}
