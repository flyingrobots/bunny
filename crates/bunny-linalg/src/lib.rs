#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Linear algebra primitives for Bunny graphics contracts.

use bunny_num::Scalar;

mod fixed_vec2;
mod fixed_vec3;

pub use fixed_vec2::FixedVec2;
pub use fixed_vec3::FixedVec3;

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

/// A normalized two-dimensional vector using deterministic Q32.32 fixed-point representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedUnitVec2(FixedVec2);

impl FixedUnitVec2 {
    /// Creates a new `FixedUnitVec2` by normalizing the given vector.
    ///
    /// Returns `None` if normalization fails (vector has zero length or overflows/underflows).
    #[must_use]
    pub fn new(v: FixedVec2) -> Option<Self> {
        v.normalize().map(Self)
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
    /// Creates a new `FixedUnitVec3` by normalizing the given vector.
    ///
    /// Returns `None` if normalization fails (vector has zero length or overflows/underflows).
    #[must_use]
    pub fn new(v: FixedVec3) -> Option<Self> {
        v.normalize().map(Self)
    }

    /// Gets the underlying `FixedVec3`.
    #[must_use]
    pub const fn into_inner(self) -> FixedVec3 {
        self.0
    }
}
