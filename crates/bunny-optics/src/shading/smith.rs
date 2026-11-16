//! The Smith masking-shadowing ratio of the GGX distribution, in exact-rule fixed point.

use bunny_num::FixedQ32_32;

use super::wide::{div_round, ONE};

#[cfg(test)]
mod tests;

/// The smallest accepted GGX roughness, raw: `2^16` (`2^-16`), whose square is one raw unit.
pub const MIN_ALPHA: i128 = 1 << 16;

/// `sqrt(alpha^2 + (1 - alpha^2) c^2)` in units of `2^-60`, floored, for raw `alpha` in
/// `2^16..=2^32` and raw `c` in `0..=2^32` (each clamped into its range first).
///
/// The radicand is taken as `1 - (1 - alpha^2)(1 - c^2)`, whose subtrahend
/// `p = (2^64 - alpha^2)(2^64 - c^2)` is exact in units of `2^-128` and below `2^128`; then
/// `floor((2^128 - p) / 2^8) = 2^120 - ceil(p / 2^8)` is exact, and its square root is floored
/// once. The result is in `2^44..=2^60` and at least `c * 2^28`.
fn smith_root(alpha: i128, cosine: i128) -> i128 {
    const UNIT_SQUARED: u128 = 1 << 64;
    let alpha = alpha.clamp(MIN_ALPHA, ONE).unsigned_abs();
    let cosine = cosine.clamp(0, ONE).unsigned_abs();
    let product = (UNIT_SQUARED - alpha * alpha) * (UNIT_SQUARED - cosine * cosine);
    let root = FixedQ32_32::sqrt_u128((1 << 120) - ((product + 255) >> 8));
    i128::try_from(root).unwrap_or(0)
}

/// The ratio `G2(v, l) / G1(v)` of the height-correlated Smith term, raw Q32.32 in `0..=1.0`,
/// for raw roughness `alpha` and the raw cosines `view` and `light` of the two directions
/// against the normal, each in `1..=2^32`.
///
/// With `s(c) = sqrt(alpha^2 + (1 - alpha^2) c^2)`, the ratio is
/// `light (view + s(view)) / (light s(view) + view s(light))`. Both roots are floored in units
/// of `2^-60` ([`smith_root`]); the numerator and the denominator are then exact in units of
/// `2^-92` (each below `2^94`), and the quotient is rounded once to nearest, ties to even. It
/// is within `1/2 + 2^-11` raw units of the exact ratio at the given raw values, and never
/// above 1.0, because the floored `s(light)` is at least `light`.
///
/// Returns zero when a cosine is zero or below.
pub fn masking_ratio(alpha: i128, view: i128, light: i128) -> i128 {
    let (view, light) = (view.clamp(0, ONE), light.clamp(0, ONE));
    let (view_root, light_root) = (smith_root(alpha, view), smith_root(alpha, light));
    let numerator = light * ((view << 28) + view_root);
    let denominator = light * view_root + view * light_root;
    if denominator == 0 {
        return 0;
    }
    div_round(numerator << 32, denominator)
}
