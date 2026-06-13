#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Linear algebra primitives for Bunny graphics contracts.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use bunny_num::{FixedQ32_32, Scalar};

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

/// Two-dimensional vector using deterministic Q32.32 fixed-point representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedVec2 {
    /// X component.
    pub x: FixedQ32_32,
    /// Y component.
    pub y: FixedQ32_32,
}

impl FixedVec2 {
    /// Creates a new fixed-point vector.
    #[must_use]
    pub const fn new(x: FixedQ32_32, y: FixedQ32_32) -> Self {
        Self { x, y }
    }

    /// Computes the dot product of two vectors.
    #[must_use]
    pub fn dot(self, rhs: Self) -> FixedQ32_32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Computes the squared length of the vector.
    #[must_use]
    pub fn length_squared(self) -> FixedQ32_32 {
        self.dot(self)
    }

    /// Computes the length of the vector.
    #[must_use]
    pub fn length(self) -> Option<FixedQ32_32> {
        self.length_squared().sqrt()
    }

    /// Normalizes the vector.
    #[must_use]
    pub fn normalize(self) -> Option<Self> {
        let len = self.length()?;
        if len == FixedQ32_32::ZERO {
            None
        } else {
            Some(self / len)
        }
    }
}

/// Three-dimensional vector using deterministic Q32.32 fixed-point representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedVec3 {
    /// X component.
    pub x: FixedQ32_32,
    /// Y component.
    pub y: FixedQ32_32,
    /// Z component.
    pub z: FixedQ32_32,
}

impl FixedVec3 {
    /// Creates a new fixed-point vector.
    #[must_use]
    pub const fn new(x: FixedQ32_32, y: FixedQ32_32, z: FixedQ32_32) -> Self {
        Self { x, y, z }
    }

    /// Computes the dot product of two vectors.
    #[must_use]
    pub fn dot(self, rhs: Self) -> FixedQ32_32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Computes the cross product of two vectors.
    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    /// Computes the squared length of the vector.
    #[must_use]
    pub fn length_squared(self) -> FixedQ32_32 {
        self.dot(self)
    }

    /// Computes the length of the vector.
    #[must_use]
    pub fn length(self) -> Option<FixedQ32_32> {
        self.length_squared().sqrt()
    }

    /// Normalizes the vector.
    #[must_use]
    pub fn normalize(self) -> Option<Self> {
        let len = self.length()?;
        if len == FixedQ32_32::ZERO {
            None
        } else {
            Some(self / len)
        }
    }
}

impl Add for FixedVec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for FixedVec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedVec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for FixedVec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for FixedVec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<FixedQ32_32> for FixedVec2 {
    type Output = Self;
    fn mul(self, rhs: FixedQ32_32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<FixedQ32_32> for FixedVec2 {
    fn mul_assign(&mut self, rhs: FixedQ32_32) {
        *self = *self * rhs;
    }
}

impl Div<FixedQ32_32> for FixedVec2 {
    type Output = Self;
    fn div(self, rhs: FixedQ32_32) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<FixedQ32_32> for FixedVec2 {
    fn div_assign(&mut self, rhs: FixedQ32_32) {
        *self = *self / rhs;
    }
}

impl Add for FixedVec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for FixedVec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedVec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for FixedVec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for FixedVec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<FixedQ32_32> for FixedVec3 {
    type Output = Self;
    fn mul(self, rhs: FixedQ32_32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<FixedQ32_32> for FixedVec3 {
    fn mul_assign(&mut self, rhs: FixedQ32_32) {
        *self = *self * rhs;
    }
}

impl Div<FixedQ32_32> for FixedVec3 {
    type Output = Self;
    fn div(self, rhs: FixedQ32_32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl DivAssign<FixedQ32_32> for FixedVec3 {
    fn div_assign(&mut self, rhs: FixedQ32_32) {
        *self = *self / rhs;
    }
}

impl From<Vec2> for FixedVec2 {
    fn from(v: Vec2) -> Self {
        Self::new(FixedQ32_32::from_f32(v.x), FixedQ32_32::from_f32(v.y))
    }
}

impl From<FixedVec2> for Vec2 {
    fn from(v: FixedVec2) -> Self {
        Self::new(v.x.to_f32(), v.y.to_f32())
    }
}

impl From<Vec3> for FixedVec3 {
    fn from(v: Vec3) -> Self {
        Self::new(
            FixedQ32_32::from_f32(v.x),
            FixedQ32_32::from_f32(v.y),
            FixedQ32_32::from_f32(v.z),
        )
    }
}

impl From<FixedVec3> for Vec3 {
    fn from(v: FixedVec3) -> Self {
        Self::new(v.x.to_f32(), v.y.to_f32(), v.z.to_f32())
    }
}
