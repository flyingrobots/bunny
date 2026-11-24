//! Axis-angle to quaternion for `bunny-quaternion/1`.

use bunny_num::FixedAngle;

use super::normalize::normalize;
use super::FixedUnitQuat;
use crate::FixedUnitVec3;

impl FixedUnitQuat {
    /// The rotation by `angle` about `axis`: `(sin(h) u, cos(h))` for the half angle `h`,
    /// normalised exactly and rounded once per component.
    ///
    /// The angle is read as signed, in `[-1/2, 1/2)` turn, and halved toward zero, so `h` is
    /// within a quarter turn and `w >= 0`; for an odd raw angle `h` is `2^-65` turn nearer
    /// zero than the exact half. `(s, c) = h.sin_cos()` and `u` is the axis as given; the
    /// exact `(s u, c)` is normalised by the rule of [`FixedUnitQuat::try_normalize`], so the
    /// result is within 1/2 raw unit per component of the exact normalised `(s u, c)` and in
    /// the length band, whatever the axis's own length.
    ///
    /// The angle zero gives exactly [`FixedUnitQuat::IDENTITY`], the negated angle exactly the
    /// conjugate (except at the half turn, which is its own negation), and the half turn about
    /// an axis `u` the quaternion `(-u, 0)`, normalised.
    #[must_use]
    pub fn from_axis_angle(axis: FixedUnitVec3, angle: FixedAngle) -> Self {
        let half = FixedAngle::from_raw((angle.raw().cast_signed() / 2).cast_unsigned());
        let (sin, cos) = half.sin_cos();
        let u = axis.into_inner();
        let s = i128::from(sin.raw());
        // |s| <= 2^32 and each axis component is below 2^32 + 2, so each product is below 2^65;
        // the vector is nonzero because s and c are within one raw unit of a sine and cosine.
        Self::from_raw(normalize([
            s * i128::from(u.x.raw()),
            s * i128::from(u.y.raw()),
            s * i128::from(u.z.raw()),
            i128::from(cos.raw()) << 32,
        ]))
    }
}
