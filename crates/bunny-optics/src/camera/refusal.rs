//! The diagnostic path of [`PinholeCamera::ray`]: the same ray, with the reason for a refusal
//! (flyingrobots/bunny#445).

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceRayError};

use super::{narrow, sample_direction, PinholeCamera, PixelSample};

/// Why [`PinholeCamera::try_ray`] refused a sample. Each variant carries the values the check
/// saw.
///
/// [`PinholeCamera::ray`] returns `None` exactly when `try_ray` returns an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraRayError {
    /// The column is at or past the image width.
    ColumnOutsideImage {
        /// The sample's column, as given.
        column: u32,
        /// The camera's image width in pixels.
        width: u32,
    },
    /// The horizontal sub-pixel offset is outside `[0, 1)`: below raw 0 or at least raw `2^32`.
    SubXOutsideUnitInterval {
        /// The offset, as given.
        sub_x: FixedQ32_32,
    },
    /// The row is at or past the image height.
    RowOutsideImage {
        /// The sample's row, as given.
        row: u32,
        /// The camera's image height in pixels.
        height: u32,
    },
    /// The vertical sub-pixel offset is outside `[0, 1)`: below raw 0 or at least raw `2^32`.
    SubYOutsideUnitInterval {
        /// The offset, as given.
        sub_y: FixedQ32_32,
    },
    /// A direction component does not fit Q32.32. This is a defensive refusal: every
    /// in-range sample of a constructed camera has components of at most 1.0 in magnitude, so
    /// no input is known to reach it.
    DirectionUnrepresentable,
    /// `TraceRay::try_new` refused the ray. This is a defensive refusal: the position is in
    /// the trace domain and every in-range direction is nonzero with components of at most
    /// 1.0, so no input is known to reach it.
    RayRefused {
        /// Why the trace kernel refused it; also [`core::error::Error::source`].
        reason: TraceRayError,
    },
}

impl PinholeCamera {
    /// The diagnostic path of [`Self::ray`]: the same ray, with the reason for a refusal. Call
    /// it when `ray` returns `None`; `ray` is the fast path. `OPTICS-TP-335` checks over
    /// generated samples that `ray` returns `Some(r)` exactly when this returns `Ok(r)`.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`CameraRayError::ColumnOutsideImage`] when `sample.x` is at
    /// least the width; [`CameraRayError::SubXOutsideUnitInterval`] when `sample.sub_x` is
    /// outside `[0, 1)`; [`CameraRayError::RowOutsideImage`] when `sample.y` is at least the
    /// height; [`CameraRayError::SubYOutsideUnitInterval`] when `sample.sub_y` is outside
    /// `[0, 1)`. The defensive [`CameraRayError::DirectionUnrepresentable`] and
    /// [`CameraRayError::RayRefused`] are reached by no known input.
    pub fn try_ray(&self, sample: PixelSample) -> Result<TraceRay, CameraRayError> {
        check_sample(self.width(), self.height(), sample)?;
        let wide = sample_direction(&self.plane, &self.lens, sample)
            .ok_or(CameraRayError::DirectionUnrepresentable)?;
        let [dx, dy, dz] = wide.map(narrow);
        let (Some(dx), Some(dy), Some(dz)) = (dx, dy, dz) else {
            return Err(CameraRayError::DirectionUnrepresentable);
        };
        TraceRay::try_new(self.position, FixedVec3::new(dx, dy, dz))
            .map_err(|reason| CameraRayError::RayRefused { reason })
    }
}

/// The sample checks of the image-plane offset, in the order it makes them.
fn check_sample(width: u32, height: u32, sample: PixelSample) -> Result<(), CameraRayError> {
    let unit = |offset: FixedQ32_32| (0..ONE_RAW).contains(&offset.raw());
    let PixelSample { x, y, sub_x, sub_y } = sample;
    if x >= width {
        return Err(CameraRayError::ColumnOutsideImage { column: x, width });
    }
    if !unit(sub_x) {
        return Err(CameraRayError::SubXOutsideUnitInterval { sub_x });
    }
    if y >= height {
        return Err(CameraRayError::RowOutsideImage { row: y, height });
    }
    if unit(sub_y) {
        Ok(())
    } else {
        Err(CameraRayError::SubYOutsideUnitInterval { sub_y })
    }
}

impl fmt::Display for CameraRayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnOutsideImage { column, width } => {
                write!(f, "sample column {column} is outside an image {width} pixels wide")
            }
            Self::SubXOutsideUnitInterval { sub_x } => {
                write!(f, "sub-pixel x raw {} is outside [0, 1)", sub_x.raw())
            }
            Self::RowOutsideImage { row, height } => {
                write!(f, "sample row {row} is outside an image {height} pixels high")
            }
            Self::SubYOutsideUnitInterval { sub_y } => {
                write!(f, "sub-pixel y raw {} is outside [0, 1)", sub_y.raw())
            }
            Self::DirectionUnrepresentable => {
                f.write_str("ray direction is not representable in Q32.32")
            }
            Self::RayRefused { reason } => write!(f, "camera ray is not a trace ray: {reason}"),
        }
    }
}

impl core::error::Error for CameraRayError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::RayRefused { reason } => Some(reason),
            _ => None,
        }
    }
}
