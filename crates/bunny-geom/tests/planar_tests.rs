//! Constructed cases for the 2D primitives and queries (`bunny-planar/1`,
//! `docs/topics/planar-geometry/`).
//!
//! Every value is a raw Q32.32 integer. Each case sits on a boundary or one raw unit either
//! side of it, at small magnitudes and at the ends of the raw range, and each is also checked
//! against the independent big-integer oracle in `planar_oracle`.

// The queries name their points `a` to `d` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod planar_oracle;
mod predicates_oracle;

use bunny_geom::{
    FixedAabb2, FixedCircle2, FixedRay2, FixedSegment2, GeomError, PLANAR_CONTRACT_ID,
};
use bunny_linalg::{FixedUnitVec2, FixedVec2};
use bunny_num::FixedQ32_32;
use predicates_oracle::rounds_to;
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const U: i64 = 1 << 32;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn v(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(raw(x), raw(y))
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

fn reversed(s: FixedSegment2) -> FixedSegment2 {
    FixedSegment2::new(s.end, s.start)
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_001_constructors_refuse_only_their_stated_inputs() {
    assert_eq!(PLANAR_CONTRACT_ID, "bunny-planar/1");

    // Boxes: min above max by one raw unit on either axis is refused; flat and point boxes
    // and the whole raw range are accepted.
    assert_eq!(FixedAabb2::try_new(v(0, 0), v(0, 0)), Ok(rect(0, 0, 0, 0)));
    assert_eq!(FixedAabb2::try_new(v(0, 0), v(0, 9)), Ok(rect(0, 0, 0, 9)));
    assert_eq!(FixedAabb2::try_new(v(MIN, MIN), v(MAX, MAX)), Ok(rect(MIN, MIN, MAX, MAX)));
    assert_eq!(FixedAabb2::try_new(v(1, 0), v(0, 0)), Err(GeomError::InvalidAabbBounds));
    assert_eq!(FixedAabb2::try_new(v(0, 1), v(0, 0)), Err(GeomError::InvalidAabbBounds));
    assert_eq!(FixedAabb2::try_new(v(MAX, 0), v(MAX - 1, 0)), Err(GeomError::InvalidAabbBounds));
    // `new` holds what it is given.
    let inverted = rect(1, 0, 0, 0);
    assert_eq!((inverted.min, inverted.max), (v(1, 0), v(0, 0)));
    assert!(!inverted.is_ordered());
    assert!(rect(0, 0, 0, 0).is_ordered());
    assert!(!rect(0, 1, 0, 0).is_ordered());

    // Circles: radius raw -1 and MIN refused; zero and MAX accepted.
    assert_eq!(FixedCircle2::try_new(v(3, 4), raw(0)), Ok(circle(3, 4, 0)));
    assert_eq!(FixedCircle2::try_new(v(MIN, MAX), raw(MAX)), Ok(circle(MIN, MAX, MAX)));
    assert_eq!(FixedCircle2::try_new(v(3, 4), raw(-1)), Err(GeomError::NegativeSphereRadius));
    assert_eq!(FixedCircle2::try_new(v(3, 4), raw(MIN)), Err(GeomError::NegativeSphereRadius));
    assert_eq!(circle(0, 0, -1).radius, raw(-1));

    // Rays: the direction is normalized by `FixedUnitVec2::new`, as `FixedRay3::try_new` does.
    let ray = FixedRay2::try_new(v(5, 6), v(3 * U, 4 * U)).expect("a nonzero direction");
    assert_eq!(ray.origin, v(5, 6));
    assert_eq!(Some(ray.direction), FixedUnitVec2::new(v(3 * U, 4 * U)));
    assert_eq!(
        FixedRay2::try_new(v(0, 0), v(U, 0)),
        Ok(FixedRay2::new(v(0, 0), FixedUnitVec2::UNIT_X))
    );
    assert_eq!(FixedRay2::try_new(v(0, 0), v(0, 0)), Err(GeomError::InvalidRayDirection));
    assert_eq!(
        FixedRay2::try_new(v(0, 0), v(MAX, MAX)).is_err(),
        FixedUnitVec2::new(v(MAX, MAX)).is_none()
    );

    // Segments keep their endpoint order; a zero-length segment is a value.
    let s = seg((1, 2), (3, 4));
    assert_eq!((s.start, s.end), (v(1, 2), v(3, 4)));
    assert_ne!(s, reversed(s));
    assert_eq!(seg((7, 7), (7, 7)).start, seg((7, 7), (7, 7)).end);
}

/// Asserts `contains_point` against the expectation and the oracle.
fn check_box_point(aabb: FixedAabb2, p: FixedVec2, expected: bool) {
    assert_eq!(aabb.contains_point(p), expected, "{aabb:?} {p:?}");
    assert_eq!(planar_oracle::box_contains(aabb, p), expected, "oracle {aabb:?} {p:?}");
}

/// Asserts `overlaps` in both orders against the expectation and the oracle.
fn check_boxes(a: FixedAabb2, b: FixedAabb2, expected: bool) {
    assert_eq!(a.overlaps(&b), expected, "{a:?} {b:?}");
    assert_eq!(b.overlaps(&a), expected, "{b:?} {a:?}");
    assert_eq!(planar_oracle::boxes_meet(a, b), expected, "oracle {a:?} {b:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_002_box_containment_and_overlap_are_closed() {
    let square = rect(0, 0, 10, 10);
    for (p, inside) in [
        (v(0, 0), true),
        (v(10, 10), true),
        (v(10, 5), true),
        (v(5, 0), true),
        (v(5, 5), true),
        (v(11, 5), false),
        (v(-1, 5), false),
        (v(5, 11), false),
        (v(5, -1), false),
        (v(11, 11), false),
    ] {
        check_box_point(square, p, inside);
    }
    // A flat box is a segment and a point box a point.
    check_box_point(rect(0, 0, 0, 10), v(0, 5), true);
    check_box_point(rect(0, 0, 0, 10), v(1, 5), false);
    check_box_point(rect(4, 4, 4, 4), v(4, 4), true);
    check_box_point(rect(4, 4, 4, 4), v(4, 5), false);
    // The whole raw range.
    let all = rect(MIN, MIN, MAX, MAX);
    check_box_point(all, v(MIN, MAX), true);
    check_box_point(all, v(MAX, MIN), true);
    check_box_point(rect(MIN, MIN, MAX - 1, MAX), v(MAX, 0), false);
    // An inverted box is empty, even for a point between its bounds on the other axis.
    check_box_point(rect(1, 0, 0, 10), v(0, 5), false);
    check_box_point(rect(1, 0, 0, 10), v(1, 5), false);
    check_box_point(rect(0, 10, 10, 0), v(5, 5), false);

    // Touching edges and corners overlap; one raw unit apart does not.
    check_boxes(square, rect(10, 0, 20, 10), true);
    check_boxes(square, rect(11, 0, 20, 10), false);
    check_boxes(square, rect(10, 10, 20, 20), true);
    check_boxes(square, rect(11, 11, 20, 20), false);
    check_boxes(square, rect(10, 11, 20, 20), false);
    check_boxes(square, rect(-5, -5, 0, 0), true);
    check_boxes(square, rect(-5, -5, -1, 0), false);
    check_boxes(square, rect(2, 2, 3, 3), true);
    check_boxes(square, square, true);
    check_boxes(square, rect(5, 5, 5, 5), true);
    // A box crossing another without containing a corner of it.
    check_boxes(square, rect(4, -5, 6, 15), true);
    // At the ends of the range.
    check_boxes(all, rect(MIN, MIN, MIN, MIN), true);
    check_boxes(all, rect(MAX, MAX, MAX, MAX), true);
    check_boxes(rect(MIN, MIN, MAX - 1, MAX - 1), rect(MAX, MAX, MAX, MAX), false);
    check_boxes(rect(MIN, MIN, MAX - 1, MAX), rect(MAX - 1, MAX, MAX, MAX), true);
    // An inverted box overlaps nothing, not even a box its bounds straddle, nor itself.
    let inverted = rect(5, 0, 4, 10);
    check_boxes(inverted, square, false);
    check_boxes(inverted, all, false);
    check_boxes(inverted, inverted, false);
    check_boxes(rect(0, 6, 10, 5), square, false);
}

/// Asserts `closest_point` against the expectation and the oracle.
fn check_box_closest(aabb: FixedAabb2, p: FixedVec2, expected: Option<FixedVec2>) {
    let got = aabb.closest_point(p);
    assert_eq!(got, expected, "{aabb:?} {p:?}");
    let oracle = planar_oracle::box_closest(aabb, p).map(|[x, y]| v(x, y));
    assert_eq!(oracle, expected, "oracle {aabb:?} {p:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_003_box_closest_point_clamps_exactly() {
    let square = rect(0, 0, 10, 10);
    check_box_closest(square, v(5, 5), Some(v(5, 5)));
    check_box_closest(square, v(10, 0), Some(v(10, 0)));
    check_box_closest(square, v(-3, 5), Some(v(0, 5)));
    check_box_closest(square, v(15, 20), Some(v(10, 10)));
    check_box_closest(square, v(10, -1), Some(v(10, 0)));
    check_box_closest(square, v(11, 10), Some(v(10, 10)));
    check_box_closest(square, v(MIN, MAX), Some(v(0, 10)));
    check_box_closest(square, v(MAX, MIN), Some(v(10, 0)));
    check_box_closest(rect(4, 4, 4, 4), v(MIN, 9), Some(v(4, 4)));
    check_box_closest(rect(0, 0, 0, 10), v(3, 4), Some(v(0, 4)));
    check_box_closest(rect(MIN, MIN, MAX, MAX), v(MIN, MAX), Some(v(MIN, MAX)));
    check_box_closest(rect(MAX, MIN, MAX, MIN), v(MIN, MAX), Some(v(MAX, MIN)));
    // An inverted box has no points, so no nearest one.
    check_box_closest(rect(1, 0, 0, 10), v(0, 0), None);
    check_box_closest(rect(0, 1, 10, 0), v(5, 5), None);
}

/// Asserts `contains_point` against the expectation and the oracle.
fn check_circle_point(c: FixedCircle2, p: FixedVec2, expected: bool) {
    assert_eq!(c.contains_point(p), expected, "{c:?} {p:?}");
    assert_eq!(planar_oracle::circle_contains(c, p), expected, "oracle {c:?} {p:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_004_circle_containment_is_closed_and_exact() {
    // Radius 5 raw: `3^2 + 4^2 = 25` is on the circle, one raw unit out is outside.
    let c = circle(0, 0, 5);
    for (p, inside) in [
        (v(3, 4), true),
        (v(4, 3), true),
        (v(-3, -4), true),
        (v(5, 0), true),
        (v(0, -5), true),
        (v(3, 3), true),
        (v(0, 0), true),
        (v(3, 5), false),
        (v(4, 4), false),
        (v(5, 1), false),
        (v(6, 0), false),
        (v(0, -6), false),
    ] {
        check_circle_point(c, p, inside);
    }
    // Moved off the origin, so the centre is subtracted.
    check_circle_point(circle(100, -50, 5), v(103, -46), true);
    check_circle_point(circle(100, -50, 5), v(103, -45), false);
    // A zero radius is its centre; a negative one is empty, even at its centre.
    check_circle_point(circle(7, 7, 0), v(7, 7), true);
    check_circle_point(circle(7, 7, 0), v(7, 8), false);
    check_circle_point(circle(7, 7, -1), v(7, 7), false);
    check_circle_point(circle(7, 7, MIN), v(7, 7), false);
    // At the ends of the range: `MAX - MIN = 2^64 - 1` does not fit `i64`, and its square
    // does not fit `i128`.
    check_circle_point(circle(MIN, 0, MAX), v(-1, 0), true);
    check_circle_point(circle(MIN, 0, MAX), v(0, 0), false);
    check_circle_point(circle(MIN, MIN, MAX), v(MAX, MAX), false);
    check_circle_point(circle(MAX, MAX, MAX), v(0, 0), false);
    // `dx = 2^64 - 1` and `dy = 6074001000`: the distance squared is
    // `2^128 + 581896768`, which a 128-bit sum wraps to `581896768 < 2^80 = r^2`.
    check_circle_point(circle(MIN, 0, 1 << 40), v(MAX, 6_074_001_000), false);
    check_circle_point(circle(MIN, 0, MAX), v(MAX, 6_074_001_000), false);
}

/// Asserts the two overlap tests in both orders against the expectation and the oracle.
fn check_circles(a: FixedCircle2, b: FixedCircle2, expected: bool) {
    assert_eq!(a.overlaps_circle(&b), expected, "{a:?} {b:?}");
    assert_eq!(b.overlaps_circle(&a), expected, "{b:?} {a:?}");
    assert_eq!(planar_oracle::circles_meet(a, b), expected, "oracle {a:?} {b:?}");
}

fn check_circle_box(c: FixedCircle2, aabb: FixedAabb2, expected: bool) {
    assert_eq!(c.overlaps_aabb(&aabb), expected, "{c:?} {aabb:?}");
    assert_eq!(planar_oracle::circle_meets_box(c, aabb), expected, "oracle {c:?} {aabb:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_005_circle_overlaps_are_closed_and_exact() {
    // Centres 10 raw apart: radii summing to 10 touch, to 9 do not.
    check_circles(circle(0, 0, 4), circle(6, 8, 6), true);
    check_circles(circle(0, 0, 4), circle(6, 8, 5), false);
    check_circles(circle(0, 0, 4), circle(6, 8, 7), true);
    check_circles(circle(0, 0, 0), circle(6, 8, 10), true);
    check_circles(circle(0, 0, 0), circle(6, 8, 9), false);
    check_circles(circle(2, 2, 0), circle(2, 2, 0), true);
    check_circles(circle(2, 2, 0), circle(2, 3, 0), false);
    check_circles(circle(0, 0, 50), circle(1, 1, 1), true);
    check_circles(circle(0, 0, -1), circle(0, 0, 5), false);
    check_circles(circle(0, 0, 5), circle(0, 0, MIN), false);
    // At the ends of the range: centres `2^64 - 1` apart and radii summing to `2^64 - 2`, then
    // one raw unit closer. `(2^64 - 2)^2` does not fit `i128`.
    check_circles(circle(MIN, 0, MAX), circle(MAX, 0, MAX), false);
    check_circles(circle(MIN, 0, MAX), circle(MAX - 1, 0, MAX), true);
    check_circles(circle(MIN, MIN, MAX), circle(MAX, MAX, MAX), false);

    let square = rect(0, 0, 10, 10);
    // Corner: `(13, 14)` is `(3, 4)` from `(10, 10)`.
    check_circle_box(circle(13, 14, 5), square, true);
    check_circle_box(circle(13, 14, 4), square, false);
    // `(13, 13)` is `sqrt(18)` from the corner, so radius 4 misses and 5 meets.
    check_circle_box(circle(13, 13, 4), square, false);
    check_circle_box(circle(13, 13, 5), square, true);
    // Edges.
    check_circle_box(circle(15, 5, 5), square, true);
    check_circle_box(circle(16, 5, 5), square, false);
    check_circle_box(circle(5, -3, 3), square, true);
    check_circle_box(circle(5, -4, 3), square, false);
    // Inside, on the boundary with a zero radius, and enclosing the box.
    check_circle_box(circle(5, 5, 0), square, true);
    check_circle_box(circle(10, 5, 0), square, true);
    check_circle_box(circle(11, 5, 0), square, false);
    check_circle_box(circle(5, 5, 100), square, true);
    // Empty inputs.
    check_circle_box(circle(5, 5, -1), square, false);
    check_circle_box(circle(5, 5, 100), rect(1, 0, 0, 10), false);
    // At the ends of the range.
    check_circle_box(circle(MIN, MAX, MAX), rect(MAX, MAX, MAX, MAX), false);
    check_circle_box(circle(-1, MAX, MAX), rect(MAX, MAX, MAX, MAX), false);
    check_circle_box(circle(0, MAX, MAX), rect(MAX, MAX, MAX, MAX), true);
    check_circle_box(circle(MIN, MIN, MAX), rect(MAX, MAX, MAX, MAX), false);
    check_circle_box(circle(0, 0, 0), rect(MIN, MIN, MAX, MAX), true);
}

/// Asserts `contains_point` for both endpoint orders against the expectation and the oracle.
fn check_on_segment(s: FixedSegment2, p: FixedVec2, expected: bool) {
    assert_eq!(s.contains_point(p), expected, "{s:?} {p:?}");
    assert_eq!(reversed(s).contains_point(p), expected, "reversed {s:?} {p:?}");
    assert_eq!(planar_oracle::on_segment(s.start, s.end, p), expected, "oracle {s:?} {p:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_006_segment_containment_is_closed_and_exact() {
    let s = seg((0, 0), (4, 2));
    check_on_segment(s, v(0, 0), true);
    check_on_segment(s, v(4, 2), true);
    check_on_segment(s, v(2, 1), true);
    // On the line, past each end.
    check_on_segment(s, v(6, 3), false);
    check_on_segment(s, v(-2, -1), false);
    // One raw unit off the line.
    check_on_segment(s, v(2, 2), false);
    check_on_segment(s, v(2, 0), false);
    check_on_segment(s, v(3, 1), false);
    // Axis-aligned.
    check_on_segment(seg((0, 0), (0, 5)), v(0, 5), true);
    check_on_segment(seg((0, 0), (0, 5)), v(0, 6), false);
    check_on_segment(seg((0, 0), (0, 5)), v(1, 3), false);
    check_on_segment(seg((0, 3), (9, 3)), v(9, 3), true);
    check_on_segment(seg((0, 3), (9, 3)), v(10, 3), false);
    // A zero-length segment is its point.
    check_on_segment(seg((3, 3), (3, 3)), v(3, 3), true);
    check_on_segment(seg((3, 3), (3, 3)), v(3, 4), false);
    check_on_segment(seg((3, 3), (3, 3)), v(4, 4), false);
    // The diagonal of the raw range.
    let diagonal = seg((MIN, MIN), (MAX, MAX));
    check_on_segment(diagonal, v(0, 0), true);
    check_on_segment(diagonal, v(-1, -1), true);
    check_on_segment(diagonal, v(MAX, MAX), true);
    check_on_segment(diagonal, v(0, 1), false);
    check_on_segment(diagonal, v(1, 0), false);
    check_on_segment(diagonal, v(MAX, MAX - 1), false);
    check_on_segment(seg((MIN, MIN), (-1, -1)), v(0, 0), false);
    // `(MIN, MAX)` to `(MAX, MIN)` is the line `x + y = -1`.
    let anti = seg((MIN, MAX), (MAX, MIN));
    check_on_segment(anti, v(0, -1), true);
    check_on_segment(anti, v(0, 0), false);
}

/// Asserts `intersects` for every order of the segments and their endpoints against the
/// expectation and the oracle.
fn check_meet(a: FixedSegment2, b: FixedSegment2, expected: bool) {
    for (x, y) in [(a, b), (reversed(a), b), (a, reversed(b)), (reversed(a), reversed(b))] {
        assert_eq!(x.intersects(&y), expected, "{x:?} {y:?}");
        assert_eq!(y.intersects(&x), expected, "{y:?} {x:?}");
    }
    let oracle = planar_oracle::segments_meet(a.start, a.end, b.start, b.end);
    assert_eq!(oracle, expected, "oracle {a:?} {b:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_007_segment_intersection_is_closed_and_exact() {
    // A proper crossing.
    check_meet(seg((0, 0), (4, 4)), seg((0, 4), (4, 0)), true);
    // An endpoint on the other's interior, and one raw unit short of it.
    check_meet(seg((0, 0), (4, 0)), seg((2, 0), (2, 5)), true);
    check_meet(seg((0, 0), (4, 0)), seg((2, 1), (2, 5)), false);
    check_meet(seg((0, 0), (4, 0)), seg((4, 0), (4, 5)), true);
    check_meet(seg((0, 0), (4, 0)), seg((5, 0), (5, 5)), false);
    // Shared endpoints.
    check_meet(seg((0, 0), (2, 2)), seg((2, 2), (4, 0)), true);
    // Collinear: overlapping, touching at one point, one raw unit apart, and nested.
    check_meet(seg((0, 0), (4, 0)), seg((2, 0), (6, 0)), true);
    check_meet(seg((0, 0), (2, 0)), seg((2, 0), (4, 0)), true);
    check_meet(seg((0, 0), (2, 0)), seg((3, 0), (5, 0)), false);
    check_meet(seg((0, 0), (9, 9)), seg((3, 3), (4, 4)), true);
    check_meet(seg((0, 0), (2, 1)), seg((4, 2), (6, 3)), false);
    // Parallel, one raw unit apart.
    check_meet(seg((0, 0), (4, 0)), seg((0, 1), (4, 1)), false);
    check_meet(seg((0, 0), (4, 2)), seg((0, 1), (4, 3)), false);
    // Lines that cross outside one segment.
    check_meet(seg((0, 0), (4, 4)), seg((2, 1), (4, -1)), false);
    check_meet(seg((0, 0), (4, 4)), seg((2, 1), (1, 2)), true);
    check_meet(seg((0, 0), (4, 4)), seg((5, 0), (6, 0)), false);
    // Zero-length segments.
    check_meet(seg((2, 2), (2, 2)), seg((0, 0), (4, 4)), true);
    check_meet(seg((2, 3), (2, 3)), seg((0, 0), (4, 4)), false);
    check_meet(seg((5, 5), (5, 5)), seg((0, 0), (4, 4)), false);
    check_meet(seg((1, 1), (1, 1)), seg((1, 1), (1, 1)), true);
    check_meet(seg((1, 1), (1, 1)), seg((1, 2), (1, 2)), false);
    // At the ends of the range: the diagonals cross at `(-1/2, -1/2)`, which is past the end
    // of the half that stops at `(-1, 0)` and on the half that stops at `(0, -1)`.
    let diagonal = seg((MIN, MIN), (MAX, MAX));
    check_meet(diagonal, seg((MIN, MAX), (MAX, MIN)), true);
    check_meet(diagonal, seg((MIN, MAX), (-1, 0)), false);
    check_meet(diagonal, seg((MIN, MAX), (0, -1)), true);
    // Collinear along the diagonal, touching and one diagonal raw step apart.
    check_meet(seg((MIN, MIN), (0, 0)), seg((0, 0), (MAX, MAX)), true);
    check_meet(seg((MIN, MIN), (-1, -1)), seg((0, 0), (MAX, MAX)), false);
    // Parallel to the diagonal, one raw unit off it.
    check_meet(diagonal, seg((MIN, MIN + 1), (MAX - 1, MAX)), false);
}

/// Asserts `closest_point` for both endpoint orders against the expectation, and that the
/// expectation is the oracle's exact point rounded once, ties to even.
fn check_closest(s: FixedSegment2, p: FixedVec2, expected: FixedVec2) {
    assert_eq!(s.closest_point(p), Some(expected), "{s:?} {p:?}");
    assert_eq!(reversed(s).closest_point(p), Some(expected), "reversed {s:?} {p:?}");
    let ([nx, ny], den) = planar_oracle::segment_closest_exact(s, p);
    assert!(rounds_to(&nx, &den, Some(expected.x.raw())), "oracle x {s:?} {p:?}");
    assert!(rounds_to(&ny, &den, Some(expected.y.raw())), "oracle y {s:?} {p:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_008_segment_closest_point_rounds_once_to_even() {
    let s = seg((0, 0), (10, 0));
    check_closest(s, v(3, 5), v(3, 0));
    // Parameters at and past each end give the endpoint exactly.
    check_closest(s, v(0, 7), v(0, 0));
    check_closest(s, v(-4, 2), v(0, 0));
    check_closest(s, v(10, -7), v(10, 0));
    check_closest(s, v(14, -1), v(10, 0));
    // Ties: `(1/2, 1/2)` goes to `(0, 0)` and `(3/2, 3/2)` to `(2, 2)`.
    check_closest(seg((0, 0), (1, 1)), v(1, 0), v(0, 0));
    check_closest(seg((1, 1), (2, 2)), v(2, 1), v(2, 2));
    // Along `(3, 1)`, parameter `n / 10`: `n = 4` gives `(6/5, 2/5)`, `n = 5` the tie
    // `(3/2, 1/2)` and `n = 6` gives `(9/5, 3/5)`.
    check_closest(seg((0, 0), (3, 1)), v(1, 1), v(1, 0));
    check_closest(seg((0, 0), (3, 1)), v(2, -1), v(2, 0));
    check_closest(seg((0, 0), (3, 1)), v(2, 0), v(2, 1));
    // The same tie from an odd start: `(5/2, 3/2)` goes to `(2, 2)`. Rounding the offset
    // from the start instead of the coordinate would give `(3, 1)`.
    check_closest(seg((1, 1), (4, 2)), v(3, 0), v(2, 2));
    // A zero-length segment is its point.
    check_closest(seg((5, 5), (5, 5)), v(9, -3), v(5, 5));
    check_closest(seg((MIN, MAX), (MIN, MAX)), v(MAX, MIN), v(MIN, MAX));
    // The diagonal of the raw range: the foot of `(MAX, MIN + k)` is `(k - 1) / 2` on both
    // axes, so `k = 0` is the tie `-1/2` going to 0, `k = 1` is exactly 0, `k = 2` the tie
    // `1/2` going to 0 and `k = 4` the tie `3/2` going to 2.
    let diagonal = seg((MIN, MIN), (MAX, MAX));
    check_closest(diagonal, v(MAX, MIN), v(0, 0));
    check_closest(diagonal, v(MIN, MAX), v(0, 0));
    check_closest(diagonal, v(MAX, MIN + 1), v(0, 0));
    check_closest(diagonal, v(MAX, MIN + 2), v(0, 0));
    check_closest(diagonal, v(MAX, MIN + 4), v(2, 2));
    check_closest(diagonal, v(MAX, MAX), v(MAX, MAX));
    // An offset from the start of `2^64 - 2` raw units does not fit `i64`; the coordinate
    // does.
    check_closest(seg((MIN, 0), (MAX, 0)), v(MAX - 1, 5), v(MAX - 1, 0));
    check_closest(seg((MIN, 0), (MAX, 0)), v(MAX, MIN), v(MAX, 0));
    check_closest(seg((MIN, 0), (MAX, 0)), v(MIN, MAX), v(MIN, 0));
}
