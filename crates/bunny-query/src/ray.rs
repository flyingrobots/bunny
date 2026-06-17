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
    let v = sphere.center - ray.origin;
    let t = sphere_hit_distance(v, dir, sphere.radius)?;
    let hit_pos = ray.origin + dir * t;
    let normal = (hit_pos - sphere.center).normalize()?;
    Some((hit_pos, normal))
}

fn sphere_hit_distance(
    center_delta: FixedVec3,
    direction: FixedVec3,
    radius: FixedQ32_32,
) -> Option<FixedQ32_32> {
    let tca = center_delta.dot(direction);
    let d2 = center_delta.dot(center_delta) - tca * tca;
    let r2 = radius * radius;
    if d2 > r2 {
        return None;
    }
    let thc = sphere_half_chord(r2, d2)?;
    positive_sphere_hit(tca - thc, tca + thc)
}

fn sphere_half_chord(r2: FixedQ32_32, d2: FixedQ32_32) -> Option<FixedQ32_32> {
    let thc2 = if r2 > d2 { r2 - d2 } else { FixedQ32_32::ZERO };
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

        let t0 = (axis.min_bound - axis.origin).checked_div(axis.direction)?;
        let t1 = (axis.max_bound - axis.origin).checked_div(axis.direction)?;
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
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let pvec = dir.cross(edge2);
    let inv_det = inverse_triangle_determinant(edge1, pvec)?;
    let tvec = ray.origin - v0;
    let u = triangle_u(tvec, pvec, inv_det)?;
    let qvec = tvec.cross(edge1);
    triangle_v(dir, qvec, inv_det, u)?;
    let t = triangle_t(edge2, qvec, inv_det)?;
    Some(ray.origin + dir * t)
}

fn inverse_triangle_determinant(edge1: FixedVec3, pvec: FixedVec3) -> Option<FixedQ32_32> {
    let det = edge1.dot(pvec);
    if det == FixedQ32_32::ZERO {
        None
    } else {
        FixedQ32_32::ONE.checked_div(det)
    }
}

fn triangle_u(tvec: FixedVec3, pvec: FixedVec3, inv_det: FixedQ32_32) -> Option<FixedQ32_32> {
    let u = tvec.dot(pvec) * inv_det;
    (u >= FixedQ32_32::ZERO && u <= FixedQ32_32::ONE).then_some(u)
}

fn triangle_v(
    direction: FixedVec3,
    qvec: FixedVec3,
    inv_det: FixedQ32_32,
    u: FixedQ32_32,
) -> Option<FixedQ32_32> {
    let v = direction.dot(qvec) * inv_det;
    (!outside_triangle_v(v, u)).then_some(v)
}

fn triangle_t(edge2: FixedVec3, qvec: FixedVec3, inv_det: FixedQ32_32) -> Option<FixedQ32_32> {
    let t = edge2.dot(qvec) * inv_det;
    (t >= FixedQ32_32::ZERO).then_some(t)
}

fn outside_triangle_v(v: FixedQ32_32, u: FixedQ32_32) -> bool {
    v < FixedQ32_32::ZERO || u + v > FixedQ32_32::ONE
}
