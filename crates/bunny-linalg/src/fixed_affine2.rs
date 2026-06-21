//! Two-dimensional deterministic affine transform primitives.

use bunny_num::FixedQ32_32;

use crate::{FixedMat2, FixedVec2};

/// A 2D affine transform using a fixed-point linear part and translation.
///
/// Vectors are treated as column vectors. `checked_mul_affine` composes
/// right-to-left: `outer.checked_mul_affine(inner)` applies `inner` first and
/// `outer` second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedAffine2 {
    /// Linear 2x2 matrix part.
    pub linear: FixedMat2,
    /// Translation applied to points after the linear part.
    pub translation: FixedVec2,
}

impl FixedAffine2 {
    /// Identity affine transform.
    pub const IDENTITY: Self =
        Self::from_parts(FixedMat2::IDENTITY, FixedVec2::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO));

    /// Creates an affine transform from a linear matrix and translation.
    #[must_use]
    pub const fn from_parts(linear: FixedMat2, translation: FixedVec2) -> Self {
        Self { linear, translation }
    }

    /// Transforms a point by the linear part and translation.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_transform_point(self, point: FixedVec2) -> Option<FixedVec2> {
        checked_add_vec2(self.checked_transform_vector(point)?, self.translation)
    }

    /// Transforms a vector by the linear part only.
    ///
    /// Translation does not affect vectors. Returns `None` when intermediate
    /// checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_transform_vector(self, vector: FixedVec2) -> Option<FixedVec2> {
        self.linear.checked_mul_vec2(vector)
    }

    /// Multiplies this affine transform by another transform.
    ///
    /// The result applies `rhs` first and `self` second when used with column
    /// vectors and points.
    #[must_use]
    pub fn checked_mul_affine(self, rhs: Self) -> Option<Self> {
        Some(Self::from_parts(
            self.linear.checked_mul_mat2(rhs.linear)?,
            self.checked_transform_point(rhs.translation)?,
        ))
    }

    /// Computes the inverse affine transform.
    ///
    /// Returns `None` when the linear part is singular or intermediate checked
    /// Q32.32 arithmetic overflows.
    #[must_use]
    pub fn try_inverse(self) -> Option<Self> {
        let inverse_linear = self.linear.try_inverse()?;
        let scaled_translation = inverse_linear.checked_mul_vec2(self.translation)?;
        Some(Self::from_parts(inverse_linear, checked_neg_vec2(scaled_translation)?))
    }
}

fn checked_add_vec2(lhs: FixedVec2, rhs: FixedVec2) -> Option<FixedVec2> {
    Some(FixedVec2::new(lhs.x.checked_add(rhs.x)?, lhs.y.checked_add(rhs.y)?))
}

fn checked_neg_vec2(value: FixedVec2) -> Option<FixedVec2> {
    Some(FixedVec2::new(value.x.checked_neg()?, value.y.checked_neg()?))
}
