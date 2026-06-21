//! Four-dimensional fixed-point matrix primitives.

use bunny_num::FixedQ32_32;

use crate::{FixedMat4Row, FixedVec3};

/// A 4x4 matrix using deterministic Q32.32 fixed-point entries.
///
/// Entries are exposed through `mRC()` accessors, where `R` is the zero-based
/// row and `C` is the zero-based column. Multiplication treats vectors as
/// column vectors. This type intentionally exposes checked matrix-matrix
/// multiplication first; point, vector, normal, projection, and viewport
/// helpers are added by later transform-specific roadmap slices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedMat4 {
    rows: (FixedMat4Row, FixedMat4Row, FixedMat4Row, FixedMat4Row),
}

impl FixedMat4 {
    /// Identity 4x4 matrix.
    pub const IDENTITY: Self = Self::from_rows(
        (FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        (FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        (FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO),
        (FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );

    /// Creates a matrix from row-major rows.
    #[must_use]
    pub const fn from_rows(
        row0: FixedMat4Row,
        row1: FixedMat4Row,
        row2: FixedMat4Row,
        row3: FixedMat4Row,
    ) -> Self {
        Self { rows: (row0, row1, row2, row3) }
    }

    /// Returns row 0.
    #[must_use]
    pub const fn row0(self) -> FixedMat4Row {
        self.rows.0
    }

    /// Returns row 1.
    #[must_use]
    pub const fn row1(self) -> FixedMat4Row {
        self.rows.1
    }

    /// Returns row 2.
    #[must_use]
    pub const fn row2(self) -> FixedMat4Row {
        self.rows.2
    }

    /// Returns row 3.
    #[must_use]
    pub const fn row3(self) -> FixedMat4Row {
        self.rows.3
    }

    /// Returns column 0.
    #[must_use]
    pub const fn column0(self) -> FixedMat4Row {
        (self.rows.0 .0, self.rows.1 .0, self.rows.2 .0, self.rows.3 .0)
    }

    /// Returns column 1.
    #[must_use]
    pub const fn column1(self) -> FixedMat4Row {
        (self.rows.0 .1, self.rows.1 .1, self.rows.2 .1, self.rows.3 .1)
    }

    /// Returns column 2.
    #[must_use]
    pub const fn column2(self) -> FixedMat4Row {
        (self.rows.0 .2, self.rows.1 .2, self.rows.2 .2, self.rows.3 .2)
    }

    /// Returns column 3.
    #[must_use]
    pub const fn column3_row(self) -> FixedMat4Row {
        (self.rows.0 .3, self.rows.1 .3, self.rows.2 .3, self.rows.3 .3)
    }

    /// Returns the XYZ entries of row 3.
    #[must_use]
    pub const fn row3_xyz(self) -> FixedVec3 {
        FixedVec3::new(self.rows.3 .0, self.rows.3 .1, self.rows.3 .2)
    }

    /// Returns the XYZ entries of column 3.
    #[must_use]
    pub const fn column3(self) -> FixedVec3 {
        FixedVec3::new(self.rows.0 .3, self.rows.1 .3, self.rows.2 .3)
    }

    /// Returns row 0, column 0.
    #[must_use]
    pub const fn m00(self) -> FixedQ32_32 {
        self.rows.0 .0
    }

    /// Returns row 0, column 1.
    #[must_use]
    pub const fn m01(self) -> FixedQ32_32 {
        self.rows.0 .1
    }

    /// Returns row 0, column 2.
    #[must_use]
    pub const fn m02(self) -> FixedQ32_32 {
        self.rows.0 .2
    }

    /// Returns row 0, column 3.
    #[must_use]
    pub const fn m03(self) -> FixedQ32_32 {
        self.rows.0 .3
    }

    /// Returns row 1, column 0.
    #[must_use]
    pub const fn m10(self) -> FixedQ32_32 {
        self.rows.1 .0
    }

    /// Returns row 1, column 1.
    #[must_use]
    pub const fn m11(self) -> FixedQ32_32 {
        self.rows.1 .1
    }

    /// Returns row 1, column 2.
    #[must_use]
    pub const fn m12(self) -> FixedQ32_32 {
        self.rows.1 .2
    }

    /// Returns row 1, column 3.
    #[must_use]
    pub const fn m13(self) -> FixedQ32_32 {
        self.rows.1 .3
    }

    /// Returns row 2, column 0.
    #[must_use]
    pub const fn m20(self) -> FixedQ32_32 {
        self.rows.2 .0
    }

    /// Returns row 2, column 1.
    #[must_use]
    pub const fn m21(self) -> FixedQ32_32 {
        self.rows.2 .1
    }

    /// Returns row 2, column 2.
    #[must_use]
    pub const fn m22(self) -> FixedQ32_32 {
        self.rows.2 .2
    }

    /// Returns row 2, column 3.
    #[must_use]
    pub const fn m23(self) -> FixedQ32_32 {
        self.rows.2 .3
    }

    /// Returns row 3, column 0.
    #[must_use]
    pub const fn m30(self) -> FixedQ32_32 {
        self.rows.3 .0
    }

    /// Returns row 3, column 1.
    #[must_use]
    pub const fn m31(self) -> FixedQ32_32 {
        self.rows.3 .1
    }

    /// Returns row 3, column 2.
    #[must_use]
    pub const fn m32(self) -> FixedQ32_32 {
        self.rows.3 .2
    }

    /// Returns row 3, column 3.
    #[must_use]
    pub const fn m33(self) -> FixedQ32_32 {
        self.rows.3 .3
    }

    /// Returns the transposed matrix.
    #[must_use]
    pub const fn transpose(self) -> Self {
        Self::from_rows(self.column0(), self.column1(), self.column2(), self.column3_row())
    }

    /// Multiplies this matrix by another 4x4 matrix.
    ///
    /// The resulting transform applies `rhs` first and `self` second when used
    /// with column vectors.
    #[must_use]
    pub fn checked_mul_mat4(self, rhs: Self) -> Option<Self> {
        Some(Self::from_rows(
            checked_mul_row(self.row0(), rhs)?,
            checked_mul_row(self.row1(), rhs)?,
            checked_mul_row(self.row2(), rhs)?,
            checked_mul_row(self.row3(), rhs)?,
        ))
    }
}

fn checked_mul_row(row: FixedMat4Row, rhs: FixedMat4) -> Option<FixedMat4Row> {
    Some((
        checked_dot4(row, rhs.column0())?,
        checked_dot4(row, rhs.column1())?,
        checked_dot4(row, rhs.column2())?,
        checked_dot4(row, rhs.column3_row())?,
    ))
}

fn checked_dot4(lhs: FixedMat4Row, rhs: FixedMat4Row) -> Option<FixedQ32_32> {
    lhs.0
        .checked_mul(rhs.0)?
        .checked_add(lhs.1.checked_mul(rhs.1)?)?
        .checked_add(lhs.2.checked_mul(rhs.2)?)?
        .checked_add(lhs.3.checked_mul(rhs.3)?)
}
