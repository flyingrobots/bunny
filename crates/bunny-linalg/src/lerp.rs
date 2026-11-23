//! Component-wise linear interpolation of three channels (`bunny-lerp/1`).

use bunny_num::FixedQ32_32;

/// Interpolates three channels from `a` to `b` by `t`, each by [`FixedQ32_32::lerp`]
/// (`bunny-lerp/1`).
///
/// Each channel is rounded once, on its own, by the scalar rule; `t = 0` returns `a` and
/// `t = 1.0` returns `b` exactly, and `t` outside `[0, 1]` extrapolates. The channels are
/// interpolated as given: no colour space is assumed or converted, so an sRGB-encoded colour
/// is interpolated in its encoded values. Decode to linear first to interpolate light.
///
/// Returns `None` when any channel's result is outside the Q32.32 range.
#[must_use]
pub const fn lerp_channels(
    a: [FixedQ32_32; 3],
    b: [FixedQ32_32; 3],
    t: FixedQ32_32,
) -> Option<[FixedQ32_32; 3]> {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    match (FixedQ32_32::lerp(a0, b0, t), FixedQ32_32::lerp(a1, b1, t), FixedQ32_32::lerp(a2, b2, t))
    {
        (Some(c0), Some(c1), Some(c2)) => Some([c0, c1, c2]),
        _ => None,
    }
}
