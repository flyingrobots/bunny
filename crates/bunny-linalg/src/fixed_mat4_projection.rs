//! Orthographic and perspective projection builders for [`FixedMat4`].
//!
//! Both builders produce `clip_from_view` matrices for Bunny's right-handed
//! view space, where the camera looks down `-Z`. Visible depth maps to NDC
//! `z` in `[0, 1]`: the near plane maps to `0` and the far plane to `1`. See
//! `docs/topics/coordinate-law/README.md`.

use std::fmt;

use bunny_num::FixedQ32_32;

use crate::FixedMat4;

/// Error returned by the projection matrix builders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    /// A half extent, tangent, or aspect ratio was zero or negative.
    NonPositiveExtent,
    /// The near distance was negative, or zero for a perspective projection.
    InvalidNear,
    /// The near distance was not strictly less than the far distance.
    InvalidDepthRange,
    /// A checked Q32.32 operation overflowed, or a divisor rounded to zero.
    Overflow,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonPositiveExtent => "projection extent must be positive",
            Self::InvalidNear => "projection near distance is not valid",
            Self::InvalidDepthRange => "projection near distance must be less than far distance",
            Self::Overflow => "projection arithmetic is outside the Q32.32 range",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProjectionError {}

const NEG_ONE: FixedQ32_32 = FixedQ32_32::from_raw(-bunny_num::fixed_q32_32::ONE_RAW);

impl FixedMat4 {
    /// Builds a symmetric orthographic `clip_from_view` matrix.
    ///
    /// The view volume spans `[-half_width, half_width]` in `x`,
    /// `[-half_height, half_height]` in `y`, and distances `near..=far` in
    /// front of the camera (view-space `z` from `-near` to `-far`). Depth maps
    /// to NDC `z` in `[0, 1]`. The last row is `(0, 0, 0, 1)`.
    ///
    /// # Errors
    /// Returns [`ProjectionError::NonPositiveExtent`] when a half extent is
    /// not positive, [`ProjectionError::InvalidNear`] when `near` is negative,
    /// [`ProjectionError::InvalidDepthRange`] when `near >= far`, and
    /// [`ProjectionError::Overflow`] when checked arithmetic fails.
    pub fn try_orthographic(
        half_width: FixedQ32_32,
        half_height: FixedQ32_32,
        near: FixedQ32_32,
        far: FixedQ32_32,
    ) -> Result<Self, ProjectionError> {
        require_positive(half_width)?;
        require_positive(half_height)?;
        if near < FixedQ32_32::ZERO {
            return Err(ProjectionError::InvalidNear);
        }
        let depth = depth_range(near, far)?;
        let m22 = negate(divide(FixedQ32_32::ONE, depth)?)?;
        let m23 = negate(divide(near, depth)?)?;
        Ok(from_diagonal_and_depth(
            divide(FixedQ32_32::ONE, half_width)?,
            divide(FixedQ32_32::ONE, half_height)?,
            (m22, m23),
            (FixedQ32_32::ZERO, FixedQ32_32::ONE),
        ))
    }

    /// Builds a symmetric perspective `clip_from_view` matrix.
    ///
    /// `tan_half_fov_y` is the tangent of half the vertical field of view, so
    /// no angle function is needed. `aspect` is width divided by height.
    /// Depth maps to NDC `z` in `[0, 1]` after the divide by `w = -z_view`.
    /// The last row is `(0, 0, -1, 0)`.
    ///
    /// # Errors
    /// Returns [`ProjectionError::NonPositiveExtent`] when the tangent or
    /// aspect is not positive, [`ProjectionError::InvalidNear`] when `near` is
    /// not positive, [`ProjectionError::InvalidDepthRange`] when
    /// `near >= far`, and [`ProjectionError::Overflow`] when checked
    /// arithmetic fails, including `far * near`.
    pub fn try_perspective(
        tan_half_fov_y: FixedQ32_32,
        aspect: FixedQ32_32,
        near: FixedQ32_32,
        far: FixedQ32_32,
    ) -> Result<Self, ProjectionError> {
        require_positive(tan_half_fov_y)?;
        require_positive(aspect)?;
        if near <= FixedQ32_32::ZERO {
            return Err(ProjectionError::InvalidNear);
        }
        let depth = depth_range(near, far)?;
        let half_width_scale = checked(tan_half_fov_y.checked_mul(aspect))?;
        let m22 = negate(divide(far, depth)?)?;
        let m23 = negate(divide(checked(far.checked_mul(near))?, depth)?)?;
        Ok(from_diagonal_and_depth(
            divide(FixedQ32_32::ONE, half_width_scale)?,
            divide(FixedQ32_32::ONE, tan_half_fov_y)?,
            (m22, m23),
            (NEG_ONE, FixedQ32_32::ZERO),
        ))
    }
}

fn from_diagonal_and_depth(
    m00: FixedQ32_32,
    m11: FixedQ32_32,
    depth: (FixedQ32_32, FixedQ32_32),
    last_row_zw: (FixedQ32_32, FixedQ32_32),
) -> FixedMat4 {
    let zero = FixedQ32_32::ZERO;
    FixedMat4::from_rows(
        (m00, zero, zero, zero),
        (zero, m11, zero, zero),
        (zero, zero, depth.0, depth.1),
        (zero, zero, last_row_zw.0, last_row_zw.1),
    )
}

fn require_positive(value: FixedQ32_32) -> Result<(), ProjectionError> {
    if value > FixedQ32_32::ZERO {
        Ok(())
    } else {
        Err(ProjectionError::NonPositiveExtent)
    }
}

fn depth_range(near: FixedQ32_32, far: FixedQ32_32) -> Result<FixedQ32_32, ProjectionError> {
    if near >= far {
        return Err(ProjectionError::InvalidDepthRange);
    }
    checked(far.checked_sub(near))
}

fn checked(value: Option<FixedQ32_32>) -> Result<FixedQ32_32, ProjectionError> {
    value.ok_or(ProjectionError::Overflow)
}

fn divide(numerator: FixedQ32_32, divisor: FixedQ32_32) -> Result<FixedQ32_32, ProjectionError> {
    checked(numerator.checked_div(divisor))
}

fn negate(value: FixedQ32_32) -> Result<FixedQ32_32, ProjectionError> {
    checked(value.checked_neg())
}
