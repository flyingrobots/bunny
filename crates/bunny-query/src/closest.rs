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
    let query = TriangleQuery::new(v0, v1, v2, p);
    let dots = TriangleDots::new(&query);

    if let Some(point) = triangle_vertex_region(&query, &dots) {
        return point;
    }
    if let Some(point) = triangle_edge_region(&query, &dots) {
        return point;
    }
    triangle_face_region(&query, &dots)
}

#[derive(Clone, Copy)]
struct TriangleQuery {
    v0: FixedVec3,
    v1: FixedVec3,
    v2: FixedVec3,
    ab: FixedVec3,
    ac: FixedVec3,
    p: FixedVec3,
}

impl TriangleQuery {
    fn new(v0: FixedVec3, v1: FixedVec3, v2: FixedVec3, p: FixedVec3) -> Self {
        Self { v0, v1, v2, ab: v1 - v0, ac: v2 - v0, p }
    }
}

#[derive(Clone, Copy)]
struct TriangleDots {
    d1: FixedQ32_32,
    d2: FixedQ32_32,
    d3: FixedQ32_32,
    d4: FixedQ32_32,
    d5: FixedQ32_32,
    d6: FixedQ32_32,
}

impl TriangleDots {
    fn new(query: &TriangleQuery) -> Self {
        let ap = query.p - query.v0;
        let bp = query.p - query.v1;
        let cp = query.p - query.v2;
        Self {
            d1: query.ab.dot(ap),
            d2: query.ac.dot(ap),
            d3: query.ab.dot(bp),
            d4: query.ac.dot(bp),
            d5: query.ab.dot(cp),
            d6: query.ac.dot(cp),
        }
    }
}

fn triangle_vertex_region(query: &TriangleQuery, dots: &TriangleDots) -> Option<FixedVec3> {
    if triangle_v0_region(dots) {
        Some(query.v0)
    } else if triangle_v1_region(dots) {
        Some(query.v1)
    } else if triangle_v2_region(dots) {
        Some(query.v2)
    } else {
        None
    }
}

fn triangle_v0_region(dots: &TriangleDots) -> bool {
    dots.d1 <= FixedQ32_32::ZERO && dots.d2 <= FixedQ32_32::ZERO
}

fn triangle_v1_region(dots: &TriangleDots) -> bool {
    dots.d3 >= FixedQ32_32::ZERO && dots.d4 <= dots.d3
}

fn triangle_v2_region(dots: &TriangleDots) -> bool {
    dots.d6 >= FixedQ32_32::ZERO && dots.d5 <= dots.d6
}

fn triangle_edge_region(query: &TriangleQuery, dots: &TriangleDots) -> Option<FixedVec3> {
    if triangle_ab_region(dots) {
        let v = ratio_or_zero(dots.d1, dots.d1 - dots.d3);
        Some(query.v0 + query.ab * v)
    } else if triangle_ac_region(dots) {
        let w = ratio_or_zero(dots.d2, dots.d2 - dots.d6);
        Some(query.v0 + query.ac * w)
    } else if triangle_bc_region(dots) {
        let w = ratio_or_zero(dots.d4 - dots.d3, (dots.d4 - dots.d3) + (dots.d5 - dots.d6));
        Some(query.v1 + (query.v2 - query.v1) * w)
    } else {
        None
    }
}

fn triangle_ab_region(dots: &TriangleDots) -> bool {
    triangle_vc(dots) <= FixedQ32_32::ZERO
        && dots.d1 >= FixedQ32_32::ZERO
        && dots.d3 <= FixedQ32_32::ZERO
}

fn triangle_ac_region(dots: &TriangleDots) -> bool {
    triangle_vb(dots) <= FixedQ32_32::ZERO
        && dots.d2 >= FixedQ32_32::ZERO
        && dots.d6 <= FixedQ32_32::ZERO
}

fn triangle_bc_region(dots: &TriangleDots) -> bool {
    triangle_va(dots) <= FixedQ32_32::ZERO
        && (dots.d4 - dots.d3) >= FixedQ32_32::ZERO
        && (dots.d5 - dots.d6) >= FixedQ32_32::ZERO
}

fn triangle_face_region(query: &TriangleQuery, dots: &TriangleDots) -> FixedVec3 {
    let denom = triangle_va(dots) + triangle_vb(dots) + triangle_vc(dots);
    let v = ratio_or_zero(triangle_vb(dots), denom);
    let w = ratio_or_zero(triangle_vc(dots), denom);
    query.v0 + query.ab * v + query.ac * w
}

fn triangle_va(dots: &TriangleDots) -> FixedQ32_32 {
    dots.d3 * dots.d6 - dots.d5 * dots.d4
}

fn triangle_vb(dots: &TriangleDots) -> FixedQ32_32 {
    dots.d5 * dots.d2 - dots.d1 * dots.d6
}

fn triangle_vc(dots: &TriangleDots) -> FixedQ32_32 {
    dots.d1 * dots.d4 - dots.d3 * dots.d2
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
    let query = SegmentQuery::new(p1, q1, p2, q2);
    let dots = SegmentDots::new(&query);
    let params = SegmentParams::new(&dots).clamp_s(&dots).clamp_t(&dots);
    let s = clamp(ratio_or_zero(params.s_num, params.s_denom), FixedQ32_32::ZERO, FixedQ32_32::ONE);
    let t = clamp(ratio_or_zero(params.t_num, params.t_denom), FixedQ32_32::ZERO, FixedQ32_32::ONE);
    (p1 + query.d1 * s, p2 + query.d2 * t)
}

#[derive(Clone, Copy)]
struct SegmentQuery {
    d1: FixedVec3,
    d2: FixedVec3,
    r: FixedVec3,
}

impl SegmentQuery {
    fn new(p1: FixedVec3, q1: FixedVec3, p2: FixedVec3, q2: FixedVec3) -> Self {
        Self { d1: q1 - p1, d2: q2 - p2, r: p1 - p2 }
    }
}

#[derive(Clone, Copy)]
struct SegmentDots {
    d1_len_sq: FixedQ32_32,
    d1_dot_d2: FixedQ32_32,
    d2_len_sq: FixedQ32_32,
    d1_dot_r: FixedQ32_32,
    d2_dot_r: FixedQ32_32,
    denom: FixedQ32_32,
}

impl SegmentDots {
    fn new(query: &SegmentQuery) -> Self {
        let d1_len_sq = query.d1.dot(query.d1);
        let d1_dot_d2 = query.d1.dot(query.d2);
        let d2_len_sq = query.d2.dot(query.d2);
        let d1_dot_r = query.d1.dot(query.r);
        let d2_dot_r = query.d2.dot(query.r);
        let length_product = d1_len_sq * d2_len_sq;
        let alignment_product = d1_dot_d2 * d1_dot_d2;
        let denom = length_product - alignment_product;
        Self { d1_len_sq, d1_dot_d2, d2_len_sq, d1_dot_r, d2_dot_r, denom }
    }
}

#[derive(Clone, Copy)]
struct SegmentParams {
    s_num: FixedQ32_32,
    s_denom: FixedQ32_32,
    t_num: FixedQ32_32,
    t_denom: FixedQ32_32,
}

impl SegmentParams {
    fn new(dots: &SegmentDots) -> Self {
        if dots.denom == FixedQ32_32::ZERO {
            return Self::parallel(dots);
        }
        Self::skew(dots).clamp_initial_s(dots)
    }

    const fn parallel(dots: &SegmentDots) -> Self {
        Self {
            s_num: FixedQ32_32::ZERO,
            s_denom: FixedQ32_32::ONE,
            t_num: dots.d2_dot_r,
            t_denom: dots.d2_len_sq,
        }
    }

    fn skew(dots: &SegmentDots) -> Self {
        Self {
            s_num: dots.d1_dot_d2 * dots.d2_dot_r - dots.d2_len_sq * dots.d1_dot_r,
            s_denom: dots.denom,
            t_num: dots.d1_len_sq * dots.d2_dot_r - dots.d1_dot_d2 * dots.d1_dot_r,
            t_denom: dots.denom,
        }
    }

    fn clamp_initial_s(mut self, dots: &SegmentDots) -> Self {
        if self.s_num < FixedQ32_32::ZERO {
            self.s_num = FixedQ32_32::ZERO;
            self.t_num = dots.d2_dot_r;
            self.t_denom = dots.d2_len_sq;
        } else if self.s_num > self.s_denom {
            self.s_num = self.s_denom;
            self.t_num = dots.d2_dot_r + dots.d1_dot_d2;
            self.t_denom = dots.d2_len_sq;
        }
        self
    }

    fn clamp_s(mut self, dots: &SegmentDots) -> Self {
        if self.t_num < FixedQ32_32::ZERO {
            self.t_num = FixedQ32_32::ZERO;
            self = self.project_s(-dots.d1_dot_r, dots.d1_len_sq);
        }
        self
    }

    fn clamp_t(mut self, dots: &SegmentDots) -> Self {
        if self.t_num > self.t_denom {
            self.t_num = self.t_denom;
            self = self.project_s(-dots.d1_dot_r + dots.d1_dot_d2, dots.d1_len_sq);
        }
        self
    }

    fn project_s(mut self, value: FixedQ32_32, max: FixedQ32_32) -> Self {
        self.s_num = clamp(value, FixedQ32_32::ZERO, max);
        self.s_denom = max;
        self
    }
}

fn ratio_or_zero(numer: FixedQ32_32, denom: FixedQ32_32) -> FixedQ32_32 {
    if denom == FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else {
        numer.checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
    }
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
