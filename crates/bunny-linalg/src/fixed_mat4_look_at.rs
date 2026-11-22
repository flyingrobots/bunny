//! Look-at view matrix builder for [`FixedMat4`].
//!
//! `try_look_at` builds a `view_from_world` matrix for Bunny's right-handed
//! frame, where the camera looks down `-Z` (see
//! `docs/topics/coordinate-law/README.md`). The rows are the camera's right,
//! true up, and back (negated forward) axes, followed by the translation
//! `-(axis . eye)`.
//!
//! Each axis is normalized from exact `i128` raw arithmetic, so a basis vector
//! has at most 1 raw unit of rounding error per component. The three axes are
//! therefore orthonormal only within rounding: each length is within 2 raw
//! units of one and each pairwise dot product is within 2 raw units of zero.
//! See `docs/topics/matrix-types/README.md#look-at-view-matrix`.

use std::fmt;

use bunny_num::FixedQ32_32;

use crate::{FixedMat4, FixedMat4Row, FixedVec3};

use bunny_num::fixed_q32_32::FRAC_BITS;

/// Error returned by [`FixedMat4::try_look_at`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookAtError {
    /// The eye and the target are the same point.
    CoincidentEyeTarget,
    /// The up vector is zero or parallel to the view direction.
    ParallelUp,
    /// A checked Q32.32 operation overflowed.
    Overflow,
}

impl fmt::Display for LookAtError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CoincidentEyeTarget => "look-at eye and target are the same point",
            Self::ParallelUp => "look-at up vector is zero or parallel to the view direction",
            Self::Overflow => "look-at arithmetic is outside the Q32.32 range",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LookAtError {}

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

/// The exact 256-bit product of two magnitudes below `2^66`, as `(high, low)`.
///
/// Proof of no overflow: split each factor into 64-bit limbs `a = a1 * 2^64 +
/// a0`. With `a, b < 2^66` the high limbs are below 4, so every partial product
/// is below `2^128` and the cross sum `a0 * b1 + a1 * b0` is below `2^67`.
fn wide_product(a: u128, b: u128) -> (u128, u128) {
    const LIMB: u32 = 64;
    let mask = (1_u128 << LIMB) - 1;
    let (a0, a1) = (a & mask, a >> LIMB);
    let (b0, b1) = (b & mask, b >> LIMB);
    let middle = a0 * b1 + a1 * b0;
    let (low, carry) = (a0 * b0).overflowing_add(middle << LIMB);
    (a1 * b1 + (middle >> LIMB) + u128::from(carry), low)
}

/// Whether `a * b == c * d` exactly. Each factor is a raw difference or a raw
/// coordinate, so its magnitude is at most `2^64`, within [`wide_product`]'s bound.
fn products_equal(a: i128, b: i128, c: i128, d: i128) -> bool {
    let magnitude = |x: i128, y: i128| wide_product(x.unsigned_abs(), y.unsigned_abs());
    a.signum() * b.signum() == c.signum() * d.signum() && magnitude(a, b) == magnitude(c, d)
}

/// Whether the cross product of the two raw vectors is exactly zero, which holds
/// exactly when they are parallel, antiparallel, or either is zero.
fn exactly_parallel(a: Raw3, b: Raw3) -> bool {
    products_equal(a.y, b.z, a.z, b.y)
        && products_equal(a.z, b.x, a.x, b.z)
        && products_equal(a.x, b.y, a.y, b.x)
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

fn checked_negate(v: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(v.x.checked_neg()?, v.y.checked_neg()?, v.z.checked_neg()?))
}

/// `magnitude / 2^32` rounded to nearest, ties to even, as `checked_mul` rounds.
const fn round_shift(magnitude: u128) -> u128 {
    let (quotient, remainder) = (magnitude >> FRAC_BITS, magnitude & ((1 << FRAC_BITS) - 1));
    let half = 1_u128 << (FRAC_BITS - 1);
    if remainder > half || (remainder == half && quotient & 1 == 1) {
        quotient + 1
    } else {
        quotient
    }
}

/// A Q32.32 product as an unchecked wide integer, rounded exactly as
/// `FixedQ32_32::checked_mul` rounds it, but without the range check.
fn wide_term(lhs: FixedQ32_32, rhs: FixedQ32_32) -> i128 {
    let product = i128::from(lhs.raw()) * i128::from(rhs.raw());
    let rounded = i128::try_from(round_shift(product.unsigned_abs())).unwrap_or(i128::MAX);
    if product < 0 {
        -rounded
    } else {
        rounded
    }
}

/// The translation entry `-(axis . eye)`. Each term is at most `2^96` and the
/// sum of three at most `2^98`, so the `i128` accumulator cannot overflow; only
/// the finished entry is checked against the Q32.32 range.
fn translation_entry(axis: FixedVec3, eye: FixedVec3) -> Option<FixedQ32_32> {
    let sum = wide_term(axis.x, eye.x) + wide_term(axis.y, eye.y) + wide_term(axis.z, eye.z);
    Some(FixedQ32_32::from_raw(i64::try_from(-sum).ok()?))
}

/// One matrix row: the axis followed by `-(axis . eye)`.
fn view_row(axis: FixedVec3, eye: FixedVec3) -> Result<FixedMat4Row, LookAtError> {
    let translation = translation_entry(axis, eye).ok_or(LookAtError::Overflow)?;
    Ok((axis.x, axis.y, axis.z, translation))
}

impl FixedMat4 {
    /// Builds a `view_from_world` matrix looking from `eye` toward `target`.
    ///
    /// The frame is right-handed with the camera looking down `-Z`, `+X` to
    /// its right, and `+Y` up, so `p_view = view_from_world * p_world`. The
    /// rows are `right = normalize(forward x up)`, `true_up = right x forward`
    /// and `-forward`, with translation `-(row . eye)`. The last row is
    /// `(0, 0, 0, 1)`. Axis-aligned cameras give exact results.
    ///
    /// The three axes come from normalizing fixed-point vectors, so they are
    /// orthonormal only within rounding: each is within 1 raw unit per
    /// component of its ideal direction, each length is within 2 raw units of
    /// one, and each pairwise dot product is within 2 raw units of zero. This
    /// holds however close `up` is to the view direction, but a nearly
    /// parallel `up` is accepted and fixes the camera roll only to rounding
    /// noise; only exact parallelism is refused.
    ///
    /// # Errors
    /// Returns [`LookAtError::CoincidentEyeTarget`] when `target == eye`,
    /// [`LookAtError::ParallelUp`] when `up` is zero or exactly parallel (or
    /// antiparallel) to `target - eye`, decided on the exact integer inputs
    /// before any rounding, and [`LookAtError::Overflow`] when a finished
    /// translation entry is outside the Q32.32 range.
    pub fn try_look_at(
        eye: FixedVec3,
        target: FixedVec3,
        up: FixedVec3,
    ) -> Result<Self, LookAtError> {
        let direction = Raw3::difference(target, eye);
        let forward = unit(direction).ok_or(LookAtError::CoincidentEyeTarget)?;
        if exactly_parallel(direction, Raw3::from_fixed(up)) {
            return Err(LookAtError::ParallelUp);
        }
        let (raw_forward, raw_up) = (Raw3::from_fixed(forward), Raw3::from_fixed(up));
        let right = unit(raw_forward.cross(raw_up)).ok_or(LookAtError::ParallelUp)?;
        let true_up =
            unit(Raw3::from_fixed(right).cross(raw_forward)).ok_or(LookAtError::ParallelUp)?;
        let back = checked_negate(forward).ok_or(LookAtError::Overflow)?;
        Ok(Self::from_rows(
            view_row(right, eye)?,
            view_row(true_up, eye)?,
            view_row(back, eye)?,
            (FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
        ))
    }
}
