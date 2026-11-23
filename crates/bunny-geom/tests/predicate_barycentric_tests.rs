//! Golden vectors for the exact barycentric numerators, the location classification and the
//! rounded weights (`bunny-predicates/1`, `docs/topics/predicates/`).

// The predicates name their points `a` to `c` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod predicates_oracle;

use bunny_geom::{
    barycentric2, barycentric3_coplanar, ExactBarycentric, TriangleEdge, TriangleLocation,
    TriangleVertex,
};
use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use predicates_oracle::{det2, rounds_to, same, Big};
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const U: i64 = 1 << 32;

/// A triangle, a point in its plane, and the projection that keeps two axes.
type Plane = ([FixedVec3; 4], fn(FixedVec3) -> FixedVec2);

fn v2(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y))
}

fn v3(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

fn numerators(bary: &ExactBarycentric) -> [Option<i128>; 4] {
    let [a, b, c] = bary.numerators();
    [a.to_i128(), b.to_i128(), c.to_i128(), bary.denominator().to_i128()]
}

/// Asserts the location, both containment forms, and the numerators against the oracle's
/// sub-triangle determinants, normalized by the sign of the triangle's.
fn check(p: FixedVec2, tri: [FixedVec2; 3], expected: TriangleLocation) -> ExactBarycentric {
    let [a, b, c] = tri;
    let context = format!("{p:?} in {tri:?}");
    let bary = barycentric2(p, a, b, c).expect(&context);
    assert_eq!(bary.location(), expected, "{context}");
    assert_eq!(bary.contains_closed(), expected != TriangleLocation::Outside, "{context}");
    assert_eq!(bary.contains_open(), expected == TriangleLocation::Inside, "{context}");
    let den = det2(a, b, c);
    let flip = |x: Big| if den.signum() < 0 { x.neg() } else { x };
    let [na, nb, nc] = bary.numerators();
    assert!(same(na, &flip(det2(p, b, c))), "{context}");
    assert!(same(nb, &flip(det2(a, p, c))), "{context}");
    assert!(same(nc, &flip(det2(a, b, p))), "{context}");
    assert!(same(bary.denominator(), &den.abs()), "{context}");
    bary
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_005_barycentric_locations_on_and_off_the_boundary() {
    use TriangleEdge::{AB, BC, CA};
    use TriangleLocation::{Inside, OnEdge, OnVertex, Outside};
    use TriangleVertex::{A, B, C};

    // a = (0, 0), b = (4, 0), c = (0, 4) raw: D2 = 16.
    let tri = [v2(0, 0), v2(4, 0), v2(0, 4)];
    let exact = |p, location, n: [i128; 3]| {
        let got = numerators(&check(p, tri, location));
        assert_eq!(got, [Some(n[0]), Some(n[1]), Some(n[2]), Some(16)], "{p:?}");
    };
    exact(v2(0, 0), OnVertex(A), [16, 0, 0]);
    exact(v2(4, 0), OnVertex(B), [0, 16, 0]);
    exact(v2(0, 4), OnVertex(C), [0, 0, 16]);
    exact(v2(2, 0), OnEdge(AB), [8, 8, 0]);
    exact(v2(2, 2), OnEdge(BC), [0, 8, 8]);
    exact(v2(0, 2), OnEdge(CA), [8, 0, 8]);
    exact(v2(1, 1), Inside, [8, 4, 4]);
    // One raw unit either side of each edge.
    exact(v2(2, 1), Inside, [4, 8, 4]);
    exact(v2(2, -1), Outside, [12, 8, -4]);
    exact(v2(1, 2), Inside, [4, 4, 8]);
    exact(v2(3, 2), Outside, [-4, 12, 8]);
    exact(v2(-1, 2), Outside, [12, -4, 8]);
    // On the line of an edge but past a vertex: one weight zero, another negative.
    exact(v2(5, 0), Outside, [-4, 20, 0]);
    exact(v2(-1, 0), Outside, [20, -4, 0]);
    // One raw unit off a vertex, inside and outside.
    exact(v2(1, 0), OnEdge(AB), [12, 4, 0]);
    exact(v2(-1, -1), Outside, [24, -4, -4]);

    // The clockwise order (a, c, b) has D2 = -16; the numerators are negated with it, so the
    // denominator is positive and the weights are those of the same point.
    let cw = [v2(0, 0), v2(0, 4), v2(4, 0)];
    let got = numerators(&check(v2(1, 1), cw, Inside));
    assert_eq!(got, [Some(8), Some(4), Some(4), Some(16)]);
    assert_eq!(check(v2(0, 2), cw, OnEdge(AB)).location(), OnEdge(AB));

    // Zero-area triangles are refused, exactly.
    assert_eq!(barycentric2(v2(0, 0), v2(0, 0), v2(1, 1), v2(2, 2)), None);
    assert_eq!(barycentric2(v2(9, 9), v2(3, 3), v2(3, 3), v2(3, 3)), None);
    assert_eq!(barycentric2(v2(0, 0), v2(MIN, MIN), v2(0, 0), v2(MAX, MAX)), None);
    assert!(barycentric2(v2(0, 0), v2(MIN, MIN), v2(0, 1), v2(MAX, MAX)).is_some());

    // The raw square's corner triangle; its hypotenuse is x + y = -1. D2 = (2^64 - 1)^2.
    let big = [v2(MIN, MIN), v2(MAX, MIN), v2(MIN, MAX)];
    let l = (1_i128 << 64) - 1;
    let got = numerators(&check(v2(0, -1), big, OnEdge(BC)));
    assert_eq!(got, [Some(0), Some((1 << 63) * l), Some(((1 << 63) - 1) * l), None]);
    check(v2(0, -2), big, Inside);
    check(v2(0, 0), big, Outside);
    check(v2(MAX, MIN), big, OnVertex(B));
    check(v2(MIN, MAX), big, OnVertex(C));
    check(v2(MIN, MIN), big, OnVertex(A));
    check(v2(MIN, 0), big, OnEdge(CA));
    check(v2(MIN + 1, 0), big, Inside);
    check(v2(0, MIN), big, OnEdge(AB));
    check(v2(MAX - 1, MIN), big, OnEdge(AB));
    check(v2(MAX, MIN + 1), big, Outside);
    check(v2(MAX - 1, MIN + 1), big, OnEdge(BC));
    check(v2(MAX - 2, MIN + 1), big, Inside);
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_006_coplanar_3d_barycentrics() {
    // The plane z = x + y: n = (4, 0, 4) x (0, 4, 4) = (-16, -16, 16). The first nonzero
    // normal axis is x, so the projection keeps (y, z).
    let (a, b, c) = (v3(0, 0, 0), v3(4, 0, 4), v3(0, 4, 4));
    let p = v3(1, 1, 2);
    let got = barycentric3_coplanar(p, a, b, c).expect("coplanar");
    let yz = |q: FixedVec3| FixedVec2::new(q.y, q.z);
    assert_eq!(Some(got), barycentric2(yz(p), yz(a), yz(b), yz(c)));
    assert_eq!(numerators(&got), [Some(8), Some(4), Some(4), Some(16)]);
    assert_eq!(got.location(), TriangleLocation::Inside);
    // The plane z = 2x + 3y has n = (-2, -3, 1): the (y, z) projection's doubled area is 2
    // where (x, y) would give 1, so the stated axis order is visible in the denominator.
    let (b2, c2, p2) = (v3(1, 0, 2), v3(0, 1, 3), v3(1, 1, 5));
    let got2 = barycentric3_coplanar(p2, a, b2, c2).expect("coplanar");
    assert_eq!(Some(got2), barycentric2(yz(p2), yz(a), yz(b2), yz(c2)));
    assert_eq!(numerators(&got2), [Some(-2), Some(2), Some(2), Some(2)]);
    // The plane z = 3y contains the x axis, n = (0, -6, 2): the (z, x) projection is used,
    // denominator 6 where (x, y) would give 2.
    let zx = |q: FixedVec3| FixedVec2::new(q.z, q.x);
    let (b3, c3, p3) = (v3(2, 0, 0), v3(0, 1, 3), v3(1, 1, 3));
    let got3 = barycentric3_coplanar(p3, a, b3, c3).expect("coplanar");
    assert_eq!(Some(got3), barycentric2(zx(p3), zx(a), zx(b3), zx(c3)));
    assert_eq!(got3.denominator().to_i128(), Some(6));
    // One raw unit off the plane, and a point off the plane over an edge: refused.
    assert_eq!(barycentric3_coplanar(v3(1, 1, 3), a, b, c), None);
    assert_eq!(barycentric3_coplanar(v3(2, 0, 3), a, b, c), None);
    let on_edge = barycentric3_coplanar(v3(2, 0, 2), a, b, c).expect("coplanar");
    assert_eq!(on_edge.location(), TriangleLocation::OnEdge(TriangleEdge::AB));

    // Each axis plane uses its own projection: x = 0 keeps (y, z), y = 0 keeps (z, x) and
    // z = 0 keeps (x, y).
    let xy = |q: FixedVec3| FixedVec2::new(q.x, q.y);
    let planes: [Plane; 3] = [
        ([v3(0, 0, 0), v3(0, 5, 0), v3(0, 0, 5), v3(0, 1, 2)], |q| FixedVec2::new(q.y, q.z)),
        ([v3(0, 0, 0), v3(0, 0, 5), v3(5, 0, 0), v3(2, 0, 9)], zx),
        ([v3(MIN, MIN, 7), v3(MAX, MIN, 7), v3(MIN, MAX, 7), v3(0, -1, 7)], xy),
    ];
    for ([a, b, c, p], project) in planes {
        let got = barycentric3_coplanar(p, a, b, c);
        assert!(got.is_some(), "{a:?} {b:?} {c:?} {p:?}");
        assert_eq!(got, barycentric2(project(p), project(a), project(b), project(c)));
    }
    // A zero-area triangle, collinear in 3D, is refused even with a point on its line.
    let (a, b, c) = (v3(0, 0, 0), v3(1, 2, 3), v3(2, 4, 6));
    assert_eq!(barycentric3_coplanar(v3(3, 6, 9), a, b, c), None);
    assert_eq!(barycentric3_coplanar(a, a, a, a), None);
}

fn weights(p: FixedVec2, tri: [FixedVec2; 3]) -> Option<[i64; 3]> {
    let [a, b, c] = tri;
    let bary = barycentric2(p, a, b, c).expect("not degenerate");
    let got = bary.weights_rounded().map(|w| w.map(FixedQ32_32::raw));
    // Each weight is checked against the oracle's numerator times 2^32 over the denominator.
    let den = det2(a, b, c);
    let flip = |x: Big| if den.signum() < 0 { x.neg() } else { x };
    let nums = [det2(p, b, c), det2(a, p, c), det2(a, b, p)].map(|n| flip(n).shl(32));
    match got {
        Some(w) => {
            for (n, w) in nums.iter().zip(w) {
                assert!(rounds_to(n, &den.abs(), Some(w)), "{p:?} {tri:?}");
            }
        }
        None => assert!(nums.iter().any(|n| rounds_to(n, &den.abs(), None)), "{p:?} {tri:?}"),
    }
    got
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_007_rounded_weights_and_their_sum() {
    // Thirds: 2^32 / 3 = 1431655765.33 rounds down each time; the sum is 2^32 - 1.
    let third = 1_431_655_765;
    let w = weights(v2(1, 1), [v2(0, 0), v2(3, 0), v2(0, 3)]);
    assert_eq!(w, Some([third, third, third]));
    assert_eq!(w.map(|w| w.iter().sum::<i64>()), Some(U - 1));

    // (2/3, 1/6, 1/6): 2863311530.67 and 715827882.67 round up; the sum is 2^32 + 1.
    let w = weights(v2(1, 1), [v2(0, 0), v2(6, 0), v2(0, 6)]);
    assert_eq!(w, Some([2_863_311_531, 715_827_883, 715_827_883]));
    assert_eq!(w.map(|w| w.iter().sum::<i64>()), Some(U + 1));

    // Ties: D2 = 2^33, so a numerator of 1 is half a raw unit (to 0) and 3 is 1.5 (to 2);
    // the weight of a, 2^32 - 1/2 and 2^32 - 3/2, goes to 2^32 and 2^32 - 2.
    let tie = [v2(0, 0), v2(2 * U, 0), v2(0, 1)];
    assert_eq!(weights(v2(1, 0), tie), Some([U, 0, 0]));
    assert_eq!(weights(v2(3, 0), tie), Some([U - 2, 2, 0]));
    assert_eq!(weights(v2(5, 0), tie), Some([U - 2, 2, 0]));
    assert_eq!(weights(v2(7, 0), tie), Some([U - 4, 4, 0]));

    // A vertex is exactly (1.0, 0, 0).
    assert_eq!(weights(v2(4, 0), [v2(0, 0), v2(4, 0), v2(0, 4)]), Some([0, U, 0]));

    // D2 = 1 raw squared: each weight is its numerator times 2^32. A weight of
    // 2^63 - 2^32 and one of exactly i64::MIN are kept; a weight of 2^63 is refused.
    let unit = [v2(0, 0), v2(1, 0), v2(0, 1)];
    let w = weights(v2((1 << 31) - 1, 0), unit);
    assert_eq!(w, Some([-((1 << 31) - 2) * U, MAX - (U - 1), 0]));
    assert_eq!(weights(v2(1 << 31, 0), unit), None);
    let w = weights(v2(-(1 << 31), (1 << 31) - 1), unit);
    assert_eq!(w, Some([2 * U, MIN, MAX - (U - 1)]));
    assert_eq!(weights(v2(-(1 << 31), 1 << 31), unit), None);
}
