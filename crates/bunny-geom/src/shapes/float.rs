//! Float boundary constructors for the shape types, built only from the existing component
//! conversions (`FixedVec3::try_from_float`, `FixedQ32_32::try_from_f32`): each float is
//! rounded to Q32.32 from its exact value, ties to even, with no float arithmetic.
//!
//! Every constructor first refuses a non-finite input, then a negative float radius or half
//! extent (before rounding, as the sphere's float path does), then a value outside Q32.32,
//! and finally applies the fixed constructor's own refusals to the rounded values.

use bunny_linalg::{FixedMat3, FixedVec3, Vec3};
// dojo: allow float-boundary -- imports the f32 DTO alias for the shape ingress checks
use bunny_num::{is_finite, FixedQ32_32, FloatConversionError, Scalar};

use super::{FixedCapsule3, FixedLine3, FixedObb3, FixedTriangle3, ShapeError};
use crate::{vec3_is_finite, FixedPlane3, FixedSegment3};

const fn shape_error(error: FloatConversionError) -> ShapeError {
    match error {
        FloatConversionError::NonFinite => ShapeError::NonFiniteCoordinate,
        FloatConversionError::OutOfRange => ShapeError::FixedValueOutOfRange,
    }
}

fn require_finite(values: &[Vec3]) -> Result<(), ShapeError> {
    if values.iter().all(|&v| vec3_is_finite(v)) {
        Ok(())
    } else {
        Err(ShapeError::NonFiniteCoordinate)
    }
}

fn fixed_vec3(v: Vec3) -> Result<FixedVec3, ShapeError> {
    FixedVec3::try_from_float(v).map_err(shape_error)
}

// dojo: allow float-boundary -- validated f32 DTO ingress into canonical Q32.32
fn fixed_scalar(value: Scalar) -> Result<FixedQ32_32, ShapeError> {
    FixedQ32_32::try_from_f32(value).map_err(shape_error)
}

// dojo: allow float-boundary -- sign test of a finite f32 boundary value; -0.0 is not negative
fn is_negative(value: Scalar) -> bool {
    // dojo: allow float-boundary -- positive zero, built from its bit pattern
    value < Scalar::from_bits(0)
}

impl FixedLine3 {
    /// Converts a float point and direction, then normalizes as [`Self::try_new`] does.
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`], [`ShapeError::FixedValueOutOfRange`], then
    /// [`ShapeError::InvalidLineDirection`] for the rounded direction.
    pub fn try_from_float(point: Vec3, direction: Vec3) -> Result<Self, ShapeError> {
        require_finite(&[point, direction])?;
        Self::try_new(fixed_vec3(point)?, fixed_vec3(direction)?)
    }
}

impl FixedTriangle3 {
    /// Converts three float vertices, in order, then refuses a degenerate result as
    /// [`Self::try_new`] does: degeneracy is decided exactly on the rounded vertices.
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`], [`ShapeError::FixedValueOutOfRange`], then
    /// [`ShapeError::DegenerateTriangle`].
    pub fn try_from_float(vertices: [Vec3; 3]) -> Result<Self, ShapeError> {
        require_finite(&vertices)?;
        let [v0, v1, v2] = vertices;
        Self::try_new(fixed_vec3(v0)?, fixed_vec3(v1)?, fixed_vec3(v2)?)
    }
}

impl FixedCapsule3 {
    // dojo: allow float-boundary -- validated constructor from an f32 boundary radius
    /// Converts a float segment and radius.
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`] for an endpoint, [`ShapeError::NonFiniteRadius`],
    /// [`ShapeError::NegativeCapsuleRadius`] for a float radius below zero (`-0.0` is not, and
    /// a tiny negative radius is refused even though it would round to zero), then
    /// [`ShapeError::FixedValueOutOfRange`].
    pub fn try_from_float(start: Vec3, end: Vec3, radius: Scalar) -> Result<Self, ShapeError> {
        require_finite(&[start, end])?;
        if !is_finite(radius) {
            return Err(ShapeError::NonFiniteRadius);
        }
        if is_negative(radius) {
            return Err(ShapeError::NegativeCapsuleRadius);
        }
        let segment = FixedSegment3::new(fixed_vec3(start)?, fixed_vec3(end)?);
        Self::try_new(segment, fixed_scalar(radius)?)
    }
}

impl FixedObb3 {
    /// Converts a float centre, three float axes (the columns) and half extents, then checks
    /// the rounded axes as [`Self::try_new`] does.
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`], [`ShapeError::NegativeHalfExtent`] for the first
    /// float half extent below zero (before rounding), [`ShapeError::FixedValueOutOfRange`],
    /// then the axis refusals of [`Self::try_new`].
    pub fn try_from_float(
        center: Vec3,
        axes: [Vec3; 3],
        half_extents: Vec3,
    ) -> Result<Self, ShapeError> {
        let [a0, a1, a2] = axes;
        require_finite(&[center, a0, a1, a2, half_extents])?;
        let extents = [half_extents.x, half_extents.y, half_extents.z];
        if let Some(axis) = extents.iter().position(|&extent| is_negative(extent)) {
            return Err(ShapeError::NegativeHalfExtent { axis });
        }
        let center = fixed_vec3(center)?;
        let rows = FixedMat3::from_rows(fixed_vec3(a0)?, fixed_vec3(a1)?, fixed_vec3(a2)?);
        Self::try_new(center, rows.transpose(), fixed_vec3(half_extents)?)
    }
}

impl FixedSegment3 {
    /// Converts two float endpoints, in order. Every pair of points is a segment, a
    /// zero-length one included.
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`], then [`ShapeError::FixedValueOutOfRange`].
    pub fn try_from_float(start: Vec3, end: Vec3) -> Result<Self, ShapeError> {
        require_finite(&[start, end])?;
        Ok(Self::new(fixed_vec3(start)?, fixed_vec3(end)?))
    }
}

impl FixedPlane3 {
    // dojo: allow float-boundary -- validated constructor from an f32 boundary offset
    /// Converts a float normal and offset. A zero normal is accepted, as in [`Self::new`].
    ///
    /// # Errors
    /// [`ShapeError::NonFiniteCoordinate`] for the normal or the offset, then
    /// [`ShapeError::FixedValueOutOfRange`].
    pub fn try_from_float(normal: Vec3, offset: Scalar) -> Result<Self, ShapeError> {
        require_finite(&[normal])?;
        if !is_finite(offset) {
            return Err(ShapeError::NonFiniteCoordinate);
        }
        Ok(Self::new(fixed_vec3(normal)?, fixed_scalar(offset)?))
    }
}
