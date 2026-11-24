//! The Hamilton product of `bunny-quaternion/1`, renormalised in the same single rounding.

use core::ops::Mul;

use super::normalize::normalize;
use super::FixedUnitQuat;

impl FixedUnitQuat {
    /// The Hamilton product `self * rhs`, normalised: the exact product of the stored values,
    /// divided exactly by its length, each component rounded once to the nearest raw unit.
    ///
    /// The rotation `self * rhs` applies `rhs` first, then `self`, as the matrix product
    /// `A * B` does on column vectors (right to left).
    ///
    /// Each component is within 1/2 raw unit of the exact unit quaternion in the direction of
    /// the exact product, and within `5/2 + 2^-32` raw units of the exact (unnormalised)
    /// product. The result is in the length band, so it is a [`FixedUnitQuat`] and the
    /// product never refuses. It is one rounding, not a product followed by a renormalisation.
    #[must_use]
    pub const fn product(self, rhs: Self) -> Self {
        let [ax, ay, az, aw] = wide(self.raw());
        let [bx, by, bz, bw] = wide(rhs.raw());
        // Each raw product is below (2^32 + 1)^2 < 2^65, so each sum of four is below 2^67.
        let exact = [
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz,
        ];
        // |exact| = |self| |rhs| > 0, and each component is below 2^66 in magnitude.
        Self::from_raw(normalize(exact))
    }
}

/// [`FixedUnitQuat::product`]: `a * b` applies `b` first.
impl Mul for FixedUnitQuat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        self.product(rhs)
    }
}

/// Raw components widened to `i128`.
const fn wide(raw: [i64; 4]) -> [i128; 4] {
    let [x, y, z, w] = raw;
    [x as i128, y as i128, z as i128, w as i128]
}
