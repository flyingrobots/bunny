//! Constructed cases of the separating-axis kernels (`bunny-sat/1`,
//! `docs/topics/convex-collision/`).
//!
//! Every case sits on a contact or one raw unit either side of it, and every case is also
//! checked against both oracles of `sat_oracle`: the corner projections give the expected
//! result with its axis, and the brute-force Minkowski hull gives the class.

mod predicates_oracle;
mod sat_oracle;

use bunny_geom::{
    sat_obb_obb, sat_triangle_aabb, FixedAabb3, FixedObb3, FixedTriangle3, ObbAxis, SatClass,
    TriangleAabbAxis, SAT_CONTRACT_ID,
};
use bunny_linalg::{FixedMat3, FixedUnitVec3, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};
use sat_oracle::{
    aabb_corners, expected, hull, kind, obb_axes, obb_corners, triangle_aabb_axes, triangle_points,
};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const EIGHTH_TURN: u64 = 1 << 61;

fn pt(v: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(v[0]),
        FixedQ32_32::from_raw(v[1]),
        FixedQ32_32::from_raw(v[2]),
    )
}

fn obb(center: [i64; 3], axes: FixedMat3, half: [i64; 3]) -> FixedObb3 {
    FixedObb3::try_new(pt(center), axes, pt(half)).expect("a valid box")
}

fn cube(center: [i64; 3], half: i64) -> FixedObb3 {
    obb(center, FixedMat3::IDENTITY, [half; 3])
}

fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(pt(min), pt(max))
}

fn tri(a: [i64; 3], b: [i64; 3], c: [i64; 3]) -> FixedTriangle3 {
    FixedTriangle3::new(pt(a), pt(b), pt(c))
}

fn rotation(axis: [i64; 3], angle: u64) -> FixedMat3 {
    FixedMat3::rotation_axis_angle(
        FixedUnitVec3::new(pt(axis.map(|c| c * ONE))).expect("a nonzero axis"),
        FixedAngle::from_raw(angle),
    )
}

/// The kernel's answer for two boxes, after checking it against both oracles and checking
/// that the swapped call has the same class.
fn boxes(a: &FixedObb3, b: &FixedObb3, context: &str) -> SatClass<ObbAxis> {
    let got = sat_obb_obb(a, b);
    let (ca, cb) = (obb_corners(a), obb_corners(b));
    assert_eq!(got, expected(&ca, &cb, &obb_axes(a, b)), "{context}: corner projections");
    assert_eq!(kind(&got), hull(&ca, &cb), "{context}: Minkowski hull");
    let swapped = sat_obb_obb(b, a);
    assert_eq!(kind(&swapped), kind(&got), "{context}: swapped class");
    assert_eq!(swapped, expected(&cb, &ca, &obb_axes(b, a)), "{context}: swapped axis");
    got
}

/// The kernel's answer for a triangle and a box, after checking it against both oracles and
/// checking that the inverted box gives the same answer.
fn triangle_box(t: &FixedTriangle3, b: &FixedAabb3, context: &str) -> SatClass<TriangleAabbAxis> {
    let got = sat_triangle_aabb(t, b);
    let (ct, cb) = (triangle_points(t), aabb_corners(b));
    assert_eq!(got, expected(&ct, &cb, &triangle_aabb_axes(t)), "{context}: projections");
    assert_eq!(kind(&got), hull(&ct, &cb), "{context}: Minkowski hull");
    let inverted = FixedAabb3::new(b.max, b.min);
    assert_eq!(sat_triangle_aabb(t, &inverted), got, "{context}: inverted box");
    got
}

const fn separated<A>(axis: A) -> SatClass<A> {
    SatClass::Separated { axis }
}

const fn touching<A>(axis: A) -> SatClass<A> {
    SatClass::Touching { axis }
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_001_axis_aligned_boxes_on_and_one_raw_unit_off_each_face() {
    assert_eq!(SAT_CONTRACT_ID, "bunny-sat/1");
    let a = cube([0, 0, 0], ONE);
    for k in 0..3 {
        for (delta, want) in [
            (1, separated(ObbAxis::FaceOfFirst(k))),
            (0, touching(ObbAxis::FaceOfFirst(k))),
            (-1, SatClass::Overlapping),
        ] {
            let mut center = [0; 3];
            center[k] = 2 * ONE + delta;
            let got = boxes(&a, &cube(center, ONE), &format!("axis {k} delta {delta}"));
            assert_eq!(got, want, "axis {k} delta {delta}");
            assert_eq!(got.intersects(), delta <= 0);
            assert_eq!(got.axis(), want.axis());
            // The negative side reports the same face.
            center[k] = -center[k];
            assert_eq!(boxes(&a, &cube(center, ONE), "negative side"), want);
        }
    }
    // Different sizes: the second box's faces come second in the order, but here the first
    // box's face separates first.
    let small = obb([0, 0, 0], FixedMat3::IDENTITY, [ONE / 2, ONE, 3 * ONE]);
    let big = obb([3 * ONE / 2, 0, 0], FixedMat3::IDENTITY, [ONE, ONE, ONE]);
    assert_eq!(boxes(&small, &big, "sizes"), touching(ObbAxis::FaceOfFirst(0)));
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_002_rotated_boxes_face_to_face_on_and_one_raw_unit_off() {
    // The second box is the first moved by exactly m columns a_0 (raw), so the face normal
    // a_1 x a_2 sees a distance of m |det| and radii (h_0 + g_0) |det|: contact when
    // h_0 + g_0 = m 2^32, exactly, whatever the rounding of the rotation.
    let rotations = [
        rotation([1, 2, 3], 0x1234_5678_9ABC_DEF0),
        rotation([0, 0, 1], EIGHTH_TURN),
        FixedMat3::rotation_x(FixedAngle::from_raw(0x2AAA_AAAA_AAAA_AAAB)),
    ];
    let m = 3_i64;
    for (r, axes) in rotations.iter().enumerate() {
        let a0 = axes.column0();
        let shift = [a0.x.raw(), a0.y.raw(), a0.z.raw()];
        let ca = [5 * ONE, -3 * ONE, 7 * ONE];
        let cb = [0, 1, 2].map(|i| ca[i] + m * shift[i]);
        let first = obb(ca, *axes, [5 * ONE / 4, ONE / 2, 2 * ONE]);
        for (delta, want) in [
            (-1, separated(ObbAxis::FaceOfFirst(0))),
            (0, touching(ObbAxis::FaceOfFirst(0))),
            (1, SatClass::Overlapping),
        ] {
            let second = obb(cb, *axes, [m * ONE - 5 * ONE / 4 + delta, 3 * ONE / 4, 3 * ONE / 2]);
            let context = format!("rotation {r} delta {delta}");
            assert_eq!(boxes(&first, &second, &context), want, "{context}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_003_only_an_edge_pair_separates_crossed_diamonds() {
    // A is a unit cube turned an eighth about z, B one turned an eighth about y. Along x each
    // reaches sqrt(2) from its centre, through A's z edge and B's y edge; z x y is -x, the
    // ninth edge pair (2, 1). Every face normal still overlaps.
    let a = obb([0, 0, 0], FixedMat3::rotation_z(FixedAngle::from_raw(EIGHTH_TURN)), [ONE; 3]);
    let b_axes = FixedMat3::rotation_y(FixedAngle::from_raw(EIGHTH_TURN));
    let reach = 12_148_001_999_i64; // 2 sqrt(2) 2^32 = 12 148 001 999.7...
    for (offset, want) in [
        (ONE / 16, separated(ObbAxis::EdgePair { first: 2, second: 1 })),
        (-ONE / 16, SatClass::Overlapping),
    ] {
        let b = obb([reach + offset, 0, 0], b_axes, [ONE; 3]);
        assert_eq!(boxes(&a, &b, &format!("offset {offset}")), want);
    }
    // Swapped, the same axis is edge pair (1, 2).
    let b = obb([reach + ONE / 16, 0, 0], b_axes, [ONE; 3]);
    assert_eq!(sat_obb_obb(&b, &a), separated(ObbAxis::EdgePair { first: 1, second: 2 }));
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_004_flat_segment_and_point_boxes_and_parallel_edges() {
    let turned = rotation([2, -1, 3], 0x4321_0000_1234_5678);
    let point = |c: [i64; 3]| obb(c, turned, [0; 3]);
    let unit = cube([0, 0, 0], ONE);
    // Two point boxes: coincident points touch; one raw unit apart they are separated.
    let p = point([ONE, 2 * ONE, -ONE]);
    let got = boxes(&p, &p, "coincident points");
    assert!(matches!(got, SatClass::Touching { axis: ObbAxis::FaceOfFirst(0) }), "{got:?}");
    let q = point([ONE + 1, 2 * ONE, -ONE]);
    assert!(matches!(boxes(&p, &q, "points apart"), SatClass::Separated { .. }));
    // A point inside a box overlaps it; on a face it touches; one raw unit out it is apart.
    assert_eq!(boxes(&unit, &point([ONE / 3, -ONE / 5, ONE / 7]), "inside"), SatClass::Overlapping);
    assert_eq!(boxes(&unit, &point([ONE, 0, 0]), "on face"), touching(ObbAxis::FaceOfFirst(0)));
    assert_eq!(
        boxes(&unit, &point([ONE + 1, 0, 0]), "one raw unit out"),
        separated(ObbAxis::FaceOfFirst(0))
    );
    // A segment through a box overlaps it.
    let segment = obb([0, 0, 0], turned, [3 * ONE, 0, 0]);
    assert_eq!(boxes(&unit, &segment, "segment"), SatClass::Overlapping);
    // Two flat squares in one plane meet only within it.
    let flat = |c: [i64; 3]| obb(c, FixedMat3::IDENTITY, [ONE, ONE, 0]);
    assert_eq!(
        boxes(&flat([0, 0, 0]), &flat([ONE, ONE, 0]), "coplanar squares"),
        touching(ObbAxis::FaceOfFirst(2))
    );
    // Identical axes make the three edge pairs (i, i) exactly zero: they are skipped, so two
    // overlapping cubes are not reported as touching on a zero axis.
    assert_eq!(boxes(&unit, &cube([ONE / 2, 0, 0], ONE), "parallel edges"), SatClass::Overlapping);
    assert_eq!(boxes(&unit, &unit, "same box"), SatClass::Overlapping);
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_005_boxes_at_the_ends_of_the_raw_range() {
    let turned = rotation([1, 1, 1], 0x7FFF_FFFF_FFFF_FFFF);
    let other = rotation([3, -2, 5], 0x0123_4567_89AB_CDEF);
    let big = i64::MAX;
    let cases = [
        (obb([i64::MIN; 3], turned, [big; 3]), obb([big; 3], other, [big; 3])),
        (obb([i64::MIN, 0, big], turned, [big, 0, 1]), obb([big, 0, i64::MIN], other, [big; 3])),
        (obb([big; 3], turned, [0; 3]), obb([big; 3], turned, [0; 3])),
        (obb([big, big, big], other, [1 << 62; 3]), obb([0, 0, 0], other, [1 << 62; 3])),
        (cube([i64::MIN; 3], big), cube([big - 1; 3], big)),
        (cube([i64::MIN; 3], big), cube([big - 2; 3], big)),
    ];
    for (n, (a, b)) in cases.iter().enumerate() {
        boxes(a, b, &format!("extreme case {n}"));
    }
    // Cubes of half extent i64::MAX raw: one centred at i64::MIN spans [-2^64 + 1, -1] raw
    // on each axis, one centred at i64::MAX - 1 spans [-1, 2^64 - 2]. They share one end
    // point, and one raw unit closer they overlap.
    assert_eq!(boxes(&cases[4].0, &cases[4].1, "4"), touching(ObbAxis::FaceOfFirst(0)));
    assert_eq!(boxes(&cases[5].0, &cases[5].1, "5"), SatClass::Overlapping);
    let apart = cube([big; 3], big);
    assert_eq!(boxes(&cases[4].0, &apart, "apart"), separated(ObbAxis::FaceOfFirst(0)));
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_006_triangles_on_and_one_raw_unit_off_each_box_face() {
    let b = aabb([0; 3], [ONE; 3]);
    for k in 0..3 {
        for (delta, want) in [
            (1, separated(TriangleAabbAxis::BoxFace(k))),
            (0, touching(TriangleAabbAxis::BoxFace(k))),
            (-1, SatClass::Overlapping),
        ] {
            // A triangle in the plane coordinate_k = 1 + delta raw, inside the face's extent.
            let at = |u: i64, w: i64| {
                let mut p = [0; 3];
                p[k] = ONE + delta;
                p[(k + 1) % 3] = u;
                p[(k + 2) % 3] = w;
                p
            };
            let t = tri(at(ONE / 4, ONE / 4), at(3 * ONE / 4, ONE / 4), at(ONE / 2, 3 * ONE / 4));
            let context = format!("face {k} delta {delta}");
            assert_eq!(triangle_box(&t, &b, &context), want, "{context}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_007_triangle_face_on_and_one_raw_unit_off_a_box_corner() {
    // The plane x + y + z = 3 + delta raw passes the corner (1, 1, 1) of the unit box; the
    // box faces and the edge cross products all overlap.
    let b = aabb([0; 3], [ONE; 3]);
    for (delta, want) in [
        (1, separated(TriangleAabbAxis::TriangleFace)),
        (0, touching(TriangleAabbAxis::TriangleFace)),
        (-1, SatClass::Overlapping),
    ] {
        let s = 3 * ONE + delta;
        let t = tri([s, 0, 0], [0, s, 0], [0, 0, s]);
        let context = format!("delta {delta}");
        assert_eq!(triangle_box(&t, &b, &context), want, "{context}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_008_an_edge_cross_product_on_and_one_raw_unit_off_a_box_edge() {
    // Edge 0 runs along x + y = 2 + delta raw at z = 1/2; the box's z edge at (1, 1) is on
    // that line. Dropping z, the edge's line separates; no box face or the tilted triangle
    // face does.
    let b = aabb([0; 3], [ONE; 3]);
    for (delta, want) in [
        (1, separated(TriangleAabbAxis::EdgeCross { edge: 0, axis: 2 })),
        (0, touching(TriangleAabbAxis::EdgeCross { edge: 0, axis: 2 })),
        (-1, SatClass::Overlapping),
    ] {
        let s = 2 * ONE + delta;
        let t = tri([s, 0, ONE / 2], [0, s, ONE / 2], [3 * ONE, 3 * ONE, 10 * ONE]);
        let context = format!("delta {delta}");
        assert_eq!(triangle_box(&t, &b, &context), want, "{context}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_009_degenerate_triangles_and_flat_and_inverted_boxes() {
    let b = aabb([0; 3], [ONE; 3]);
    let half = ONE / 2;
    let p = |x: [i64; 3]| tri(x, x, x);
    assert_eq!(triangle_box(&p([half; 3]), &b, "point inside"), SatClass::Overlapping);
    assert_eq!(
        triangle_box(&p([half, half, ONE]), &b, "point on face"),
        touching(TriangleAabbAxis::BoxFace(2))
    );
    assert_eq!(
        triangle_box(&p([half, half, ONE + 1]), &b, "point one raw unit out"),
        separated(TriangleAabbAxis::BoxFace(2))
    );
    // A segment (repeated vertex) through the box, and one along a box edge.
    let through = tri([-ONE, -ONE, -ONE], [2 * ONE; 3], [-ONE, -ONE, -ONE]);
    assert_eq!(triangle_box(&through, &b, "segment through"), SatClass::Overlapping);
    let along = tri([ONE, ONE, -ONE], [ONE, ONE, 2 * ONE], [ONE, ONE, half]);
    assert!(matches!(triangle_box(&along, &b, "segment on an edge"), SatClass::Touching { .. }));
    let off = tri([ONE + 1, ONE, -ONE], [ONE + 1, ONE, 2 * ONE], [ONE + 1, ONE, half]);
    assert_eq!(triangle_box(&off, &b, "segment off"), separated(TriangleAabbAxis::BoxFace(0)));
    // A point box inside a triangle's plane and region, and one raw unit above it.
    let flat = tri([-ONE, -ONE, 0], [ONE, -ONE, 0], [0, ONE, 0]);
    assert_eq!(
        triangle_box(&flat, &aabb([0; 3], [0; 3]), "point box"),
        touching(TriangleAabbAxis::BoxFace(2))
    );
    assert_eq!(
        triangle_box(&flat, &aabb([0, 0, 1], [0, 0, 1]), "point box above"),
        separated(TriangleAabbAxis::BoxFace(2))
    );
    // A flat box crossing a vertical triangle: two flat shapes crossing through each other's
    // relative interiors overlap, since every small translation keeps them meeting.
    let wall = tri([half, -ONE, -ONE], [half, 2 * ONE, -ONE], [half, half, 2 * ONE]);
    assert_eq!(
        triangle_box(&wall, &aabb([0, 0, half], [ONE, ONE, half]), "flat box"),
        SatClass::Overlapping
    );
    // The same flat box lying in the triangle's plane only touches it.
    assert_eq!(
        triangle_box(&wall, &aabb([half, 0, 0], [half, ONE, ONE]), "flat box in the plane"),
        touching(TriangleAabbAxis::BoxFace(0))
    );
    // Inverted boxes: every check above also ran with min and max swapped; here one axis only.
    let partly = aabb([0, ONE, 0], [ONE, 0, ONE]);
    assert_eq!(sat_triangle_aabb(&wall, &partly), sat_triangle_aabb(&wall, &b));
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_010_triangles_and_boxes_at_the_ends_of_the_raw_range() {
    let (lo, hi) = (i64::MIN, i64::MAX);
    let everything = aabb([lo; 3], [hi; 3]);
    let floor = tri([lo, lo, lo], [hi, lo, lo], [lo, hi, lo]);
    assert_eq!(
        triangle_box(&floor, &everything, "floor of the range"),
        touching(TriangleAabbAxis::BoxFace(2))
    );
    let diagonal = tri([lo, lo, hi], [hi, lo, lo], [lo, hi, lo]);
    triangle_box(&diagonal, &everything, "diagonal");
    let corner = aabb([hi - 1; 3], [hi; 3]);
    triangle_box(&diagonal, &corner, "far corner");
    let near = aabb([lo; 3], [lo + 1; 3]);
    triangle_box(&diagonal, &near, "near corner");
    let tilted = tri([hi, hi, hi], [lo, hi - 1, hi], [hi, lo, hi - 2]);
    triangle_box(&tilted, &corner, "tilted");
    triangle_box(&tilted, &everything, "tilted everything");
}
