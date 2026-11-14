//! Exact `i128` vector arithmetic and the two rounding rules the shading steps use.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

/// Raw Q32.32 one.
pub const ONE: i128 = 1 << 32;
/// Vectors are scaled by a power of two until their largest component magnitude is in
/// `[2^61, 2^62)` before they are normalized, as the camera basis does.
const PRESCALE_BIT: u32 = 61;

pub type Wide = [i128; 3];

pub fn wide(v: FixedVec3) -> Wide {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

/// A vector of raw values, each clamped to `i64`. Every caller passes values inside `i64` (below
/// `2^62` in magnitude); the clamp only makes the conversion total.
pub fn narrow(v: Wide) -> FixedVec3 {
    let [x, y, z] = v.map(|c| FixedQ32_32::from_raw(narrow_raw(c)));
    FixedVec3::new(x, y, z)
}

#[allow(clippy::cast_possible_truncation)] // Clamped into the `i64` range first.
const fn narrow_raw(value: i128) -> i64 {
    let max = i64::MAX as i128;
    (if value > max {
        max
    } else if value < -max {
        -max
    } else {
        value
    }) as i64
}

pub const fn sub(a: Wide, b: Wide) -> Wide {
    let ([ax, ay, az], [bx, by, bz]) = (a, b);
    [ax - bx, ay - by, az - bz]
}

pub const fn dot(a: Wide, b: Wide) -> i128 {
    let ([ax, ay, az], [bx, by, bz]) = (a, b);
    ax * bx + ay * by + az * bz
}

pub const fn cross(a: Wide, b: Wide) -> Wide {
    let ([ax, ay, az], [bx, by, bz]) = (a, b);
    [ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx]
}

/// `numerator / denominator` rounded to nearest, ties to even, for a nonzero denominator.
/// The rule is symmetric: a negative quotient rounds as its magnitude does.
pub const fn div_round(numerator: i128, denominator: i128) -> i128 {
    let (n, d) = (numerator.unsigned_abs(), denominator.unsigned_abs());
    let (quotient, remainder) = (n / d, n % d);
    // remainder < d <= 2^64 in every caller, so doubling it cannot overflow.
    let twice = remainder * 2;
    let magnitude =
        if twice > d || (twice == d && quotient & 1 == 1) { quotient + 1 } else { quotient };
    #[allow(clippy::cast_possible_wrap)] // Every quotient here is below 2^70.
    let magnitude = magnitude as i128;
    if (numerator < 0) == (denominator < 0) {
        magnitude
    } else {
        -magnitude
    }
}

/// `value / 2^shift` rounded to nearest, ties to even, for `shift` in `1..=126`.
pub const fn round_shift(value: i128, shift: u32) -> i128 {
    let floor = value >> shift;
    let remainder = value - (floor << shift);
    let half = 1 << (shift - 1);
    if remainder > half || (remainder == half && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// The unit vector along a nonzero vector of any length up to `2^126`: the vector is scaled by a
/// power of two that puts its largest component magnitude in `[2^61, 2^62)` (exact when it
/// scales up, floored when it scales down), then normalized by `FixedUnitVec3::new`.
///
/// Returns `None` for the zero vector, and, defensively, if `FixedUnitVec3::new` refuses the
/// prescaled vector; no input in the tests reaches that.
pub fn unit(v: Wide) -> Option<FixedUnitVec3> {
    let [x, y, z] = v.map(i128::unsigned_abs);
    let top = x.max(y).max(z);
    let bits = top.checked_ilog2()?;
    let scale = |c: i128| {
        if bits <= PRESCALE_BIT {
            c << (PRESCALE_BIT - bits)
        } else {
            c >> (bits - PRESCALE_BIT)
        }
    };
    FixedUnitVec3::new(narrow(v.map(scale)))
}

/// Component `axis` of `vector`, for `axis` in `0..3` (any larger `axis` reads `z`).
pub const fn pick(vector: Wide, axis: usize) -> i128 {
    let [x, y, z] = vector;
    match axis {
        0 => x,
        1 => y,
        _ => z,
    }
}
