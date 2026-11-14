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

    fn cross(self, rhs: Self) -> Self {
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
fn round_div(numerator: i128, divisor: i128) -> i128 {
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

fn checked_dot(a: FixedVec3, b: FixedVec3) -> Option<FixedQ32_32> {
    let x = a.x.checked_mul(b.x)?;
    let y = a.y.checked_mul(b.y)?;
    let z = a.z.checked_mul(b.z)?;
    x.checked_add(y)?.checked_add(z)
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
    /// parallel `up` fixes the camera roll only to rounding noise.
    ///
    /// # Errors
    /// Returns [`LookAtError::CoincidentEyeTarget`] when `target == eye`,
    /// [`LookAtError::ParallelUp`] when `up` is zero or exactly parallel to
    /// the view direction after normalization, and [`LookAtError::Overflow`]
    /// when a translation entry is outside the Q32.32 range.
    pub fn try_look_at(
        eye: FixedVec3,
        target: FixedVec3,
        up: FixedVec3,
    ) -> Result<Self, LookAtError> {
        let forward =
            unit(Raw3::difference(target, eye)).ok_or(LookAtError::CoincidentEyeTarget)?;
        let (raw_forward, raw_up) = (Raw3::from_fixed(forward), Raw3::from_fixed(up));
        let right = unit(raw_forward.cross(raw_up)).ok_or(LookAtError::ParallelUp)?;
        let true_up = unit(Raw3::from_fixed(right).cross(raw_forward))
            .ok_or(LookAtError::ParallelUp)?;
        let back = checked_negate(forward).ok_or(LookAtError::Overflow)?;
        Ok(Self::from_rows(
            view_row(right, eye)?,
            view_row(true_up, eye)?,
            view_row(back, eye)?,
            (FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
        ))
    }
}
