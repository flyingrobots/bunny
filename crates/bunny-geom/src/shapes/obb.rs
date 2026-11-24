//! The oriented bounding box and its exact axis check.

use bunny_linalg::{FixedMat3, FixedVec3};

use super::ShapeError;

/// How far, in raw units, the length of each [`FixedObb3`] axis may be from one.
///
/// It is the column-length contract of `bunny-rotation/1` (`FixedMat3::rotation_axis_angle`,
/// within 11 raw units), the widest of the repository's rotation constructors; the axis
/// rotations (3/2) and `bunny-look-rotation/1` (2) are inside it.
pub const OBB_AXIS_LENGTH_TOLERANCE_RAW: u64 = 11;

/// How far, in raw units, the dot product of two [`FixedObb3`] axes may be from zero.
///
/// It is the column dot-product contract of `bunny-rotation/1` (within 12 raw units); the axis
/// rotations (exactly zero) and `bunny-look-rotation/1` (2) are inside it.
pub const OBB_AXIS_DOT_TOLERANCE_RAW: u64 = 12;

#[allow(clippy::cast_lossless)] // `u128::from` is not `const`; widening a `u64` is lossless.
const LENGTH_TOLERANCE: u128 = OBB_AXIS_LENGTH_TOLERANCE_RAW as u128;
#[allow(clippy::cast_lossless)] // `u128::from` is not `const`; widening a `u64` is lossless.
const DOT_TOLERANCE: u128 = OBB_AXIS_DOT_TOLERANCE_RAW as u128;
const ONE_RAW: u128 = 1 << 32;
const LENGTH_SQUARED_MIN: u128 = (ONE_RAW - LENGTH_TOLERANCE) * (ONE_RAW - LENGTH_TOLERANCE);
const LENGTH_SQUARED_MAX: u128 = (ONE_RAW + LENGTH_TOLERANCE) * (ONE_RAW + LENGTH_TOLERANCE);
const DOT_MAX: u128 = DOT_TOLERANCE << 32;

/// A 3D oriented box: the points `center + axes * l` for every local `l` with
/// `abs(l_i) <= half_extents_i`, in fixed-point coordinates.
///
/// The axes are the columns of `axes`. An exactly orthonormal basis is not required: fixed
/// point holds one only in special cases (the identity and the other signed axis
/// permutations), not for a general rotation. The invariant is the checked bound the
/// repository's rotation constructors guarantee:
/// with lengths and dot products taken exactly on the raw entries (a dot product is the exact
/// sum of raw products divided by `2^32`), every column's length is within
/// [`OBB_AXIS_LENGTH_TOLERANCE_RAW`] raw units of one and every pair's dot product within
/// [`OBB_AXIS_DOT_TOLERANCE_RAW`] of zero. Handedness is not checked: a reflection is accepted
/// and describes the same set of points as the rotation it mirrors. Every half extent is
/// non-negative; a zero half extent (a flat box, a segment or a point) is accepted.
///
/// The fields are private so the invariant holds for every value. The corners are not exposed:
/// `center + axes * l` rounds, so they are not exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedObb3 {
    center: FixedVec3,
    axes: FixedMat3,
    half_extents: FixedVec3,
}

impl FixedObb3 {
    /// Creates an oriented box, checking the half extents and then the axes exactly.
    ///
    /// # Errors
    /// In this order: [`ShapeError::NegativeHalfExtent`] for the first half extent below zero
    /// (raw `< 0`); [`ShapeError::AxisLengthOutOfTolerance`] for the first column whose raw sum
    /// of squares is outside `[(2^32 - 11)^2, (2^32 + 11)^2]`; and
    /// [`ShapeError::AxesNotOrthogonal`] for the first pair, in the order `(0, 1)`, `(0, 2)`,
    /// `(1, 2)`, whose raw dot product exceeds `12 * 2^32` in magnitude.
    pub fn try_new(
        center: FixedVec3,
        axes: FixedMat3,
        half_extents: FixedVec3,
    ) -> Result<Self, ShapeError> {
        let extents = [half_extents.x, half_extents.y, half_extents.z];
        if let Some(axis) = extents.iter().position(|extent| extent.raw() < 0) {
            return Err(ShapeError::NegativeHalfExtent { axis });
        }
        check_axes([axes.column0(), axes.column1(), axes.column2()].map(raw3))?;
        Ok(Self { center, axes, half_extents })
    }

    /// The centre.
    #[must_use]
    pub const fn center(&self) -> FixedVec3 {
        self.center
    }

    /// The axes, as the columns of a matrix.
    #[must_use]
    pub const fn axes(&self) -> FixedMat3 {
        self.axes
    }

    /// The half extents along the three axes, each non-negative.
    #[must_use]
    pub const fn half_extents(&self) -> FixedVec3 {
        self.half_extents
    }
}

const fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// Lengths first, then dot products. Each raw square is at most `2^126`, so a column's sum of
/// three fits `u128`. Once every length is in tolerance each component has magnitude at most
/// `2^32 + 11`, so each product is below `2^65` and each dot product fits `i128`.
fn check_axes(columns: [[i64; 3]; 3]) -> Result<(), ShapeError> {
    if let Some(column) = columns.iter().position(|c| !length_in_tolerance(*c)) {
        return Err(ShapeError::AxisLengthOutOfTolerance { column });
    }
    let [c0, c1, c2] = columns;
    for (first, second, a, b) in [(0, 1, c0, c1), (0, 2, c0, c2), (1, 2, c1, c2)] {
        if dot(a, b).unsigned_abs() > DOT_MAX {
            return Err(ShapeError::AxesNotOrthogonal { first, second });
        }
    }
    Ok(())
}

fn length_in_tolerance([x, y, z]: [i64; 3]) -> bool {
    let square = |v: i64| u128::from(v.unsigned_abs()) * u128::from(v.unsigned_abs());
    let sum = square(x) + square(y) + square(z);
    (LENGTH_SQUARED_MIN..=LENGTH_SQUARED_MAX).contains(&sum)
}

fn dot(a: [i64; 3], b: [i64; 3]) -> i128 {
    a.iter().zip(&b).map(|(&x, &y)| i128::from(x) * i128::from(y)).sum()
}
