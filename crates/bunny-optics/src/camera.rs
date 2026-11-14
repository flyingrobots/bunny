//! The pinhole camera and its samples.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};

use crate::basis::Basis;
use crate::plane::{Lens, Offset, Plane};
use crate::CameraError;

/// The largest accepted image width or height, in pixels (32 768).
pub const MAX_IMAGE_DIMENSION: u32 = 1 << 15;

/// Inclusive bound on the raw tangent of half the vertical field of view (256.0, a vertical
/// field of view just below 179.56 degrees).
pub const TAN_HALF_FOV_LIMIT_RAW: i64 = 1 << 40;

/// What a [`PinholeCamera`] is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraSpec {
    /// The eye point and origin of every ray; it must lie inside the trace domain.
    pub position: FixedVec3,
    /// The viewing direction; any nonzero length that `FixedUnitVec3::new` can normalize.
    pub look: FixedVec3,
    /// A hint that fixes the roll: the camera's up is the part of it orthogonal to `look`.
    pub up: FixedVec3,
    /// The tangent of half the vertical field of view, in `(0, 256]`.
    pub tan_half_fov: FixedQ32_32,
    /// Image width in pixels, in `1..=MAX_IMAGE_DIMENSION`.
    pub width: u32,
    /// Image height in pixels, in `1..=MAX_IMAGE_DIMENSION`.
    pub height: u32,
}

/// A point inside one pixel: column `x`, row `y` and a sub-pixel offset in `[0, 1)` on each
/// axis. Pixel `(0, 0)` is the bottom-left pixel; columns grow toward the camera's right and
/// rows toward its up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelSample {
    /// Column, counted from the left edge.
    pub x: u32,
    /// Row, counted from the bottom edge.
    pub y: u32,
    /// Horizontal offset inside the pixel, in `[0, 1)`.
    pub sub_x: FixedQ32_32,
    /// Vertical offset inside the pixel, in `[0, 1)`.
    pub sub_y: FixedQ32_32,
}

impl PixelSample {
    /// A sample at an explicit sub-pixel offset.
    #[must_use]
    pub const fn new(x: u32, y: u32, sub_x: FixedQ32_32, sub_y: FixedQ32_32) -> Self {
        Self { x, y, sub_x, sub_y }
    }

    /// The centre of pixel `(x, y)`: sub-pixel offset one half on both axes.
    #[must_use]
    pub const fn centre(x: u32, y: u32) -> Self {
        let half = FixedQ32_32::from_raw(ONE_RAW / 2);
        Self::new(x, y, half, half)
    }
}

/// A deterministic pinhole camera that generates [`TraceRay`]s.
///
/// The image is the rectangle on the plane one unit ahead of the position, spanning
/// `-tan..=tan` along up and `-tan * width / height..=tan * width / height` along right.
/// Every ray direction is `2^-scale_shift` times the offset from the position to its sample
/// point on that rectangle, floored to raw Q32.32; see `docs/topics/optics/README.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinholeCamera {
    position: FixedVec3,
    lens: Lens,
    scale_shift: u32,
    plane: Plane,
}

impl PinholeCamera {
    /// Builds a camera.
    ///
    /// # Errors
    /// Returns the [`CameraError`] for the first invalid input, checked in this order:
    /// position, image size, tangent, look direction, up hint. Returns
    /// [`CameraError::Unrepresentable`] if no direction scale fits (not reached by any
    /// accepted input).
    pub fn new(spec: &CameraSpec) -> Result<Self, CameraError> {
        check_position(spec.position)?;
        check_image(spec.width, spec.height)?;
        check_tangent(spec.tan_half_fov)?;
        let basis = Basis::derive(spec.look, spec.up)?;
        let lens = Lens {
            basis,
            tan_raw: spec.tan_half_fov.raw(),
            width: spec.width,
            height: spec.height,
        };
        let (scale_shift, plane) = Plane::fit(&lens).ok_or(CameraError::Unrepresentable)?;
        Ok(Self { position: spec.position, lens, scale_shift, plane })
    }

    /// The ray through `sample`, or `None` when the sample is outside the image: a column or
    /// row at or past the image size, or a sub-pixel offset outside `[0, 1)`.
    ///
    /// Every in-range sample of a constructed camera yields a ray: each direction component is
    /// at most 1.0 in magnitude and the direction points ahead of the camera.
    #[must_use]
    pub fn ray(&self, sample: PixelSample) -> Option<TraceRay> {
        let x = doubled_offset(sample.x, sample.sub_x, self.lens.width)?;
        let y = doubled_offset(sample.y, sample.sub_y, self.lens.height)?;
        let [dx, dy, dz] = self.plane.direction(Offset { x, y }).map(narrow);
        TraceRay::new(self.position, FixedVec3::new(dx?, dy?, dz?))
    }

    /// The eye point, the origin of every ray.
    #[must_use]
    pub const fn position(&self) -> FixedVec3 {
        self.position
    }

    /// The unit viewing direction.
    #[must_use]
    pub const fn forward(&self) -> FixedUnitVec3 {
        self.lens.basis.forward
    }

    /// The unit direction of growing columns: `forward x up`.
    #[must_use]
    pub const fn right(&self) -> FixedUnitVec3 {
        self.lens.basis.right
    }

    /// The unit direction of growing rows, orthogonal to `forward` on the up hint's side.
    #[must_use]
    pub const fn up(&self) -> FixedUnitVec3 {
        self.lens.basis.up
    }

    /// The tangent of half the vertical field of view.
    #[must_use]
    pub const fn tan_half_fov(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.lens.tan_raw)
    }

    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.lens.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.lens.height
    }

    /// `k` such that every direction is `2^-k` times its image-plane offset (before the
    /// floor). A hit at distance `t` is `t * 2^-k` units ahead along `forward`, up to rounding.
    #[must_use]
    pub const fn scale_shift(&self) -> u32 {
        self.scale_shift
    }
}

fn check_position(position: FixedVec3) -> Result<(), CameraError> {
    let inside = |c: FixedQ32_32| c.raw().unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs();
    if inside(position.x) && inside(position.y) && inside(position.z) {
        Ok(())
    } else {
        Err(CameraError::PositionOutsideTraceDomain)
    }
}

const fn check_image(width: u32, height: u32) -> Result<(), CameraError> {
    if width == 0 || height == 0 {
        Err(CameraError::EmptyImage)
    } else if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        Err(CameraError::ImageTooLarge)
    } else {
        Ok(())
    }
}

const fn check_tangent(tan: FixedQ32_32) -> Result<(), CameraError> {
    if tan.raw() <= 0 {
        Err(CameraError::TangentNotPositive)
    } else if tan.raw() > TAN_HALF_FOV_LIMIT_RAW {
        Err(CameraError::TangentTooLarge)
    } else {
        Ok(())
    }
}

/// The raw Q32.32 value of `2 * (index + sub) - size`, or `None` outside the image.
fn doubled_offset(index: u32, sub: FixedQ32_32, size: u32) -> Option<i64> {
    let sub = sub.raw();
    let inside = index < size && (0..ONE_RAW).contains(&sub);
    inside.then(|| (2 * i64::from(index) - i64::from(size)) * ONE_RAW + 2 * sub)
}

fn narrow(component: i128) -> Option<FixedQ32_32> {
    i64::try_from(component).ok().map(FixedQ32_32::from_raw)
}
