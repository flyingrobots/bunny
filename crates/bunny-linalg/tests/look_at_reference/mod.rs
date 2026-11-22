//! The look-at implementation exactly as shipped in 0.7.0, kept verbatim as the
//! reference for the differential fidelity test. Do not edit: it is the oracle.
#![allow(dead_code)]

use bunny_linalg::{FixedMat4, FixedMat4Row, FixedVec3, LookAtError};
use bunny_num::FixedQ32_32;

use bunny_num::fixed_q32_32::FRAC_BITS;

/// An exact integer vector in raw Q32.32 units, wide enough for cross products.
#[derive(Clone, Copy)]
struct Raw3 {
    x: i128,
    y: i128,
    z: i128,
}

impl Raw3 {
    fn from_fixed(v: FixedVec3) -> Self {
        Self { x: i128::from(v.x.raw()), y: i128::from(v.y.raw()), z: i128::from(v.z.raw()) }
    }

    fn difference(to: FixedVec3, from: FixedVec3) -> Self {
        let (a, b) = (Self::from_fixed(to), Self::from_fixed(from));
        Self { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z }
    }

    const fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    fn max_abs(self) -> u128 {
        self.x.unsigned_abs().max(self.y.unsigned_abs()).max(self.z.unsigned_abs())
    }

    fn map(self, f: impl Fn(i128) -> i128) -> Self {
        Self { x: f(self.x), y: f(self.y), z: f(self.z) }
    }

    fn length_squared(self) -> u128 {
        let square = |c: i128| c.unsigned_abs() * c.unsigned_abs();
        square(self.x) + square(self.y) + square(self.z)
    }
}

/// Target bit length of the largest component before the division.
const SCALED_BITS: u32 = 61;

/// Scales every component so the largest has exactly [`SCALED_BITS`] bits. The
/// direction changes by at most about `2^-60` relative.
fn rescale(v: Raw3, largest: u128) -> Raw3 {
    let bits = u128::BITS - largest.leading_zeros();
    if bits <= SCALED_BITS {
        let shift = SCALED_BITS - bits;
        v.map(|c| c << shift)
    } else {
        let shift = bits - SCALED_BITS;
        v.map(|c| (c + (1 << (shift - 1))) >> shift)
    }
}

/// Divides `numerator` by `divisor > 0`, rounding half away from zero.
const fn round_div(numerator: i128, divisor: i128) -> i128 {
    let magnitude = (numerator.abs() + divisor / 2) / divisor;
    if numerator < 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// Normalizes an exact raw vector to a unit `FixedVec3`.
///
/// Returns `None` for the zero vector. Each returned component is within 1 raw
/// unit of the ideal unit direction, however small or large the input is.
fn unit(v: Raw3) -> Option<FixedVec3> {
    let largest = v.max_abs();
    if largest == 0 {
        return None;
    }
    let scaled = rescale(v, largest);
    let length = i128::try_from(FixedQ32_32::sqrt_u128(scaled.length_squared())).ok()?;
    let component = |c: i128| {
        let raw = i64::try_from(round_div(c << FRAC_BITS, length)).ok()?;
        Some(FixedQ32_32::from_raw(raw))
    };
    Some(FixedVec3::new(component(scaled.x)?, component(scaled.y)?, component(scaled.z)?))
}

fn checked_dot(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedQ32_32> {
    let x_term = lhs.x.checked_mul(rhs.x)?;
    let y_term = lhs.y.checked_mul(rhs.y)?;
    let z_term = lhs.z.checked_mul(rhs.z)?;
    x_term.checked_add(y_term)?.checked_add(z_term)
}

fn checked_negate(v: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(v.x.checked_neg()?, v.y.checked_neg()?, v.z.checked_neg()?))
}

/// One matrix row: the axis followed by `-(axis . eye)`.
fn view_row(axis: FixedVec3, eye: FixedVec3) -> Result<FixedMat4Row, LookAtError> {
    let translation = checked_dot(axis, eye).and_then(FixedQ32_32::checked_neg);
    let translation = translation.ok_or(LookAtError::Overflow)?;
    Ok((axis.x, axis.y, axis.z, translation))
}

pub(crate) fn reference_try_look_at(
    eye: FixedVec3,
    target: FixedVec3,
    up: FixedVec3,
) -> Result<FixedMat4, LookAtError> {
    let forward = unit(Raw3::difference(target, eye)).ok_or(LookAtError::CoincidentEyeTarget)?;
    let (raw_forward, raw_up) = (Raw3::from_fixed(forward), Raw3::from_fixed(up));
    let right = unit(raw_forward.cross(raw_up)).ok_or(LookAtError::ParallelUp)?;
    let true_up =
        unit(Raw3::from_fixed(right).cross(raw_forward)).ok_or(LookAtError::ParallelUp)?;
    let back = checked_negate(forward).ok_or(LookAtError::Overflow)?;
    Ok(FixedMat4::from_rows(
        view_row(right, eye)?,
        view_row(true_up, eye)?,
        view_row(back, eye)?,
        (FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    ))
}
