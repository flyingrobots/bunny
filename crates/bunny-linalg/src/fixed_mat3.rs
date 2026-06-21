//! Three-dimensional fixed-point matrix primitives.

use bunny_num::FixedQ32_32;

use crate::FixedVec3;

/// A 3x3 matrix using deterministic Q32.32 fixed-point entries.
///
/// Entries are named `mRC`, where `R` is the zero-based row and `C` is the
/// zero-based column. Multiplication treats vectors as column vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedMat3 {
    /// Row 0, column 0.
    pub m00: FixedQ32_32,
    /// Row 0, column 1.
    pub m01: FixedQ32_32,
    /// Row 0, column 2.
    pub m02: FixedQ32_32,
    /// Row 1, column 0.
    pub m10: FixedQ32_32,
    /// Row 1, column 1.
    pub m11: FixedQ32_32,
    /// Row 1, column 2.
    pub m12: FixedQ32_32,
    /// Row 2, column 0.
    pub m20: FixedQ32_32,
    /// Row 2, column 1.
    pub m21: FixedQ32_32,
    /// Row 2, column 2.
    pub m22: FixedQ32_32,
}

impl FixedMat3 {
    /// Identity 3x3 matrix.
    pub const IDENTITY: Self = Self::from_rows(
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );

    /// Creates a matrix from row vectors.
    #[must_use]
    pub const fn from_rows(row0: FixedVec3, row1: FixedVec3, row2: FixedVec3) -> Self {
        Self {
            m00: row0.x,
            m01: row0.y,
            m02: row0.z,
            m10: row1.x,
            m11: row1.y,
            m12: row1.z,
            m20: row2.x,
            m21: row2.y,
            m22: row2.z,
        }
    }

    /// Returns row 0 as a vector.
    #[must_use]
    pub const fn row0(self) -> FixedVec3 {
        FixedVec3::new(self.m00, self.m01, self.m02)
    }

    /// Returns row 1 as a vector.
    #[must_use]
    pub const fn row1(self) -> FixedVec3 {
        FixedVec3::new(self.m10, self.m11, self.m12)
    }

    /// Returns row 2 as a vector.
    #[must_use]
    pub const fn row2(self) -> FixedVec3 {
        FixedVec3::new(self.m20, self.m21, self.m22)
    }

    /// Returns column 0 as a vector.
    #[must_use]
    pub const fn column0(self) -> FixedVec3 {
        FixedVec3::new(self.m00, self.m10, self.m20)
    }

    /// Returns column 1 as a vector.
    #[must_use]
    pub const fn column1(self) -> FixedVec3 {
        FixedVec3::new(self.m01, self.m11, self.m21)
    }

    /// Returns column 2 as a vector.
    #[must_use]
    pub const fn column2(self) -> FixedVec3 {
        FixedVec3::new(self.m02, self.m12, self.m22)
    }

    /// Returns the transposed matrix.
    #[must_use]
    pub const fn transpose(self) -> Self {
        Self::from_rows(self.column0(), self.column1(), self.column2())
    }

    /// Computes the determinant.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn determinant(self) -> Option<FixedQ32_32> {
        let cofactor0 = self.minor_00()?;
        let cofactor1 = self.minor_01()?;
        let cofactor2 = self.minor_02()?;

        self.m00
            .checked_mul(cofactor0)?
            .checked_sub(self.m01.checked_mul(cofactor1)?)?
            .checked_add(self.m02.checked_mul(cofactor2)?)
    }

    /// Multiplies this matrix by a 3D column vector.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn checked_mul_vec3(self, rhs: FixedVec3) -> Option<FixedVec3> {
        Some(FixedVec3::new(
            checked_dot3(self.row0(), rhs)?,
            checked_dot3(self.row1(), rhs)?,
            checked_dot3(self.row2(), rhs)?,
        ))
    }

    /// Multiplies this matrix by another 3x3 matrix.
    ///
    /// The resulting transform applies `rhs` first and `self` second when used
    /// with column vectors.
    #[must_use]
    pub fn checked_mul_mat3(self, rhs: Self) -> Option<Self> {
        Some(Self::from_rows(
            checked_mul_row(self.row0(), rhs)?,
            checked_mul_row(self.row1(), rhs)?,
            checked_mul_row(self.row2(), rhs)?,
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
        Some(Self::from_rows(
            FixedVec3::new(
                self.minor_00()?.checked_div(determinant)?,
                checked_div_then_neg(self.minor_10()?, determinant)?,
                self.minor_20()?.checked_div(determinant)?,
            ),
            FixedVec3::new(
                checked_div_then_neg(self.minor_01()?, determinant)?,
                self.minor_11()?.checked_div(determinant)?,
                checked_div_then_neg(self.minor_21()?, determinant)?,
            ),
            FixedVec3::new(
                self.minor_02()?.checked_div(determinant)?,
                checked_div_then_neg(self.minor_12()?, determinant)?,
                self.minor_22()?.checked_div(determinant)?,
            ),
        ))
    }

    fn minor_00(self) -> Option<FixedQ32_32> {
        checked_det2(self.m11, self.m12, self.m21, self.m22)
    }

    fn minor_01(self) -> Option<FixedQ32_32> {
        checked_det2(self.m10, self.m12, self.m20, self.m22)
    }

    fn minor_02(self) -> Option<FixedQ32_32> {
        checked_det2(self.m10, self.m11, self.m20, self.m21)
    }

    fn minor_10(self) -> Option<FixedQ32_32> {
        checked_det2(self.m01, self.m02, self.m21, self.m22)
    }

    fn minor_11(self) -> Option<FixedQ32_32> {
        checked_det2(self.m00, self.m02, self.m20, self.m22)
    }

    fn minor_12(self) -> Option<FixedQ32_32> {
        checked_det2(self.m00, self.m01, self.m20, self.m21)
    }

    fn minor_20(self) -> Option<FixedQ32_32> {
        checked_det2(self.m01, self.m02, self.m11, self.m12)
    }

    fn minor_21(self) -> Option<FixedQ32_32> {
        checked_det2(self.m00, self.m02, self.m10, self.m12)
    }

    fn minor_22(self) -> Option<FixedQ32_32> {
        checked_det2(self.m00, self.m01, self.m10, self.m11)
    }
}

fn checked_mul_row(row: FixedVec3, rhs: FixedMat3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        checked_dot3(row, rhs.column0())?,
        checked_dot3(row, rhs.column1())?,
        checked_dot3(row, rhs.column2())?,
    ))
}

fn checked_dot3(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedQ32_32> {
    lhs.x
        .checked_mul(rhs.x)?
        .checked_add(lhs.y.checked_mul(rhs.y)?)?
        .checked_add(lhs.z.checked_mul(rhs.z)?)
}

fn checked_det2(
    m00: FixedQ32_32,
    m01: FixedQ32_32,
    m10: FixedQ32_32,
    m11: FixedQ32_32,
) -> Option<FixedQ32_32> {
    m00.checked_mul(m11)?.checked_sub(m01.checked_mul(m10)?)
}

fn checked_div_then_neg(value: FixedQ32_32, determinant: FixedQ32_32) -> Option<FixedQ32_32> {
    value.checked_div(determinant)?.checked_neg()
}
