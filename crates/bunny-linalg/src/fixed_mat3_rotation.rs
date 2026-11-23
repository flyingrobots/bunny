//! Rotations from a [`FixedAngle`] (`bunny-rotation/1`): about the coordinate axes and about a
//! unit axis by Rodrigues' formula. See `docs/topics/rotation/README.md`.
//!
//! Positive angles turn counter-clockwise when the axis points at the viewer: the right-hand
//! rule of the coordinate law, the sense of [`FixedAngle`] about `+Z`. Matrices act on column
//! vectors, as [`FixedMat3::try_look_rotation`] does.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};

use crate::{FixedMat3, FixedUnitVec3, FixedVec3};

/// Contract identity of the rotation constructors.
pub const ROTATION_CONTRACT_ID: &str = "bunny-rotation/1";

const ZERO: FixedQ32_32 = FixedQ32_32::ZERO;
const ONE: FixedQ32_32 = FixedQ32_32::ONE;

impl FixedMat3 {
    /// The rotation by `angle` about `+X`: rows `(1, 0, 0)`, `(0, c, -s)` and `(0, s, c)`,
    /// with `(s, c) = angle.sin_cos()`. A quarter turn maps `+Y` onto `+Z`.
    ///
    /// The entries are `sin_cos` copied, so each is within one raw unit of the exact value
    /// and the cardinal angles give exactly `0`, `1` and `-1`. Rows and columns have exactly
    /// zero dot products and lengths `sqrt(c^2 + s^2)`, within 3/2 raw units of one; the
    /// determinant is exactly `c^2 + s^2`, within 3 raw units of one.
    #[must_use]
    pub fn rotation_x(angle: FixedAngle) -> Self {
        let (s, c) = angle.sin_cos();
        Self::from_rows(
            FixedVec3::new(ONE, ZERO, ZERO),
            FixedVec3::new(ZERO, c, -s),
            FixedVec3::new(ZERO, s, c),
        )
    }

    /// The rotation by `angle` about `+Y`: rows `(c, 0, s)`, `(0, 1, 0)` and `(-s, 0, c)`,
    /// with `(s, c) = angle.sin_cos()`. A quarter turn maps `+Z` onto `+X` and `+X` onto `-Z`.
    ///
    /// Exactness and bounds are those of [`FixedMat3::rotation_x`].
    #[must_use]
    pub fn rotation_y(angle: FixedAngle) -> Self {
        let (s, c) = angle.sin_cos();
        Self::from_rows(
            FixedVec3::new(c, ZERO, s),
            FixedVec3::new(ZERO, ONE, ZERO),
            FixedVec3::new(-s, ZERO, c),
        )
    }

    /// The rotation by `angle` about `+Z`: rows `(c, -s, 0)`, `(s, c, 0)` and `(0, 0, 1)`,
    /// with `(s, c) = angle.sin_cos()`. A quarter turn maps `+X` onto `+Y`.
    ///
    /// Exactness and bounds are those of [`FixedMat3::rotation_x`].
    #[must_use]
    pub fn rotation_z(angle: FixedAngle) -> Self {
        let (s, c) = angle.sin_cos();
        Self::from_rows(
            FixedVec3::new(c, -s, ZERO),
            FixedVec3::new(s, c, ZERO),
            FixedVec3::new(ZERO, ZERO, ONE),
        )
    }

    /// The rotation by `angle` about `axis` by Rodrigues' formula,
    /// `c I + s [u]x + (1 - c) u u^T` with `(s, c) = angle.sin_cos()` and `u` the axis as
    /// given (it is not renormalized).
    ///
    /// Each entry is computed exactly from the raw values over `2^64` in `i128` (every term is
    /// below `2^99`) and rounded once to nearest, ties to even. Each entry is within
    /// `3/2 + 2^-30` raw units of the exact formula on the given axis at the exact angle. The
    /// axis of a [`FixedUnitVec3`] is shorter than `1 + 2^-31` and at least `1 - 2^-32`, and
    /// that departure dominates: each entry is within `19/2 + 2^-28` raw units of the exact
    /// rotation about the axis direction. Row and column lengths are within 11 raw units of
    /// one, dot products between rows or between columns within 12 of zero, and the
    /// determinant within 15 of one. Angle zero gives exactly the identity, the negated angle
    /// exactly the transpose, and the six unit axis constants exactly the axis rotations.
    #[must_use]
    pub fn rotation_axis_angle(axis: FixedUnitVec3, angle: FixedAngle) -> Self {
        let (sin, cos) = angle.sin_cos();
        let direction = axis.into_inner();
        let terms = Rodrigues {
            c: i128::from(cos.raw()),
            s: i128::from(sin.raw()),
            t: i128::from(ONE_RAW) - i128::from(cos.raw()),
        };
        let [x, y, z] = [direction.x, direction.y, direction.z].map(|v| i128::from(v.raw()));
        Self::from_rows(
            FixedVec3::new(terms.diagonal(x), terms.off(x, y, -z), terms.off(x, z, y)),
            FixedVec3::new(terms.off(x, y, z), terms.diagonal(y), terms.off(y, z, -x)),
            FixedVec3::new(terms.off(x, z, -y), terms.off(y, z, x), terms.diagonal(z)),
        )
    }
}

/// Raw cosine, sine and `1 - cosine` of the angle.
struct Rodrigues {
    c: i128,
    s: i128,
    t: i128,
}

impl Rodrigues {
    /// `c + t a^2`, from `c 2^64 + t a a` over `2^64`.
    const fn diagonal(&self, a: i128) -> FixedQ32_32 {
        round_q64((self.c << 64) + self.t * a * a)
    }

    /// `t a b + s k`, from `t a b + s k 2^32` over `2^64`.
    const fn off(&self, a: i128, b: i128, k: i128) -> FixedQ32_32 {
        round_q64(self.t * a * b + ((self.s * k) << 32))
    }
}

/// `value / 2^64` rounded to nearest, ties to even, as raw Q32.32.
///
/// Every value here is below `2^99` in magnitude, so the result is below `2^35` and fits.
#[allow(clippy::cast_possible_truncation)]
const fn round_q64(value: i128) -> FixedQ32_32 {
    let floor = value >> 64;
    let remainder = value & ((1 << 64) - 1);
    let half = 1 << 63;
    let rounded =
        if remainder > half || (remainder == half && floor & 1 == 1) { floor + 1 } else { floor };
    FixedQ32_32::from_raw(rounded as i64)
}
