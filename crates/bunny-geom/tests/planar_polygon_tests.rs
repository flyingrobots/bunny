//! Constructed cases for point in polygon (`bunny-planar/1`, `docs/topics/planar-geometry/`).
//!
//! Every value is a raw Q32.32 integer. Points sit on edges and vertices, one raw unit either
//! side of them, level with vertices (where a crossing count can count twice), at small
//! magnitudes and at the ends of the raw range. Each case is checked for both windings and
//! every starting vertex, and against the independent oracle in `planar_cast_oracle`.

mod planar_cast_oracle;
mod planar_oracle;
mod predicates_oracle;

use bunny_geom::{FixedPolygon2, PolygonLocation};
use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const IN: PolygonLocation = PolygonLocation::Inside;
const ON: PolygonLocation = PolygonLocation::Boundary;
const OUT: PolygonLocation = PolygonLocation::Outside;

fn v(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y))
}

fn points(raw: &[(i64, i64)]) -> Vec<FixedVec2> {
    raw.iter().map(|&(x, y)| v(x, y)).collect()
}

/// The oracle's answer: boundary when on an edge, otherwise inside exactly when the winding
/// number is nonzero.
fn oracle(vertices: &[FixedVec2], p: FixedVec2) -> PolygonLocation {
    match planar_cast_oracle::winding(vertices, p) {
        None => ON,
        Some(0) => OUT,
        Some(_) => IN,
    }
}

/// Asserts the classification of `p` for every rotation of the vertex list and its reverse,
/// against the expectation and the oracle.
fn check(raw: &[(i64, i64)], p: (i64, i64), expected: PolygonLocation) {
    let p = v(p.0, p.1);
    let forward = points(raw);
    let mut backward = forward.clone();
    backward.reverse();
    for list in [forward, backward] {
        for k in 0..list.len().max(1) {
            let mut rotated = list.clone();
            rotated.rotate_left(k.min(list.len()));
            let polygon = FixedPolygon2::new(&rotated);
            assert_eq!(polygon.classify_point(p), expected, "{rotated:?} {p:?}");
            assert_eq!(polygon.contains_point(p), expected != OUT, "{rotated:?} {p:?}");
            assert_eq!(oracle(&rotated, p), expected, "oracle {rotated:?} {p:?}");
        }
    }
}

const SQUARE: [(i64, i64); 4] = [(0, 0), (10, 0), (10, 10), (0, 10)];

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_016_point_in_polygon_is_exact_and_closed() {
    assert_eq!(FixedPolygon2::new(&points(&SQUARE)).vertices, points(&SQUARE).as_slice());
    for (p, expected) in [
        ((5, 5), IN),
        ((1, 1), IN),
        ((9, 9), IN),
        // Edges and vertices, and one raw unit outside each edge.
        ((0, 0), ON),
        ((10, 10), ON),
        ((10, 5), ON),
        ((5, 0), ON),
        ((0, 7), ON),
        ((11, 5), OUT),
        ((-1, 5), OUT),
        ((5, 11), OUT),
        ((5, -1), OUT),
        // Level with the horizontal edges, outside them.
        ((-1, 0), OUT),
        ((11, 10), OUT),
        ((MIN, 10), OUT),
    ] {
        check(&SQUARE, p, expected);
    }

    // A diamond: a horizontal ray through `p` passes through the vertices `(0, 5)` and
    // `(10, 5)`, where a crossing count can count an edge pair twice or not at all.
    let diamond = [(5, 0), (10, 5), (5, 10), (0, 5)];
    for (p, expected) in [
        ((5, 5), IN),
        ((1, 5), IN),
        ((-1, 5), OUT),
        ((11, 5), OUT),
        ((0, 5), ON),
        ((10, 5), ON),
        ((5, 0), ON),
        ((2, 3), ON),
        ((2, 2), OUT),
        ((3, 3), IN),
        ((5, -1), OUT),
    ] {
        check(&diamond, p, expected);
    }

    // A concave polygon with a notch from above, and a spike vertex level with points.
    let notched = [(0, 0), (10, 0), (10, 10), (6, 10), (5, 4), (4, 10), (0, 10)];
    for (p, expected) in [
        ((5, 8), OUT),
        ((5, 4), ON),
        ((5, 3), IN),
        ((2, 8), IN),
        ((8, 8), IN),
        ((5, 10), OUT),
        ((0, 4), ON),
        ((-1, 4), OUT),
        ((1, 4), IN),
    ] {
        check(&notched, p, expected);
    }

    // At the ends of the range: the whole raw square, and the corner triangle whose
    // hypotenuse is `x + y = -1`.
    let all = [(MIN, MIN), (MAX, MIN), (MAX, MAX), (MIN, MAX)];
    check(&all, (0, 0), IN);
    check(&all, (MAX, 0), ON);
    check(&all, (MIN, MAX), ON);
    check(&all, (MAX - 1, MAX - 1), IN);
    let corner = [(MIN, MIN), (MAX, MIN), (MIN, MAX)];
    check(&corner, (-1, -1), IN);
    check(&corner, (0, -1), ON);
    check(&corner, (-1, 0), ON);
    check(&corner, (0, 0), OUT);
    check(&corner, (MAX, MAX), OUT);
    check(&corner, (MIN + 1, MIN + 1), IN);
    check(&corner, (MIN, 0), ON);
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_017_polygon_degenerate_and_self_intersecting_inputs() {
    // No vertices: the empty set.
    check(&[], (0, 0), OUT);
    // One vertex: the point.
    check(&[(3, 3)], (3, 3), ON);
    check(&[(3, 3)], (3, 4), OUT);
    // Two vertices: the closed segment, with no inside.
    check(&[(0, 0), (4, 2)], (2, 1), ON);
    check(&[(0, 0), (4, 2)], (4, 2), ON);
    check(&[(0, 0), (4, 2)], (2, 2), OUT);
    check(&[(0, 0), (4, 2)], (6, 3), OUT);
    // Repeated consecutive vertices and a collinear run change nothing.
    let repeated = [(0, 0), (0, 0), (10, 0), (10, 10), (10, 10), (10, 10), (0, 10)];
    let run = [(0, 0), (3, 0), (7, 0), (10, 0), (10, 10), (0, 10), (0, 5)];
    for polygon in [&SQUARE[..], &repeated[..], &run[..]] {
        check(polygon, (5, 5), IN);
        check(polygon, (3, 0), ON);
        check(polygon, (0, 5), ON);
        check(polygon, (5, -1), OUT);
        check(polygon, (-1, 5), OUT);
        check(polygon, (-1, 0), OUT);
    }
    // Every vertex the same point.
    check(&[(2, 2), (2, 2), (2, 2)], (2, 2), ON);
    check(&[(2, 2), (2, 2), (2, 2)], (2, 3), OUT);
    // Zero area: all on one line, doubling back. Its edges are its boundary; it has no
    // inside.
    let flat = [(0, 0), (10, 0), (5, 0)];
    check(&flat, (7, 0), ON);
    check(&flat, (5, 1), OUT);
    check(&flat, (11, 0), OUT);
    check(&[(0, 0), (4, 4), (8, 8), (2, 2)], (3, 3), ON);
    check(&[(0, 0), (4, 4), (8, 8), (2, 2)], (3, 2), OUT);

    // Self-intersecting input is not detected. A bow tie: two lobes of opposite winding, both
    // inside (nonzero winding), and the crossing point on the boundary.
    let bow = [(0, 0), (10, 10), (10, 0), (0, 10)];
    check(&bow, (2, 5), IN);
    check(&bow, (8, 5), IN);
    check(&bow, (5, 5), ON);
    check(&bow, (5, 2), OUT);
    check(&bow, (5, 8), OUT);
    // A pentagram: the centre has winding number 2, so it is inside by the nonzero rule,
    // though the even-odd rule would put it outside.
    let star = [(0, 30), (18, -24), (-29, 9), (29, 9), (-18, -24)];
    check(&star, (0, 0), IN);
    let centre = points(&star);
    assert_eq!(planar_cast_oracle::winding(&centre, v(0, 0)).map(i64::abs), Some(2));
    assert_eq!(planar_cast_oracle::crossings_odd(&centre, v(0, 0)), Some(false));
    // A point tip.
    check(&star, (0, 20), IN);
    check(&star, (0, 31), OUT);
    // A square traced twice: winding number 2 inside.
    let twice = [(0, 0), (10, 0), (10, 10), (0, 10), (0, 0), (10, 0), (10, 10), (0, 10)];
    check(&twice, (5, 5), IN);
    check(&twice, (5, 11), OUT);
}
