//! Division-free ray kernels for closest-hit ray casting.
//!
//! A [`TraceRay`] carries one reciprocal per direction component, so a box test multiplies
//! where [`crate::ray_intersects_aabb`] divides. The box test is conservative: it never
//! rejects a box that the ray reaches. The triangle test is exact: every intermediate is an
//! integer wide enough to hold its true value, and the one division happens only after the
//! ray is known to hit. See `docs/topics/ray-tracing/README.md`.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Exclusive bound on the magnitude of every raw coordinate in the trace domain (256 units).
///
/// With coordinates below this bound, every product in the exact triangle test fits `i128`.
pub const TRACE_COORD_LIMIT_RAW: i64 = 1 << 40;

/// Inclusive bound on the magnitude of every raw direction component (1.0).
pub const TRACE_DIRECTION_LIMIT_RAW: i64 = 1 << 32;

/// Reciprocals are `floor(2^63 / |d|)`.
const RECIPROCAL_BITS: u32 = 63;
/// `(n * reciprocal) >> 31` is `n * 2^32 / |d|`, a raw Q32.32 distance.
const RECIPROCAL_SHIFT: u32 = RECIPROCAL_BITS - FixedQ32_32::FRACTIONAL_BITS;

type Wide3 = [i128; 3];

/// A ray prepared for repeated box and triangle tests.
///
/// The direction need not have unit length. Distances are multiples of the direction: the
/// point at distance `t` is `origin + direction * t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceRay {
    origin: [i64; 3],
    direction: [i64; 3],
    /// `floor(2^63 / |direction|)` per axis; zero where the direction component is zero.
    reciprocal: [u64; 3],
}

impl TraceRay {
    /// Prepares a ray.
    ///
    /// Returns `None` when an origin coordinate is outside the trace domain
    /// ([`TRACE_COORD_LIMIT_RAW`]), a direction component exceeds 1.0 in magnitude
    /// ([`TRACE_DIRECTION_LIMIT_RAW`]), or the direction is zero.
    #[must_use]
    pub fn new(origin: FixedVec3, direction: FixedVec3) -> Option<Self> {
        let origin = raw3(origin);
        let direction = raw3(direction);
        let valid = origin.iter().all(|&c| in_trace_domain(c))
            && direction.iter().all(|&c| in_direction_domain(c))
            && direction != [0; 3];
        valid.then(|| Self { origin, direction, reciprocal: direction.map(reciprocal) })
    }

    /// Prepares a ray from a unit-direction ray.
    ///
    /// Returns `None` when the origin is outside the trace domain.
    #[must_use]
    pub fn from_ray(ray: &FixedRay3) -> Option<Self> {
        Self::new(ray.origin, ray.direction.into_inner())
    }

    /// The ray origin.
    #[must_use]
    pub const fn origin(&self) -> FixedVec3 {
        vec3(self.origin)
    }

    /// The ray direction as given, not normalized.
    #[must_use]
    pub const fn direction(&self) -> FixedVec3 {
        vec3(self.direction)
    }

    /// Conservative ray–box test over distances `0..=t_max`.
    ///
    /// Returns a lower bound on the distance at which the ray enters `aabb`, or `None` when
    /// the ray certainly does not reach the box within `t_max`. It never returns `None` for a
    /// box the ray reaches; it may return a bound for a box the ray narrowly misses.
    #[must_use]
    #[inline]
    // `&` on the flags keeps the test free of data-dependent branches (ledger entry 7).
    #[allow(clippy::needless_bitwise_bool)]
    pub fn box_entry(&self, aabb: &FixedAabb3, t_max: FixedQ32_32) -> Option<FixedQ32_32> {
        let lines = map2(self.origin, self.direction, pair);
        let bounds = map2(raw3(aabb.min), raw3(aabb.max), pair);
        // Straight-line integer work with one validity flag: no axis returns early, so the
        // only branch is the caller's on the result.
        let [x, y, z] = map2(lines, bounds, offsets);
        let planes = map2([x, y, z], self.reciprocal, pair);
        let [lx, ly, lz] = planes.map(|((near, _, _), reciprocal)| scaled(near, reciprocal));
        let enter = lx.max(ly).max(lz);
        let [ux, uy, uz] = map2(planes, self.direction, |plane, d| before_exit(plane, d, enter));
        let limit = t_max.raw();
        let valid = x.2 & y.2 & z.2 & (limit >= 0) & (enter <= limit.cast_unsigned());
        // `enter <= limit <= i64::MAX` when valid, so it converts exactly.
        (valid & ux & uy & uz).then(|| FixedQ32_32::from_raw(enter.cast_signed()))
    }

    /// Exact ray–triangle test over distances `t_min..=t_max`.
    ///
    /// The triangle is two-sided and its edges and corners belong to it. The returned
    /// distance is the floor of the true distance in raw units.
    #[must_use]
    #[inline]
    pub fn hit_triangle(
        &self,
        triangle: &TraceTriangle,
        t_min: FixedQ32_32,
        t_max: FixedQ32_32,
    ) -> Option<TriangleHit> {
        let Crossing { u, v, t, det, front_face } = self.crossing(triangle)?;
        let distance = FixedQ32_32::from_raw(shifted_quotient(t, det)?);
        (t_min <= distance && distance <= t_max).then_some(TriangleHit {
            distance,
            front_face,
            u,
            v,
            det,
        })
    }

    /// The exact integer part of the triangle test: whether the ray's line crosses the
    /// triangle, edges included, and the numerators of the hit over their common `det > 0`.
    #[inline]
    fn crossing(&self, triangle: &TraceTriangle) -> Option<Crossing> {
        let direction = wide3(self.direction);
        let (edge1, edge2) = (wide3(triangle.edge1), wide3(triangle.edge2));
        let pvec = cross(direction, edge2);
        let (det, sign) = oriented(dot(edge1, pvec))?;
        let tvec = sub3(self.origin, triangle.v0);
        let u = dot(tvec, pvec) * sign;
        if !(0..=det).contains(&u) {
            return None;
        }
        let qvec = cross(tvec, edge1);
        let v = dot(direction, qvec) * sign;
        if v < 0 || u + v > det {
            return None;
        }
        let t = dot(edge2, qvec) * sign;
        Some(Crossing { u, v, t, det, front_face: sign > 0 })
    }
}

/// Barycentric and distance numerators of a crossing, each over `det`.
struct Crossing {
    u: i128,
    v: i128,
    t: i128,
    det: i128,
    front_face: bool,
}

/// A triangle prepared for [`TraceRay::hit_triangle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceTriangle {
    v0: [i64; 3],
    edge1: [i64; 3],
    edge2: [i64; 3],
}

impl TraceTriangle {
    /// Prepares a triangle.
    ///
    /// Returns `None` when a vertex coordinate is outside the trace domain
    /// ([`TRACE_COORD_LIMIT_RAW`]).
    #[must_use]
    pub fn new(v0: FixedVec3, v1: FixedVec3, v2: FixedVec3) -> Option<Self> {
        let (v0, v1, v2) = (raw3(v0), raw3(v1), raw3(v2));
        let valid = [v0, v1, v2].iter().flatten().all(|&c| in_trace_domain(c));
        valid.then(|| Self { v0, edge1: narrow_sub3(v1, v0), edge2: narrow_sub3(v2, v0) })
    }

    /// The three vertices.
    #[must_use]
    pub fn vertices(&self) -> [FixedVec3; 3] {
        // Each sum is a vertex given to `new`, so it fits.
        let add = |edge| map2(self.v0, edge, |origin, offset| origin + offset);
        [vec3(self.v0), vec3(add(self.edge1)), vec3(add(self.edge2))]
    }

    /// The smallest box that contains the triangle.
    #[must_use]
    pub fn bounds(&self) -> FixedAabb3 {
        let [a, b, c] = self.vertices().map(raw3);
        let min = map2(map2(a, b, i64::min), c, i64::min);
        let max = map2(map2(a, b, i64::max), c, i64::max);
        FixedAabb3::new(vec3(min), vec3(max))
    }
}

/// Where a ray hit a triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangleHit {
    /// Distance along the ray, in multiples of the ray direction, rounded down.
    pub distance: FixedQ32_32,
    /// Whether the ray hit the side from which the vertices appear counter-clockwise.
    pub front_face: bool,
    u: i128,
    v: i128,
    det: i128,
}

impl TriangleHit {
    /// Barycentric weights of the second and third vertices, each rounded down.
    ///
    /// The hit point is `v0 + (v1 - v0) * u + (v2 - v0) * v`.
    #[must_use]
    pub fn barycentrics(&self) -> (FixedQ32_32, FixedQ32_32) {
        let weight = |numerator| {
            FixedQ32_32::from_raw(shifted_quotient(numerator, self.det).unwrap_or(i64::MAX))
        };
        (weight(self.u), weight(self.v))
    }
}

const fn in_trace_domain(raw: i64) -> bool {
    raw.unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs()
}

const fn in_direction_domain(raw: i64) -> bool {
    raw.unsigned_abs() <= TRACE_DIRECTION_LIMIT_RAW.unsigned_abs()
}

const fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

const fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = raw;
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// Applies `f` to matching components of two triples.
#[inline]
fn map2<A, B, C>(lhs: [A; 3], rhs: [B; 3], f: impl Fn(A, B) -> C) -> [C; 3] {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [f(a0, b0), f(a1, b1), f(a2, b2)]
}

const fn pair<A, B>(lhs: A, rhs: B) -> (A, B) {
    (lhs, rhs)
}

/// The raw coordinate distances `(near, far)` from the origin to one axis slab's planes, in
/// the direction of travel, from the ray's `(origin, direction)` on that axis and the slab's
/// `(min, max)`; a near plane behind the origin counts as zero. The flag is false when the
/// slab rules the box out, and the distances are then meaningless.
#[inline]
// `&` keeps the flag free of a data-dependent branch, as in `box_entry`.
#[allow(clippy::needless_bitwise_bool)]
const fn offsets((origin, direction): (i64, i64), (min, max): (i64, i64)) -> (u64, u64, bool) {
    // Against a negative direction, `!` (which reverses the order of `i64` and keeps
    // `!a - !b == b - a`) turns the slab into one crossed forwards. A zero component is
    // treated as forwards too.
    let flip = direction >> 63;
    let (near, far) = if direction < 0 { (max, min) } else { (min, max) };
    let (origin, near, far) = (origin ^ flip, near ^ flip, far ^ flip);
    // A far plane behind the origin rejects the box; so does a near plane ahead of it when
    // the ray runs parallel to the slab.
    let ahead = near > origin;
    let valid = (far >= origin) & !((direction == 0) & ahead);
    // A difference `a - b` with `a >= b` lies in `0..2^64`, so the wrapping `i64` difference
    // read as `u64` is exact.
    let far = far.wrapping_sub(origin).cast_unsigned();
    let near = if ahead { near.wrapping_sub(origin).cast_unsigned() } else { 0 };
    (near, far, valid)
}

/// Whether `enter <= upper`, where `upper` is the slab upper bound
/// `floor(far * (r + 1) / 2^31) + 1` (saturating; `u64::MAX` on a parallel axis) for the
/// slab's offsets, its reciprocal `r` and the direction component.
///
/// For `enter <= 2^63`, `enter <= floor(F / 2^31) + 1` holds exactly when
/// `(enter - 1) * 2^31 <= F` (taking `enter - 1` as 0 for `enter == 0`), and saturation
/// never changes it because `(enter - 1) * 2^31 < 2^94` while a saturated bound needs
/// `F >= 2^95 - 2^31`. So one 128-bit comparison replaces the shift, the saturations and the
/// minimum. A larger `enter` exceeds every limit, so its answer does not matter.
#[inline]
// `|` keeps the flag free of a data-dependent branch, as in `box_entry`.
#[allow(clippy::needless_bitwise_bool)]
fn before_exit(((_, far, _), reciprocal): ((u64, u64, bool), u64), direction: i64, enter: u64) -> bool {
    let exit = u128::from(far) * u128::from(reciprocal + 1);
    let needed = u128::from(enter.saturating_sub(1)) << RECIPROCAL_SHIFT;
    (direction == 0) | (needed <= exit)
}

/// `floor(2^63 / |d|)`, or zero for a zero component.
const fn reciprocal(direction: i64) -> u64 {
    match direction.unsigned_abs() {
        0 => 0,
        magnitude => (1_u64 << RECIPROCAL_BITS) / magnitude,
    }
}

/// `floor(value * reciprocal / 2^31)`, saturating.
#[inline]
fn scaled(value: u64, reciprocal: u64) -> u64 {
    let product = u128::from(value) * u128::from(reciprocal);
    u64::try_from(product >> RECIPROCAL_SHIFT).unwrap_or(u64::MAX)
}

fn wide3(v: [i64; 3]) -> Wide3 {
    v.map(i128::from)
}

/// Difference of two points in the trace domain; fits `i64`.
fn narrow_sub3(lhs: [i64; 3], rhs: [i64; 3]) -> [i64; 3] {
    map2(lhs, rhs, |a, b| a - b)
}

fn sub3(lhs: [i64; 3], rhs: [i64; 3]) -> Wide3 {
    wide3(narrow_sub3(lhs, rhs))
}

/// `(|value|, sign)` for a nonzero value, `None` for zero (a ray parallel to the plane).
#[inline]
const fn oriented(value: i128) -> Option<(i128, i128)> {
    match value.signum() {
        0 => None,
        sign => Some((value * sign, sign)),
    }
}

#[inline]
const fn cross(lhs: Wide3, rhs: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}

#[inline]
const fn dot(lhs: Wide3, rhs: Wide3) -> i128 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    a0 * b0 + a1 * b1 + a2 * b2
}

/// `floor(numerator * 2^32 / denominator)` for a positive denominator, or `None` when the
/// numerator is negative or the quotient does not fit `i64`.
///
/// The numerator can be too wide to shift at once, so the 32 bits are brought down in steps
/// as large as the denominator's leading zeros allow (long division in a variable base).
fn shifted_quotient(numerator: i128, denominator: i128) -> Option<i64> {
    let numerator = u128::try_from(numerator).ok()?;
    let denominator = u128::try_from(denominator).ok().filter(|&d| d != 0)?;
    let mut quotient = numerator / denominator;
    let mut remainder = numerator % denominator;
    let mut pending = FixedQ32_32::FRACTIONAL_BITS;
    while pending > 0 {
        let step = pending.min(denominator.leading_zeros());
        if step == 0 || quotient.leading_zeros() < step {
            return None;
        }
        remainder <<= step;
        quotient = (quotient << step) | (remainder / denominator);
        remainder %= denominator;
        pending -= step;
    }
    i64::try_from(quotient).ok()
}
