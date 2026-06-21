//! Determinant and inverse support for four-dimensional matrices.

use bunny_num::FixedQ32_32;

use crate::{FixedMat3, FixedMat4, FixedMat4Row, FixedVec3};

impl FixedMat4 {
    /// Computes the determinant.
    ///
    /// Returns `None` when intermediate checked Q32.32 arithmetic overflows.
    #[must_use]
    pub fn determinant(self) -> Option<FixedQ32_32> {
        let cofactors = self.cofactor_row0()?;
        self.m00()
            .checked_mul(cofactors.0)?
            .checked_add(self.m01().checked_mul(cofactors.1)?)?
            .checked_add(self.m02().checked_mul(cofactors.2)?)?
            .checked_add(self.m03().checked_mul(cofactors.3)?)
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

        let cofactors = self.cofactor_matrix()?;
        Some(Self::from_rows(
            checked_div_row4(cofactors.column0(), determinant)?,
            checked_div_row4(cofactors.column1(), determinant)?,
            checked_div_row4(cofactors.column2(), determinant)?,
            checked_div_row4(cofactors.column3_row(), determinant)?,
        ))
    }

    fn cofactor_matrix(self) -> Option<Self> {
        Some(Self::from_rows(
            self.cofactor_row0()?,
            self.cofactor_row1()?,
            self.cofactor_row2()?,
            self.cofactor_row3()?,
        ))
    }

    fn cofactor_row0(self) -> Option<FixedMat4Row> {
        Some((
            self.minor_00()?,
            checked_neg(self.minor_01()?)?,
            self.minor_02()?,
            checked_neg(self.minor_03()?)?,
        ))
    }

    fn cofactor_row1(self) -> Option<FixedMat4Row> {
        Some((
            checked_neg(self.minor_10()?)?,
            self.minor_11()?,
            checked_neg(self.minor_12()?)?,
            self.minor_13()?,
        ))
    }

    fn cofactor_row2(self) -> Option<FixedMat4Row> {
        Some((
            self.minor_20()?,
            checked_neg(self.minor_21()?)?,
            self.minor_22()?,
            checked_neg(self.minor_23()?)?,
        ))
    }

    fn cofactor_row3(self) -> Option<FixedMat4Row> {
        Some((
            checked_neg(self.minor_30()?)?,
            self.minor_31()?,
            checked_neg(self.minor_32()?)?,
            self.minor_33()?,
        ))
    }

    fn minor_00(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m11(), self.m12(), self.m13()),
            row(self.m21(), self.m22(), self.m23()),
            row(self.m31(), self.m32(), self.m33()),
        )
    }

    fn minor_01(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m10(), self.m12(), self.m13()),
            row(self.m20(), self.m22(), self.m23()),
            row(self.m30(), self.m32(), self.m33()),
        )
    }

    fn minor_02(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m10(), self.m11(), self.m13()),
            row(self.m20(), self.m21(), self.m23()),
            row(self.m30(), self.m31(), self.m33()),
        )
    }

    fn minor_03(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m10(), self.m11(), self.m12()),
            row(self.m20(), self.m21(), self.m22()),
            row(self.m30(), self.m31(), self.m32()),
        )
    }

    fn minor_10(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m01(), self.m02(), self.m03()),
            row(self.m21(), self.m22(), self.m23()),
            row(self.m31(), self.m32(), self.m33()),
        )
    }

    fn minor_11(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m02(), self.m03()),
            row(self.m20(), self.m22(), self.m23()),
            row(self.m30(), self.m32(), self.m33()),
        )
    }

    fn minor_12(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m03()),
            row(self.m20(), self.m21(), self.m23()),
            row(self.m30(), self.m31(), self.m33()),
        )
    }

    fn minor_13(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m02()),
            row(self.m20(), self.m21(), self.m22()),
            row(self.m30(), self.m31(), self.m32()),
        )
    }

    fn minor_20(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m01(), self.m02(), self.m03()),
            row(self.m11(), self.m12(), self.m13()),
            row(self.m31(), self.m32(), self.m33()),
        )
    }

    fn minor_21(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m02(), self.m03()),
            row(self.m10(), self.m12(), self.m13()),
            row(self.m30(), self.m32(), self.m33()),
        )
    }

    fn minor_22(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m03()),
            row(self.m10(), self.m11(), self.m13()),
            row(self.m30(), self.m31(), self.m33()),
        )
    }

    fn minor_23(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m02()),
            row(self.m10(), self.m11(), self.m12()),
            row(self.m30(), self.m31(), self.m32()),
        )
    }

    fn minor_30(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m01(), self.m02(), self.m03()),
            row(self.m11(), self.m12(), self.m13()),
            row(self.m21(), self.m22(), self.m23()),
        )
    }

    fn minor_31(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m02(), self.m03()),
            row(self.m10(), self.m12(), self.m13()),
            row(self.m20(), self.m22(), self.m23()),
        )
    }

    fn minor_32(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m03()),
            row(self.m10(), self.m11(), self.m13()),
            row(self.m20(), self.m21(), self.m23()),
        )
    }

    fn minor_33(self) -> Option<FixedQ32_32> {
        det3(
            row(self.m00(), self.m01(), self.m02()),
            row(self.m10(), self.m11(), self.m12()),
            row(self.m20(), self.m21(), self.m22()),
        )
    }
}

const fn row(x: FixedQ32_32, y: FixedQ32_32, z: FixedQ32_32) -> FixedVec3 {
    FixedVec3::new(x, y, z)
}

fn det3(row0: FixedVec3, row1: FixedVec3, row2: FixedVec3) -> Option<FixedQ32_32> {
    FixedMat3::from_rows(row0, row1, row2).determinant()
}

const fn checked_neg(value: FixedQ32_32) -> Option<FixedQ32_32> {
    value.checked_neg()
}

fn checked_div_row4(row: FixedMat4Row, determinant: FixedQ32_32) -> Option<FixedMat4Row> {
    Some((
        row.0.checked_div(determinant)?,
        row.1.checked_div(determinant)?,
        row.2.checked_div(determinant)?,
        row.3.checked_div(determinant)?,
    ))
}
