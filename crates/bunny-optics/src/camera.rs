//! The pinhole camera and its samples (planned; not implemented yet).

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::CameraError;

/// The largest accepted image width or height, in pixels (32 768).
pub const MAX_IMAGE_DIMENSION: u32 = 1 << 15;

/// Inclusive bound on the raw tangent of half the vertical field of view (256.0).
pub const TAN_HALF_FOV_LIMIT_RAW: i64 = 1 << 40;

/// What a [`PinholeCamera`] is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraSpec {
    /// The eye point.
    pub position: FixedVec3,
    /// The viewing direction.
    pub look: FixedVec3,
    /// A hint that fixes the roll.
    pub up: FixedVec3,
    /// The tangent of half the vertical field of view.
    pub tan_half_fov: FixedQ32_32,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// A point inside one pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelSample {
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
    /// Horizontal offset inside the pixel.
    pub sub_x: FixedQ32_32,
    /// Vertical offset inside the pixel.
    pub sub_y: FixedQ32_32,
}

impl PixelSample {
    /// A sample at an explicit sub-pixel offset.
    #[must_use]
    pub const fn new(x: u32, y: u32, sub_x: FixedQ32_32, sub_y: FixedQ32_32) -> Self {
        Self { x, y, sub_x, sub_y }
    }

    /// The centre of pixel `(x, y)`.
    #[must_use]
    pub const fn centre(x: u32, y: u32) -> Self {
        let half = FixedQ32_32::from_raw(ONE_RAW / 2);
        Self::new(x, y, half, half)
    }
}

/// A pinhole camera (planned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinholeCamera {
    spec: CameraSpec,
}

impl PinholeCamera {
    /// Builds a camera (planned: always refuses).
    ///
    /// # Errors
    /// Always returns [`CameraError::Unrepresentable`] until the camera is implemented.
    pub const fn new(spec: &CameraSpec) -> Result<Self, CameraError> {
        let _ = spec;
        Err(CameraError::Unrepresentable)
    }

    /// The ray through `sample` (planned: always `None`).
    #[must_use]
    pub const fn ray(&self, sample: PixelSample) -> Option<TraceRay> {
        let _ = (self, sample);
        None
    }

    /// The eye point.
    #[must_use]
    pub const fn position(&self) -> FixedVec3 {
        self.spec.position
    }

    /// The unit viewing direction (planned).
    #[must_use]
    pub const fn forward(&self) -> FixedUnitVec3 {
        FixedUnitVec3::NEG_UNIT_Z
    }

    /// The unit right direction (planned).
    #[must_use]
    pub const fn right(&self) -> FixedUnitVec3 {
        FixedUnitVec3::UNIT_X
    }

    /// The unit up direction (planned).
    #[must_use]
    pub const fn up(&self) -> FixedUnitVec3 {
        FixedUnitVec3::UNIT_Y
    }

    /// The tangent of half the vertical field of view.
    #[must_use]
    pub const fn tan_half_fov(&self) -> FixedQ32_32 {
        self.spec.tan_half_fov
    }

    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.spec.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.spec.height
    }

    /// The direction scale shift (planned).
    #[must_use]
    pub const fn scale_shift(&self) -> u32 {
        0
    }
}
