//! A surface hit shaded with an interpolated normal: `bunny-smooth-shading/1`.

use bunny_linalg::FixedUnitVec3;
use bunny_query::BarycentricNumerators;

use super::frame::ShadingFrame;
use super::hit::SurfaceHit;
use super::light::{DirectionalLight, LightSample};
use super::smooth_normal::smooth_normal;
use super::wide::{dot, wide};

/// The versioned name of the smooth-shaded steps: [`SmoothHit::new`], [`SmoothHit::frame`] and
/// [`SmoothHit::light_sample`]. `bunny-diffuse-shading/1` does not change.
pub const SMOOTH_SHADING_ID: &str = "bunny-smooth-shading/1";

/// A [`SurfaceHit`] with a shading frame around the interpolated vertex normal.
///
/// The surface hit keeps its geometric frame, its start point and its exact plane; only the
/// cosine terms use the shading frame. See `docs/topics/optics/README.md#smooth-shading`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmoothHit {
    hit: SurfaceHit,
    frame: ShadingFrame,
}

impl SmoothHit {
    /// The hit `hit` shaded with its triangle's vertex `normals` (in the order `v0`, `v1`, `v2`)
    /// at the hit's exact barycentric `weights` (`TriangleHit::barycentric_numerators`).
    ///
    /// Let `n = smooth_normal(normals, weights)` and `s = n . N`, exact, where `N` is the exact
    /// normal of the face the ray arrived on (the plane of the start-point rule, chosen by
    /// `TriangleHit::front_face`). The frame is:
    ///
    /// - `ShadingFrame::new(n)` when `s > 0`;
    /// - `ShadingFrame::new(n).flipped()` when `s < 0` (the normals point to the other face);
    /// - `hit.frame()`, the geometric frame, when `s = 0`, when a vertex normal is missing
    ///   (`None`), or when `smooth_normal` returns `None` (the normals cancel).
    ///
    /// So the shading normal is never on the far side of, nor along, the geometric surface.
    #[must_use]
    pub fn new(
        hit: SurfaceHit,
        weights: BarycentricNumerators,
        normals: [Option<FixedUnitVec3>; 3],
    ) -> Self {
        let [a, b, c] = normals;
        let interpolated =
            a.zip(b).zip(c).and_then(|((a, b), c)| smooth_normal([a, b, c], weights));
        let frame = interpolated.map_or(hit.frame(), |normal| oriented(&hit, normal));
        Self { hit, frame }
    }

    /// The geometric surface hit: its frame, start point and plane, unchanged.
    #[must_use]
    pub const fn surface_hit(&self) -> SurfaceHit {
        self.hit
    }

    /// The shading frame.
    #[must_use]
    pub const fn frame(&self) -> ShadingFrame {
        self.frame
    }

    /// The direct term of `light`: the shadow ray and the side test of
    /// [`SurfaceHit::light_sample`] (on the exact geometric plane), with the cosine taken
    /// against the shading normal and clamped at zero. A light on the arrival side of the plane
    /// but behind the shading normal has its shadow ray and zero radiance.
    #[must_use]
    pub fn light_sample(&self, light: &DirectionalLight) -> LightSample {
        let geometric = self.hit.light_sample(light);
        let Some(ray) = geometric.shadow_ray().copied() else { return geometric };
        let radiance = light.reflected_radiance(light.cosine(self.frame.normal()));
        LightSample::new(Some(ray), radiance)
    }
}

/// The frame around `normal`, turned to the arrival side of the exact plane, or the geometric
/// frame when `normal` lies along the plane. Each product is below `2^33 * 2^83`, so the exact
/// dot product is below `2^118`.
fn oriented(hit: &SurfaceHit, normal: FixedUnitVec3) -> ShadingFrame {
    let side = dot(wide(normal.into_inner()), hit.facing_normal());
    match side.signum() {
        1 => ShadingFrame::new(normal),
        -1 => ShadingFrame::new(normal).flipped(),
        _ => hit.frame(),
    }
}
