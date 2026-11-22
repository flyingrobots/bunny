//! Why a camera could not be built.

use core::fmt;

/// Why a [`crate::PinholeCamera`] could not be built from a [`crate::CameraSpec`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraError {
    /// A position coordinate is outside the trace domain: `abs(raw) >= 2^40`, 256 units.
    PositionOutsideTraceDomain,
    /// The image width or height is zero.
    EmptyImage,
    /// The image width or height exceeds [`crate::MAX_IMAGE_DIMENSION`].
    ImageTooLarge,
    /// The tangent of half the vertical field of view is zero or negative.
    TangentNotPositive,
    /// The tangent exceeds [`crate::TAN_HALF_FOV_LIMIT_RAW`] (256.0).
    TangentTooLarge,
    /// The look direction is the zero vector.
    ZeroLook,
    /// The up hint is zero, or parallel or nearly parallel to the look direction (every
    /// component of `unit(look) x unit(up)` below `2^16` raw units), so it fixes no roll.
    UpParallelToLook,
    /// No scale keeps every ray direction inside the trace domain with a positive forward
    /// component. This is a defensive refusal: no accepted input is known to reach it.
    Unrepresentable,
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PositionOutsideTraceDomain => "camera position is outside the trace domain",
            Self::EmptyImage => "image width and height must be positive",
            Self::ImageTooLarge => "image width or height exceeds the camera limit",
            Self::TangentNotPositive => "tangent of half the field of view must be positive",
            Self::TangentTooLarge => "tangent of half the field of view exceeds the limit",
            Self::ZeroLook => "look direction is zero",
            Self::UpParallelToLook => "up hint is zero or parallel to the look direction",
            Self::Unrepresentable => "no scale keeps every ray inside the trace domain",
        })
    }
}

impl std::error::Error for CameraError {}
