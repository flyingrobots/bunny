//! Exact power eases and steps in Q32.32 (`bunny-ease/1`).
//!
//! Planned in `docs/topics/ease/test-plan.md`; the kernel is not written yet.

use crate::fixed_q32_32::FixedQ32_32;

/// Stable contract identifier for the eases in this module.
pub const EASE_CONTRACT_ID: &str = "bunny-ease/1";

/// `t^n`, planned.
#[must_use]
pub const fn power_in(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    let _ = (n, t);
    None
}

/// `1 - (1 - t)^n`, planned.
#[must_use]
pub const fn power_out(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    let _ = (n, t);
    None
}

/// `2^(n-1) t^n` below one half and `1 - 2^(n-1) (1 - t)^n` from it, planned.
#[must_use]
pub const fn power_in_out(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    let _ = (n, t);
    None
}

/// `k / n` on `[k/n, (k+1)/n)`, planned.
#[must_use]
pub const fn steps(n: u32, t: FixedQ32_32) -> Option<FixedQ32_32> {
    let _ = (n, t);
    None
}
