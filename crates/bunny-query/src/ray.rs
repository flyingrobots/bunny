//! Ray-intersection math solvers.

use std::cmp::{max, min};

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

mod aabb_wide;
mod prepared;
pub use prepared::{ray_intersects_aabb_prepared, PreparedRay3};

/// Computes the intersection of a ray and a sphere.
///
/// Returns the intersection point and the surface normal at the intersection,
/// or `None` if there is no intersection along the positive ray direction.
#[must_use]
pub fn ray_intersects_sphere(
    ray: &FixedRay3,
    sphere: &FixedSphere3,
) -> Option<(FixedVec3, FixedVec3)> {
    let dir = ray.direction.into_inner();
    let v = checked_sub_vec3(sphere.center, ray.origin)?;
    let t = sphere_hit_distance(v, dir, sphere.radius)?;
    let hit_pos = checked_add_vec3(ray.origin, checked_mul_vec3(dir, t)?)?;
    let normal = checked_sub_vec3(hit_pos, sphere.center)?.normalize()?;
    Some((hit_pos, normal))
}

fn sphere_hit_distance(
    center_delta: FixedVec3,
    direction: FixedVec3,
    radius: FixedQ32_32,
) -> Option<FixedQ32_32> {
    let (tca, d2) = sphere_projection(center_delta, direction)?;
    let r2 = radius.checked_mul(radius)?;
    if d2 > r2 {
        return None;
    }
    let thc = sphere_half_chord(r2, d2)?;
    positive_sphere_hit(tca.checked_sub(thc)?, tca.checked_add(thc)?)
}

fn sphere_projection(
    center_delta: FixedVec3,
    direction: FixedVec3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    let tca = checked_dot(center_delta, direction)?;
    let d2 = checked_dot(center_delta, center_delta)?.checked_sub(tca.checked_mul(tca)?)?;
    if d2 < FixedQ32_32::ZERO {
        let center_distance = center_delta.length()?;
        let clamped_tca =
            if tca < FixedQ32_32::ZERO { center_distance.checked_neg()? } else { center_distance };
        Some((clamped_tca, FixedQ32_32::ZERO))
    } else {
        Some((tca, d2))
    }
}

fn sphere_half_chord(r2: FixedQ32_32, d2: FixedQ32_32) -> Option<FixedQ32_32> {
    let thc2 = if r2 > d2 { r2.checked_sub(d2)? } else { FixedQ32_32::ZERO };
    thc2.sqrt()
}

fn positive_sphere_hit(t0: FixedQ32_32, t1: FixedQ32_32) -> Option<FixedQ32_32> {
    if t0 >= FixedQ32_32::ZERO {
        Some(t0)
    } else if t1 >= FixedQ32_32::ZERO {
        Some(t1)
    } else {
        None
    }
}

/// Computes the entry and exit intersection distances of a ray and an AABB.
///
/// Returns `Some((t_enter, t_exit))` if the ray intersects the box, where
/// `t_enter` is the hit distance of the entry point and `t_exit` is the
/// hit distance of the exit point. Returns `None` if the ray misses the box.
///
/// # Distances
///
/// For each axis with a non-zero direction component, the slab distances are
/// `(min - origin) / direction` and `(max - origin) / direction`, rounded to the
/// nearest raw unit with ties to even (the rounding of
/// [`FixedQ32_32::checked_div`]). `t_enter` is the largest near-slab distance and
/// `t_exit` the smallest far-slab distance over the three axes. An axis with a zero
/// component does not bound the interval; the ray misses if the origin lies outside
/// that slab. When no axis bounds a side, it stays at `FixedQ32_32::MIN_RAW` or
/// `FixedQ32_32::MAX_RAW`.
///
/// A slab distance that does not fit Q32.32 (a tiny direction component, or a
/// bound about 2^31 units or more from the origin) saturates to `MIN_RAW` or
/// `MAX_RAW` by its sign, so the interval is clamped by the other axes. The coordinate
/// difference is taken in `i128`, so it is never saturated before the division.
///
/// # Hit decision
///
/// When every slab distance fits, the hit is decided from the rounded distances:
/// `t_enter <= t_exit` and `t_exit >= 0`. A ray that misses an edge or corner, or
/// leaves the box behind its origin, by less than half a raw unit can therefore
/// report a hit whose rounded bounds are equal or whose exit is zero. When some slab
/// distance does not fit, the hit is decided exactly with `i128` rational arithmetic
/// before the bounds are rounded and saturated, so a saturated bound never turns a
/// miss into a hit. Every reported hit has `t_enter <= t_exit` and `t_exit >= 0`.
///
/// Until flyingrobots/bunny#211 this function returned `None` whenever a slab
/// distance or coordinate difference overflowed. Every hit it reported then is
/// still reported with the same bits.
#[must_use]
pub fn ray_intersects_aabb(
    ray: &FixedRay3,
    aabb: &FixedAabb3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    match rounded_slab_interval(ray, aabb) {
        Ok(interval) => interval.hit(),
        Err(SlabStop::Miss) => None,
        Err(SlabStop::Overflow) => aabb_wide::exact_slab_hit(ray, aabb),
    }
}

fn rounded_slab_interval(ray: &FixedRay3, aabb: &FixedAabb3) -> Result<RayInterval, SlabStop> {
    let mut interval = RayInterval::unbounded();
    for axis in AxisBounds::of(ray, aabb) {
        interval.update(axis)?;
    }
    Ok(interval)
}

/// Why the rounded slab test stopped before all three axes were applied.
enum SlabStop {
    /// The origin lies outside a slab whose direction component is zero.
    Miss,
    /// A slab distance does not fit Q32.32.
    Overflow,
}

/// One axis of a ray/AABB slab test.
#[derive(Clone, Copy)]
struct AxisBounds {
    origin: FixedQ32_32,
    direction: FixedQ32_32,
    min_bound: FixedQ32_32,
    max_bound: FixedQ32_32,
}

impl AxisBounds {
    const fn new(
        origin: FixedQ32_32,
        direction: FixedQ32_32,
        min_bound: FixedQ32_32,
        max_bound: FixedQ32_32,
    ) -> Self {
        Self { origin, direction, min_bound, max_bound }
    }

    /// The x, y and z slabs of `aabb` for `ray`, in that order.
    const fn of(ray: &FixedRay3, aabb: &FixedAabb3) -> [Self; 3] {
        let dir = ray.direction.into_inner();
        [
            Self::new(ray.origin.x, dir.x, aabb.min.x, aabb.max.x),
            Self::new(ray.origin.y, dir.y, aabb.min.y, aabb.max.y),
            Self::new(ray.origin.z, dir.z, aabb.min.z, aabb.max.z),
        ]
    }

    /// Whether the origin lies inside this slab (used when the direction is zero).
    fn contains_origin(self) -> bool {
        self.origin >= self.min_bound && self.origin <= self.max_bound
    }

    fn slab_distance(self, bound: FixedQ32_32) -> Result<FixedQ32_32, SlabStop> {
        bound
            .checked_sub(self.origin)
            .and_then(|delta| delta.checked_div(self.direction))
            .ok_or(SlabStop::Overflow)
    }
}

struct RayInterval {
    enter: FixedQ32_32,
    exit: FixedQ32_32,
}

impl RayInterval {
    const fn unbounded() -> Self {
        Self { enter: FixedQ32_32::from_raw(i64::MIN), exit: FixedQ32_32::from_raw(i64::MAX) }
    }

    fn update(&mut self, axis: AxisBounds) -> Result<(), SlabStop> {
        if axis.direction == FixedQ32_32::ZERO {
            return if axis.contains_origin() { Ok(()) } else { Err(SlabStop::Miss) };
        }

        let t0 = axis.slab_distance(axis.min_bound)?;
        let t1 = axis.slab_distance(axis.max_bound)?;
        self.include(t0, t1);
        Ok(())
    }

    fn include(&mut self, t0: FixedQ32_32, t1: FixedQ32_32) {
        self.enter = max(self.enter, min(t0, t1));
        self.exit = min(self.exit, max(t0, t1));
    }

    fn hit(&self) -> Option<(FixedQ32_32, FixedQ32_32)> {
        let miss = self.enter > self.exit || self.exit < FixedQ32_32::ZERO;
        (!miss).then_some((self.enter, self.exit))
    }
}

/// Computes the intersection of a ray and a triangle defined by three vertices.
///
/// Returns the intersection point coordinates, or `None` if there is no intersection
/// along the positive ray direction.
#[must_use]
pub fn ray_intersects_triangle(
    ray: &FixedRay3,
    v0: FixedVec3,
    v1: FixedVec3,
    v2: FixedVec3,
) -> Option<FixedVec3> {
    let dir = ray.direction.into_inner();
    let edge1 = checked_sub_vec3(v1, v0)?;
    let edge2 = checked_sub_vec3(v2, v0)?;
    let pvec = checked_cross(dir, edge2)?;
    let inv_det = inverse_triangle_determinant(edge1, pvec)?;
    let tvec = checked_sub_vec3(ray.origin, v0)?;
    let u = triangle_u(tvec, pvec, inv_det)?;
    let qvec = checked_cross(tvec, edge1)?;
    triangle_v(dir, qvec, inv_det, u)?;
    let t = triangle_t(edge2, qvec, inv_det)?;
    checked_add_vec3(ray.origin, checked_mul_vec3(dir, t)?)
}

fn inverse_triangle_determinant(edge1: FixedVec3, pvec: FixedVec3) -> Option<FixedQ32_32> {
    let det = checked_dot(edge1, pvec)?;
    if det == FixedQ32_32::ZERO {
        None
    } else {
        FixedQ32_32::ONE.checked_div(det)
    }
}

fn triangle_u(tvec: FixedVec3, pvec: FixedVec3, inv_det: FixedQ32_32) -> Option<FixedQ32_32> {
    let u = checked_dot(tvec, pvec)?.checked_mul(inv_det)?;
    (u >= FixedQ32_32::ZERO && u <= FixedQ32_32::ONE).then_some(u)
}

fn triangle_v(
    direction: FixedVec3,
    qvec: FixedVec3,
    inv_det: FixedQ32_32,
    u: FixedQ32_32,
) -> Option<FixedQ32_32> {
    let v = checked_dot(direction, qvec)?.checked_mul(inv_det)?;
    if v < FixedQ32_32::ZERO {
        return None;
    }
    let uv = u.checked_add(v)?;
    (uv <= FixedQ32_32::ONE).then_some(v)
}

fn triangle_t(edge2: FixedVec3, qvec: FixedVec3, inv_det: FixedQ32_32) -> Option<FixedQ32_32> {
    let t = checked_dot(edge2, qvec)?.checked_mul(inv_det)?;
    (t >= FixedQ32_32::ZERO).then_some(t)
}

fn checked_dot(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedQ32_32> {
    lhs.x
        .checked_mul(rhs.x)?
        .checked_add(lhs.y.checked_mul(rhs.y)?)?
        .checked_add(lhs.z.checked_mul(rhs.z)?)
}

fn checked_cross(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        lhs.y.checked_mul(rhs.z)?.checked_sub(lhs.z.checked_mul(rhs.y)?)?,
        lhs.z.checked_mul(rhs.x)?.checked_sub(lhs.x.checked_mul(rhs.z)?)?,
        lhs.x.checked_mul(rhs.y)?.checked_sub(lhs.y.checked_mul(rhs.x)?)?,
    ))
}

fn checked_add_vec3(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        lhs.x.checked_add(rhs.x)?,
        lhs.y.checked_add(rhs.y)?,
        lhs.z.checked_add(rhs.z)?,
    ))
}

fn checked_sub_vec3(lhs: FixedVec3, rhs: FixedVec3) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        lhs.x.checked_sub(rhs.x)?,
        lhs.y.checked_sub(rhs.y)?,
        lhs.z.checked_sub(rhs.z)?,
    ))
}

fn checked_mul_vec3(vec: FixedVec3, scalar: FixedQ32_32) -> Option<FixedVec3> {
    Some(FixedVec3::new(
        vec.x.checked_mul(scalar)?,
        vec.y.checked_mul(scalar)?,
        vec.z.checked_mul(scalar)?,
    ))
}
