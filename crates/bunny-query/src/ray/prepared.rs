//! Rays prepared for repeated box tests.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_num::{FixedDivisor, FixedQ32_32};

use super::{aabb_wide, AxisBounds, RayInterval, SlabStop};

/// A ray whose direction is prepared for many box tests.
///
/// Each nonzero direction component holds an exact [`FixedDivisor`], so
/// [`ray_intersects_aabb_prepared`] multiplies where [`ray_intersects_aabb`] divides.
///
/// Use it when many boxes are tested against one ray and the exact `(enter, exit)` bits of
/// [`ray_intersects_aabb`] are needed. A ray tracer that only needs a conservative entry bound
/// should use [`crate::TraceRay::box_entry`], which is faster per box and per ray (ledger
/// entry 5 in `docs/topics/performance/README.md`).
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
///
/// When a slab distance or coordinate difference does not fit Q32.32 the hit is decided by the
/// same exact wide path as [`ray_intersects_aabb`], so the two agree on every input.
#[must_use]
pub fn ray_intersects_aabb_prepared(
    prepared: &PreparedRay3,
    aabb: &FixedAabb3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    match prepared_slab_interval(prepared, aabb) {
        Ok(interval) => interval.hit(),
        Err(SlabStop::Miss) => None,
        Err(SlabStop::Overflow) => aabb_wide::exact_slab_hit(&prepared.ray, aabb),
    }
}

fn prepared_slab_interval(
    prepared: &PreparedRay3,
    aabb: &FixedAabb3,
) -> Result<RayInterval, SlabStop> {
    let mut interval = RayInterval::unbounded();
    let divisors = [prepared.x, prepared.y, prepared.z];
    for (axis, divisor) in AxisBounds::of(&prepared.ray, aabb).into_iter().zip(divisors) {
        interval.update_prepared(axis, divisor)?;
    }
    Ok(interval)
}

impl RayInterval {
    /// `update` with the direction's division prepared: the same results, bit for bit.
    fn update_prepared(
        &mut self,
        axis: AxisBounds,
        divisor: Option<FixedDivisor>,
    ) -> Result<(), SlabStop> {
        let Some(divisor) = divisor else {
            return if axis.contains_origin() { Ok(()) } else { Err(SlabStop::Miss) };
        };
        let distance = |bound: FixedQ32_32| {
            bound
                .checked_sub(axis.origin)
                .and_then(|delta| divisor.checked_div(delta))
                .ok_or(SlabStop::Overflow)
        };
        self.include(distance(axis.min_bound)?, distance(axis.max_bound)?);
        Ok(())
    }
}
