//! Sampling the GGX distribution of visible normals (Heitz 2018) in exact-rule fixed point.

use bunny_num::{FixedAngle, FixedQ32_32};

use super::frame::ShadingFrame;
use super::wide::{cross, dot, pick, round_shift, unit, wide, Wide, ONE};

/// An incoming direction seen from a frame's normal: its unit vector, its cosine and sine
/// against the normal, and the two horizontal axes of its plane of incidence.
pub struct View {
    incoming: Wide,
    normal: Wide,
    forward: Wide,
    across: Wide,
    cosine: i128,
    sine: i128,
}

impl View {
    /// The view of the ray direction `d` (pointing at the surface) from `frame`; `None` when
    /// the rounded cosine `-(unit(d) . n)` is zero or below, so no microfacet is visible.
    ///
    /// `forward` is the unit vector along `(d . n) n - d (n . n)`, the horizontal part of
    /// `-d`, and `across` the unit vector along `d x n`; both vectors are exact in `i128` and
    /// normalized once. When `d` is parallel to the normal they are `-bitangent` and
    /// `tangent`. The cosine and the sine, `-(unit(d) . forward)`, are each rounded once to
    /// nearest (ties to even) and clamped to `0..=1.0`.
    pub fn new(d: Wide, frame: &ShadingFrame) -> Option<Self> {
        let normal = wide(frame.normal());
        let incoming = unit(d).map_or([0; 3], |u| wide(u.into_inner()));
        let cosine = cosine_against(incoming, normal);
        let (forward, across) = incidence_axes(d, frame);
        let sine = cosine_against(incoming, forward);
        (cosine > 0).then_some(Self { incoming, normal, forward, across, cosine, sine })
    }

    /// The rounded cosine of the incoming direction against the frame normal, in `1..=2^32`.
    pub const fn cosine(&self) -> i128 {
        self.cosine
    }

    /// `-(unit(d) . axis)` rounded once to nearest (ties to even) and clamped to `0..=1.0`:
    /// the cosine of the incoming direction against the unit vector `axis`.
    pub fn cosine_against(&self, axis: Wide) -> i128 {
        cosine_against(self.incoming, axis)
    }

    /// The unit microfacet normal for raw roughness `alpha` and the disk point `disk`
    /// ([`disk_point`]), by Heitz's construction in the axes `(forward, across, normal)`.
    ///
    /// 1. The stretched view `(sigma, 0, kappa)` is the unit vector along
    ///    `(alpha sine, 0, cosine)`, normalized once and clamped to `0..=1.0`.
    /// 2. With the disk point `(t1, t2')`: `q = floor(sqrt(1 - t1^2))`,
    ///    `t2 = ((1 - kappa) q + (1 + kappa) t2') / 2` rounded once to nearest (ties to
    ///    even), and `t3 = floor(sqrt(max(0, 1 - t1^2 - t2^2)))`.
    /// 3. The hemisphere normal has `along = sigma t3 - kappa t2` on `forward`, `t1` on
    ///    `across` and `up = max(0, sigma t2 + kappa t3)` on the normal, `along` and `up` each
    ///    rounded once to nearest (ties to even).
    /// 4. The result is the unit vector along
    ///    `alpha (along forward + t1 across) + up normal`, exact in `i128` (below `2^99` in
    ///    units of `2^-96`) and normalized once; the frame normal if that vector is zero.
    pub fn visible_normal(&self, alpha: i128, disk: [i128; 2]) -> Wide {
        let [t1, shifted] = disk;
        let (sigma, kappa) = stretched(alpha * self.sine, self.cosine << 32);
        let q = floor_root(ONE * ONE - t1 * t1);
        let t2 = round_shift((ONE - kappa) * q + (ONE + kappa) * shifted, 33);
        let t3 = floor_root(ONE * ONE - t1 * t1 - t2 * t2);
        let along = round_shift(sigma * t3 - kappa * t2, 32);
        let up = round_shift(sigma * t2 + kappa * t3, 32).max(0);
        let axis = |i: usize| {
            let horizontal = along * pick(self.forward, i) + t1 * pick(self.across, i);
            alpha * horizontal + ((up * pick(self.normal, i)) << 32)
        };
        unit([axis(0), axis(1), axis(2)]).map_or(self.normal, |m| wide(m.into_inner()))
    }
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

/// `-(incoming . axis)` rounded once to nearest (ties to even), clamped to `0..=1.0`.
fn cosine_against(incoming: Wide, axis: Wide) -> i128 {
    round_shift(-dot(incoming, axis), 32).clamp(0, ONE)
}

/// The unit vectors along `(d . n) n - d (n . n)` and `d x n` (each component of the first
/// below `2^99`, of the second below `2^66`); `(-bitangent, tangent)` when `d` is parallel to
/// the normal `n` or, defensively, when a normalization is refused.
fn incidence_axes(d: Wide, frame: &ShadingFrame) -> (Wide, Wide) {
    let n = wide(frame.normal());
    let (nn, dn) = (dot(n, n), dot(d, n));
    let forward = [0, 1, 2].map(|i| dn * pick(n, i) - pick(d, i) * nn);
    match (unit(forward), unit(cross(d, n))) {
        (Some(forward), Some(across)) => (wide(forward.into_inner()), wide(across.into_inner())),
        _ => (wide(frame.bitangent()).map(|c| -c), wide(frame.tangent())),
    }
}

/// The unit vector along `(horizontal, 0, vertical)`, as `(sigma, kappa)` clamped to
/// `0..=1.0`; `(0, 1.0)` if it cannot be normalized (never: `vertical` is at least `2^32`).
fn stretched(horizontal: i128, vertical: i128) -> (i128, i128) {
    let Some(view) = unit([horizontal, 0, vertical]) else {
        return (0, ONE);
    };
    let [sigma, _, kappa] = wide(view.into_inner()).map(|c| c.clamp(0, ONE));
    (sigma, kappa)
}

/// `floor(sqrt(max(0, value)))` for `value` in units of `2^-64`, as raw Q32.32.
fn floor_root(value: i128) -> i128 {
    let root = FixedQ32_32::sqrt_u128(u128::try_from(value).unwrap_or(0));
    i128::try_from(root).unwrap_or(0)
}
