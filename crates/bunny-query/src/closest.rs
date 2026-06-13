//! Closest point and minimum-distance query math solvers.

use bunny_geom::{FixedAabb3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Computes the closest point on an AABB to a target point.
#[must_use]
pub fn closest_point_aabb(aabb: &FixedAabb3, point: FixedVec3) -> FixedVec3 {
    FixedVec3::new(
        clamp(point.x, aabb.min.x, aabb.max.x),
        clamp(point.y, aabb.min.y, aabb.max.y),
        clamp(point.z, aabb.min.z, aabb.max.z),
    )
}

/// Checks whether an AABB and a sphere overlap, and returns the closest point on the AABB.
#[must_use]
pub fn aabb_intersects_sphere(aabb: &FixedAabb3, sphere: &FixedSphere3) -> Option<FixedVec3> {
    let closest = closest_point_aabb(aabb, sphere.center);
    let diff = closest - sphere.center;
    let dist_sq = diff.dot(diff);
    let r2 = sphere.radius * sphere.radius;
    if dist_sq <= r2 {
        Some(closest)
    } else {
        None
    }
}

/// Computes the closest point on a triangle to a target point.
#[must_use]
#[allow(clippy::similar_names)]
pub fn closest_point_triangle(
    v0: FixedVec3,
    v1: FixedVec3,
    v2: FixedVec3,
    p: FixedVec3,
) -> FixedVec3 {
    let ab = v1 - v0;
    let ac = v2 - v0;
    let ap = p - v0;

    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= FixedQ32_32::ZERO && d2 <= FixedQ32_32::ZERO {
        return v0;
    }

    let bp = p - v1;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= FixedQ32_32::ZERO && d4 <= d3 {
        return v1;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= FixedQ32_32::ZERO && d1 >= FixedQ32_32::ZERO && d3 <= FixedQ32_32::ZERO {
        let denom = d1 - d3;
        let v = if denom == FixedQ32_32::ZERO {
            FixedQ32_32::ZERO
        } else {
            d1.checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
        };
        return v0 + ab * v;
    }

    let cp = p - v2;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= FixedQ32_32::ZERO && d5 <= d6 {
        return v2;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= FixedQ32_32::ZERO && d2 >= FixedQ32_32::ZERO && d6 <= FixedQ32_32::ZERO {
        let denom = d2 - d6;
        let w = if denom == FixedQ32_32::ZERO {
            FixedQ32_32::ZERO
        } else {
            d2.checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
        };
        return v0 + ac * w;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= FixedQ32_32::ZERO && (d4 - d3) >= FixedQ32_32::ZERO && (d5 - d6) >= FixedQ32_32::ZERO {
        let denom = (d4 - d3) + (d5 - d6);
        let w = if denom == FixedQ32_32::ZERO {
            FixedQ32_32::ZERO
        } else {
            (d4 - d3).checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
        };
        return v1 + (v2 - v1) * w;
    }

    let denom = va + vb + vc;
    let (v, w) = if denom == FixedQ32_32::ZERO {
        (FixedQ32_32::ZERO, FixedQ32_32::ZERO)
    } else {
        (
            vb.checked_div(denom).unwrap_or(FixedQ32_32::ZERO),
            vc.checked_div(denom).unwrap_or(FixedQ32_32::ZERO),
        )
    };
    v0 + ab * v + ac * w
}

/// Computes the closest points on two line segments, and returns the points (C1, C2).
#[must_use]
#[allow(clippy::many_single_char_names)]
#[allow(clippy::suspicious_operation_groupings)]
#[allow(clippy::useless_let_if_seq)]
pub fn closest_points_segments(
    p1: FixedVec3,
    q1: FixedVec3,
    p2: FixedVec3,
    q2: FixedVec3,
) -> (FixedVec3, FixedVec3) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;

    let a = d1.dot(d1);
    let b = d1.dot(d2);
    let c = d2.dot(d2);
    let d = d1.dot(r);
    let e = d2.dot(r);

    let denom = a * c - b * b;

    let mut s_num;
    let mut s_denom = denom;
    let mut t_num;
    let mut t_denom = denom;

    if denom == FixedQ32_32::ZERO {
        s_num = FixedQ32_32::ZERO;
        s_denom = FixedQ32_32::ONE;
        t_num = e;
        t_denom = c;
    } else {
        s_num = b * e - c * d;
        t_num = a * e - b * d;

        if s_num < FixedQ32_32::ZERO {
            s_num = FixedQ32_32::ZERO;
            t_num = e;
            t_denom = c;
        } else if s_num > s_denom {
            s_num = s_denom;
            t_num = e + b;
            t_denom = c;
        }
    }

    if t_num < FixedQ32_32::ZERO {
        t_num = FixedQ32_32::ZERO;
        if -d < FixedQ32_32::ZERO {
            s_num = FixedQ32_32::ZERO;
        } else if -d > a {
            s_num = a;
            s_denom = a;
        } else {
            s_num = -d;
            s_denom = a;
        }
    } else if t_num > t_denom {
        t_num = t_denom;
        if -d + b < FixedQ32_32::ZERO {
            s_num = FixedQ32_32::ZERO;
        } else if -d + b > a {
            s_num = a;
            s_denom = a;
        } else {
            s_num = -d + b;
            s_denom = a;
        }
    }

    let s = if s_denom == FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else {
        s_num.checked_div(s_denom).unwrap_or(FixedQ32_32::ZERO)
    };
    let t = if t_denom == FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else {
        t_num.checked_div(t_denom).unwrap_or(FixedQ32_32::ZERO)
    };

    let s_clamped = clamp(s, FixedQ32_32::ZERO, FixedQ32_32::ONE);
    let t_clamped = clamp(t, FixedQ32_32::ZERO, FixedQ32_32::ONE);

    (p1 + d1 * s_clamped, p2 + d2 * t_clamped)
}

fn clamp(val: FixedQ32_32, min_val: FixedQ32_32, max_val: FixedQ32_32) -> FixedQ32_32 {
    if val < min_val {
        min_val
    } else if val > max_val {
        max_val
    } else {
        val
    }
}
