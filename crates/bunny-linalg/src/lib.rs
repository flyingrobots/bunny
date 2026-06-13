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
