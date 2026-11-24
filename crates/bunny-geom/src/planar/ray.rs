//! Ray queries: a ray against a box, a circle and a segment.
//!
//! A ray is every point `o + t d` with `t >= 0`, where `d` is the raw value of its
//! [`bunny_linalg::FixedUnitVec2`] direction, taken as given: it is unit only to within one
//! raw unit of length, and the queries do not renormalize it. Every hit is decided exactly on
//! raw values. The first hit parameter is that exact `t` in raw units (`2^32 t`), rounded
//! once.

use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;

use super::{FixedAabb2, FixedCircle2, FixedRay2, FixedSegment2};
use crate::wide::{Limbs, Wide};

mod root;

use root::first_root;

/// Where a ray first meets a shape: [`FixedRay2::hit_aabb`], [`FixedRay2::hit_circle`] and
/// [`FixedRay2::hit_segment`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayHit2 {
    /// The ray and the shape share no point.
    Miss,
    /// The least parameter `t >= 0` of a shared point, rounded once to nearest raw unit, ties
    /// to even: within half a raw unit of the exact value. Zero when the origin is in the
    /// shape.
    At(FixedQ32_32),
    /// The ray meets the shape, but its least parameter rounds above `i64::MAX` raw (about
    /// `2^31` direction lengths from the origin).
    Beyond,
}

impl RayHit2 {
    /// Whether the ray meets the shape: `At` or `Beyond`.
    #[must_use]
    pub const fn is_hit(self) -> bool {
        !matches!(self, Self::Miss)
    }
}

/// An exact parameter `num / den` with `den > 0`.
#[derive(Clone, Copy)]
struct Ratio {
    num: Wide,
    den: Wide,
}

impl Ratio {
    /// `num / den` for a nonzero `den` of either sign.
    fn of(num: Wide, den: Wide) -> Self {
        if den.is_negative() {
            Self { num: num.negated(), den: den.negated() }
        } else {
            Self { num, den }
        }
    }

    fn zero() -> Self {
        Self { num: Wide::difference(0, 0), den: Wide::power_of_two(0) }
    }

    /// `self < other`, by cross-multiplication.
    fn less(&self, other: &Self) -> bool {
        self.num.times(other.den).minus(other.num.times(self.den)).is_negative()
    }

    /// `self > 1`.
    fn above_one(&self) -> bool {
        let excess = self.num.minus(self.den);
        !excess.is_negative() && !excess.is_zero()
    }

    /// `2^32 num / den`, rounded once.
    fn rounded(self) -> RayHit2 {
        rounded(self.num.times(Wide::power_of_two(32)), &self.den.magnitude())
    }
}

/// `num / den` rounded once to nearest, ties to even, for `num >= 0`; `Beyond` above `i64`.
fn rounded(num: Wide, den: &Limbs) -> RayHit2 {
    num.round_div(den).map_or(RayHit2::Beyond, |t| RayHit2::At(FixedQ32_32::from_raw(t)))
}

/// The exact raw difference `to - from`, per axis.
fn offset(to: FixedVec2, from: FixedVec2) -> [Wide; 2] {
    [Wide::difference(to.x.raw(), from.x.raw()), Wide::difference(to.y.raw(), from.y.raw())]
}

fn dot(first: [Wide; 2], second: [Wide; 2]) -> Wide {
    let ([ux, uy], [vx, vy]) = (first, second);
    ux.times(vx).plus(uy.times(vy))
}

fn cross(first: [Wide; 2], second: [Wide; 2]) -> Wide {
    let ([ux, uy], [vx, vy]) = (first, second);
    ux.times(vy).minus(uy.times(vx))
}

/// One axis's parameter interval; `None` bounds on an axis the direction does not move along.
struct Slab {
    enter: Option<Ratio>,
    exit: Option<Ratio>,
}

impl Slab {
    /// The interval of `t` with `lo <= o + t d <= hi`, or `None` when it is empty because
    /// `d = 0` and `o` is outside `[lo, hi]`.
    fn of(o: i64, d: i64, lo: i64, hi: i64) -> Option<Self> {
        if d == 0 {
            return (lo <= o && o <= hi).then_some(Self { enter: None, exit: None });
        }
        let (near, far) = if d > 0 { (lo, hi) } else { (hi, lo) };
        let den = Wide::difference(d, 0);
        Some(Self {
            enter: Some(Ratio::of(Wide::difference(near, o), den)),
            exit: Some(Ratio::of(Wide::difference(far, o), den)),
        })
    }
}

fn later(a: Ratio, b: Option<Ratio>) -> Ratio {
    b.filter(|b| a.less(b)).unwrap_or(a)
}

fn earlier(a: Option<Ratio>, b: Option<Ratio>) -> Option<Ratio> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b.less(&a) { b } else { a }),
        (a, None) => a,
        (None, b) => b,
    }
}

/// How a ray meets a disc.
enum CircleContact {
    /// The origin is in the closed disc.
    Origin,
    /// From outside, first at `(b - sqrt(disc)) / a`, with `a = |d|^2`, `b = (c - o) . d` and
    /// `disc = a r^2 - ((c - o) x d)^2`.
    Ahead { a: Wide, b: Wide, disc: Wide },
}

impl FixedRay2 {
    /// Whether the ray meets the closed box: some `o + t d`, `t >= 0`, is in it.
    ///
    /// Exact slab intervals: on each axis with `d != 0` the parameters of the two bounds are
    /// the exact fractions `(bound - o) / d`; an axis with `d = 0` requires `o` within its
    /// bounds. The ray hits when `max(0, entries) <= min(exits)`, compared by
    /// cross-multiplication (products below `2^97`), with no rounding. Grazing an edge or a
    /// corner hits; an origin on or in the box hits. An unordered box is the empty set.
    #[must_use]
    pub fn intersects_aabb(&self, aabb: &FixedAabb2) -> bool {
        self.aabb_entry(aabb).is_some()
    }

    /// Where the ray first meets the closed box: `max(0, entries)` of
    /// [`Self::intersects_aabb`], rounded once ([`RayHit2`]).
    #[must_use]
    pub fn hit_aabb(&self, aabb: &FixedAabb2) -> RayHit2 {
        self.aabb_entry(aabb).map_or(RayHit2::Miss, Ratio::rounded)
    }

    fn aabb_entry(&self, aabb: &FixedAabb2) -> Option<Ratio> {
        if !aabb.is_ordered() {
            return None;
        }
        let (o, d) = (self.origin, self.direction.into_inner());
        let x = Slab::of(o.x.raw(), d.x.raw(), aabb.min.x.raw(), aabb.max.x.raw())?;
        let y = Slab::of(o.y.raw(), d.y.raw(), aabb.min.y.raw(), aabb.max.y.raw())?;
        let enter = later(later(Ratio::zero(), x.enter), y.enter);
        let exit = earlier(x.exit, y.exit);
        exit.is_none_or(|exit| !exit.less(&enter)).then_some(enter)
    }

    /// Whether the ray meets the closed disc.
    ///
    /// With `w = c - o`, `a = |d|^2`, `b = w . d` on raw values: the ray hits when the origin
    /// is in the disc ([`FixedCircle2::contains_point`]), or when `b > 0` and
    /// `a r^2 - (w x d)^2 >= 0`. Exact in 256 bits (`|b| < 2^98`, the discriminant below
    /// `2^196`). A tangent hits. A negative radius is the empty set.
    #[must_use]
    pub fn intersects_circle(&self, circle: &FixedCircle2) -> bool {
        self.circle_contact(circle).is_some()
    }

    /// Where the ray first meets the closed disc ([`RayHit2`]): zero from an origin in it,
    /// otherwise the first root `t = (b - sqrt(a r^2 - (w x d)^2)) / a`.
    ///
    /// The raw parameter `2^32 t = (u - sqrt(V)) / a`, with `u = 2^32 b` and `V = 2^64` times
    /// the discriminant, is rounded once to nearest, ties to even, from `s = floor(sqrt(V))`:
    /// when `V = s^2` the quotient `(u - s) / a` is rounded; otherwise `sqrt(V)` is irrational
    /// and lies in `(s, s + 1/2)` or `(s + 1/2, s + 1)` (`V <= s^2 + s` tells which), so
    /// `2 a (2^32 t)` is strictly between two consecutive integers, no rounding boundary lies
    /// between it and their midpoint, and that midpoint over `2 a` is rounded instead. `V` is
    /// below `2^255` because `|d| <= 2^32 + 2` raw for every `FixedUnitVec2`.
    #[must_use]
    pub fn hit_circle(&self, circle: &FixedCircle2) -> RayHit2 {
        match self.circle_contact(circle) {
            None => RayHit2::Miss,
            Some(CircleContact::Origin) => RayHit2::At(FixedQ32_32::ZERO),
            Some(CircleContact::Ahead { a, b, disc }) => first_root(a, b, disc),
        }
    }

    /// The direction's raw value, exactly.
    fn step(&self) -> [Wide; 2] {
        let direction = self.direction.into_inner();
        [Wide::difference(direction.x.raw(), 0), Wide::difference(direction.y.raw(), 0)]
    }

    fn circle_contact(&self, circle: &FixedCircle2) -> Option<CircleContact> {
        let radius = circle.radius.raw();
        if radius < 0 {
            return None;
        }
        if circle.contains_point(self.origin) {
            return Some(CircleContact::Origin);
        }
        let (step, to_centre) = (self.step(), offset(circle.center, self.origin));
        let ahead = dot(to_centre, step);
        if ahead.is_negative() || ahead.is_zero() {
            return None;
        }
        let (length, radius, side) =
            (dot(step, step), Wide::difference(radius, 0), cross(to_centre, step));
        let disc = length.times(radius).times(radius).minus(side.times(side));
        (!disc.is_negative()).then_some(CircleContact::Ahead { a: length, b: ahead, disc })
    }

    /// Whether the ray meets the closed segment from `c = start` to `e = end`.
    ///
    /// With `q = c - o`, `s = e - c`: when `d x s != 0` the lines cross at
    /// `t = (q x s) / (d x s)` along the ray and `u = (q x d) / (d x s)` along the segment,
    /// and the ray hits when `t >= 0` and `0 <= u <= 1`. When `d x s = 0` (parallel, or a
    /// zero-length segment) the ray misses unless `q x d = 0`, so that the segment is on the
    /// ray's line; it then hits when the segment's farther end along `d` has `t >= 0`. Exact:
    /// every product is below `2^130`. Endpoint contacts and an origin on the segment hit;
    /// endpoint order does not matter.
    #[must_use]
    pub fn intersects_segment(&self, segment: &FixedSegment2) -> bool {
        self.segment_entry(segment).is_some()
    }

    /// Where the ray first meets the closed segment ([`RayHit2`]): `t` of
    /// [`Self::intersects_segment`] when the lines cross, and for a collinear segment
    /// `max(0, (nearer end - o) . d / |d|^2)`, rounded once.
    #[must_use]
    pub fn hit_segment(&self, segment: &FixedSegment2) -> RayHit2 {
        self.segment_entry(segment).map_or(RayHit2::Miss, Ratio::rounded)
    }

    fn segment_entry(&self, segment: &FixedSegment2) -> Option<Ratio> {
        let step = self.step();
        let to_start = offset(segment.start, self.origin);
        let along = offset(segment.end, segment.start);
        let (den, side) = (cross(step, along), cross(to_start, step));
        if den.is_zero() {
            return self.collinear_entry(segment, side);
        }
        let ray_t = Ratio::of(cross(to_start, along), den);
        let segment_u = Ratio::of(side, den);
        let hit = !ray_t.num.is_negative() && !segment_u.num.is_negative();
        (hit && !segment_u.above_one()).then_some(ray_t)
    }

    /// The first parameter on a segment parallel to the ray, `side` being `(c - o) x d`.
    fn collinear_entry(&self, segment: &FixedSegment2, side: Wide) -> Option<Ratio> {
        if !side.is_zero() {
            return None;
        }
        let step = self.step();
        let along =
            |end: FixedVec2| Ratio::of(dot(offset(end, self.origin), step), dot(step, step));
        let (first, second) = (along(segment.start), along(segment.end));
        let (near, far) = if second.less(&first) { (second, first) } else { (first, second) };
        (!far.num.is_negative()).then(|| later(Ratio::zero(), Some(near)))
    }
}
