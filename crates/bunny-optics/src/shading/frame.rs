//! An orthonormal frame around a unit normal, and the cosine-weighted hemisphere direction.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};

use super::refusal::SecondaryRayError;
use super::wide::{div_round, dot, narrow, pick, round_shift, wide, Wide, ONE};

/// A tangent, a bitangent and a normal, orthonormal to within a few raw units and right-handed
/// (`tangent x bitangent = normal`), with the normal pointing to the side rays leave from.
///
/// [`Self::new`] builds the frame of Duff, Burgess, Christensen, Hery, Kensler, Liani and
/// Villemin ("Building an Orthonormal Basis, Revisited", JCGT 6(1), 2017), which has no branch
/// except the sign of the normal's `z`. See
/// `docs/topics/optics/README.md#orthonormal-frame`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShadingFrame {
    tangent: Wide,
    bitangent: Wide,
    normal: Wide,
}

impl ShadingFrame {
    /// The frame around `normal`. With `s = 1` when `normal.z >= 0` and `-1` otherwise, and
    /// `q(a) = a / (s + z)` rounded to nearest (ties to even) in raw units:
    ///
    /// ```text
    /// tangent   = (1 - s q(x x), -s q(x y), -s x)
    /// bitangent = (-q(x y), s - q(y y), -y)
    /// ```
    ///
    /// Each component is within half a raw unit of the exact formula on the given normal.
    #[must_use]
    pub fn new(normal: FixedUnitVec3) -> Self {
        let normal = wide(normal.into_inner());
        let [x, y, z] = normal;
        let sign = if z >= 0 { 1 } else { -1 };
        // `|z| <= 1` within a raw unit, so `|s + z|` is at least 1.0 and the quotients exist.
        let denominator = sign * ONE + z;
        let q = |a: i128| div_round(a, denominator);
        let xy = q(x * y);
        Self {
            tangent: [ONE - sign * q(x * x), -sign * xy, -sign * x],
            bitangent: [-xy, sign * ONE - q(y * y), -y],
            normal,
        }
    }

    /// The same frame seen from the other side: `(bitangent, tangent, -normal)`, still
    /// right-handed. No arithmetic beyond negation.
    #[must_use]
    pub fn flipped(self) -> Self {
        Self { tangent: self.bitangent, bitangent: self.tangent, normal: self.normal.map(|c| -c) }
    }

    /// The tangent, raw Q32.32.
    #[must_use]
    pub fn tangent(&self) -> FixedVec3 {
        narrow(self.tangent)
    }

    /// The bitangent, raw Q32.32.
    #[must_use]
    pub fn bitangent(&self) -> FixedVec3 {
        narrow(self.bitangent)
    }

    /// The normal, raw Q32.32.
    #[must_use]
    pub fn normal(&self) -> FixedVec3 {
        narrow(self.normal)
    }

    /// The cosine-weighted direction on the normal's side for the unit-square point
    /// `sample = [u, v]`, each coordinate in `[0, 1)`: the polar disk point
    /// `(sqrt(u) cos(2 pi v), sqrt(u) sin(2 pi v))` lifted to the hemisphere, with
    /// `cos(theta) = sqrt(1 - u)`.
    ///
    /// The two square roots are floored, the angle is exact in turns and `FixedAngle::sin_cos`
    /// is within one raw unit; the frame combination is exact in `i128` and rounded once per
    /// component (to nearest, ties to even). Each component is then clamped to `-1.0..=1.0`, so
    /// the direction is a valid trace direction. Each component is within 4 raw units of the
    /// ideal map on this frame, and the dot product with the normal is at least
    /// [`super::MIN_BOUNCE_COSINE_RAW`] raw units.
    ///
    /// Returns `None` exactly when [`Self::try_cosine_direction`] returns an error: when a
    /// coordinate is outside `[0, 1)`.
    #[must_use]
    pub fn cosine_direction(&self, sample: [FixedQ32_32; 2]) -> Option<FixedVec3> {
        self.try_cosine_direction(sample).ok()
    }

    /// [`Self::cosine_direction`], naming the refusal.
    ///
    /// # Errors
    ///
    /// [`SecondaryRayError::SampleOutsideUnitInterval`] when a coordinate of `sample` is
    /// outside `[0, 1)`.
    #[inline]
    pub fn try_cosine_direction(
        &self,
        sample: [FixedQ32_32; 2],
    ) -> Result<FixedVec3, SecondaryRayError> {
        let [u, v] = sample;
        let outside = SecondaryRayError::SampleOutsideUnitInterval { sample };
        if !in_unit_interval(u) || !in_unit_interval(v) {
            return Err(outside);
        }
        // sqrt(u) is in [0, 1); sqrt(1 - u) is at least sqrt(2^-32) = 2^-16, raw 2^16.
        // `FixedQ32_32::sqrt` refuses only a negative value, and both values are non-negative
        // for a sample inside the unit square, so these refusals are never taken.
        let radius = i128::from(u.sqrt().ok_or(outside)?.raw());
        let height = FixedQ32_32::from_raw(ONE_RAW - u.raw()).sqrt().ok_or(outside)?;
        let height = i128::from(height.raw());
        let (sin, cos) = FixedAngle::from_turns(v).sin_cos();
        let weights =
            [radius * i128::from(cos.raw()), radius * i128::from(sin.raw()), height * ONE];
        Ok(narrow(self.combine(weights)))
    }

    /// `weights . (tangent_i, bitangent_i, normal_i)` for each axis `i`, in units of `2^-96`
    /// before one rounding to raw units; then clamped to `-1.0..=1.0`. Each weight is at most
    /// `2^64` (plus a raw unit) and each frame component at most `2^32` plus a few raw units,
    /// so every sum is below `2^98`.
    fn combine(&self, weights: Wide) -> Wide {
        let axis = |i: usize| {
            let column = [self.tangent, self.bitangent, self.normal].map(|v| pick(v, i));
            round_shift(dot(weights, column), 64).clamp(-ONE, ONE)
        };
        [axis(0), axis(1), axis(2)]
    }
}

const ONE_RAW: i64 = 1 << 32;

const fn in_unit_interval(value: FixedQ32_32) -> bool {
    value.raw() >= 0 && value.raw() < ONE_RAW
}
