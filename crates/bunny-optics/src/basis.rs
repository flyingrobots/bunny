//! The camera basis: forward, right and up unit vectors.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

use crate::CameraError;

const ZERO: FixedVec3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
/// Vectors are scaled by a power of two until their largest component magnitude is in
/// `[2^61, 2^62)` before they are normalized.
const PRESCALE_BIT: u32 = 61;
/// The up hint is refused as parallel when every component of `unit(look) x unit(up)` is
/// below this many raw units: the sine of the angle between them is below about `2^-16`.
const MIN_SIDE_RAW: u64 = 1 << 16;

/// Right-handed unit vectors: `right` is `forward x up`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Basis {
    pub forward: FixedUnitVec3,
    pub right: FixedUnitVec3,
    pub up: FixedUnitVec3,
}

impl Basis {
    /// Derives the basis from a look direction and an up hint.
    ///
    /// `forward` is the normalized look direction. A first right vector is the normalized
    /// `forward x unit(hint)`; `up` is the normalized `right x forward` and the final `right`
    /// is the normalized `forward x up`. Every vector is scaled by a power of two before it is
    /// normalized, so vectors of any length normalize to full precision.
    pub fn derive(look: FixedVec3, up_hint: FixedVec3) -> Result<Self, CameraError> {
        if look == ZERO {
            return Err(CameraError::ZeroLook);
        }
        if up_hint == ZERO {
            return Err(CameraError::UpParallelToLook);
        }
        let forward = unit(look)?;
        let side = forward.into_inner().cross(unit(up_hint)?.into_inner());
        if largest_magnitude(side) < MIN_SIDE_RAW {
            return Err(CameraError::UpParallelToLook);
        }
        let first_right = unit(side)?;
        let up = unit(first_right.into_inner().cross(forward.into_inner()))?;
        let right = unit(forward.into_inner().cross(up.into_inner()))?;
        Ok(Self { forward, right, up })
    }
}

/// `FixedUnitVec3::new` of the prescaled vector. It does not fail for a nonzero vector in
/// any test; a failure is reported as the defensive `Unrepresentable`.
fn unit(v: FixedVec3) -> Result<FixedUnitVec3, CameraError> {
    FixedUnitVec3::new(prescaled(v)).ok_or(CameraError::Unrepresentable)
}

/// `v` times the power of two that puts its largest component magnitude in `[2^61, 2^62)`:
/// exact when it scales up, floored when it scales down. Only the direction matters.
fn prescaled(v: FixedVec3) -> FixedVec3 {
    let top = largest_magnitude(v).max(1).ilog2();
    let scale = |c: FixedQ32_32| {
        let raw = c.raw();
        let scaled = if top <= PRESCALE_BIT {
            raw << (PRESCALE_BIT - top)
        } else {
            raw >> (top - PRESCALE_BIT)
        };
        FixedQ32_32::from_raw(scaled)
    };
    FixedVec3::new(scale(v.x), scale(v.y), scale(v.z))
}

fn largest_magnitude(v: FixedVec3) -> u64 {
    v.x.raw().unsigned_abs().max(v.y.raw().unsigned_abs()).max(v.z.raw().unsigned_abs())
}
