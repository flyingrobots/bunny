//! Independent exact oracle for the separating-axis kernels (`bunny-sat/1`).
//!
//! It works from the corners of each shape, never from its centre and extents, in the
//! unbounded integers of `predicates_oracle::Big`. An oriented box's corners are exact on the
//! grid of `2^-64` units (`centre 2^32 + sum(+-h_i a_i)` on raw values); a triangle's vertices
//! and a box's corners are exact on the raw grid. Two answers are computed separately:
//!
//! - [`hull`] places the origin against the convex hull of the pairwise differences of the two
//!   point sets (the Minkowski difference) by brute force over every plane through three of
//!   its points, knowing nothing of separating axes or of which points are vertices;
//! - [`expected`] projects every corner onto each candidate axis, built here from the raw
//!   vertices and columns in the chapter's order, and reports the first strictly separating
//!   axis, else the first axis with a zero gap.
//!
//! Neither shares a helper with the kernel, which projects a centre and sums extents in a
//! 256-bit two's complement integer, or calls the exact predicates.

#![allow(dead_code)]

use core::cmp::Ordering;

use bunny_geom::{FixedAabb3, FixedObb3, FixedTriangle3, ObbAxis, SatClass, TriangleAabbAxis};
use bunny_linalg::FixedVec3;

use crate::predicates_oracle::Big;

/// An exact point or vector.
pub(crate) type P = [Big; 3];

/// Where the origin lies against the Minkowski difference `first - second`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hull {
    /// Outside the closed hull: the shapes are disjoint.
    Separated,
    /// In the hull but not in its interior: the shapes meet, and no neighbourhood of the
    /// origin is inside.
    Touching,
    /// In the interior: every small enough translation keeps the shapes meeting.
    Overlapping,
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn big3(v: [i64; 3]) -> P {
    v.map(Big::from_i64)
}

fn sub(a: &P, b: &P) -> P {
    [a[0].sub(&b[0]), a[1].sub(&b[1]), a[2].sub(&b[2])]
}

fn dot(a: &P, b: &P) -> Big {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

fn cross(a: &P, b: &P) -> P {
    [
        a[1].mul(&b[2]).sub(&a[2].mul(&b[1])),
        a[2].mul(&b[0]).sub(&a[0].mul(&b[2])),
        a[0].mul(&b[1]).sub(&a[1].mul(&b[0])),
    ]
}

fn is_zero(a: &P) -> bool {
    a.iter().all(Big::is_zero)
}

fn lexicographic(a: &P, b: &P) -> Ordering {
    a[0].cmp(&b[0]).then_with(|| a[1].cmp(&b[1])).then_with(|| a[2].cmp(&b[2]))
}

/// The eight corners of an oriented box on the `2^-64` grid.
pub(crate) fn obb_corners(obb: &FixedObb3) -> Vec<P> {
    let axes = obb.axes();
    let columns = [axes.column0(), axes.column1(), axes.column2()].map(|c| big3(raw3(c)));
    let h = raw3(obb.half_extents());
    let centre = big3(raw3(obb.center())).map(|c| c.shl(32));
    let mut corners = Vec::new();
    for mask in 0..8 {
        let mut corner = centre.clone();
        for (i, column) in columns.iter().enumerate() {
            let extent = Big::from_i64(if mask & (1 << i) == 0 { -h[i] } else { h[i] });
            for (slot, entry) in corner.iter_mut().zip(column) {
                *slot = slot.add(&entry.mul(&extent));
            }
        }
        corners.push(corner);
    }
    corners
}

/// The eight corners of an axis-aligned box, as given: an inverted box has the same eight.
pub(crate) fn aabb_corners(aabb: &FixedAabb3) -> Vec<P> {
    let (lo, hi) = (raw3(aabb.min), raw3(aabb.max));
    (0..8)
        .map(|mask| big3([0, 1, 2].map(|k| if mask & (1 << k) == 0 { lo[k] } else { hi[k] })))
        .collect()
}

/// The three vertices of a triangle on the raw grid.
pub(crate) fn triangle_points(triangle: &FixedTriangle3) -> Vec<P> {
    triangle.vertices().iter().map(|v| big3(raw3(*v))).collect()
}

/// The candidate axes of two oriented boxes in the chapter's order, zero axes included.
pub(crate) fn obb_axes(first: &FixedObb3, second: &FixedObb3) -> Vec<(ObbAxis, P)> {
    let columns = |o: &FixedObb3| {
        let m = o.axes();
        [m.column0(), m.column1(), m.column2()].map(|c| big3(raw3(c)))
    };
    let (a, b) = (columns(first), columns(second));
    let mut axes = Vec::new();
    for k in 0..3 {
        axes.push((ObbAxis::FaceOfFirst(k), cross(&a[(k + 1) % 3], &a[(k + 2) % 3])));
    }
    for k in 0..3 {
        axes.push((ObbAxis::FaceOfSecond(k), cross(&b[(k + 1) % 3], &b[(k + 2) % 3])));
    }
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            axes.push((ObbAxis::EdgePair { first: i, second: j }, cross(ai, bj)));
        }
    }
    axes
}

/// The candidate axes of a triangle against an axis-aligned box in the chapter's order.
pub(crate) fn triangle_aabb_axes(triangle: &FixedTriangle3) -> Vec<(TriangleAabbAxis, P)> {
    let v = triangle_points(triangle);
    let unit = |k: usize| big3([0, 1, 2].map(|i| i64::from(i == k)));
    let mut axes = Vec::new();
    for k in 0..3 {
        axes.push((TriangleAabbAxis::BoxFace(k), unit(k)));
    }
    axes.push((TriangleAabbAxis::TriangleFace, cross(&sub(&v[1], &v[0]), &sub(&v[2], &v[0]))));
    for edge in 0..3 {
        let e = sub(&v[(edge + 1) % 3], &v[edge]);
        for axis in 0..3 {
            axes.push((TriangleAabbAxis::EdgeCross { edge, axis }, cross(&e, &unit(axis))));
        }
    }
    axes
}

fn extremes(points: &[P], axis: &P) -> (Big, Big) {
    let mut values = points.iter().map(|p| dot(p, axis));
    let first = values.next().expect("a point");
    values.fold((first.clone(), first), |(lo, hi), x| {
        let lo = if x.cmp(&lo).is_lt() { x.clone() } else { lo };
        let hi = if x.cmp(&hi).is_gt() { x } else { hi };
        (lo, hi)
    })
}

/// The sign of the gap between the projections of the two point sets on `axis`:
/// `max(min2 - max1, min1 - max2)`.
pub(crate) fn gap(first: &[P], second: &[P], axis: &P) -> Ordering {
    let (lo1, hi1) = extremes(first, axis);
    let (lo2, hi2) = extremes(second, axis);
    let left = lo2.sub(&hi1);
    let right = lo1.sub(&hi2);
    let larger = if left.cmp(&right).is_ge() { left } else { right };
    larger.cmp(&Big::zero())
}

/// The first strictly separating nonzero axis in order, else the first with a zero gap,
/// else `Overlapping`.
pub(crate) fn expected<A: Copy>(first: &[P], second: &[P], axes: &[(A, P)]) -> SatClass<A> {
    let mut touching = None;
    for (name, axis) in axes {
        if is_zero(axis) {
            continue;
        }
        match gap(first, second, axis) {
            Ordering::Greater => return SatClass::Separated { axis: *name },
            Ordering::Equal if touching.is_none() => touching = Some(*name),
            _ => {}
        }
    }
    touching.map_or(SatClass::Overlapping, |axis| SatClass::Touching { axis })
}

/// The integer operations the hull test needs, so it can run on `Big` and, for small
/// coordinates, on checked `i128` (an overflow panics; it never wraps).
pub(crate) trait Num: Clone {
    fn zero() -> Self;
    fn plus(&self, rhs: &Self) -> Self;
    fn minus(&self, rhs: &Self) -> Self;
    fn times(&self, rhs: &Self) -> Self;
    fn sign(&self) -> i32;
    fn order(&self, rhs: &Self) -> Ordering;
}

impl Num for Big {
    fn zero() -> Self {
        Self::zero()
    }
    fn plus(&self, rhs: &Self) -> Self {
        self.add(rhs)
    }
    fn minus(&self, rhs: &Self) -> Self {
        self.sub(rhs)
    }
    fn times(&self, rhs: &Self) -> Self {
        self.mul(rhs)
    }
    fn sign(&self) -> i32 {
        self.signum()
    }
    fn order(&self, rhs: &Self) -> Ordering {
        self.cmp(rhs)
    }
}

/// A checked `i128`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Small(i128);

impl Num for Small {
    fn zero() -> Self {
        Self(0)
    }
    fn plus(&self, rhs: &Self) -> Self {
        Self(self.0.checked_add(rhs.0).expect("i128 oracle overflow"))
    }
    fn minus(&self, rhs: &Self) -> Self {
        Self(self.0.checked_sub(rhs.0).expect("i128 oracle overflow"))
    }
    fn times(&self, rhs: &Self) -> Self {
        Self(self.0.checked_mul(rhs.0).expect("i128 oracle overflow"))
    }
    fn sign(&self) -> i32 {
        i32::from(self.0 > 0) - i32::from(self.0 < 0)
    }
    fn order(&self, rhs: &Self) -> Ordering {
        self.0.cmp(&rhs.0)
    }
}

type V<T> = [T; 3];

fn vsub<T: Num>(a: &V<T>, b: &V<T>) -> V<T> {
    [a[0].minus(&b[0]), a[1].minus(&b[1]), a[2].minus(&b[2])]
}

fn vdot<T: Num>(a: &V<T>, b: &V<T>) -> T {
    a[0].times(&b[0]).plus(&a[1].times(&b[1])).plus(&a[2].times(&b[2]))
}

fn vcross<T: Num>(a: &V<T>, b: &V<T>) -> V<T> {
    [
        a[1].times(&b[2]).minus(&a[2].times(&b[1])),
        a[2].times(&b[0]).minus(&a[0].times(&b[2])),
        a[0].times(&b[1]).minus(&a[1].times(&b[0])),
    ]
}

fn vzero<T: Num>(a: &V<T>) -> bool {
    a.iter().all(|c| c.sign() == 0)
}

fn vorder<T: Num>(a: &V<T>, b: &V<T>) -> Ordering {
    a[0].order(&b[0]).then_with(|| a[1].order(&b[1])).then_with(|| a[2].order(&b[2]))
}

/// Which side of the plane `normal . (x - base) = 0` every point is on: `Some` with the
/// orientation that makes them all `>= 0`, `None` when points lie on both sides.
fn supporting<T: Num>(points: &[V<T>], base: &V<T>, normal: &V<T>) -> Option<V<T>> {
    let (mut above, mut below) = (false, false);
    for p in points {
        match vdot(normal, &vsub(p, base)).sign() {
            1 => above = true,
            -1 => below = true,
            _ => {}
        }
        if above && below {
            return None;
        }
    }
    let flip = |c: &T| T::zero().minus(c);
    Some(if below {
        [flip(&normal[0]), flip(&normal[1]), flip(&normal[2])]
    } else {
        normal.clone()
    })
}

/// The bound below which every coordinate of a difference point lets the three-dimensional
/// test run in `i128`: differences of two points are below `2^41`, a cross product of two
/// below `2^84`, and a dot product with a difference below `3 2^125 < 2^127`.
const SMALL_BOUND: i128 = 1 << 39;

/// The origin against the hull of `first - second`, by brute force.
///
/// In three dimensions every facet of the hull lies in a plane through three non-collinear
/// difference points with every point on one closed side, and every such plane supports the
/// hull, so the origin is outside exactly when one of them has it strictly on the other side,
/// and on the boundary exactly when none does and one passes through it. A hull of lower
/// dimension has no interior: the origin is in it or not, tested in its plane or on its line.
pub(crate) fn hull(first: &[P], second: &[P]) -> Hull {
    let mut points: Vec<P> =
        first.iter().flat_map(|a| second.iter().map(move |b| sub(a, b))).collect();
    points.sort_by(lexicographic);
    points.dedup_by(|a, b| lexicographic(a, b).is_eq());
    if let Some(answer) = flat_hull(&points) {
        return answer;
    }
    let bound = Big::from_i128(SMALL_BOUND);
    let small = points.iter().flatten().all(|c| c.abs().cmp(&bound).is_lt());
    if small {
        let fit = |c: &Big| {
            let limbs = c.limbs().expect("small");
            let magnitude = i128::from(limbs[0]) | (i128::from(limbs[1]) << 64);
            Small(if c.signum() < 0 { -magnitude } else { magnitude })
        };
        let points: Vec<V<Small>> =
            points.iter().map(|p| [fit(&p[0]), fit(&p[1]), fit(&p[2])]).collect();
        solid_hull(&points)
    } else {
        solid_hull(&points)
    }
}

/// The answer when the difference points lie on one plane, line or point; `None` when they
/// span three dimensions.
fn flat_hull(points: &[P]) -> Option<Hull> {
    type T = Big;
    let origin: V<T> = [T::zero(), T::zero(), T::zero()];
    let p0 = points[0].clone();
    let Some(p1) = points.iter().find(|p| vorder(p, &p0).is_ne()).cloned() else {
        return Some(if vzero(&p0) { Hull::Touching } else { Hull::Separated });
    };
    let d = vsub(&p1, &p0);
    let plane = points.iter().map(|p| vcross(&d, &vsub(p, &p0))).find(|n| !vzero(n));
    let Some(normal) = plane else {
        // One line: the origin must be on it and between the extreme points.
        if !vzero(&vcross(&d, &vsub(&origin, &p0))) {
            return Some(Hull::Separated);
        }
        let t: Vec<T> = points.iter().map(|p| vdot(&d, p)).collect();
        let below = t.iter().any(|x| x.sign() <= 0);
        let above = t.iter().any(|x| x.sign() >= 0);
        return Some(if below && above { Hull::Touching } else { Hull::Separated });
    };
    if points.iter().all(|p| vdot(&normal, &vsub(p, &p0)).sign() == 0) {
        // One plane: the origin must be on it and inside every supporting line.
        if vdot(&normal, &vsub(&origin, &p0)).sign() != 0 {
            return Some(Hull::Separated);
        }
        for (i, pi) in points.iter().enumerate() {
            for pj in &points[i + 1..] {
                let m = vcross(&normal, &vsub(pj, pi));
                if let Some(m) = supporting(points, pi, &m) {
                    if vdot(&m, &vsub(&origin, pi)).sign() < 0 {
                        return Some(Hull::Separated);
                    }
                }
            }
        }
        return Some(Hull::Touching);
    }
    None
}

/// The three-dimensional test.
fn solid_hull<T: Num>(points: &[V<T>]) -> Hull {
    let origin: V<T> = [T::zero(), T::zero(), T::zero()];
    // Each supporting plane is scanned once: the points on it are recorded as a bit mask,
    // and a later triple whose three points are all on a recorded plane is skipped.
    assert!(points.len() <= 64, "at most 64 difference points");
    let mut planes: Vec<u64> = Vec::new();
    let mut boundary = false;
    for (i, pi) in points.iter().enumerate() {
        let d: Vec<V<T>> = points.iter().map(|p| vsub(p, pi)).collect();
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                let triple = (1_u64 << i) | (1_u64 << j) | (1_u64 << k);
                if planes.iter().any(|mask| mask & triple == triple) {
                    continue;
                }
                let n = vcross(&d[j], &d[k]);
                if vzero(&n) {
                    continue;
                }
                let Some(n) = supporting(points, pi, &n) else {
                    continue;
                };
                let on = d.iter().enumerate().filter(|(_, x)| vdot(&n, x).sign() == 0);
                planes.push(on.fold(0, |mask, (l, _)| mask | (1 << l)));
                match vdot(&n, &vsub(&origin, pi)).sign() {
                    -1 => return Hull::Separated,
                    0 => boundary = true,
                    _ => {}
                }
            }
        }
    }
    if boundary {
        Hull::Touching
    } else {
        Hull::Overlapping
    }
}

/// The kind of a kernel result, for comparison with [`hull`].
pub(crate) const fn kind<A>(class: &SatClass<A>) -> Hull {
    match class {
        SatClass::Separated { .. } => Hull::Separated,
        SatClass::Touching { .. } => Hull::Touching,
        SatClass::Overlapping => Hull::Overlapping,
    }
}
