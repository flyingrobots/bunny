//! Three-dimensional deterministic affine transform primitives.

use bunny_num::FixedQ32_32;

use crate::{FixedMat3, FixedVec3};

/// A 3D affine transform using a fixed-point linear part and translation.
///
/// Vectors are treated as column vectors. `checked_mul_affine` composes
/// right-to-left: `outer.checked_mul_affine(inner)` applies `inner` first and
/// `outer` second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedAffine3 {
    /// Linear 3x3 matrix part.
    pub linear: FixedMat3,
    /// Translation applied to points after the linear part.
    pub translation: FixedVec3,
}

impl FixedAffine3 {
    /// Identity affine transform.
    pub const IDENTITY: Self = Self::from_parts(
        FixedMat3::IDENTITY,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    );

    /// Creates an affine transform from a linear matrix and translation.
    #[must_use]
    pub const fn from_parts(linear: FixedMat3, translation: FixedVec3) -> Self {
        Self { linear, translation }
    }

    /// Transforms a point by the linear part and translation.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_transform_point(self, point: FixedVec3) -> Option<FixedVec3> {
        checked_add_vec3(self.checked_transform_vector(point)?, self.translation)
    }

    /// Transforms a vector by the linear part only.
    ///
    /// Translation does not affect vectors. Returns `None` when intermediate
    /// checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_transform_vector(self, vector: FixedVec3) -> Option<FixedVec3> {
        self.linear.checked_mul_vec3(vector)
    }

    /// Multiplies this affine transform by another transform.
    ///
    /// The result applies `rhs` first and `self` second when used with column
    /// vectors and points.
    #[must_use]
    pub fn checked_mul_affine(self, rhs: Self) -> Option<Self> {
        Some(Self::from_parts(
            self.linear.checked_mul_mat3(rhs.linear)?,
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
        let scaled_translation = inverse_linear.checked_mul_vec3(self.translation)?;
        Some(Self::from_parts(inverse_linear, checked_neg_vec3(scaled_translation)?))
    }
}

fn checked_add_vec3(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        lhs.x.checked_add(rhs.x)?,
        lhs.y.checked_add(rhs.y)?,
        lhs.z.checked_add(rhs.z)?,
    ))
}

fn checked_neg_vec3(value: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(value.x.checked_neg()?, value.y.checked_neg()?, value.z.checked_neg()?))
}
