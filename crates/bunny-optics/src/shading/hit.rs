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
    /// Returns `None` when the point is not representable, is more than two raw units outside
    /// the trace domain, cannot reach the arrival side in two steps, or ends outside the trace
    /// domain: exactly when [`Self::try_new`] returns an error.
    ///
    /// **Fast path and diagnostic path.** This is the fast path, the 0.7.1 code plus one check
    /// that keeps the exact plane test inside `i128`. It says only whether the hit was refused.
    /// [`Self::try_new`] is the diagnostic path that says why: a renderer calls this function
    /// per hit and calls `try_new` only when it returns `None`.
    ///
    /// ```
    /// # use bunny_linalg::FixedVec3;
    /// # use bunny_num::FixedQ32_32;
    /// # use bunny_optics::{SecondaryRayError, SurfaceHit, TriangleSurface};
    /// # use bunny_query::{TraceRay, TraceTriangle};
    /// # let unit = |x: i64| FixedQ32_32::from_raw(x << 32);
    /// # let v = |x, y, z| FixedVec3::new(unit(x), unit(y), unit(z));
    /// # let triangle = TraceTriangle::new(v(-8, 0, 8), v(8, 0, 8), v(0, 0, -8)).ok_or("tri")?;
    /// # let surface = TriangleSurface::new(&triangle).ok_or("surface")?;
    /// # let ray = TraceRay::new(v(0, 2, 0), v(0, -1, 0)).ok_or("ray")?;
    /// # let far = FixedQ32_32::from_raw(i64::MAX);
    /// # let mut hit = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, far).ok_or("hit")?;
    /// hit.front_face = !hit.front_face; // a hit that does not belong to this ray and face
    /// let Some(start) = SurfaceHit::new(&surface, &ray, &hit) else {
    ///     // Only now pay for the reason.
    ///     let why = SurfaceHit::try_new(&surface, &ray, &hit).err();
    ///     assert!(matches!(why, Some(SecondaryRayError::NotOnArrivalSide { .. })));
    ///     return Ok(());
    /// };
    /// # let _ = start;
    /// # Ok::<(), &str>(())
    /// ```
    #[must_use]
    pub fn new(surface: &TriangleSurface, ray: &TraceRay, hit: &TriangleHit) -> Option<Self> {
        let (plane, offset) = surface.facing_plane(hit.front_face);
        let start = wide(point_at(ray, hit.distance)?);
        // Under this bound every coordinate is below 2^41, so with plane normal components
        // below 2^83 the plane test stays below 2^126; no point outside it can step inside.
        let domain = u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs());
        if !within(start, domain + MAX_STEPS.unsigned_abs()) {
            return None;
        }
        let point = arrival_side(start, plane, offset)?;
        let inside = within(point, domain);
        let frame = surface.facing_frame(hit.front_face);
        inside.then(|| Self { point: narrow(point), frame, plane })
    }

    /// The diagnostic path of [`Self::new`]: the same hit, with the reason for a refusal. Call
    /// it when `new` returns `None`; `new` is the fast path. `OPTICS-TP-062` checks over
    /// generated inputs that `new` returns `Some(h)` exactly when this returns `Ok(h)`.
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
    pub fn try_new(
        surface: &TriangleSurface,
        ray: &TraceRay,
        hit: &TriangleHit,
    ) -> Result<Self, SecondaryRayError> {
        let (plane, offset) = surface.facing_plane(hit.front_face);
        let distance = hit.distance;
        let start = point_at(ray, distance)
            .ok_or(SecondaryRayError::HitPointUnrepresentable { distance })?;
        let point = diagnosed_start(start, plane, offset)?;
        Ok(Self { point: narrow(point), frame: surface.facing_frame(hit.front_face), plane })
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

    /// The exact normal of the face the ray arrived on: `(v1 - v0) x (v2 - v0)`, negated for the
    /// back face, each component below `2^83`. Every side decision is made against it.
    pub(crate) const fn facing_normal(&self) -> Wide {
        self.plane
    }

    /// The bounce ray for the unit-square point `sample`: from [`Self::point`] along
    /// [`ShadingFrame::cosine_direction`]. Query it with a minimum distance of zero.
    ///
    /// Returns `None` when a coordinate of `sample` is outside `[0, 1)`, exactly when
    /// [`Self::try_bounce_ray`] returns an error. This is the fast path (the 0.7.1 code);
    /// `try_bounce_ray` is the diagnostic path, for a caller that needs the reason after a
    /// `None`.
    #[must_use]
    pub fn bounce_ray(&self, sample: [FixedQ32_32; 2]) -> Option<TraceRay> {
        TraceRay::new(self.point, self.frame.cosine_direction(sample)?)
    }

    /// The diagnostic path of [`Self::bounce_ray`]: the same ray, with the reason for a refusal.
    /// Call it when `bounce_ray` returns `None`.
    ///
    /// # Errors
    ///
    /// [`SecondaryRayError::SampleOutsideUnitInterval`] when a coordinate of `sample` is
    /// outside `[0, 1)`; the defensive [`SecondaryRayError::BounceRayRefused`] if
    /// `TraceRay::new` refuses the ray, which no input is known to reach.
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

/// The stepped start point of [`SurfaceHit::try_new`] from the hit point `start`, or the reason
/// it is refused, checked in the order its documentation gives. A hit point beyond reach of the
/// domain is refused before the plane test, which keeps that test inside `i128`.
fn diagnosed_start(start: FixedVec3, plane: Wide, offset: i128) -> Result<Wide, SecondaryRayError> {
    let domain = u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs());
    if !within(wide(start), domain + MAX_STEPS.unsigned_abs()) {
        return Err(SecondaryRayError::HitPointOutsideTraceDomain { point: start });
    }
    let point = arrival_side(wide(start), plane, offset)
        .ok_or(SecondaryRayError::NotOnArrivalSide { point: start })?;
    if within(point, domain) {
        Ok(point)
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
