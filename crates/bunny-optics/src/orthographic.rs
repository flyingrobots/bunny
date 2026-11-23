//! The orthographic camera `bunny-orthographic-camera/1`: parallel rays along `forward`, one
//! from each sample's point on a view rectangle centred on the position.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};

use crate::basis::Basis;
use crate::camera::{check_image, check_position, doubled_offset, narrow};
use crate::plane::{round_div, wide, Wide3};
use crate::{CameraError, PixelSample};

/// The versioned name of [`OrthographicCamera::ray`]. Any change to its output is a new
/// version.
pub const ORTHOGRAPHIC_CAMERA_ID: &str = "bunny-orthographic-camera/1";

/// Inclusive bound on the raw half height of the view rectangle (256.0 units, the trace
/// domain's half width). It keeps every product of the origin formula below `2^120`.
pub const ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW: i64 = 1 << 40;

/// What an [`OrthographicCamera`] is built from: a [`crate::CameraSpec`] with the tangent of
/// half the field of view replaced by half the height of the view rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrthographicSpec {
    /// The centre of the view rectangle; it must lie inside the trace domain.
    pub position: FixedVec3,
    /// The viewing direction, the direction of every ray; any nonzero length.
    pub look: FixedVec3,
    /// A hint that fixes the roll: the camera's up is the part of it orthogonal to `look`.
    pub up: FixedVec3,
    /// Half the height of the view rectangle in world units, in `(0, 256]`; half its width is
    /// `half_height * width / height`.
    pub half_height: FixedQ32_32,
    /// Image width in pixels, in `1..=MAX_IMAGE_DIMENSION`.
    pub width: u32,
    /// Image height in pixels, in `1..=MAX_IMAGE_DIMENSION`.
    pub height: u32,
}

/// Why an [`OrthographicCamera`] could not be built from an [`OrthographicSpec`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrthographicError {
    /// A position coordinate is outside the trace domain: `abs(raw) >= 2^40`, 256 units.
    PositionOutsideTraceDomain,
    /// The image width or height is zero.
    EmptyImage,
    /// The image width or height exceeds [`crate::MAX_IMAGE_DIMENSION`].
    ImageTooLarge,
    /// The half height is zero or negative.
    HalfHeightNotPositive,
    /// The half height exceeds [`ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW`] (256.0).
    HalfHeightTooLarge,
    /// The look direction is the zero vector.
    ZeroLook,
    /// The up hint is zero, or parallel or nearly parallel to the look direction, by the
    /// pinhole camera's rule ([`CameraError::UpParallelToLook`]).
    UpParallelToLook,
    /// A corner of the view rectangle has a rounded coordinate outside the trace domain, so
    /// some sample's ray origin could be.
    ViewOutsideTraceDomain,
    /// A basis vector could not be normalized. This is a defensive refusal: no accepted input
    /// is known to reach it.
    Unrepresentable,
}

impl core::fmt::Display for OrthographicError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::PositionOutsideTraceDomain => "camera position is outside the trace domain",
            Self::EmptyImage => "image width and height must be positive",
            Self::ImageTooLarge => "image width or height exceeds the camera limit",
            Self::HalfHeightNotPositive => "half height of the view must be positive",
            Self::HalfHeightTooLarge => "half height of the view exceeds the limit",
            Self::ZeroLook => "look direction is zero",
            Self::UpParallelToLook => "up hint is zero or parallel to the look direction",
            Self::ViewOutsideTraceDomain => "view rectangle reaches outside the trace domain",
            Self::Unrepresentable => "a basis vector could not be normalized",
        })
    }
}

impl std::error::Error for OrthographicError {}

impl From<CameraError> for OrthographicError {
    fn from(error: CameraError) -> Self {
        match error {
            CameraError::PositionOutsideTraceDomain => Self::PositionOutsideTraceDomain,
            CameraError::EmptyImage => Self::EmptyImage,
            CameraError::ImageTooLarge => Self::ImageTooLarge,
            CameraError::ZeroLook => Self::ZeroLook,
            CameraError::UpParallelToLook => Self::UpParallelToLook,
            // The shared checks this camera calls return none of the tangent refusals.
            CameraError::TangentNotPositive
            | CameraError::TangentTooLarge
            | CameraError::Unrepresentable => Self::Unrepresentable,
        }
    }
}

/// A deterministic orthographic camera that generates [`TraceRay`]s.
///
/// The view rectangle is centred on the position, spanned by `right` and `up`, and spans
/// `-half_height..=half_height` along up and `-half_height * width / height..=` its negation
/// along right. Every ray has the direction `forward` and starts at its sample's point on the
/// rectangle, rounded once per component; see `docs/topics/optics/README.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrthographicCamera {
    position: FixedVec3,
    basis: Basis,
    half_height_raw: i64,
    width: u32,
    height: u32,
}

impl OrthographicCamera {
    /// Builds a camera.
    ///
    /// # Errors
    /// Returns the [`OrthographicError`] for the first invalid input, checked in this order:
    /// position, image size, half height, look direction, up hint, view rectangle.
    pub fn new(spec: &OrthographicSpec) -> Result<Self, OrthographicError> {
        check_position(spec.position)?;
        check_image(spec.width, spec.height)?;
        check_half_height(spec.half_height)?;
        let camera = Self {
            position: spec.position,
            basis: Basis::derive(spec.look, spec.up)?,
            half_height_raw: spec.half_height.raw(),
            width: spec.width,
            height: spec.height,
        };
        camera.check_view()?;
        Ok(camera)
    }

    /// The ray through `sample`, or `None` when the sample is outside the image, as for
    /// [`crate::PinholeCamera::ray`].
    ///
    /// Its direction is [`Self::forward`]. Its origin is `position + o`, where
    /// `o_i = round((a h R_i + b h U_i) / (height 2^64))`, to nearest with ties to even, from
    /// the exact `i128` numerator (`a`, `b` the raw doubled offsets, `h` the raw half height,
    /// `R`, `U` the raw basis): within half a raw unit of the exact point. Every in-range
    /// sample of a constructed camera yields a ray.
    #[must_use]
    pub fn ray(&self, sample: PixelSample) -> Option<TraceRay> {
        let across = doubled_offset(sample.x, sample.sub_x, self.width)?;
        // Rows grow downward, against `up`.
        let along_up = -doubled_offset(sample.y, sample.sub_y, self.height)?;
        let [x, y, z] = self.origin(across, along_up).map(narrow);
        TraceRay::new(FixedVec3::new(x?, y?, z?), self.basis.forward.into_inner())
    }

    /// The exact origin of doubled raw offsets `(a, b)`, rounded once per component.
    fn origin(&self, a: i64, b: i64) -> Wide3 {
        let half = i128::from(self.half_height_raw);
        let (a, b) = (i128::from(a) * half, i128::from(b) * half);
        let divisor = i128::from(self.height) << (2 * FixedQ32_32::FRACTIONAL_BITS);
        let [[r0, r1, r2], [u0, u1, u2]] = [wide(self.basis.right), wide(self.basis.up)];
        let [p0, p1, p2] = raw3(self.position);
        let axis = |p: i128, r: i128, u: i128| p + round_div(a * r + b * u, divisor);
        [axis(p0, r0, u0), axis(p1, r1, u1), axis(p2, r2, u2)]
    }

    /// Refuses a view whose rounded corners leave the trace domain. Each origin component is
    /// a monotone rounding of an affine function of `(a, b)`, so over the closed rectangle
    /// `|a| <= width`, `|b| <= height` it lies between its values at the four corners.
    fn check_view(&self) -> Result<(), OrthographicError> {
        let (w, h) = (i64::from(self.width) * ONE_RAW, i64::from(self.height) * ONE_RAW);
        let limit = i128::from(TRACE_COORD_LIMIT_RAW);
        let corners = [(-w, -h), (w, -h), (-w, h), (w, h)].map(|(a, b)| self.origin(a, b));
        if corners.iter().flatten().all(|c| c.abs() < limit) {
            Ok(())
        } else {
            Err(OrthographicError::ViewOutsideTraceDomain)
        }
    }

    /// The centre of the view rectangle.
    #[must_use]
    pub const fn position(&self) -> FixedVec3 {
        self.position
    }

    /// The unit viewing direction, the direction of every ray.
    #[must_use]
    pub const fn forward(&self) -> FixedUnitVec3 {
        self.basis.forward
    }

    /// The unit direction of growing columns: `forward x up`.
    #[must_use]
    pub const fn right(&self) -> FixedUnitVec3 {
        self.basis.right
    }

    /// The unit up direction, orthogonal to `forward` on the up hint's side; rows grow against it.
    #[must_use]
    pub const fn up(&self) -> FixedUnitVec3 {
        self.basis.up
    }

    /// Half the height of the view rectangle.
    #[must_use]
    pub const fn half_height(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.half_height_raw)
    }

    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

const fn check_half_height(half_height: FixedQ32_32) -> Result<(), OrthographicError> {
    if half_height.raw() <= 0 {
        Err(OrthographicError::HalfHeightNotPositive)
    } else if half_height.raw() > ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW {
        Err(OrthographicError::HalfHeightTooLarge)
    } else {
        Ok(())
    }
}

fn raw3(v: FixedVec3) -> Wide3 {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}
