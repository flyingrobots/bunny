#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Linear algebra primitives for Bunny graphics contracts.

use bunny_num::{FixedQ32_32, Scalar};

mod fixed_mat2;
mod fixed_mat3;
mod fixed_mat4;
mod fixed_mat4_inverse;
mod fixed_vec2;
mod fixed_vec3;
pub(crate) mod matrix_common;

pub use fixed_mat2::FixedMat2;
pub use fixed_mat3::FixedMat3;
pub use fixed_mat4::FixedMat4;
pub use fixed_vec2::FixedVec2;
pub use fixed_vec3::FixedVec3;
pub use matrix_common::FixedMat4Row;

const UNIT_LENGTH_TOLERANCE_RAW: i128 = 1;
const NEG_ONE: FixedQ32_32 = FixedQ32_32::from_raw(-bunny_num::fixed_q32_32::ONE_RAW);

/// Two-dimensional vector using floating-point coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    /// X component.
    pub x: Scalar,
    /// Y component.
    pub y: Scalar,
}

impl Vec2 {
    /// Creates a new vector from components.
    #[must_use]
    pub const fn new(x: Scalar, y: Scalar) -> Self {
        Self { x, y }
    }
}

/// Three-dimensional vector using floating-point coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    /// X component.
    pub x: Scalar,
    /// Y component.
    pub y: Scalar,
    /// Z component.
    pub z: Scalar,
}

impl Vec3 {
    /// Creates a new vector from components.
    #[must_use]
    pub const fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) const fn checked_u128_to_i64(value: u128) -> Option<i64> {
    if value > i64::MAX as u128 {
        None
    } else {
        Some(value as i64)
    }
}

const fn is_unit_length(length: FixedQ32_32) -> bool {
    is_unit_length_raw(length.to_raw())
}

const fn abs_i128(value: i128) -> i128 {
    if value < 0 {
        -value
    } else {
        value
    }
}

#[allow(clippy::cast_lossless)]
const fn is_unit_length_raw(length_raw: i64) -> bool {
    let delta = length_raw as i128 - FixedQ32_32::ONE.to_raw() as i128;
    abs_i128(delta) <= UNIT_LENGTH_TOLERANCE_RAW
}

#[allow(clippy::cast_sign_loss)]
const fn abs_i64_as_u128(value: i64) -> u128 {
    if value < 0 {
        -(value as i128) as u128
    } else {
        value as u128
    }
}

const fn raw_square(value: FixedQ32_32) -> u128 {
    let abs = abs_i64_as_u128(value.to_raw());
    abs * abs
}

const fn add_nonnegative_u128(lhs: u128, rhs: u128) -> Option<u128> {
    if lhs > u128::MAX - rhs {
        None
    } else {
        Some(lhs + rhs)
    }
}

const fn raw_squares_have_unit_length(sum: u128) -> bool {
    match checked_u128_to_i64(FixedQ32_32::sqrt_u128(sum)) {
        Some(length_raw) => is_unit_length_raw(length_raw),
        None => false,
    }
}

const fn is_fixed_unit_vec2(v: FixedVec2) -> bool {
    let x_squared = raw_square(v.x);
    let y_squared = raw_square(v.y);
    match add_nonnegative_u128(x_squared, y_squared) {
        Some(sum) => raw_squares_have_unit_length(sum),
        None => false,
    }
}

const fn is_fixed_unit_vec3(v: FixedVec3) -> bool {
    let x_squared = raw_square(v.x);
    let y_squared = raw_square(v.y);
    let z_squared = raw_square(v.z);
    match add_nonnegative_u128(x_squared, y_squared) {
        Some(xy_sum) => match add_nonnegative_u128(xy_sum, z_squared) {
            Some(sum) => raw_squares_have_unit_length(sum),
            None => false,
        },
        None => false,
    }
}

/// A normalized two-dimensional vector using deterministic Q32.32 fixed-point representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedUnitVec2(FixedVec2);

impl FixedUnitVec2 {
    /// The positive X axis unit vector.
    pub const UNIT_X: Self = Self(FixedVec2::new(FixedQ32_32::ONE, FixedQ32_32::ZERO));

    /// The negative X axis unit vector.
    pub const NEG_UNIT_X: Self = Self(FixedVec2::new(NEG_ONE, FixedQ32_32::ZERO));

    /// The positive Y axis unit vector.
    pub const UNIT_Y: Self = Self(FixedVec2::new(FixedQ32_32::ZERO, FixedQ32_32::ONE));

    /// The negative Y axis unit vector.
    pub const NEG_UNIT_Y: Self = Self(FixedVec2::new(FixedQ32_32::ZERO, NEG_ONE));

    /// Creates a new `FixedUnitVec2` by normalizing the given vector.
    ///
    /// Returns `None` if normalization fails (vector has zero length or overflows/underflows).
    #[must_use]
    pub fn new(v: FixedVec2) -> Option<Self> {
        let normalized = v.normalize()?;
        let length = normalized.length()?;
        if is_unit_length(length) {
            Some(Self(normalized))
        } else {
            None
        }
    }

    /// Const-validates an already-normalized fixed vector as a unit vector.
    ///
    /// Unlike `new`, this function does not normalize its input. It exists for
    /// compile-time known vectors where the caller needs a unit-vector proof
    /// without runtime normalization.
    #[must_use]
    pub const fn try_from_unit(v: FixedVec2) -> Option<Self> {
        if is_fixed_unit_vec2(v) {
            Some(Self(v))
        } else {
            None
        }
    }

    /// Gets the underlying `FixedVec2`.
    #[must_use]
    pub const fn into_inner(self) -> FixedVec2 {
        self.0
    }
}

/// A normalized three-dimensional vector using deterministic Q32.32 fixed-point representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedUnitVec3(FixedVec3);

impl FixedUnitVec3 {
    /// The positive X axis unit vector.
    pub const UNIT_X: Self =
        Self(FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO));

    /// The negative X axis unit vector.
    pub const NEG_UNIT_X: Self =
        Self(FixedVec3::new(NEG_ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO));

    /// The positive Y axis unit vector.
    pub const UNIT_Y: Self =
        Self(FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO));

    /// The negative Y axis unit vector.
    pub const NEG_UNIT_Y: Self =
        Self(FixedVec3::new(FixedQ32_32::ZERO, NEG_ONE, FixedQ32_32::ZERO));

    /// The positive Z axis unit vector.
    pub const UNIT_Z: Self =
        Self(FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE));

    /// The negative Z axis unit vector.
    pub const NEG_UNIT_Z: Self =
        Self(FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, NEG_ONE));

    /// Creates a new `FixedUnitVec3` by normalizing the given vector.
    ///
    /// Returns `None` if normalization fails (vector has zero length or overflows/underflows).
    #[must_use]
    pub fn new(v: FixedVec3) -> Option<Self> {
        let normalized = v.normalize()?;
        let length = normalized.length()?;
        if is_unit_length(length) {
            Some(Self(normalized))
        } else {
            None
        }
    }

    /// Const-validates an already-normalized fixed vector as a unit vector.
    ///
    /// Unlike `new`, this function does not normalize its input. It exists for
    /// compile-time known vectors where the caller needs a unit-vector proof
    /// without runtime normalization.
    #[must_use]
    pub const fn try_from_unit(v: FixedVec3) -> Option<Self> {
        if is_fixed_unit_vec3(v) {
            Some(Self(v))
        } else {
            None
        }
    }

    /// Gets the underlying `FixedVec3`.
    #[must_use]
    pub const fn into_inner(self) -> FixedVec3 {
        self.0
    }
}
