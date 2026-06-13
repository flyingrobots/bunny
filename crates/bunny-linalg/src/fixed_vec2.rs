//! Two-dimensional vector using deterministic Q32.32 fixed-point representation.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::Vec2;
use bunny_num::FixedQ32_32;

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
        let x_raw = u128::from(self.x.to_raw().unsigned_abs());
        let y_raw = u128::from(self.y.to_raw().unsigned_abs());
        let sum_sq = x_raw * x_raw + y_raw * y_raw;
        let root = FixedQ32_32::sqrt_u128(sum_sq);
        Some(FixedQ32_32::from_raw(crate::saturate_u128_to_i64(root)))
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
