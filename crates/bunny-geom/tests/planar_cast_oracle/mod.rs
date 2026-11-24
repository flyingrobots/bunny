//! Independent exact oracle for the 2D ray queries and point in polygon (`bunny-planar/1`).
//!
//! Every answer is computed on the unbounded integers of `predicates_oracle::Big` (32-bit
//! digits, schoolbook arithmetic, no division), by a different formulation from the kernel's:
//!
//! - a ray meets a segment where the segment crosses the ray's line, found by interpolating
//!   between the endpoints' signed distances from that line, and the parameter is the
//!   crossing point's projection on the direction (the kernel solves with Cramer's rule);
//! - a ray meets a box when the origin is in the box or the ray meets one of its four edges,
//!   the first parameter being the least over the edges (the kernel intersects slabs);
//! - a ray meets a disc when the point of the ray nearest the centre is in the disc, and a
//!   rounded first parameter `T` is checked by evaluating the quadratic `|o + y d - c|^2 - r^2`
//!   at `y = (T -+ 1/2) 2^-32`, with no square root (the kernel takes an integer square root);
//! - a point's winding number counts the polygon's edges crossing the horizontal half-line to
//!   its right, at their exact rational crossing abscissa (the kernel reads orientation signs).
//!
//! It shares no helper with the kernel.

#![allow(dead_code)]
// The queries name their points `a` to `e`, `o` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

use core::cmp::Ordering;

use bunny_geom::{FixedAabb2, FixedCircle2, FixedRay2, RayHit2};
use bunny_linalg::FixedVec2;

use crate::planar_oracle::on_segment;
use crate::predicates_oracle::{rounds_to, Big};

fn big(v: i64) -> Big {
    Big::from_i64(v)
}

fn minus(p: FixedVec2, q: FixedVec2) -> [Big; 2] {
    [big(p.x.raw()).sub(&big(q.x.raw())), big(p.y.raw()).sub(&big(q.y.raw()))]
}

fn dot(u: &[Big; 2], v: &[Big; 2]) -> Big {
    u[0].mul(&v[0]).add(&u[1].mul(&v[1]))
}

fn cross(u: &[Big; 2], v: &[Big; 2]) -> Big {
    u[0].mul(&v[1]).sub(&u[1].mul(&v[0]))
}

fn direction(ray: FixedRay2) -> [Big; 2] {
    let d = ray.direction.into_inner();
    [big(d.x.raw()), big(d.y.raw())]
}

/// An exact parameter `num / den` with `den > 0`, in units of the direction as given.
pub(crate) type Fraction = (Big, Big);

fn less(p: &Fraction, q: &Fraction) -> bool {
    p.0.mul(&q.1).cmp(&q.0.mul(&p.1)).is_lt()
}

fn positive(num: Big, den: Big) -> Fraction {
    if den.signum() < 0 {
        (num.neg(), den.neg())
    } else {
        (num, den)
    }
}

/// The first parameter at which the ray meets the closed segment `c`-`e`, or `None`.
pub(crate) fn ray_segment(ray: FixedRay2, c: FixedVec2, e: FixedVec2) -> Option<Fraction> {
    let d = direction(ray);
    let a = dot(&d, &d);
    let (qc, qe) = (minus(c, ray.origin), minus(e, ray.origin));
    // Signed distances of the endpoints from the ray's line, times `|d|`.
    let (side_c, side_e) = (cross(&d, &qc), cross(&d, &qe));
    if side_c.signum() * side_e.signum() > 0 {
        return None;
    }
    if side_c.is_zero() && side_e.is_zero() {
        let (tc, te) = (dot(&qc, &d), dot(&qe, &d));
        let (near, far) = if tc.cmp(&te).is_le() { (tc, te) } else { (te, tc) };
        if far.signum() < 0 {
            return None;
        }
        let near = if near.signum() < 0 { Big::zero() } else { near };
        return Some((near, a));
    }
    // The crossing point is `c + (e - c) side_c / (side_c - side_e)`; its parameter is its
    // offset from the origin dotted with `d`, over `a`.
    let spread = side_c.sub(&side_e);
    let num = dot(&qc, &d).mul(&spread).add(&dot(&minus(e, c), &d).mul(&side_c));
    let (num, den) = positive(num, a.mul(&spread));
    (num.signum() >= 0).then_some((num, den))
}

fn ordered(aabb: FixedAabb2) -> bool {
    aabb.min.x.raw() <= aabb.max.x.raw() && aabb.min.y.raw() <= aabb.max.y.raw()
}

/// The first parameter at which the ray meets the closed box, or `None`: zero when the origin
/// is in the box, otherwise the least first parameter over its four edges.
pub(crate) fn ray_box(ray: FixedRay2, aabb: FixedAabb2) -> Option<Fraction> {
    if !ordered(aabb) {
        return None;
    }
    let o = ray.origin;
    let inside = (aabb.min.x.raw()..=aabb.max.x.raw()).contains(&o.x.raw())
        && (aabb.min.y.raw()..=aabb.max.y.raw()).contains(&o.y.raw());
    if inside {
        return Some((Big::zero(), Big::from_i64(1)));
    }
    let corner = |x: bool, y: bool| {
        FixedVec2::new(
            if x { aabb.max.x } else { aabb.min.x },
            if y { aabb.max.y } else { aabb.min.y },
        )
    };
    let edges = [
        (corner(false, false), corner(true, false)),
        (corner(true, false), corner(true, true)),
        (corner(true, true), corner(false, true)),
        (corner(false, true), corner(false, false)),
    ];
    edges.into_iter().filter_map(|(c, e)| ray_segment(ray, c, e)).reduce(|best, next| {
        if less(&next, &best) {
            next
        } else {
            best
        }
    })
}

/// Whether the ray meets the closed disc: the point of the ray nearest the centre is in it.
pub(crate) fn ray_meets_circle(ray: FixedRay2, circle: FixedCircle2) -> bool {
    let r = circle.radius.raw();
    if r < 0 {
        return false;
    }
    let d = direction(ray);
    let a = dot(&d, &d);
    let w = minus(circle.center, ray.origin);
    let b = dot(&w, &d);
    let rr = big(r).mul(&big(r));
    if b.signum() <= 0 {
        return dot(&w, &w).cmp(&rr).is_le();
    }
    // The nearest point is `o + (b / a) d`; `a (w - (b / a) d) = a w - b d`.
    let gap = [w[0].mul(&a).sub(&d[0].mul(&b)), w[1].mul(&a).sub(&d[1].mul(&b))];
    dot(&gap, &gap).cmp(&a.mul(&a).mul(&rr)).is_le()
}

/// The quadratic `f(y) = a y^2 - 2 b y + (|w|^2 - r^2)` at `y = m 2^-33`, times `2^66`, and
/// whether `y <= b / a` (the falling side of `f`, where the first root is).
fn quadratic_at(ray: FixedRay2, circle: FixedCircle2, m: &Big) -> (Ordering, bool) {
    let d = direction(ray);
    let a = dot(&d, &d);
    let w = minus(circle.center, ray.origin);
    let b = dot(&w, &d);
    let r = big(circle.radius.raw());
    let c0 = dot(&w, &w).sub(&r.mul(&r));
    let value = a.mul(m).mul(m).sub(&b.mul(m).shl(34)).add(&c0.shl(66));
    (value.signum().cmp(&0), m.mul(&a).cmp(&b.shl(33)).is_le())
}

/// How the first root `t` compares with `y = m 2^-33`, for a ray that meets the disc from an
/// origin outside it.
fn first_root_against(ray: FixedRay2, circle: FixedCircle2, m: &Big) -> Ordering {
    match quadratic_at(ray, circle, m) {
        (Ordering::Equal, true) => Ordering::Equal,
        (Ordering::Greater, true) => Ordering::Greater,
        _ => Ordering::Less,
    }
}

/// Whether `hit` is the first parameter of the ray on the disc rounded once to nearest, ties
/// to even, in raw units: `At(0)` from an origin in the disc, `Beyond` when the rounded value
/// is above `i64::MAX`, and `Miss` exactly when the ray misses.
pub(crate) fn circle_hit_is(ray: FixedRay2, circle: FixedCircle2, hit: RayHit2) -> bool {
    if !ray_meets_circle(ray, circle) {
        return hit == RayHit2::Miss;
    }
    let w = minus(circle.center, ray.origin);
    let r = big(circle.radius.raw());
    if dot(&w, &w).cmp(&r.mul(&r)).is_le() {
        return hit == RayHit2::At(bunny_num::FixedQ32_32::ZERO);
    }
    let odd = |t: i128| Big::from_i128(2 * t + 1);
    match hit {
        RayHit2::Miss => false,
        RayHit2::Beyond => {
            first_root_against(ray, circle, &odd(i128::from(i64::MAX))) != Ordering::Less
        }
        RayHit2::At(t) => {
            let t = i128::from(t.raw());
            let low = first_root_against(ray, circle, &odd(t - 1));
            let high = first_root_against(ray, circle, &odd(t));
            let even = t % 2 == 0;
            let above_low = low == Ordering::Greater || (low == Ordering::Equal && even);
            let below_high = high == Ordering::Less || (high == Ordering::Equal && even);
            above_low && below_high
        }
    }
}

/// Whether `hit` is the exact parameter `fraction` rounded once to nearest, ties to even, in
/// raw units (`2^32` times the parameter), with `Beyond` above `i64::MAX` and `Miss` for none.
pub(crate) fn parameter_is(fraction: Option<&Fraction>, hit: RayHit2) -> bool {
    match (fraction, hit) {
        (None, RayHit2::Miss) => true,
        (Some((num, den)), RayHit2::At(t)) => rounds_to(&num.shl(32), den, Some(t.raw())),
        (Some((num, den)), RayHit2::Beyond) => rounds_to(&num.shl(32), den, None),
        _ => false,
    }
}

/// The winding number of the closed polygon about `p`, or `None` when `p` is on an edge.
///
/// Counts the edges crossing the half-line `y = p.y`, `x > p.x`: an edge going up (from
/// `y <= p.y` to `y > p.y`) adds one, an edge going down subtracts one, each where its exact
/// crossing abscissa `a.x + (p.y - a.y) (b.x - a.x) / (b.y - a.y)` is right of `p.x`.
pub(crate) fn winding(vertices: &[FixedVec2], p: FixedVec2) -> Option<i64> {
    let n = vertices.len();
    let mut total = 0_i64;
    for (i, &a) in vertices.iter().enumerate() {
        let b = vertices[(i + 1) % n];
        if on_segment(a, b, p) {
            return None;
        }
        let (ay, by, py) = (a.y.raw(), b.y.raw(), p.y.raw());
        let up = ay <= py && py < by;
        let down = by <= py && py < ay;
        if !(up || down) {
            continue;
        }
        let rise = big(by).sub(&big(ay));
        let num = big(a.x.raw())
            .sub(&big(p.x.raw()))
            .mul(&rise)
            .add(&big(py).sub(&big(ay)).mul(&big(b.x.raw()).sub(&big(a.x.raw()))));
        if num.signum() * rise.signum() > 0 {
            total += if up { 1 } else { -1 };
        }
    }
    Some(total)
}

/// The crossing parity of the polygon about `p` (the even-odd rule), `None` on an edge.
pub(crate) fn crossings_odd(vertices: &[FixedVec2], p: FixedVec2) -> Option<bool> {
    let n = vertices.len();
    let mut odd = false;
    for (i, &a) in vertices.iter().enumerate() {
        let b = vertices[(i + 1) % n];
        if on_segment(a, b, p) {
            return None;
        }
        let (ay, by, py) = (a.y.raw(), b.y.raw(), p.y.raw());
        if (ay <= py) == (by <= py) {
            continue;
        }
        let rise = big(by).sub(&big(ay));
        let num = big(a.x.raw())
            .sub(&big(p.x.raw()))
            .mul(&rise)
            .add(&big(py).sub(&big(ay)).mul(&big(b.x.raw()).sub(&big(a.x.raw()))));
        odd ^= num.signum() * rise.signum() > 0;
    }
    Some(odd)
}
