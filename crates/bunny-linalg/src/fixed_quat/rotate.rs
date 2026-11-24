//! Vector rotation `q v q^-1` and the rotation matrix of `bunny-quaternion/1`.
//!
//! With raw components `(x, y, z, w)` and `S = x^2 + y^2 + z^2 + w^2`, `q v q^-1` is
//! `(1 / S) P v` for the matrix `P` of degree-two polynomials below. `P / S` is exactly the
//! rotation by the direction of the stored quaternion, so dividing by `S` removes the length
//! of `q` from the result instead of scaling by it. No rounding here can meet an exact half:
//! see `docs/topics/quaternion/README.md#no-exact-halves`.

use bunny_num::FixedQ32_32;

use super::wide::U256;
use super::FixedUnitQuat;
use crate::{FixedMat3, FixedVec3};

/// `P` in raw squared units, and `S`. Every entry is at most `S` in magnitude.
struct Polynomial {
    rows: [[i128; 3]; 3],
    sum: i128,
}

impl Polynomial {
    const fn of(quat: FixedUnitQuat) -> Self {
        let [x, y, z, w] = quat.raw();
        let [x, y, z, w] = [x as i128, y as i128, z as i128, w as i128];
        let (xx, yy, zz, ww) = (x * x, y * y, z * z, w * w);
        let (xy, xz, yz) = (2 * x * y, 2 * x * z, 2 * y * z);
        let (wx, wy, wz) = (2 * w * x, 2 * w * y, 2 * w * z);
        Self {
            rows: [
                [ww + xx - yy - zz, xy - wz, xz + wy],
                [xy + wz, ww - xx + yy - zz, yz - wx],
                [xz - wy, yz + wx, ww - xx - yy + zz],
            ],
            sum: ww + xx + yy + zz,
        }
    }
}

impl FixedUnitQuat {
    /// The rotation matrix of the stored quaternion's direction, for column vectors: entry
    /// `(i, j)` is `P_ij 2^32 / S` on raw values, rounded once to nearest (an exact half cannot
    /// occur).
    ///
    /// `P / S` is exactly orthonormal with determinant one, so each entry is within 1/2 raw
    /// unit of the exact rotation; each row and column length is within 1 raw unit of one, each
    /// dot product between rows or between columns within 2 of zero and the determinant within
    /// 3 of one. `q` and `-q` give the same matrix bit for bit.
    #[must_use]
    pub fn to_mat3(self) -> FixedMat3 {
        let p = Polynomial::of(self);
        let row = |[a, b, c]: [i128; 3]| {
            FixedVec3::new(ratio(a << 32, p.sum), ratio(b << 32, p.sum), ratio(c << 32, p.sum))
        };
        let [row0, row1, row2] = p.rows;
        FixedMat3::from_rows(row(row0), row(row1), row(row2))
    }

    /// `q v q^-1`: `v` rotated by the stored quaternion's direction, each component the exact
    /// `(P v)_i / S` on raw values rounded once to nearest (an exact half cannot occur).
    ///
    /// Each component is within 1/2 raw unit of the exact rotation of `v`, so the result's
    /// length is within `sqrt(3) / 2` raw units of `|v|`, whatever `|v|` is: the quaternion's
    /// own length is divided out exactly. `q` and `-q` give the same result bit for bit.
    ///
    /// Returns `None` when a rounded component is outside the Q32.32 range; never when every
    /// component of `v` is at most `2^62` raw in magnitude.
    #[must_use]
    pub fn rotate_vec3(self, v: FixedVec3) -> Option<FixedVec3> {
        let p = Polynomial::of(self);
        let v = [v.x.raw(), v.y.raw(), v.z.raw()];
        let sum = p.sum.unsigned_abs();
        let [row0, row1, row2] = p.rows;
        Some(FixedVec3::new(apply(row0, v, sum)?, apply(row1, v, sum)?, apply(row2, v, sum)?))
    }
}

/// `numerator / denominator` rounded to nearest for `denominator > 0` and a quotient within
/// `[-2^33, 2^33]`. Twice the remainder never equals the denominator here.
#[allow(clippy::cast_possible_truncation)]
const fn ratio(numerator: i128, denominator: i128) -> FixedQ32_32 {
    let (floor, remainder) = (numerator.div_euclid(denominator), numerator.rem_euclid(denominator));
    let up = 2 * remainder > denominator;
    FixedQ32_32::from_raw((floor + if up { 1 } else { 0 }) as i64)
}

/// `sum_j row_j v_j / sum` rounded to nearest (twice the remainder never equals `sum`);
/// `None` outside the raw range.
fn apply(row: [i128; 3], v: [i64; 3], sum: u128) -> Option<FixedQ32_32> {
    let (negative, magnitude) = signed_sum(row, v);
    // The magnitude is below 3 2^128 and S is at least 2^63, so the quotient is below 2^67.
    let (quotient, remainder) = magnitude.div_rem(sum);
    let up = remainder > sum - remainder;
    let rounded = i128::try_from(quotient + u128::from(up)).ok()?;
    let signed = if negative { -rounded } else { rounded };
    i64::try_from(signed).ok().map(FixedQ32_32::from_raw)
}

/// The exact `sum_j row_j v_j` as a sign (`true` when negative) and a 256-bit magnitude.
fn signed_sum(row: [i128; 3], v: [i64; 3]) -> (bool, U256) {
    // Each |row_j| <= S < 2^65 and |v_j| <= 2^63, so each product is below 2^128 and the
    // three are summed by sign in 256 bits.
    let (mut positive, mut negative) = (U256::ZERO, U256::ZERO);
    for (entry, component) in row.iter().zip(v) {
        let term = U256::product(entry.unsigned_abs(), u128::from(component.unsigned_abs()));
        if (*entry < 0) == (component < 0) {
            positive = positive.add(term);
        } else {
            negative = negative.add(term);
        }
    }
    if negative.le(positive) {
        (false, positive.sub(negative))
    } else {
        (true, negative.sub(positive))
    }
}
