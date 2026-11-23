//! Translate, rotate and scale composition for [`FixedAffine3`] (`bunny-transform/1`), and
//! rotation or scale about a pivot. See `docs/topics/transform/README.md`.
//!
//! The conventions are those of the matrix types: column vectors, row-major `mRC` entries and
//! right-to-left composition, so `outer * inner` applies `inner` first.

use bunny_num::FixedQ32_32;

use crate::{FixedAffine3, FixedMat3, FixedVec3};

/// Contract identity of the transform composition constructors.
pub const TRANSFORM_CONTRACT_ID: &str = "bunny-transform/1";

impl FixedAffine3 {
    /// The transform `T * R * S`: a point is scaled, then rotated, then translated,
    /// `p' = rotation * (scale * p) + translation`.
    ///
    /// The linear part is `rotation * diag(scale)`: entry `(i, j)` is `rotation.mij *
    /// scale_j`, one Q32.32 product rounded once to nearest, ties to even, so it is within 1/2
    /// raw unit of the exact product of the given entries. The translation is copied. Zero and
    /// negative scales are accepted. Returns `None` when a rounded entry is outside the Q32.32
    /// range; that is the only refusal.
    #[must_use]
    pub fn from_trs(translation: FixedVec3, rotation: FixedMat3, scale: FixedVec3) -> Option<Self> {
        let linear = FixedMat3::from_rows(
            scale_columns(rotation.row0(), scale)?,
            scale_columns(rotation.row1(), scale)?,
            scale_columns(rotation.row2(), scale)?,
        );
        Some(Self::from_parts(linear, translation))
    }

    /// The rotation `rotation` about `pivot`, `T(pivot) * R * T(-pivot)`.
    ///
    /// The linear part is `rotation`, copied. The translation is `pivot - rotation * pivot`,
    /// with `rotation * pivot` computed by [`FixedMat3::checked_mul_vec3`] (three products,
    /// each rounded once to nearest, ties to even, summed exactly), so each component is within
    /// 3/2 raw units of the exact value and [`FixedAffine3::checked_transform_point`] maps the
    /// pivot onto itself exactly. Returns `None` when a rounded product, a partial sum or the
    /// subtraction is outside the Q32.32 range.
    #[must_use]
    pub fn rotate_about_point(pivot: FixedVec3, rotation: FixedMat3) -> Option<Self> {
        let moved = rotation.checked_mul_vec3(pivot)?;
        Some(Self::from_parts(rotation, checked_sub_vec3(pivot, moved)?))
    }

    /// The scale `scale` about `pivot`, `T(pivot) * S * T(-pivot)`.
    ///
    /// The linear part is `diag(scale)`, copied. The translation component `i` is `pivot_i -
    /// scale_i * pivot_i`, the product rounded once to nearest, ties to even, so it is within
    /// 1/2 raw unit of the exact value and the pivot maps onto itself exactly. Zero and
    /// negative scales are accepted. Returns `None` when the product or the subtraction is
    /// outside the Q32.32 range.
    #[must_use]
    pub fn scale_about_point(pivot: FixedVec3, scale: FixedVec3) -> Option<Self> {
        let zero = FixedQ32_32::ZERO;
        let diagonal = FixedMat3::from_rows(
            FixedVec3::new(scale.x, zero, zero),
            FixedVec3::new(zero, scale.y, zero),
            FixedVec3::new(zero, zero, scale.z),
        );
        Self::rotate_about_point(pivot, diagonal)
    }
}

/// `row` with each component multiplied by the matching component of `scale`.
fn scale_columns(row: FixedVec3, scale: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        row.x.checked_mul(scale.x)?,
        row.y.checked_mul(scale.y)?,
        row.z.checked_mul(scale.z)?,
    ))
}

fn checked_sub_vec3(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        lhs.x.checked_sub(rhs.x)?,
        lhs.y.checked_sub(rhs.y)?,
        lhs.z.checked_sub(rhs.z)?,
    ))
}
