//! Ray-intersection math solvers.

use std::cmp::{max, min};

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

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
#[must_use]
pub fn ray_intersects_aabb(
    ray: &FixedRay3,
    aabb: &FixedAabb3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    let dir = ray.direction.into_inner();
    let mut interval = RayInterval::unbounded();

    interval.update(AxisBounds::new(ray.origin.x, dir.x, aabb.min.x, aabb.max.x))?;
    interval.update(AxisBounds::new(ray.origin.y, dir.y, aabb.min.y, aabb.max.y))?;
    interval.update(AxisBounds::new(ray.origin.z, dir.z, aabb.min.z, aabb.max.z))?;

    if interval.is_miss() {
        None
    } else {
        Some((interval.enter, interval.exit))
    }
}

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
}

struct RayInterval {
    enter: FixedQ32_32,
    exit: FixedQ32_32,
}

impl RayInterval {
    const fn unbounded() -> Self {
        Self { enter: FixedQ32_32::from_raw(i64::MIN), exit: FixedQ32_32::from_raw(i64::MAX) }
    }

    fn update(&mut self, axis: AxisBounds) -> Option<()> {
        if axis.direction == FixedQ32_32::ZERO {
            return axis_contains_origin(axis.origin, axis.min_bound, axis.max_bound);
        }

        let t0 = axis.min_bound.checked_sub(axis.origin)?.checked_div(axis.direction)?;
        let t1 = axis.max_bound.checked_sub(axis.origin)?.checked_div(axis.direction)?;
        self.enter = max(self.enter, min(t0, t1));
        self.exit = min(self.exit, max(t0, t1));
        Some(())
    }

    fn is_miss(&self) -> bool {
        self.enter > self.exit || self.exit < FixedQ32_32::ZERO
    }
}

fn axis_contains_origin(
    origin: FixedQ32_32,
    min_bound: FixedQ32_32,
    max_bound: FixedQ32_32,
) -> Option<()> {
    (origin >= min_bound && origin <= max_bound).then_some(())
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
