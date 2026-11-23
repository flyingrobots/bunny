//! Segment, triangle and box classification against a plane: constructed cases.

use bunny_geom::{FixedAabb3, FixedPlane3, FixedSegment3, PlaneSide, PlaneSpan};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT: i64 = 1_i64 << 32;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn bx(min: FixedVec3, max: FixedVec3) -> FixedAabb3 {
    FixedAabb3::new(min, max)
}

/// The plane `z = 1`.
fn plane_z1() -> FixedPlane3 {
    FixedPlane3::new(pt(0, 0, UNIT), raw(UNIT))
}

/// A box spanning x and y over `0..=1` with z over `lo..=hi` (raw).
fn slab(lo: i64, hi: i64) -> FixedAabb3 {
    bx(pt(0, 0, lo), pt(UNIT, UNIT, hi))
}

/// The eight-corner definition, through the existing point rule.
fn eight_corners(plane: FixedPlane3, b: FixedAabb3) -> Option<PlaneSpan> {
    let mut front = false;
    let mut behind = false;
    for x in [b.min.x, b.max.x] {
        for y in [b.min.y, b.max.y] {
            for z in [b.min.z, b.max.z] {
                match plane.classify(FixedVec3::new(x, y, z))? {
                    PlaneSide::Front => front = true,
                    PlaneSide::Behind => behind = true,
                    PlaneSide::On => {}
                }
            }
        }
    }
    Some(match (front, behind) {
        (false, false) => PlaneSpan::Coplanar,
        (true, false) => PlaneSpan::Front,
        (false, true) => PlaneSpan::Behind,
        (true, true) => PlaneSpan::Straddling,
    })
}

fn rotations(t: [FixedVec3; 3]) -> [[FixedVec3; 3]; 6] {
    let [a, b, c] = t;
    [[a, b, c], [b, c, a], [c, a, b], [a, c, b], [c, b, a], [b, a, c]]
}

fn assert_triangle(plane: FixedPlane3, t: [FixedVec3; 3], want: PlaneSpan) {
    for order in rotations(t) {
        assert_eq!(plane.classify_triangle(order), Some(want), "triangle {order:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_025_triangle_crossing_coplanar_touching_and_outside() {
    let p = plane_z1();
    let (on, above, below) = (UNIT, UNIT + 1, UNIT - 1);
    // Crossing: one vertex each side, with and without a vertex on the plane.
    assert_triangle(
        p,
        [pt(0, 0, below), pt(UNIT, 0, above), pt(0, UNIT, above)],
        PlaneSpan::Straddling,
    );
    assert_triangle(
        p,
        [pt(0, 0, below), pt(UNIT, 0, above), pt(0, UNIT, on)],
        PlaneSpan::Straddling,
    );
    // Wholly on one side, the nearest vertex one raw unit away.
    assert_triangle(
        p,
        [pt(0, 0, above), pt(UNIT, 0, 3 * UNIT), pt(0, UNIT, 2 * UNIT)],
        PlaneSpan::Front,
    );
    assert_triangle(p, [pt(0, 0, below), pt(UNIT, 0, 0), pt(0, UNIT, -UNIT)], PlaneSpan::Behind);
    // Coplanar.
    assert_triangle(p, [pt(0, 0, on), pt(UNIT, 0, on), pt(0, UNIT, on)], PlaneSpan::Coplanar);
    // Touching at a vertex or an edge: the side of the other vertices.
    assert_triangle(p, [pt(0, 0, on), pt(UNIT, 0, above), pt(0, UNIT, 2 * UNIT)], PlaneSpan::Front);
    assert_triangle(p, [pt(0, 0, on), pt(UNIT, 0, on), pt(0, UNIT, above)], PlaneSpan::Front);
    assert_triangle(p, [pt(0, 0, on), pt(UNIT, 0, below), pt(0, UNIT, 0)], PlaneSpan::Behind);
    assert_triangle(p, [pt(0, 0, on), pt(UNIT, 0, on), pt(0, UNIT, below)], PlaneSpan::Behind);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_026_segment_crossing_coplanar_touching_and_outside() {
    let p = plane_z1();
    let cases = [
        (UNIT - 1, UNIT + 1, PlaneSpan::Straddling),
        (0, 2 * UNIT, PlaneSpan::Straddling),
        (UNIT, UNIT, PlaneSpan::Coplanar),
        (UNIT, UNIT + 1, PlaneSpan::Front),
        (UNIT, UNIT - 1, PlaneSpan::Behind),
        (UNIT + 1, 5 * UNIT, PlaneSpan::Front),
        (UNIT - 1, -UNIT, PlaneSpan::Behind),
    ];
    for (a, b, want) in cases {
        for s in [
            FixedSegment3::new(pt(3, 4, a), pt(-9, 2, b)),
            FixedSegment3::new(pt(-9, 2, b), pt(3, 4, a)),
        ] {
            assert_eq!(p.classify_segment(s), Some(want), "segment z {a} to {b}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_027_box_crossing_coplanar_touching_and_outside() {
    let p = plane_z1();
    let cases = [
        (slab(0, 2 * UNIT), PlaneSpan::Straddling),
        (slab(UNIT - 1, UNIT + 1), PlaneSpan::Straddling),
        (slab(UNIT, UNIT), PlaneSpan::Coplanar),
        (slab(UNIT, 2 * UNIT), PlaneSpan::Front),
        (slab(UNIT + 1, 2 * UNIT), PlaneSpan::Front),
        (slab(0, UNIT), PlaneSpan::Behind),
        (slab(0, UNIT - 1), PlaneSpan::Behind),
    ];
    for (b, want) in cases {
        assert_eq!(p.classify_aabb(b), Some(want), "box {b:?}");
        assert_eq!(eight_corners(p, b), Some(want), "brute force {b:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_028_box_against_oblique_and_negative_normals() {
    // x + y + z = 3: the box corner (1, 1, 1) lies on the plane.
    let diag = FixedPlane3::new(pt(UNIT, UNIT, UNIT), raw(3 * UNIT));
    let unit_box = bx(pt(0, 0, 0), pt(UNIT, UNIT, UNIT));
    assert_eq!(diag.classify_aabb(unit_box), Some(PlaneSpan::Behind));
    assert_eq!(
        diag.classify_aabb(bx(pt(UNIT, UNIT, UNIT), pt(2 * UNIT, 2 * UNIT, 2 * UNIT))),
        Some(PlaneSpan::Front)
    );
    assert_eq!(
        diag.classify_aabb(bx(pt(0, 0, 0), pt(UNIT + 1, UNIT, UNIT))),
        Some(PlaneSpan::Straddling)
    );
    assert_eq!(
        diag.classify_aabb(bx(pt(0, 0, 0), pt(UNIT - 1, UNIT, UNIT))),
        Some(PlaneSpan::Behind)
    );
    assert_eq!(
        diag.classify_aabb(bx(pt(UNIT - 1, UNIT, UNIT), pt(2 * UNIT, 2 * UNIT, 2 * UNIT))),
        Some(PlaneSpan::Straddling)
    );
    // -x + y = 0: the most-along-the-normal corner has the SMALLER x.
    let anti = FixedPlane3::new(pt(-UNIT, UNIT, 0), raw(0));
    let right = bx(pt(UNIT, 0, -5), pt(2 * UNIT, UNIT, 5));
    assert_eq!(anti.classify_aabb(right), Some(PlaneSpan::Behind));
    let left = bx(pt(0, UNIT, -5), pt(UNIT, 2 * UNIT, 5));
    assert_eq!(anti.classify_aabb(left), Some(PlaneSpan::Front));
    let overlap = bx(pt(UNIT - 1, 0, 0), pt(2 * UNIT, UNIT, 0));
    assert_eq!(anti.classify_aabb(overlap), Some(PlaneSpan::Straddling));
    for b in [unit_box, right, left, overlap] {
        assert_eq!(diag.classify_aabb(b), eight_corners(diag, b));
        assert_eq!(anti.classify_aabb(b), eight_corners(anti, b));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_029_degenerate_triangles_are_classified_by_their_vertices() {
    let p = plane_z1();
    // Collinear vertices crossing the plane.
    let line = [pt(0, 0, 0), pt(UNIT, 0, UNIT), pt(2 * UNIT, 0, 2 * UNIT)];
    assert_triangle(p, line, PlaneSpan::Straddling);
    // Collinear in the plane.
    assert_triangle(p, [pt(0, 0, UNIT), pt(1, 0, UNIT), pt(2, 0, UNIT)], PlaneSpan::Coplanar);
    // Three coincident vertices: the point rule, one raw unit either side and on.
    for (z, want) in
        [(UNIT + 1, PlaneSpan::Front), (UNIT, PlaneSpan::Coplanar), (UNIT - 1, PlaneSpan::Behind)]
    {
        let q = pt(7, -7, z);
        assert_eq!(p.classify_triangle([q, q, q]), Some(want));
        assert_eq!(p.classify_segment(FixedSegment3::new(q, q)), Some(want));
    }
    // Two coincident vertices on the plane, the third behind: touching from behind.
    let q = pt(1, 1, UNIT);
    assert_triangle(p, [q, q, pt(0, 0, UNIT - 1)], PlaneSpan::Behind);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_030_inverted_box_is_the_box_with_the_same_corners() {
    let p = plane_z1();
    let anti = FixedPlane3::new(pt(-UNIT, UNIT, -UNIT), raw(0));
    let inverted = [
        bx(pt(0, 0, 2 * UNIT), pt(UNIT, UNIT, 0)),
        bx(pt(0, 0, 3 * UNIT), pt(UNIT, UNIT, UNIT)),
        bx(pt(0, 0, UNIT - 1), pt(UNIT, UNIT, -UNIT)),
        bx(pt(2 * UNIT, 0, UNIT), pt(UNIT, UNIT, UNIT)),
        bx(pt(2 * UNIT, 3 * UNIT, 5), pt(-UNIT, -UNIT, -5)),
    ];
    let want = [PlaneSpan::Straddling, PlaneSpan::Front, PlaneSpan::Behind, PlaneSpan::Coplanar];
    for (b, w) in inverted.iter().zip(want) {
        assert_eq!(p.classify_aabb(*b), Some(w), "box {b:?}");
    }
    for b in inverted {
        let swapped = bx(b.max, b.min);
        for plane in [p, anti] {
            assert_eq!(plane.classify_aabb(b), eight_corners(plane, b), "box {b:?}");
            assert_eq!(plane.classify_aabb(b), plane.classify_aabb(swapped), "box {b:?}");
        }
    }
}

const fn span_of_point(side: PlaneSide) -> PlaneSpan {
    match side {
        PlaneSide::Front => PlaneSpan::Front,
        PlaneSide::Behind => PlaneSpan::Behind,
        PlaneSide::On => PlaneSpan::Coplanar,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_031_zero_extent_box_follows_the_point_rule() {
    let planes = [plane_z1(), FixedPlane3::new(pt(3, -UNIT, 2 * UNIT + 1), raw(-17))];
    let points = [
        pt(5, 7, UNIT),
        pt(5, 7, UNIT + 1),
        pt(5, 7, UNIT - 1),
        pt(-UNIT, 9 * UNIT, 4 * UNIT),
        pt(0, 0, 0),
        pt(0, 17, 0),
    ];
    for plane in planes {
        for q in points {
            let want = plane.classify(q).map(span_of_point);
            assert_eq!(plane.classify_aabb(bx(q, q)), want, "point {q:?}");
            assert_eq!(plane.classify_triangle([q, q, q]), want, "point {q:?}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_032_zero_normal_plane_uses_the_sign_of_the_offset() {
    let (min, max) = (i64::MIN, i64::MAX);
    let everything = bx(pt(min, min, min), pt(max, max, max));
    let tri = [pt(min, 0, max), pt(max, max, min), pt(0, 0, 0)];
    let seg = FixedSegment3::new(pt(min, min, min), pt(max, max, max));
    let cases = [(-1, PlaneSpan::Front), (0, PlaneSpan::Coplanar), (1, PlaneSpan::Behind)];
    for (offset, want) in cases {
        let flat = FixedPlane3::new(pt(0, 0, 0), raw(offset));
        assert_eq!(flat.classify_aabb(everything), Some(want));
        assert_eq!(flat.classify_triangle(tri), Some(want));
        assert_eq!(flat.classify_segment(seg), Some(want));
    }
    // The distance is `0 - offset`, which overflows only for the offset raw i64::MIN.
    let edge = FixedPlane3::new(pt(0, 0, 0), raw(min));
    assert_eq!(edge.classify(pt(0, 0, 0)), None);
    assert_eq!(edge.classify_aabb(everything), None);
    assert_eq!(edge.classify_triangle(tri), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_033_overflow_returns_none_as_the_point_rule_does() {
    let max = i64::MAX;
    let huge = FixedPlane3::new(pt(max, 0, 0), raw(0));
    // Only the corners at x = MAX overflow; the eight-corner definition is None too.
    let b = bx(pt(0, 0, 0), pt(max, 1, 1));
    assert_eq!(huge.classify(pt(max, 0, 0)), None);
    assert_eq!(huge.classify_aabb(b), None);
    assert_eq!(eight_corners(huge, b), None);
    assert_eq!(huge.classify_triangle([pt(0, 0, 0), pt(1, 0, 0), pt(max, 0, 0)]), None);
    assert_eq!(huge.classify_segment(FixedSegment3::new(pt(max, 0, 0), pt(0, 0, 0))), None);
    // Every product fits, but the partial sum x + y does not, at one corner only.
    let sum = FixedPlane3::new(pt(UNIT, UNIT, 0), raw(0));
    let b = bx(pt(0, 0, 0), pt(max, 1, 0));
    assert_eq!(sum.classify(pt(max, 1, 0)), None);
    assert_eq!(sum.classify_aabb(b), None);
    assert_eq!(eight_corners(sum, b), None);
    // The same box one raw unit narrower fits.
    let b = bx(pt(0, 0, 0), pt(max - 1, 1, 0));
    assert_eq!(sum.classify_aabb(b), Some(PlaneSpan::Front));
    assert_eq!(eight_corners(sum, b), Some(PlaneSpan::Front));
}
