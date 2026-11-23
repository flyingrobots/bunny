//! Equirectangular environment coordinates: a direction to `(u, v)` and back,
//! `bunny-environment-equirect/1`.

use core::fmt;

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};

/// The versioned name of [`equirect_from_direction`] and [`direction_from_equirect`]. Any
/// change to an output bit is a new version.
pub const ENVIRONMENT_EQUIRECT_ID: &str = "bunny-environment-equirect/1";

/// One raw unit of 1.0 in Q32.32.
const ONE: i64 = 1 << 32;

/// The bit width the polar pair is scaled to: its larger magnitude is in `[2^61, 2^62)`.
const POLAR_BITS: u32 = 62;

/// Why a direction or a coordinate pair has no image in the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquirectError {
    /// The direction is the zero vector: every component is exactly zero.
    ZeroDirection,
    /// `v` is below zero or above 1.0 (raw outside `0..=2^32`).
    VOutsideUnitInterval,
}

impl fmt::Display for EquirectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroDirection => "the zero vector has no direction",
            Self::VOutsideUnitInterval => "v is outside [0, 1]",
        })
    }
}

impl std::error::Error for EquirectError {}

/// The equirectangular coordinates `(u, v)` of a direction (`x` is `u`, `y` is `v`).
///
/// `u` is the longitude `atan2(-x, z)` in turns, counter-clockwise seen from `+y`: `+z` is
/// `u = 0` (the seam), `-x` is `1/4`, `-z` is `1/2` (the image centre) and `+x` is `3/4`;
/// `u` is in `[0, 1)`. `v` is the polar angle from `+y` in half turns: `+y` is `v = 0` (the
/// top row), the horizon `1/2` and `-y` is `1`; `v` is in `[0, 1]`. At the poles, where
/// `x = z = 0` exactly, `u` is 0.
///
/// The direction may have any length; only its direction matters, and doubling it, where it
/// still fits, gives the same bits.
///
/// **Bound:** `u` is within `1/2 + 2^-32` raw units of the exact longitude, measured around
/// the circle, and `v` within `1/2 + 2^-29` raw units of the exact polar angle. Each is one
/// rounding to nearest, ties to even, of a `FixedAngle` from `FixedAngle::atan2`. See
/// `docs/topics/optics/README.md#environment-coordinates`.
///
/// # Errors
/// [`EquirectError::ZeroDirection`] when every component is zero.
///
/// # Examples
///
/// ```
/// use bunny_linalg::FixedVec3;
/// use bunny_num::FixedQ32_32;
/// use bunny_optics::equirect_from_direction;
///
/// let back = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, -FixedQ32_32::ONE);
/// let uv = equirect_from_direction(back).expect("a direction");
/// assert_eq!((uv.x.raw(), uv.y.raw()), (1 << 31, 1 << 31));
/// ```
pub fn equirect_from_direction(direction: FixedVec3) -> Result<FixedVec2, EquirectError> {
    let raw = [direction.x, direction.y, direction.z].map(FixedQ32_32::raw);
    if raw == [0; 3] {
        return Err(EquirectError::ZeroDirection);
    }
    let longitude = FixedAngle::atan2(direction.x, direction.z);
    let u = longitude.map_or(0, |angle| turn_fraction(angle.wrapping_neg()));
    let (horizontal, vertical) = polar_pair(raw);
    let polar = FixedAngle::atan2(fixed(horizontal), fixed(vertical));
    let v = polar.map_or(0, |angle| half_turns(angle.raw()));
    Ok(FixedVec2::new(fixed(u), fixed(v)))
}

/// The unit direction of the equirectangular coordinates `(u, v)` (`x` is `u`, `y` is `v`),
/// the inverse of [`equirect_from_direction`].
///
/// With `theta = v` half turns and `phi = u` turns, the direction is
/// `(-sin theta sin phi, cos theta, sin theta cos phi)`. Any `u` is accepted: whole turns are
/// discarded, so `u` and `u + 1` give the same bits. `v = 0` gives `(0, 1, 0)` and `v = 1`
/// gives `(0, -1, 0)` exactly, for every `u`.
///
/// **Bound:** `x` and `z` are each within `5/2` raw units and `y` within one raw unit of the
/// exact direction, so the length is within `3.68` raw units of 1.0. The sines and cosines
/// are `FixedAngle::sin_cos`, and each of the two products is rounded once to nearest, ties
/// to even.
///
/// # Errors
/// [`EquirectError::VOutsideUnitInterval`] when `v` is below 0 or above 1.0.
///
/// # Examples
///
/// ```
/// use bunny_linalg::FixedVec2;
/// use bunny_num::FixedQ32_32;
/// use bunny_optics::direction_from_equirect;
///
/// let centre = FixedVec2::new(FixedQ32_32::from_raw(1 << 31), FixedQ32_32::from_raw(1 << 31));
/// let d = direction_from_equirect(centre).expect("in range");
/// assert_eq!((d.x.raw(), d.y.raw(), d.z.raw()), (0, 0, -(1 << 32)));
/// ```
pub fn direction_from_equirect(uv: FixedVec2) -> Result<FixedVec3, EquirectError> {
    let v = uv.y.raw();
    if !(0..=ONE).contains(&v) {
        return Err(EquirectError::VOutsideUnitInterval);
    }
    // `v <= 2^32`, so `v 2^31 <= 2^63`: exactly `v` half turns.
    let polar = FixedAngle::from_raw(v.unsigned_abs() << 31);
    let (sin_polar, cos_polar) = polar.sin_cos();
    let (sin_long, cos_long) = FixedAngle::from_turns(uv.x).sin_cos();
    let x = rounded_product(sin_polar.raw(), sin_long.raw()).wrapping_neg();
    let z = rounded_product(sin_polar.raw(), cos_long.raw());
    Ok(FixedVec3::new(fixed(x), cos_polar, fixed(z)))
}

const fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

/// An angle in turns, rounded to raw Q32.32 by `to_turns` (ties to even), with a full turn
/// wrapped to 0 so the result is in `[0, 1)`.
const fn turn_fraction(angle: FixedAngle) -> i64 {
    let turns = angle.to_turns().raw();
    if turns == ONE {
        0
    } else {
        turns
    }
}

/// A raw angle `a <= 2^63` in half turns: `a / 2^31` rounded to nearest, ties to even.
fn half_turns(raw: u64) -> i64 {
    let (quotient, remainder) = (raw >> 31, raw & ((1 << 31) - 1));
    let half = 1 << 30;
    let up = remainder > half || (remainder == half && quotient & 1 == 1);
    let rounded = quotient + u64::from(up);
    i64::try_from(rounded).unwrap_or(ONE)
}

/// `(floor(r 2^k), floor(y 2^k))` with `r = sqrt(x^2 + z^2)`, for the `k` (of either sign)
/// that puts the larger of `|y| 2^k` and `r 2^k` in `[2^61, 2^62)`.
///
/// With `L = max(bits(|y|), ceil(bits(x^2 + z^2) / 2))`, `k = 62 - L`. For `k >= 0`,
/// `(x^2 + z^2) 4^k < 2^124` and `y 2^k` is exact; for `k < 0` (`L` of 63 or 64), both are
/// shifted right, and `isqrt(floor(n / 4^s)) = floor(sqrt(n) / 2^s)`. Both results are below
/// `2^62` in magnitude.
fn polar_pair([x, y, z]: [i64; 3]) -> (i64, i64) {
    let square = |component: i64| u128::from(component.unsigned_abs()).pow(2);
    // Each square is at most 2^126, so the sum is at most 2^127.
    let sum = square(x) + square(z);
    let length = bits(u128::from(y.unsigned_abs())).max(bits(sum).div_ceil(2));
    let (horizontal, vertical) = if length <= POLAR_BITS {
        let up = POLAR_BITS - length;
        (FixedQ32_32::sqrt_u128(sum << (2 * up)), i128::from(y) << up)
    } else {
        let down = length - POLAR_BITS;
        (FixedQ32_32::sqrt_u128(sum >> (2 * down)), i128::from(y) >> down)
    };
    (narrow(horizontal.cast_signed()), narrow(vertical))
}

/// A value already proven below `2^62` in magnitude, as `i64`.
fn narrow(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// The number of significant bits of `value`.
const fn bits(value: u128) -> u32 {
    128 - value.leading_zeros()
}

/// `a b / 2^32` rounded to nearest, ties to even, for `|a|, |b| <= 2^32`.
fn rounded_product(a: i64, b: i64) -> i64 {
    let product = i128::from(a) * i128::from(b);
    let (quotient, remainder) = (product >> 32, product & ((1 << 32) - 1));
    let half = 1 << 31;
    let up = remainder > half || (remainder == half && quotient & 1 == 1);
    narrow(quotient + i128::from(up))
}
