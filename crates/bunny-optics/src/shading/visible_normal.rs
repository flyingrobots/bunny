//! Sampling the GGX distribution of visible normals (Heitz 2018) in exact-rule fixed point.

use bunny_num::{FixedAngle, FixedQ32_32};

use super::frame::ShadingFrame;
use super::wide::{cross, dot, pick, round_shift, unit, wide, Wide, ONE};

#[cfg(test)]
mod tests;

/// An incoming direction seen from a frame's normal: the unit view vector, its cosine and sine
/// against the normal, and the two horizontal axes of its plane of incidence.
pub struct View {
    toward: Wide,
    normal: Wide,
    forward: Wide,
    across: Wide,
    cosine: i128,
    sine: i128,
}

impl View {
    /// The view of the ray direction `d` (pointing at the surface) from `frame`; `None` when
    /// the cosine is zero, so no microfacet is visible.
    ///
    /// The view vector `v` is the unit vector along `-d`, normalized once. The axes are those
    /// of [`incidence_axes`]. The cosine `v . n` and the sine `v . forward` are each one
    /// [`rounded_cosine`].
    pub fn new(d: Wide, frame: &ShadingFrame) -> Option<Self> {
        let normal = wide(frame.normal());
        let toward = unit(d.map(|c| -c)).map_or([0; 3], |u| wide(u.into_inner()));
        let (forward, across) = incidence_axes(d, frame);
        let cosine = rounded_cosine(toward, normal);
        let sine = rounded_cosine(toward, forward);
        (cosine > 0).then_some(Self { toward, normal, forward, across, cosine, sine })
    }

    /// The rounded cosine of the view against the frame normal, in `1..=2^32`.
    pub const fn cosine(&self) -> i128 {
        self.cosine
    }

    /// The [`rounded_cosine`] of the view against the unit vector `axis`.
    pub fn cosine_against(&self, axis: Wide) -> i128 {
        rounded_cosine(self.toward, axis)
    }

    /// The unit microfacet normal for raw roughness `alpha` and the disk point `disk`
    /// ([`disk_point`]), by Heitz's construction in the axes `(forward, across, normal)`: the
    /// stretched view ([`stretched`]), the hemisphere point ([`warp`], [`tilt`]), and the
    /// unstretched normal ([`unstretch`]).
    pub fn visible_normal(&self, alpha: i128, disk: [i128; 2]) -> Wide {
        let [t1, shifted] = disk;
        let (sigma, kappa) = stretched(alpha * self.sine, self.cosine << 32);
        let (t2, t3) = warp(kappa, t1, shifted);
        let (along, up) = tilt((sigma, kappa), t2, t3);
        self.unstretch(alpha, [along, t1, up])
    }

    /// The unit vector along `alpha (along forward + t1 across) + up normal` for the
    /// hemisphere point `[along, t1, up]`: exact in `i128` (below `2^99` in units of `2^-96`)
    /// and normalized once. The frame normal if that vector is zero.
    fn unstretch(&self, alpha: i128, point: [i128; 3]) -> Wide {
        let [along, t1, up] = point;
        let axis = |i: usize| {
            let horizontal = along * pick(self.forward, i) + t1 * pick(self.across, i);
            alpha * horizontal + ((up * pick(self.normal, i)) << 32)
        };
        unit([axis(0), axis(1), axis(2)]).map_or(self.normal, |m| wide(m.into_inner()))
    }
}

/// `a . b` rounded once to nearest (ties to even) and clamped to `0..=1.0`: the cosine between
/// two unit vectors, raw Q32.32.
pub fn rounded_cosine(a: Wide, b: Wide) -> i128 {
    round_shift(dot(a, b), 32).clamp(0, ONE)
}

/// The point `(sqrt(u) cos(2 pi v), sqrt(u) sin(2 pi v))` of the unit disk for the unit-square
/// point `[u, v]`, raw Q32.32: the square root is floored, `FixedAngle::sin_cos` is within one
/// raw unit, and each product is rounded once to nearest, ties to even.
pub fn disk_point(sample: [FixedQ32_32; 2]) -> [i128; 2] {
    let [u, v] = sample;
    let radius = u.sqrt().map_or(0, |r| i128::from(r.raw()));
    let (sin, cos) = FixedAngle::from_turns(v).sin_cos();
    [cos, sin].map(|c| round_shift(radius * i128::from(c.raw()), 32))
}

/// The horizontal axes of the plane of incidence of the ray direction `d` on `frame`:
/// `forward`, the unit vector along `(d . n) n - d (n . n)` (the horizontal part of `-d`), and
/// `across`, the unit vector along `d x n`, so `(forward, across, n)` is right-handed. Both
/// vectors are exact in `i128` (components below `2^99` and `2^66`) and normalized once.
///
/// When `d` is parallel to the normal (or, defensively, when a normalization is refused) they
/// are `-bitangent` and `tangent`, which makes the construction Heitz's listing in the frame
/// `(tangent, bitangent, normal)`.
fn incidence_axes(d: Wide, frame: &ShadingFrame) -> (Wide, Wide) {
    let n = wide(frame.normal());
    let (nn, dn) = (dot(n, n), dot(d, n));
    let forward = [0, 1, 2].map(|i| dn * pick(n, i) - pick(d, i) * nn);
    match (unit(forward), unit(cross(d, n))) {
        (Some(forward), Some(across)) => (wide(forward.into_inner()), wide(across.into_inner())),
        _ => (wide(frame.bitangent()).map(|c| -c), wide(frame.tangent())),
    }
}

/// The stretched view: the unit vector along `(horizontal, 0, vertical)`, normalized once, as
/// `(sigma, kappa)` clamped to `0..=1.0`; `(0, 1.0)` if it cannot be normalized (never:
/// `vertical` is at least `2^32`).
fn stretched(horizontal: i128, vertical: i128) -> (i128, i128) {
    let Some(view) = unit([horizontal, 0, vertical]) else {
        return (0, ONE);
    };
    let [sigma, _, kappa] = wide(view.into_inner()).map(|c| c.clamp(0, ONE));
    (sigma, kappa)
}

/// Heitz's warp of the disk point `(t1, t2')` for the stretched view's height `kappa`, and the
/// height of the hemisphere above it: `q = floor(sqrt(1 - t1^2))`,
/// `t2 = ((1 - kappa) q + (1 + kappa) t2') / 2` rounded once to nearest (ties to even), and
/// `t3 = floor(sqrt(max(0, 1 - t1^2 - t2^2)))`. Returns `(t2, t3)`.
fn warp(kappa: i128, t1: i128, shifted: i128) -> (i128, i128) {
    let q = floor_root(ONE * ONE - t1 * t1);
    let t2 = round_shift((ONE - kappa) * q + (ONE + kappa) * shifted, 33);
    (t2, floor_root(ONE * ONE - t1 * t1 - t2 * t2))
}

/// The hemisphere point's coordinates on `forward` and on the normal for the stretched view
/// `(sigma, kappa)`: `along = sigma t3 - kappa t2` and `up = max(0, sigma t2 + kappa t3)`,
/// each rounded once to nearest, ties to even.
fn tilt(view: (i128, i128), t2: i128, t3: i128) -> (i128, i128) {
    let (sigma, kappa) = view;
    let along = round_shift(sigma * t3 - kappa * t2, 32);
    (along, round_shift(sigma * t2 + kappa * t3, 32).max(0))
}

/// `floor(sqrt(max(0, value)))` for `value` in units of `2^-64`, as raw Q32.32.
fn floor_root(value: i128) -> i128 {
    let root = FixedQ32_32::sqrt_u128(u128::try_from(value).unwrap_or(0));
    i128::try_from(root).unwrap_or(0)
}
