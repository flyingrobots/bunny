//! Look rotation (`bunny-look-rotation/1`): a [`FixedMat3`] that turns an
//! object's local `-Z` onto a forward direction, with an up hint fixing roll.
//!
//! It is the rotation part of [`FixedMat4::try_look_at`](crate::FixedMat4::try_look_at)
//! with the eye at the origin, transposed: the same basis, the same rounding
//! and the same refusals. See `docs/topics/look-rotation/README.md`.

use super::{basis, Raw3};
use crate::{FixedMat3, FixedVec3, LookAtError};

impl FixedMat3 {
    /// Builds the rotation whose columns are `right`, `true_up` and `back`, so
    /// that `world = rotation * local` maps local `-Z` onto `forward`, local
    /// `+Y` onto the true up and local `+X` onto `right`.
    ///
    /// The frame is right-handed (the coordinate law's camera convention):
    /// `f = normalize(forward)`, `right = normalize(f x up)`, `true_up =
    /// normalize(right x f)` and `back = -f`. Each cross product is exact on
    /// the raw `i128` components, and each normalization rounds every component
    /// to nearest, ties away from zero, to within 1/2 + 2^-26 raw units of the
    /// ideal unit direction. Axis-aligned inputs give exact results.
    ///
    /// The result is orthonormal only within rounding. Each column has a length
    /// within 2 raw units of one, and each pair of columns a dot product within
    /// 2 raw units of zero; each row has a length within 3 raw units of one, and
    /// each pair of rows a dot product within 4 raw units of zero. Dot products
    /// are exact sums of raw products divided by `2^32`.
    ///
    /// # Errors
    /// Returns [`LookAtError::CoincidentEyeTarget`] when `forward` is exactly
    /// zero (the case `target == eye` of `try_look_at`); any nonzero raw vector,
    /// however short, is a direction. Returns [`LookAtError::ParallelUp`] when
    /// `up` is zero, exactly parallel or antiparallel to `forward` (decided on
    /// the exact raw inputs), or exactly parallel to the rounded unit `f`. A
    /// nearly parallel `up` is accepted; the bounds still hold but the roll is
    /// fixed only to rounding noise. [`LookAtError::Overflow`] is never returned:
    /// every component of `f` is within one raw unit of `[-1, 1]`.
    pub fn try_look_rotation(forward: FixedVec3, up: FixedVec3) -> Result<Self, LookAtError> {
        let [right, true_up, back] = basis(Raw3::from_fixed(forward), up)?;
        Ok(Self::from_rows(right, true_up, back).transpose())
    }
}
