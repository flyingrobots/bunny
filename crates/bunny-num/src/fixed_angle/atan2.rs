//! The angle of a direction, `atan2` (planned, flyingrobots/bunny#244).

use super::FixedAngle;
use crate::FixedQ32_32;

impl FixedAngle {
    /// The angle of the direction `(x, y)` (planned).
    #[must_use]
    pub const fn atan2(y: FixedQ32_32, x: FixedQ32_32) -> Option<Self> {
        let _ = (y, x);
        None
    }
}
