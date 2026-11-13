//! Rays prepared for repeated box tests.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_num::{FixedDivisor, FixedQ32_32};

use super::{axis_contains_origin, AxisBounds, RayInterval};

/// A ray whose direction is prepared for many box tests.
///
/// Each nonzero direction component holds an exact [`FixedDivisor`], so
/// [`ray_intersects_aabb_prepared`] multiplies where [`ray_intersects_aabb`] divides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedRay3 {
    ray: FixedRay3,
    x: Option<FixedDivisor>,
    y: Option<FixedDivisor>,
    z: Option<FixedDivisor>,
}

impl PreparedRay3 {
    /// Prepares `ray`; costs about three ordinary divisions.
    #[must_use]
    pub const fn new(ray: FixedRay3) -> Self {
        let dir = ray.direction.into_inner();
        let (x, y, z) =
            (FixedDivisor::new(dir.x), FixedDivisor::new(dir.y), FixedDivisor::new(dir.z));
        Self { ray, x, y, z }
    }

    /// The ray this was prepared from.
    #[must_use]
    pub const fn ray(&self) -> &FixedRay3 {
        &self.ray
    }
}

/// [`ray_intersects_aabb`] for a prepared ray: identical results, bit for bit, with no
/// division per box.
#[must_use]
pub fn ray_intersects_aabb_prepared(
    prepared: &PreparedRay3,
    aabb: &FixedAabb3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    let (origin, dir) = (prepared.ray.origin, prepared.ray.direction.into_inner());
    let mut interval = RayInterval::unbounded();
    interval
        .update_prepared(AxisBounds::new(origin.x, dir.x, aabb.min.x, aabb.max.x), prepared.x)?;
    interval
        .update_prepared(AxisBounds::new(origin.y, dir.y, aabb.min.y, aabb.max.y), prepared.y)?;
    interval
        .update_prepared(AxisBounds::new(origin.z, dir.z, aabb.min.z, aabb.max.z), prepared.z)?;
    (!interval.is_miss()).then_some((interval.enter, interval.exit))
}

impl RayInterval {
    /// `update` with the direction's division prepared: the same results, bit for bit.
    fn update_prepared(&mut self, axis: AxisBounds, divisor: Option<FixedDivisor>) -> Option<()> {
        let Some(divisor) = divisor else {
            return axis_contains_origin(axis.origin, axis.min_bound, axis.max_bound);
        };
        let t0 = divisor.checked_div(axis.min_bound.checked_sub(axis.origin)?)?;
        let t1 = divisor.checked_div(axis.max_bound.checked_sub(axis.origin)?)?;
        self.include(t0, t1);
        Some(())
    }
}
