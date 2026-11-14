//! A surface hit: the facing frame and the start point of every secondary ray.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TriangleHit, TRACE_COORD_LIMIT_RAW};

use super::frame::ShadingFrame;
use super::light::{DirectionalLight, LightSample};
use super::refusal::SecondaryRayError;
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
    /// Returns `None` exactly when [`Self::try_new`] returns an error.
    #[must_use]
    pub fn new(surface: &TriangleSurface, ray: &TraceRay, hit: &TriangleHit) -> Option<Self> {
        let (plane, offset) = surface.facing_plane(hit.front_face);
        let point = start_point(ray, hit.distance, plane, offset).ok()?;
        Some(Self { point, frame: surface.facing_frame(hit.front_face), plane })
    }

    /// [`Self::new`], naming the refusal.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`SecondaryRayError::HitPointUnrepresentable`] when
    /// `point_at(ray, hit.distance)` is not representable;
    /// [`SecondaryRayError::HitPointOutsideTraceDomain`] when a coordinate of that point is
    /// more than two raw units outside the trace domain, so no step can bring it inside;
    /// [`SecondaryRayError::NotOnArrivalSide`] when no step of at most two raw units puts the
    /// point strictly on the arrival side (the hit is not one the trace kernel reported for
    /// `ray` on this triangle and face); [`SecondaryRayError::StartOutsideTraceDomain`] when
    /// the stepped start point is outside the trace domain. The trace kernel's hits on a ray
    /// and triangle inside the domain reach only the last of these, on the domain's boundary.
    #[inline]
    pub fn try_new(
        surface: &TriangleSurface,
        ray: &TraceRay,
        hit: &TriangleHit,
    ) -> Result<Self, SecondaryRayError> {
        let (plane, offset) = surface.facing_plane(hit.front_face);
        let point = start_point(ray, hit.distance, plane, offset)?;
        Ok(Self { point, frame: surface.facing_frame(hit.front_face), plane })
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
    /// Returns `None` exactly when [`Self::try_bounce_ray`] returns an error: when a
    /// coordinate of `sample` is outside `[0, 1)`.
    #[must_use]
    pub fn bounce_ray(&self, sample: [FixedQ32_32; 2]) -> Option<TraceRay> {
        // The same two steps as `try_bounce_ray`, through the `Option` form of the direction:
        // going through `try_bounce_ray` here cost about 3.5 ns of the 70 ns per-hit step,
        // passing the larger `Result` of the bounce ray (`shade/hit_shading`).
        TraceRay::new(self.point, self.frame.cosine_direction(sample)?)
    }

    /// [`Self::bounce_ray`], naming the refusal.
    ///
    /// # Errors
    ///
    /// [`SecondaryRayError::SampleOutsideUnitInterval`] when a coordinate of `sample` is
    /// outside `[0, 1)`; the defensive [`SecondaryRayError::BounceRayRefused`] if
    /// `TraceRay::new` refuses the ray, which no input is known to reach.
    #[inline]
    pub fn try_bounce_ray(&self, sample: [FixedQ32_32; 2]) -> Result<TraceRay, SecondaryRayError> {
        let direction = self.frame.try_cosine_direction(sample)?;
        TraceRay::new(self.point, direction)
            .ok_or(SecondaryRayError::BounceRayRefused { origin: self.point, direction })
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

/// The start point of every secondary ray for the hit at `distance` along `ray`, on the face
/// whose exact plane is `plane . p = offset`: the one implementation of the refusals that
/// [`SurfaceHit::new`] and [`SurfaceHit::try_new`] share.
///
/// The domain checks are arranged so the common case costs one comparison per coordinate: a hit
/// point more than two raw units inside the domain stays inside after any step. Every hit point
/// that reaches the plane test has coordinates below `2^41`, so with plane normal components
/// below `2^83` the test stays below `2^126`, inside `i128`.
#[inline]
fn start_point(
    ray: &TraceRay,
    distance: FixedQ32_32,
    plane: Wide,
    offset: i128,
) -> Result<FixedVec3, SecondaryRayError> {
    let Some(start) = point_at(ray, distance) else {
        return Err(SecondaryRayError::HitPointUnrepresentable { distance });
    };
    let not_on_arrival_side = || SecondaryRayError::NotOnArrivalSide { point: start };
    let domain = u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs());
    let steps = MAX_STEPS.unsigned_abs();
    if within(wide(start), domain - steps) {
        return arrival_side(wide(start), plane, offset)
            .map(narrow)
            .ok_or_else(not_on_arrival_side);
    }
    if !within(wide(start), domain + steps) {
        return Err(SecondaryRayError::HitPointOutsideTraceDomain { point: start });
    }
    let point = arrival_side(wide(start), plane, offset).ok_or_else(not_on_arrival_side)?;
    if within(point, domain) {
        Ok(narrow(point))
    } else {
        Err(SecondaryRayError::StartOutsideTraceDomain { start: narrow(point) })
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

/// Whether every coordinate magnitude of `point` is below `limit`.
fn within(point: Wide, limit: u128) -> bool {
    point.iter().all(|c| c.unsigned_abs() < limit)
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
