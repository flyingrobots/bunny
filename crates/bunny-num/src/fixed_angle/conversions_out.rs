//! Conversions out of a binary angle: turns, degrees and radians (planned, flyingrobots/bunny#244).

use super::FixedAngle;
use crate::FixedQ32_32;

impl FixedAngle {
    /// The angle in turns (planned).
    #[must_use]
    pub const fn to_turns(self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(0)
    }

    /// The angle in degrees (planned).
    #[must_use]
    pub const fn to_degrees(self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(0)
    }

    /// The angle in radians (planned).
    #[must_use]
    pub const fn to_radians(self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(0)
    }
}
