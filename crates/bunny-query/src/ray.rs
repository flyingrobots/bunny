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
    let tca = v.dot(dir);
    let v_dot_v = v.dot(v);
    let d2 = v_dot_v - tca * tca;

    let r2 = sphere.radius * sphere.radius;
    if d2 > r2 {
        return None;
    }

    let thc2 = if r2 > d2 { r2 - d2 } else { FixedQ32_32::ZERO };
    let thc = thc2.sqrt()?;

    let t0 = tca - thc;
    let t1 = tca + thc;

    let t = if t0 >= FixedQ32_32::ZERO {
        t0
    } else if t1 >= FixedQ32_32::ZERO {
        t1
    } else {
        return None;
    };

    let hit_pos = ray.origin + dir * t;
    let normal = (hit_pos - sphere.center).normalize()?;
    Some((hit_pos, normal))
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
    let mut t_enter = FixedQ32_32::from_raw(i64::MIN);
    let mut t_exit = FixedQ32_32::from_raw(i64::MAX);

    // X axis
    if dir.x == FixedQ32_32::ZERO {
        if ray.origin.x < aabb.min.x || ray.origin.x > aabb.max.x {
            return None;
        }
    } else {
        let t0 = (aabb.min.x - ray.origin.x).checked_div(dir.x)?;
        let t1 = (aabb.max.x - ray.origin.x).checked_div(dir.x)?;
        t_enter = max(t_enter, min(t0, t1));
        t_exit = min(t_exit, max(t0, t1));
    }

    // Y axis
    if dir.y == FixedQ32_32::ZERO {
        if ray.origin.y < aabb.min.y || ray.origin.y > aabb.max.y {
            return None;
        }
    } else {
        let t0 = (aabb.min.y - ray.origin.y).checked_div(dir.y)?;
        let t1 = (aabb.max.y - ray.origin.y).checked_div(dir.y)?;
        t_enter = max(t_enter, min(t0, t1));
        t_exit = min(t_exit, max(t0, t1));
    }

    // Z axis
    if dir.z == FixedQ32_32::ZERO {
        if ray.origin.z < aabb.min.z || ray.origin.z > aabb.max.z {
            return None;
        }
    } else {
        let t0 = (aabb.min.z - ray.origin.z).checked_div(dir.z)?;
        let t1 = (aabb.max.z - ray.origin.z).checked_div(dir.z)?;
        t_enter = max(t_enter, min(t0, t1));
        t_exit = min(t_exit, max(t0, t1));
    }

    if t_enter > t_exit || t_exit < FixedQ32_32::ZERO {
        None
    } else {
        Some((t_enter, t_exit))
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
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let pvec = dir.cross(edge2);
    let det = edge1.dot(pvec);

    if det == FixedQ32_32::ZERO {
        return None;
    }

    let inv_det = FixedQ32_32::ONE.checked_div(det)?;

    let tvec = ray.origin - v0;
    let u = tvec.dot(pvec) * inv_det;
    if u < FixedQ32_32::ZERO || u > FixedQ32_32::ONE {
        return None;
    }

    let qvec = tvec.cross(edge1);
    let v = dir.dot(qvec) * inv_det;
    if v < FixedQ32_32::ZERO || u + v > FixedQ32_32::ONE {
        return None;
    }

    let t = edge2.dot(qvec) * inv_det;
    if t < FixedQ32_32::ZERO {
        return None;
    }

    Some(ray.origin + dir * t)
}
