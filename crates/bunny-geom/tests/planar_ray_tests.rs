//! Constructed cases for the 2D ray queries (`bunny-planar/1`, `docs/topics/planar-geometry/`).
//!
//! Every value is a raw Q32.32 integer. Each case sits on a contact or one raw unit either
//! side of it, at small magnitudes and at the ends of the raw range, and each is also checked
//! against the independent big-integer oracle in `planar_cast_oracle`: the hit decision, and
//! the first parameter as the exact value rounded once, ties to even.

// The queries name their points `a` to `e`, `o` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod planar_cast_oracle;
mod planar_oracle;
mod predicates_oracle;

use bunny_geom::{FixedAabb2, FixedCircle2, FixedRay2, FixedSegment2, RayHit2};
use bunny_linalg::{FixedUnitVec2, FixedVec2};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const U: i64 = 1 << 32;
const X: FixedUnitVec2 = FixedUnitVec2::UNIT_X;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn v(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(raw(x), raw(y))
}

/// A direction given in raw units, which must already be unit to within one raw unit.
fn dir(x: i64, y: i64) -> FixedUnitVec2 {
    FixedUnitVec2::try_from_unit(v(x, y)).expect("a unit direction")
}

fn ray(x: i64, y: i64, direction: FixedUnitVec2) -> FixedRay2 {
    FixedRay2::new(v(x, y), direction)
}

fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> FixedAabb2 {
    FixedAabb2::new(v(x0, y0), v(x1, y1))
}

fn circle(x: i64, y: i64, r: i64) -> FixedCircle2 {
    FixedCircle2::new(v(x, y), raw(r))
}

fn seg(a: (i64, i64), b: (i64, i64)) -> FixedSegment2 {
    FixedSegment2::new(v(a.0, a.1), v(b.0, b.1))
}

fn at(t: i64) -> RayHit2 {
    RayHit2::At(raw(t))
}

const MISS: RayHit2 = RayHit2::Miss;
const BEYOND: RayHit2 = RayHit2::Beyond;

/// Asserts both box queries against the expectation and the oracle.
fn check_box(r: FixedRay2, aabb: FixedAabb2, expected: RayHit2) {
    assert_eq!(r.hit_aabb(&aabb), expected, "{r:?} {aabb:?}");
    assert_eq!(r.intersects_aabb(&aabb), expected.is_hit(), "{r:?} {aabb:?}");
    let oracle = planar_cast_oracle::ray_box(r, aabb);
    assert!(planar_cast_oracle::parameter_is(oracle.as_ref(), expected), "oracle {r:?} {aabb:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_013_ray_box_hits_are_closed_and_exact() {
    assert!(at(0).is_hit() && BEYOND.is_hit() && !MISS.is_hit());
    let o = ray(0, 0, X);
    check_box(o, rect(5, -1, 9, 1), at(5));
    // Grazing the bottom and top edges, and one raw unit past them.
    check_box(o, rect(5, 0, 9, 3), at(5));
    check_box(o, rect(5, 1, 9, 3), MISS);
    check_box(o, rect(5, -3, 9, 0), at(5));
    check_box(o, rect(5, -3, 9, -1), MISS);
    // Behind the origin, the origin on an edge, and the origin inside.
    check_box(o, rect(-9, -1, -5, 1), MISS);
    check_box(o, rect(-9, -1, -1, 1), MISS);
    check_box(o, rect(-9, -1, 0, 1), at(0));
    check_box(o, rect(-1, -1, 1, 1), at(0));
    // The other axis directions.
    check_box(ray(0, 0, FixedUnitVec2::NEG_UNIT_X), rect(-9, -1, -5, 1), at(5));
    check_box(ray(0, 0, FixedUnitVec2::UNIT_Y), rect(-1, 7, 1, 9), at(7));
    check_box(ray(0, 0, FixedUnitVec2::NEG_UNIT_Y), rect(-1, 7, 1, 9), MISS);
    check_box(ray(2, 0, FixedUnitVec2::UNIT_Y), rect(-1, 7, 1, 9), MISS);
    check_box(ray(1, 0, FixedUnitVec2::UNIT_Y), rect(-1, 7, 1, 9), at(7));
    // Flat and point boxes; inverted boxes are empty.
    check_box(o, rect(5, -1, 5, 1), at(5));
    check_box(o, rect(5, 0, 5, 0), at(5));
    check_box(o, rect(5, 1, 5, 1), MISS);
    check_box(o, rect(9, -1, 5, 1), MISS);
    check_box(o, rect(5, 1, 9, -1), MISS);
    check_box(o, rect(0, 0, -1, 0), MISS);
    // The direction `(1, 3 2^-32)` passes below the corner `(1431655765, 1)` raw, since
    // `3 1431655765 = 2^32 - 1`, and above `(1431655766, 1)`. A slab test on rounded
    // parameters would report the first a hit: both its bounds round to 1431655765.
    let shallow = ray(0, 0, dir(U, 3));
    check_box(shallow, rect(1_431_655_665, 1, 1_431_655_765, 5), MISS);
    check_box(shallow, rect(1_431_655_665, 1, 1_431_655_766, 5), at(1_431_655_765));
    // At the ends of the range: the parameter `2^63 - 1` fits and `2^63` does not.
    let far = ray(MIN, 0, X);
    check_box(far, rect(-1, -1, MAX, 1), at(MAX));
    check_box(far, rect(0, -1, MAX, 1), BEYOND);
    check_box(far, rect(MAX, 0, MAX, 0), BEYOND);
    check_box(far, rect(MAX, 1, MAX, MAX), MISS);
    check_box(ray(MAX, 0, X), rect(MAX, 0, MAX, 0), at(0));
    check_box(ray(MAX, 0, X), rect(MIN, 1, MAX, MAX), MISS);
    check_box(ray(MIN, MIN, X), rect(MIN, MIN, MAX, MAX), at(0));
    // Climbing one raw unit per unit, from `(MIN, MIN)` the ray is at `MIN + 2^32 - 2^-32`
    // when it reaches `x = MAX`.
    let climb = ray(MIN, MIN, dir(U, 1));
    check_box(climb, rect(MAX, MIN + U - 1, MAX, MIN + U), BEYOND);
    check_box(climb, rect(MAX, MIN, MAX, MIN + U - 1), MISS);
    check_box(climb, rect(MAX, MIN + U, MAX, MAX), MISS);
}

/// Asserts both circle queries against the expectation and the oracle.
fn check_circle(r: FixedRay2, c: FixedCircle2, expected: RayHit2) {
    assert_eq!(r.hit_circle(&c), expected, "{r:?} {c:?}");
    assert_eq!(r.intersects_circle(&c), expected.is_hit(), "{r:?} {c:?}");
    assert_eq!(planar_cast_oracle::ray_meets_circle(r, c), expected.is_hit(), "oracle {r:?} {c:?}");
    assert!(planar_cast_oracle::circle_hit_is(r, c, expected), "oracle {r:?} {c:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_014_ray_circle_hits_are_closed_and_exact() {
    let o = ray(0, 0, X);
    check_circle(o, circle(10, 0, 3), at(7));
    // `3^2 + 4^2 = 5^2`: a chord, a tangent, and one raw unit short of tangent.
    check_circle(o, circle(10, 3, 5), at(6));
    check_circle(o, circle(10, 5, 5), at(10));
    check_circle(o, circle(10, -5, 5), at(10));
    check_circle(o, circle(10, 5, 4), MISS);
    check_circle(o, circle(10, 6, 5), MISS);
    // Behind, the origin on the circle, and the origin inside.
    check_circle(o, circle(-10, 0, 5), MISS);
    check_circle(o, circle(-10, 0, 10), at(0));
    check_circle(o, circle(-10, 0, 9), MISS);
    check_circle(o, circle(1, 1, 5), at(0));
    check_circle(o, circle(0, 10, 10), at(0));
    check_circle(o, circle(0, 10, 9), MISS);
    // Zero and negative radii.
    check_circle(o, circle(7, 0, 0), at(7));
    check_circle(o, circle(7, 1, 0), MISS);
    check_circle(o, circle(7, 0, -1), MISS);
    check_circle(o, circle(0, 0, MIN), MISS);
    // A shallow direction `(1, 2^-32)`, so `|d|` is `sqrt(2^64 + 1)` raw and every first
    // root here is irrational. The centre `(2^32, 1)` is on the ray; `(2^32, 2)` is
    // `2^32 / |d|`, just under one raw unit, from its line, so radius 1 grazes it.
    let shallow = ray(0, 0, dir(U, 1));
    check_circle(shallow, circle(U, 1, 1), at(4_294_967_295));
    check_circle(shallow, circle(U, 2, 1), at(4_294_967_296));
    check_circle(shallow, circle(U, 2, 0), MISS);
    check_circle(shallow, circle(U, 3, 1), MISS);
    check_circle(shallow, circle(10 * U, 0, U), at(38_654_705_664));
    // At the ends of the range: the parameter `2^63 - 1` fits and `2^63` does not.
    let far = ray(MIN, 0, X);
    check_circle(far, circle(MAX - 1, 0, MAX), at(MAX));
    check_circle(far, circle(MAX, 0, MAX), BEYOND);
    check_circle(far, circle(-1, MAX, MAX), at(MAX));
    check_circle(far, circle(0, MAX, MAX), BEYOND);
    check_circle(far, circle(-1, MAX, MAX - 1), MISS);
    check_circle(ray(MIN, MIN, X), circle(MAX, MAX, MAX), MISS);
    // From just outside the circle of radius `MAX` about `(MAX, 1)`, the first root is about
    // `2^-64` raw units ahead, which rounds to zero.
    check_circle(o, circle(MAX, 0, MAX), at(0));
    check_circle(o, circle(MAX, 1, MAX), at(0));
    check_circle(ray(MIN, 0, X), circle(MAX, 1, MAX), BEYOND);
}

/// Asserts both segment queries for both endpoint orders against the expectation and the
/// oracle.
fn check_segment(r: FixedRay2, s: FixedSegment2, expected: RayHit2) {
    for s in [s, FixedSegment2::new(s.end, s.start)] {
        assert_eq!(r.hit_segment(&s), expected, "{r:?} {s:?}");
        assert_eq!(r.intersects_segment(&s), expected.is_hit(), "{r:?} {s:?}");
        let oracle = planar_cast_oracle::ray_segment(r, s.start, s.end);
        assert!(planar_cast_oracle::parameter_is(oracle.as_ref(), expected), "oracle {r:?} {s:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_015_ray_segment_hits_are_closed_and_exact() {
    let o = ray(0, 0, X);
    // A crossing, an endpoint touching the ray, and one raw unit short.
    check_segment(o, seg((5, -1), (5, 1)), at(5));
    check_segment(o, seg((5, 0), (5, 3)), at(5));
    check_segment(o, seg((5, 1), (5, 3)), MISS);
    check_segment(o, seg((5, -3), (5, -1)), MISS);
    // Behind, and through the origin.
    check_segment(o, seg((-5, -1), (-5, 1)), MISS);
    check_segment(o, seg((0, -1), (0, 1)), at(0));
    check_segment(o, seg((-1, -1), (1, 1)), at(0));
    check_segment(o, seg((-2, -1), (0, 1)), MISS);
    // Parallel: on another line, and collinear ahead, around and behind the origin.
    check_segment(o, seg((1, 1), (9, 1)), MISS);
    check_segment(o, seg((-9, -1), (9, -1)), MISS);
    check_segment(o, seg((3, 0), (9, 0)), at(3));
    check_segment(o, seg((-3, 0), (9, 0)), at(0));
    check_segment(o, seg((-9, 0), (0, 0)), at(0));
    check_segment(o, seg((-9, 0), (-1, 0)), MISS);
    // Zero-length segments.
    check_segment(o, seg((4, 0), (4, 0)), at(4));
    check_segment(o, seg((4, 1), (4, 1)), MISS);
    check_segment(o, seg((-4, 0), (-4, 0)), MISS);
    check_segment(o, seg((0, 0), (0, 0)), at(0));
    // Ties: the crossings at 11/2 and 9/2 raw units round to 6 and 4.
    check_segment(o, seg((5, -1), (6, 1)), at(6));
    check_segment(o, seg((4, -1), (5, 1)), at(4));
    check_segment(o, seg((4, -1), (6, 2)), at(5));
    // A shallow direction: parallel one raw unit off the line, collinear, and crossing.
    let shallow = ray(0, 0, dir(U, 1));
    check_segment(shallow, seg((0, 1), (U, 2)), MISS);
    check_segment(shallow, seg((U, 1), (2 * U, 2)), at(U));
    check_segment(shallow, seg((U, 0), (U, 1)), at(U));
    check_segment(shallow, seg((U, 2), (U, 3)), MISS);
    check_segment(shallow, seg((U + 1, 0), (U + 1, 1)), at(U + 1));
    // At the ends of the range.
    let far = ray(MIN, 0, X);
    check_segment(far, seg((-1, MIN), (-1, MAX)), at(MAX));
    check_segment(far, seg((0, MIN), (0, MAX)), BEYOND);
    check_segment(far, seg((MAX, 0), (MAX, 0)), BEYOND);
    check_segment(far, seg((MIN, 0), (MAX, 0)), at(0));
    check_segment(far, seg((-1, 0), (MAX, 0)), at(MAX));
    check_segment(ray(MIN, MIN, X), seg((MAX, MIN), (MAX, MAX)), BEYOND);
    // The anti-diagonal `x + y = -1` of the raw range.
    let anti = seg((MIN, MAX), (MAX, MIN));
    check_segment(o, anti, MISS);
    check_segment(ray(0, 0, FixedUnitVec2::NEG_UNIT_X), anti, at(1));
    check_segment(ray(0, -1, X), anti, at(0));
    check_segment(ray(1, 0, FixedUnitVec2::NEG_UNIT_Y), anti, at(2));
}
