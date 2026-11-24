//! Independent exact oracle for the 2D primitives (`bunny-planar/1`).
//!
//! Every answer is computed on the unbounded integers of `predicates_oracle::Big` (32-bit
//! digits, schoolbook arithmetic, no division) by a different formulation from the kernel's:
//!
//! - segments meet when the parametric solve `a + t (b - a) = c + u (d - c)` has a solution
//!   with `t` and `u` in `[0, 1]`, or, for parallel segments, when their parameter intervals
//!   along one line overlap (the kernel uses four orientation signs and endpoint boxes);
//! - a point is on a segment when its cross product with the segment is zero and its
//!   parameter is in `[0, 1]` (the kernel uses an orientation sign and the endpoint box);
//! - a circle meets a box when its centre is in the box grown by the radius along one axis,
//!   or within the radius of a corner (the kernel clamps to the box);
//! - two boxes overlap when the larger minimum is at most the smaller maximum on each axis.
//!
//! It shares no helper with the kernel. Rounded results are checked by multiplying back.

#![allow(dead_code)]
// The queries name their points `a` to `d` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

use core::cmp::Ordering;

use bunny_geom::{FixedAabb2, FixedCircle2, FixedSegment2};
use bunny_linalg::FixedVec2;

use crate::predicates_oracle::Big;

fn big(v: i64) -> Big {
    Big::from_i64(v)
}

/// `p - q` on raw values, per axis.
fn minus(p: FixedVec2, q: FixedVec2) -> [Big; 2] {
    [big(p.x.raw()).sub(&big(q.x.raw())), big(p.y.raw()).sub(&big(q.y.raw()))]
}

fn dot(u: &[Big; 2], v: &[Big; 2]) -> Big {
    u[0].mul(&v[0]).add(&u[1].mul(&v[1]))
}

fn cross(u: &[Big; 2], v: &[Big; 2]) -> Big {
    u[0].mul(&v[1]).sub(&u[1].mul(&v[0]))
}

fn is_zero_vector(v: &[Big; 2]) -> bool {
    v[0].is_zero() && v[1].is_zero()
}

/// `0 <= n <= d` for `d > 0`.
fn in_unit(n: &Big, d: &Big) -> bool {
    n.signum() >= 0 && n.cmp(d).is_le()
}

/// Whether `p` is on the closed segment from `a` to `b`, by its parameter.
pub(crate) fn on_segment(a: FixedVec2, b: FixedVec2, p: FixedVec2) -> bool {
    let r = minus(b, a);
    let w = minus(p, a);
    if is_zero_vector(&r) {
        return is_zero_vector(&w);
    }
    cross(&r, &w).is_zero() && in_unit(&dot(&w, &r), &dot(&r, &r))
}

/// Whether the closed segments `a`-`b` and `c`-`d` share a point, by the parametric solve.
pub(crate) fn segments_meet(a: FixedVec2, b: FixedVec2, c: FixedVec2, d: FixedVec2) -> bool {
    let r = minus(b, a);
    let s = minus(d, c);
    if is_zero_vector(&r) {
        return on_segment(c, d, a);
    }
    if is_zero_vector(&s) {
        return on_segment(a, b, c);
    }
    let q = minus(c, a);
    let den = cross(&r, &s);
    if !den.is_zero() {
        let (t, u) = (cross(&q, &s), cross(&q, &r));
        let (t, u, den) =
            if den.signum() < 0 { (t.neg(), u.neg(), den.neg()) } else { (t, u, den) };
        return in_unit(&t, &den) && in_unit(&u, &den);
    }
    if !cross(&q, &r).is_zero() {
        return false;
    }
    // One line: the parameters of `c` and `d` along `r`, scaled by `r . r`.
    let t0 = dot(&q, &r);
    let t1 = dot(&minus(d, a), &r);
    let (lo, hi) = if t0.cmp(&t1).is_le() { (t0, t1) } else { (t1, t0) };
    lo.cmp(&dot(&r, &r)).is_le() && hi.signum() >= 0
}

fn distance_squared(p: FixedVec2, q: FixedVec2) -> Big {
    let v = minus(p, q);
    dot(&v, &v)
}

fn radius_squared(radius: i64) -> Big {
    big(radius).mul(&big(radius))
}

/// Whether `p` is in the closed disc; a negative radius is the empty set.
pub(crate) fn circle_contains(circle: FixedCircle2, p: FixedVec2) -> bool {
    let r = circle.radius.raw();
    r >= 0 && distance_squared(circle.center, p).cmp(&radius_squared(r)).is_le()
}

/// Whether two closed discs share a point.
pub(crate) fn circles_meet(a: FixedCircle2, b: FixedCircle2) -> bool {
    let (ra, rb) = (a.radius.raw(), b.radius.raw());
    if ra < 0 || rb < 0 {
        return false;
    }
    let sum = big(ra).add(&big(rb));
    distance_squared(a.center, b.center).cmp(&sum.mul(&sum)).is_le()
}

fn ordered(aabb: FixedAabb2) -> bool {
    aabb.min.x.raw() <= aabb.max.x.raw() && aabb.min.y.raw() <= aabb.max.y.raw()
}

/// Whether two closed boxes share a point; an inverted box is the empty set.
pub(crate) fn boxes_meet(a: FixedAabb2, b: FixedAabb2) -> bool {
    let axis = |a_lo: i64, a_hi: i64, b_lo: i64, b_hi: i64| a_lo.max(b_lo) <= a_hi.min(b_hi);
    ordered(a)
        && ordered(b)
        && axis(a.min.x.raw(), a.max.x.raw(), b.min.x.raw(), b.max.x.raw())
        && axis(a.min.y.raw(), a.max.y.raw(), b.min.y.raw(), b.max.y.raw())
}

/// Whether the closed box contains `p`, as the box meeting the point box at `p`.
pub(crate) fn box_contains(aabb: FixedAabb2, p: FixedVec2) -> bool {
    boxes_meet(aabb, FixedAabb2::new(p, p))
}

/// The point of the closed box nearest `p`, chosen per axis from the candidates `lo`, `hi`
/// and `p` itself that lie in `[lo, hi]`; `None` for an inverted box.
pub(crate) fn box_closest(aabb: FixedAabb2, p: FixedVec2) -> Option<[i64; 2]> {
    if !ordered(aabb) {
        return None;
    }
    let axis = |lo: i64, hi: i64, v: i64| {
        [lo, hi, v]
            .into_iter()
            .filter(|c| (lo..=hi).contains(c))
            .min_by_key(|c| (i128::from(*c) - i128::from(v)).unsigned_abs())
    };
    Some([
        axis(aabb.min.x.raw(), aabb.max.x.raw(), p.x.raw())?,
        axis(aabb.min.y.raw(), aabb.max.y.raw(), p.y.raw())?,
    ])
}

/// Whether the closed disc meets the closed box: the centre in the box grown by the radius
/// along `x` or along `y`, or within the radius of a corner.
pub(crate) fn circle_meets_box(circle: FixedCircle2, aabb: FixedAabb2) -> bool {
    let r = circle.radius.raw();
    if r < 0 || !ordered(aabb) {
        return false;
    }
    let (cx, cy) = (i128::from(circle.center.x.raw()), i128::from(circle.center.y.raw()));
    let (x0, x1) = (i128::from(aabb.min.x.raw()), i128::from(aabb.max.x.raw()));
    let (y0, y1) = (i128::from(aabb.min.y.raw()), i128::from(aabb.max.y.raw()));
    let r128 = i128::from(r);
    let in_x = (x0..=x1).contains(&cx);
    let in_y = (y0..=y1).contains(&cy);
    if in_y && (x0 - r128..=x1 + r128).contains(&cx) {
        return true;
    }
    if in_x && (y0 - r128..=y1 + r128).contains(&cy) {
        return true;
    }
    let corners = [(aabb.min.x, aabb.min.y), (aabb.min.x, aabb.max.y)]
        .into_iter()
        .chain([(aabb.max.x, aabb.min.y), (aabb.max.x, aabb.max.y)]);
    corners.map(|(x, y)| FixedVec2::new(x, y)).any(|corner| {
        distance_squared(circle.center, corner).cmp(&radius_squared(r)) != Ordering::Greater
    })
}

/// The exact nearest point of the segment to `p`, per axis as a numerator over a positive
/// common denominator: `(a den + (b - a) n) / den` with `n / den` the clamped parameter.
pub(crate) fn segment_closest_exact(segment: FixedSegment2, p: FixedVec2) -> ([Big; 2], Big) {
    let (a, b) = (segment.start, segment.end);
    let r = minus(b, a);
    let rr = dot(&r, &r);
    let a_big = [big(a.x.raw()), big(a.y.raw())];
    if rr.is_zero() {
        return (a_big, Big::from_i64(1));
    }
    let n = dot(&minus(p, a), &r);
    let n = if n.signum() < 0 {
        Big::zero()
    } else if n.cmp(&rr).is_gt() {
        rr.clone()
    } else {
        n
    };
    let axis = |k: usize| a_big[k].mul(&rr).add(&r[k].mul(&n));
    ([axis(0), axis(1)], rr)
}
