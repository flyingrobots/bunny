//! Two-dimensional fixed-point matrix primitives.

use bunny_num::FixedQ32_32;

use crate::FixedVec2;

/// A 2x2 matrix using deterministic Q32.32 fixed-point entries.
///
/// Entries are named `mRC`, where `R` is the zero-based row and `C` is the
/// zero-based column. Multiplication treats vectors as column vectors, so
/// `matrix.checked_mul_vec2(vector)` computes each output component as one row
/// dot the input vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedMat2 {
    /// Row 0, column 0.
    pub m00: FixedQ32_32,
    /// Row 0, column 1.
    pub m01: FixedQ32_32,
    /// Row 1, column 0.
    pub m10: FixedQ32_32,
    /// Row 1, column 1.
    pub m11: FixedQ32_32,
}

fn checked_dot2(lhs: FixedVec2, rhs: FixedVec2) -> Option<FixedQ32_32> {
    lhs.x.checked_mul(rhs.x)?.checked_add(lhs.y.checked_mul(rhs.y)?)
}

fn checked_det2(
    m00: FixedQ32_32,
    m01: FixedQ32_32,
    m10: FixedQ32_32,
    m11: FixedQ32_32,
) -> Option<FixedQ32_32> {
    m00.checked_mul(m11)?.checked_sub(m01.checked_mul(m10)?)
}

fn checked_div_then_neg(value: FixedQ32_32, divisor: FixedQ32_32) -> Option<FixedQ32_32> {
    value.checked_div(divisor)?.checked_neg()
}

impl FixedMat2 {
    /// Identity 2x2 matrix.
    pub const IDENTITY: Self =
        Self::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE);

    /// Creates a matrix from row-major entries.
    #[must_use]
    pub const fn new(
        m00: FixedQ32_32,
        m01: FixedQ32_32,
        m10: FixedQ32_32,
        m11: FixedQ32_32,
    ) -> Self {
        Self { m00, m01, m10, m11 }
    }

    /// Creates a matrix from two row vectors.
    #[must_use]
    pub const fn from_rows(row0: FixedVec2, row1: FixedVec2) -> Self {
        Self::new(row0.x, row0.y, row1.x, row1.y)
    }

    /// Returns row 0 as a vector.
    #[must_use]
    pub const fn row0(self) -> FixedVec2 {
        FixedVec2::new(self.m00, self.m01)
    }

    /// Returns row 1 as a vector.
    #[must_use]
    pub const fn row1(self) -> FixedVec2 {
        FixedVec2::new(self.m10, self.m11)
    }

    /// Returns column 0 as a vector.
    #[must_use]
    pub const fn column0(self) -> FixedVec2 {
        FixedVec2::new(self.m00, self.m10)
    }

    /// Returns column 1 as a vector.
    #[must_use]
    pub const fn column1(self) -> FixedVec2 {
        FixedVec2::new(self.m01, self.m11)
    }

    /// Returns the transposed matrix.
    #[must_use]
    pub const fn transpose(self) -> Self {
        Self::new(self.m00, self.m10, self.m01, self.m11)
    }

    /// Computes the determinant.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn determinant(self) -> Option<FixedQ32_32> {
        checked_det2(self.m00, self.m01, self.m10, self.m11)
    }

    /// Multiplies this matrix by a 2D column vector.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_mul_vec2(self, rhs: FixedVec2) -> Option<FixedVec2> {
        Some(FixedVec2::new(checked_dot2(self.row0(), rhs)?, checked_dot2(self.row1(), rhs)?))
    }

    /// Multiplies this matrix by another 2x2 matrix.
    ///
    /// The resulting transform applies `rhs` first and `self` second when used
    /// with column vectors.
    #[must_use]
    pub fn checked_mul_mat2(self, rhs: Self) -> Option<Self> {
        Some(Self::new(
            checked_dot2(self.row0(), rhs.column0())?,
            checked_dot2(self.row0(), rhs.column1())?,
            checked_dot2(self.row1(), rhs.column0())?,
            checked_dot2(self.row1(), rhs.column1())?,
        ))
    }

    /// Computes the inverse matrix.
    ///
    /// Returns `None` when the matrix is singular or intermediate checked
    /// Q32.32 arithmetic overflows.
    #[must_use]
    pub fn try_inverse(self) -> Option<Self> {
        let determinant = self.determinant()?;
        if determinant == FixedQ32_32::ZERO {
            return None;
        }

        Some(Self::new(
            self.m11.checked_div(determinant)?,
            checked_div_then_neg(self.m01, determinant)?,
            checked_div_then_neg(self.m10, determinant)?,
            self.m00.checked_div(determinant)?,
        ))
    }
}
