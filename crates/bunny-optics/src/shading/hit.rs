//! A surface hit: the facing frame and the start point of every secondary ray.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TriangleHit, TRACE_COORD_LIMIT_RAW};

use super::frame::ShadingFrame;
use super::light::{DirectionalLight, LightSample};
use super::surface::TriangleSurface;
use super::wide::{dot, narrow, pick, wide, Wide};
use crate::point_at;

/// The most raw-unit steps the start point moves to reach the arrival side. Two always suffice
/// (see [`SurfaceHit::new`]).
const MAX_STEPS: i128 = 2;

/// Where a ray hit a triangle, prepared for the secondary rays that leave it.
///
/// It holds the frame facing the incoming ray (the facing rule) and the start point shared by
/// the shadow ray and the bounce ray (the self-intersection rule). See
/// `docs/topics/optics/README.md#facing-and-secondary-rays`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceHit {
    point: FixedVec3,
    frame: ShadingFrame,
    plane: Wide,
}

impl SurfaceHit {
    /// The hit of `ray` on the triangle prepared as `surface`, where `hit` is the
    /// `TriangleHit` that `ray.hit_triangle` returned for it.
    ///
    /// **Facing.** The frame is `surface.facing_frame(hit.front_face)`: its normal points to
    /// the side the ray arrived from.
    ///
    /// **Start point.** `point_at(ray, hit.distance)`, then moved by the fewest whole raw units
    /// (zero, one or two) along the axis of the plane normal's largest component (the lowest
    /// axis on a tie), toward the arrival side, that put it strictly on the arrival side of the
    /// triangle's exact plane. The floored distance leaves the exact point on the arrival side
    /// or on the plane, and rounding moves each coordinate by at most half a raw unit, so two
    /// steps always suffice.
    ///
    /// Returns `None` when the point is not representable or ends outside the trace domain.
    #[must_use]
    pub fn new(surface: &TriangleSurface, ray: &TraceRay, hit: &TriangleHit) -> Option<Self> {
        let (plane, offset) = surface.facing_plane(hit.front_face);
        let start = wide(point_at(ray, hit.distance)?);
        let point = arrival_side(start, plane, offset)?;
        let limit = u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs());
        let inside = point.iter().all(|c| c.unsigned_abs() < limit);
        let frame = surface.facing_frame(hit.front_face);
        inside.then(|| Self { point: narrow(point), frame, plane })
    }

    /// The start point of every secondary ray.
    #[must_use]
    pub const fn point(&self) -> FixedVec3 {
        self.point
    }

    /// The frame whose normal faces the incoming ray.
    #[must_use]
    pub const fn frame(&self) -> ShadingFrame {
        self.frame
    }

    /// The bounce ray for the unit-square point `sample`: from [`Self::point`] along
    /// [`ShadingFrame::cosine_direction`]. Query it with a minimum distance of zero.
    ///
    /// Returns `None` when a coordinate of `sample` is outside `[0, 1)`.
    #[must_use]
    pub fn bounce_ray(&self, sample: [FixedQ32_32; 2]) -> Option<TraceRay> {
        TraceRay::new(self.point, self.frame.cosine_direction(sample)?)
    }

    /// The direct term of `light` at this hit and the shadow ray that decides it.
    ///
    /// When the light is not strictly on the arrival side of the triangle's exact plane
    /// (`toward_light . normal <= 0`, exact), there is no shadow ray and no light. Otherwise the
    /// shadow ray runs from [`Self::point`] along `toward_light`, to be queried with any-hit,
    /// minimum distance zero and [`super::SHADOW_T_MAX`], and the radiance is
    /// `light.reflected_radiance(light.cosine(normal))` with this hit's frame normal.
    #[must_use]
    pub fn light_sample(&self, light: &DirectionalLight) -> LightSample {
        if dot(self.plane, wide(light.toward_light())) <= 0 {
            return LightSample::DARK;
        }
        let radiance = light.reflected_radiance(light.cosine(self.frame.normal()));
        LightSample::new(TraceRay::new(self.point, light.toward_light()), radiance)
    }
}

/// `start` moved by `0..=MAX_STEPS` raw units along the plane normal's dominant axis, toward the
/// arrival side, until `plane . point > offset`.
///
/// Before moving, `plane . start - offset >= -|plane|_1 / 2`; each step adds the dominant
/// magnitude, at least `|plane|_1 / 3`, so two steps make it positive.
fn arrival_side(start: Wide, plane: Wide, offset: i128) -> Option<Wide> {
    let step = dominant_step(plane);
    (0..=MAX_STEPS)
        .map(|k| [0, 1, 2].map(|i| pick(start, i) + k * pick(step, i)))
        .find(|point| dot(plane, *point) > offset)
}

/// One raw unit along the axis of the largest `|plane_i|` (the lowest axis on a tie), with the
/// sign of that component.
fn dominant_step(plane: Wide) -> Wide {
    let [x, y, z] = plane.map(i128::unsigned_abs);
    let axis = if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    };
    [0, 1, 2].map(|i| if i == axis { pick(plane, i).signum() } else { 0 })
}
