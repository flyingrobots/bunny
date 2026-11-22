//! A frozen copy of the exact triangle test, kept as the reference oracle for
//! `TraceRay::hit_triangle`.
//!
//! The arithmetic is a verbatim copy of `hit_triangle`, its `crossing` step and
//! `shifted_quotient` as of flyingrobots/bunny#214; only the inputs are read through the
//! public accessors (`TraceRay::origin`, `TraceRay::direction`, `TraceTriangle::vertices`)
//! instead of private fields. Do not "improve" it: its value is that it stays the plain
//! version (it divides on every crossing) while the kernel may be optimized. An exact
//! replacement of the kernel must keep agreeing with it (the Fidelity Rule in
//! `docs/topics/performance/README.md`).
//!
//! Shared by `trace_equivalence_tests.rs` and the benchmark scene's tests in `bunny-perf`.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};

type Wide3 = [i128; 3];

/// Barycentric and distance numerators of a crossing, each over `det > 0`.
pub(crate) struct Crossing {
    pub(crate) u: i128,
    pub(crate) v: i128,
    pub(crate) t: i128,
    pub(crate) det: i128,
    pub(crate) front_face: bool,
}

/// Everything a `TriangleHit` reports: raw distance, facing, and raw barycentric weights.
pub(crate) type ReferenceHit = (i64, bool, (i64, i64));

/// The reference triangle test over distances `t_min..=t_max`.
pub(crate) fn hit_triangle(
    ray: &TraceRay,
    triangle: &TraceTriangle,
    t_min: FixedQ32_32,
    t_max: FixedQ32_32,
) -> Option<ReferenceHit> {
    let Crossing { u, v, t, det, front_face } = crossing(ray, triangle)?;
    let distance = FixedQ32_32::from_raw(shifted_quotient(t, det)?);
    let weight = |numerator| shifted_quotient(numerator, det).unwrap_or(i64::MAX);
    (t_min <= distance && distance <= t_max)
        .then(|| (distance.raw(), front_face, (weight(u), weight(v))))
}

/// The exact integer part of the triangle test, as `TraceRay::crossing` computes it.
pub(crate) fn crossing(ray: &TraceRay, triangle: &TraceTriangle) -> Option<Crossing> {
    let [v0, v1, v2] = triangle.vertices().map(raw3);
    let origin = raw3(ray.origin());
    let direction = wide3(raw3(ray.direction()));
    let (edge1, edge2) = (sub3(v1, v0), sub3(v2, v0));
    let pvec = cross(direction, edge2);
    let (det, sign) = oriented(dot(edge1, pvec))?;
    let tvec = sub3(origin, v0);
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

const fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn wide3(v: [i64; 3]) -> Wide3 {
    v.map(i128::from)
}

fn sub3(lhs: [i64; 3], rhs: [i64; 3]) -> Wide3 {
    let [a0, a1, a2] = lhs;
    let [b0, b1, b2] = rhs;
    wide3([a0 - b0, a1 - b1, a2 - b2])
}

const fn oriented(value: i128) -> Option<(i128, i128)> {
    match value.signum() {
        0 => None,
        sign => Some((value * sign, sign)),
    }
}

const fn cross(lhs: Wide3, rhs: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}

const fn dot(lhs: Wide3, rhs: Wide3) -> i128 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    a0 * b0 + a1 * b1 + a2 * b2
}

/// `floor(numerator * 2^32 / denominator)` for a positive denominator, or `None` when the
/// numerator is negative or the quotient does not fit `i64`.
pub(crate) fn shifted_quotient(numerator: i128, denominator: i128) -> Option<i64> {
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
