//! A surface hit shaded with an interpolated normal: `bunny-smooth-shading/1`.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{BarycentricNumerators, TraceRay};

use super::frame::ShadingFrame;
use super::hit::SurfaceHit;
use super::light::{DirectionalLight, LightSample};
use super::smooth_normal::smooth_normal;
use super::wide::{dot, wide};

/// The versioned name of the smooth-shaded steps: [`SmoothHit::new`], [`SmoothHit::frame`],
/// [`SmoothHit::light_sample`], [`SmoothHit::is_above_surface`] and [`SmoothHit::bounce`].
/// `bunny-diffuse-shading/1` does not change.
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

/// The outcome of [`SmoothHit::bounce`] for a sample in the unit square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmoothBounce {
    /// The bounce ray, from the start point along a direction strictly above the geometric
    /// surface.
    Ray(TraceRay),
    /// The direction around the shading normal is at or below the geometric surface: no ray is
    /// traced and the bounce contributes zero.
    BelowSurface,
}

impl SmoothHit {
    /// Whether `direction` points strictly to the arrival side of the exact geometric plane:
    /// `direction . N > 0`, exact, with `N` the facing plane normal of [`Self::new`]. Each
    /// product is below `2^32 * 2^83`, so the sum is below `2^117`.
    #[must_use]
    pub fn is_above_surface(&self, direction: FixedVec3) -> bool {
        dot(wide(direction), self.hit.facing_normal()) > 0
    }

    /// The bounce for the unit-square point `sample`: the cosine-weighted direction of the
    /// shading frame ([`ShadingFrame::cosine_direction`]); then [`SmoothBounce::Ray`] from the
    /// surface hit's start point when [`Self::is_above_surface`] holds, and
    /// [`SmoothBounce::BelowSurface`] when it does not. Query the ray with a minimum distance of
    /// zero, as for [`SurfaceHit::bounce_ray`].
    ///
    /// Returns `None` when a coordinate of `sample` is outside `[0, 1)`, and, defensively, if
    /// `TraceRay::new` refuses the ray (the start point is inside the trace domain and the
    /// direction a valid trace direction, so no input is known to reach that).
    #[must_use]
    pub fn bounce(&self, sample: [FixedQ32_32; 2]) -> Option<SmoothBounce> {
        let direction = self.frame.cosine_direction(sample)?;
        if !self.is_above_surface(direction) {
            return Some(SmoothBounce::BelowSurface);
        }
        TraceRay::new(self.hit.point(), direction).map(SmoothBounce::Ray)
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
