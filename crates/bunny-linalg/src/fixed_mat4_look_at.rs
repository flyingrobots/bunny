//! Look-at view matrix builder for [`FixedMat4`].

use std::fmt;

use bunny_num::FixedQ32_32;

use crate::{FixedMat4, FixedVec3};

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

impl FixedMat4 {
    /// Builds a `view_from_world` matrix (stub).
    ///
    /// # Errors
    /// Returns [`LookAtError`].
    pub const fn try_look_at(
        _eye: FixedVec3,
        _target: FixedVec3,
        _up: FixedVec3,
    ) -> Result<Self, LookAtError> {
        let _ = FixedQ32_32::ZERO;
        Err(LookAtError::Overflow)
    }
}
