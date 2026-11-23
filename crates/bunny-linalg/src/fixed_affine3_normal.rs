//! Surface-normal transform for [`FixedAffine3`]: the inverse transpose of the linear part.
//! See `docs/topics/transform/README.md#normals`.

use crate::{FixedAffine3, FixedVec3};

impl FixedAffine3 {
    /// Transforms a surface normal by the inverse transpose of the linear part, `L^-T n`.
    ///
    /// A normal is perpendicular to every tangent `t` of a surface; `L^-T n` stays
    /// perpendicular to `L t` because `(L^-T n) . (L t) = n . t`. Under a non-uniform scale the
    /// linear part alone does not keep it so. The translation is ignored. The result is **not
    /// renormalised**: its length changes with the scale, and a caller that needs a unit normal
    /// normalises it.
    ///
    /// The result is `self.linear.try_inverse()` transposed, multiplied by `normal` with
    /// [`FixedMat3::checked_mul_vec3`](crate::FixedMat3::checked_mul_vec3): each component is
    /// three products of rounded-inverse entries with `normal`, each rounded once to nearest,
    /// ties to even, from the exact `i128` product, and summed exactly, so it is within 3/2 raw
    /// of the exact product of the **rounded inverse** and `normal`. The rounded inverse carries
    /// `try_inverse`'s own rounding (rounded minors and determinant, one rounded division per
    /// entry), whose distance from the exact inverse is not bounded in raw units independently
    /// of the matrix, so no bound against the exact `L^-T n` is claimed. Where the inverse
    /// entries and every product are exact (power-of-two scales, signed permutations), the
    /// result is exact and `n' . (L t)` equals `n . t` exactly.
    ///
    /// Returns `None` when `try_inverse` is `None` (the rounded determinant is zero, which
    /// includes every singular matrix and some nonsingular ones with a tiny determinant, or the
    /// inverse overflows) or when a rounded product or partial sum is outside the Q32.32 range.
    #[must_use]
    pub fn checked_transform_normal(self, normal: FixedVec3) -> Option<FixedVec3> {
        self.linear.try_inverse()?.transpose().checked_mul_vec3(normal)
    }
}
