//! Golden vectors for the exact 2D and 3D orientation predicates and the signed area and
//! volume (`bunny-predicates/1`, `docs/topics/predicates/`).
//!
//! Every value is a raw Q32.32 integer with its derivation, and each is also checked against
//! the independent big-integer oracle in `predicates_oracle`.

// The predicates name their points `a` to `d` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod predicates_oracle;

use bunny_geom::{
    doubled_signed_area2, orientation2, orientation3, sextupled_signed_volume3,
    signed_area2_rounded, signed_volume3_rounded, Orientation2, Orientation3,
    PREDICATES_CONTRACT_ID,
};
use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use predicates_oracle::{det2, det3, rounds_to, same, Big};
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const U: i64 = 1 << 32;
/// `(2^64 - 1)^2`, the largest `|D2|`, as little-endian limbs: `2^128 - 2^65 + 1`.
const L2_LIMBS: [u64; 4] = [1, 0xFFFF_FFFF_FFFF_FFFE, 0, 0];
/// `2 (2^64 - 1)^3`, the largest `|D3|`: `2^193 - 6 2^128 + 6 2^64 - 2`.
const TWO_L3_LIMBS: [u64; 4] = [0xFFFF_FFFF_FFFF_FFFE, 5, 0xFFFF_FFFF_FFFF_FFFA, 1];

/// `b`, `c`, the exact `D2` and the rounded area, with `a` at the origin.
type AreaCase = (FixedVec2, FixedVec2, i128, Option<i64>);
/// `b`, `c`, `d`, the exact `D3` when it fits `i128`, and the rounded volume, with `a` at the
/// origin.
type VolumeCase = (FixedVec3, FixedVec3, FixedVec3, Option<i128>, Option<i64>);

fn v2(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y))
}

fn v3(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// Asserts the orientation, the exact `D2` (as an `i128` when it fits) and agreement with the
/// oracle.
fn check2(points: [FixedVec2; 3], expected: Orientation2, d2: Option<i128>) {
    let [a, b, c] = points;
    let context = format!("{a:?} {b:?} {c:?}");
    assert_eq!(orientation2(a, b, c), expected, "{context}");
    let exact = doubled_signed_area2(a, b, c);
    assert!(same(exact, &det2(a, b, c)), "{context}: {exact:?}");
    if d2.is_some() {
        assert_eq!(exact.to_i128(), d2, "{context}");
    }
}

fn check3(points: [FixedVec3; 4], expected: Orientation3) {
    let [a, b, c, d] = points;
    let context = format!("{a:?} {b:?} {c:?} {d:?}");
    assert_eq!(orientation3(a, b, c, d), expected, "{context}");
    let exact = sextupled_signed_volume3(a, b, c, d);
    assert!(same(exact, &det3(a, b, c, d)), "{context}: {exact:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_001_collinear_and_one_raw_unit_off_in_2d() {
    use Orientation2::{Clockwise, Collinear, CounterClockwise};
    assert_eq!(PREDICATES_CONTRACT_ID, "bunny-predicates/1");

    // Small: (0, 0), (1, 1), (2, 2) raw lie on y = x. Moving c by one raw unit gives
    // D2 = 1 * cy - 1 * cx = +-1 raw squared, which a Q32.32-rounded product loses.
    let (a, b) = (v2(0, 0), v2(1, 1));
    check2([a, b, v2(2, 2)], Collinear, Some(0));
    check2([a, b, v2(2, 3)], CounterClockwise, Some(1));
    check2([a, b, v2(2, 1)], Clockwise, Some(-1));
    check2([a, b, v2(3, 2)], Clockwise, Some(-1));
    check2([a, b, v2(1, 2)], CounterClockwise, Some(1));

    // The diagonal of the whole raw range: e1 = (2^63, 2^63), e2 = (2^64 - 1, 2^64 - 1).
    // The products are near 2^127 and the one-raw-unit changes are 2^63 or 2^64 - 1.
    let a = v2(MIN, MIN);
    check2([a, v2(0, 0), v2(MAX, MAX)], Collinear, Some(0));
    check2([a, v2(0, 0), v2(MAX, MAX - 1)], Clockwise, Some(-(1 << 63)));
    check2([a, v2(0, 0), v2(MAX - 1, MAX)], CounterClockwise, Some(1 << 63));
    let l = (1_i128 << 64) - 1;
    check2([a, v2(0, 1), v2(MAX, MAX)], Clockwise, Some(-l));
    check2([a, v2(1, 0), v2(MAX, MAX)], CounterClockwise, Some(l));

    // The anti-diagonal x + y = -1 through (MAX, MIN), (MIN, MAX) and (0, -1).
    let (a, b) = (v2(MAX, MIN), v2(MIN, MAX));
    check2([a, b, v2(0, -1)], Collinear, Some(0));
    check2([a, b, v2(0, 0)], Clockwise, Some(-l));
    check2([a, b, v2(0, -2)], CounterClockwise, Some(l));

    // Repeated points are collinear, at either end of the range.
    check2([v2(MIN, MAX), v2(MIN, MAX), v2(MIN, MAX)], Collinear, Some(0));
    check2([v2(MAX, MAX), v2(MAX, MAX), v2(MIN, 7)], Collinear, Some(0));
    check2([v2(MIN, MIN), v2(5, -3), v2(MIN, MIN)], Collinear, Some(0));
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_002_doubled_and_rounded_signed_area() {
    // With a = 0, b = (bx, by) and c = (cx, cy): D2 = bx cy - by cx, and the rounded area is
    // round_half_even(D2 / 2^33) raw.
    let o = v2(0, 0);
    let cases: [AreaCase; 16] = [
        // D2 = 1: a positive orientation whose rounded area is zero.
        (v2(1, 0), v2(0, 1), 1, Some(0)),
        (v2(0, 1), v2(1, 0), -1, Some(0)),
        // D2 = 2^32: half a raw unit of area, a tie that goes to 0.
        (v2(U, 0), v2(0, 1), 1 << 32, Some(0)),
        (v2(0, 1), v2(U, 0), -(1 << 32), Some(0)),
        // D2 = 2^32 + 1: just above half a raw unit.
        (v2(U + 1, 0), v2(0, 1), (1 << 32) + 1, Some(1)),
        (v2(0, 1), v2(U + 1, 0), -(1 << 32) - 1, Some(-1)),
        // D2 = 3 2^32 and 5 2^32: ties at 1.5 and 2.5 raw, both to 2.
        (v2(3 * U, 0), v2(0, 1), 3 << 32, Some(2)),
        (v2(0, 1), v2(3 * U, 0), -(3 << 32), Some(-2)),
        (v2(5 * U, 0), v2(0, 1), 5 << 32, Some(2)),
        // The top of i64: MAX 2^33 is exactly MAX; MAX 2^33 + 2^32 - 1 rounds to MAX;
        // MAX 2^33 + 2^32 is a tie that goes to 2^63 (even), outside i64.
        (v2(MAX, 0), v2(0, 2 * U), i128::from(MAX) << 33, Some(MAX)),
        (v2(MAX, -1), v2(U - 1, 2 * U), (i128::from(MAX) << 33) + (1 << 32) - 1, Some(MAX)),
        (v2(MAX, -1), v2(U, 2 * U), (i128::from(MAX) << 33) + (1 << 32), None),
        // The bottom: MIN 2^33 is MIN; MIN 2^33 - 2^32 is a tie that goes to MIN (even);
        // one raw squared lower rounds to MIN - 1, outside i64.
        (v2(MIN, 0), v2(0, 2 * U), i128::from(MIN) << 33, Some(MIN)),
        (v2(MIN, 1), v2(U, 2 * U), (i128::from(MIN) << 33) - (1 << 32), Some(MIN)),
        (v2(MIN, 1), v2(U + 1, 2 * U), (i128::from(MIN) << 33) - (1 << 32) - 1, None),
        (v2(1, 0), v2(1, 0), 0, Some(0)),
    ];
    for (b, c, d2, rounded) in cases {
        let context = format!("{b:?} {c:?}");
        assert_eq!(doubled_signed_area2(o, b, c).to_i128(), Some(d2), "{context}");
        let got = signed_area2_rounded(o, b, c).map(FixedQ32_32::raw);
        assert_eq!(got, rounded, "{context}");
        assert!(rounds_to(&det2(o, b, c), &Big::from_i128(1 << 33), got), "{context}");
    }

    // The largest |D2| is (2^64 - 1)^2, from a corner triangle of the raw square: a triangle
    // in a square has at most half its area. It does not fit i128.
    let (a, b, c) = (v2(MIN, MIN), v2(MAX, MIN), v2(MIN, MAX));
    for (b, c, negative) in [(b, c, false), (c, b, true)] {
        let exact = doubled_signed_area2(a, b, c);
        assert_eq!((exact.magnitude(), exact.is_negative()), (L2_LIMBS, negative));
        assert_eq!(exact.to_i128(), None);
        assert!(same(exact, &det2(a, b, c)));
        assert_eq!(signed_area2_rounded(a, b, c), None);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_003_coplanar_and_one_raw_unit_off_in_3d() {
    use Orientation3::{Behind, Coplanar, Front};

    // Small: the plane z = 0 with the right-hand normal +Z of (0, x, y); one raw unit of z.
    let (a, b, c) = (v3(0, 0, 0), v3(1, 0, 0), v3(0, 1, 0));
    check3([a, b, c, v3(5, 7, 0)], Coplanar);
    check3([a, b, c, v3(5, 7, 1)], Front);
    check3([a, b, c, v3(5, 7, -1)], Behind);
    assert_eq!(sextupled_signed_volume3(a, b, c, v3(5, 7, 1)).to_i128(), Some(1));

    // The face z = MIN of the raw cube, normal +Z; then d, or c, one raw unit off it.
    let (a, b, c) = (v3(MIN, MIN, MIN), v3(MAX, MIN, MIN), v3(MIN, MAX, MIN));
    check3([a, b, c, v3(MAX, MAX, MIN)], Coplanar);
    check3([a, b, c, v3(MAX, MAX, MIN + 1)], Front);
    check3([a, b, v3(MIN, MAX, MIN + 1), v3(MAX, MAX, MIN)], Behind);
    check3([a, b, c, v3(0, 0, MIN + 1)], Front);

    // The tilted plane x = y across the whole range: n = (L^2, -L^2, 0) with L = 2^64 - 1.
    let (a, b, c) = (v3(MIN, MIN, MIN), v3(MAX, MAX, MIN), v3(MIN, MIN, MAX));
    check3([a, b, c, v3(0, 0, 5)], Coplanar);
    check3([a, b, c, v3(MAX, MAX, MAX)], Coplanar);
    check3([a, b, c, v3(1, 0, 5)], Front);
    check3([a, b, c, v3(0, 1, 5)], Behind);
    check3([a, b, c, v3(MAX, MAX - 1, MAX)], Front);
    check3([a, b, c, v3(MAX - 1, MAX, MAX)], Behind);

    // Repeated points are coplanar.
    check3([a, a, b, c], Coplanar);
    check3([a, b, c, c], Coplanar);

    // The largest |D3| = 2 L^3: the regular tetrahedron on alternate corners of the cube.
    let (b, c, d) = (v3(MAX, MAX, MIN), v3(MAX, MIN, MAX), v3(MIN, MAX, MAX));
    check3([a, b, c, d], Behind);
    check3([a, b, d, c], Front);
    let exact = sextupled_signed_volume3(a, b, c, d);
    assert_eq!((exact.magnitude(), exact.is_negative()), (TWO_L3_LIMBS, true));
    assert_eq!(signed_volume3_rounded(a, b, c, d), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_004_sextupled_and_rounded_signed_volume() {
    // With a = 0 the edge rows are b, c and d; the rounded volume is
    // round_half_even(D3 / (6 2^64)) raw.
    let o = v3(0, 0, 0);
    let half = 1_i64 << 31;
    let cases: [VolumeCase; 11] = [
        // D3 = 1: Front, rounded volume zero.
        (v3(1, 0, 0), v3(0, 1, 0), v3(0, 0, 1), Some(1), Some(0)),
        // D3 = 3 2^64: half a raw unit of volume, a tie to 0; and its negative.
        (v3(3 * U, 0, 0), v3(0, U, 0), v3(0, 0, 1), Some(3 << 64), Some(0)),
        (v3(3 * U, 0, 0), v3(0, 0, 1), v3(0, U, 0), Some(-(3 << 64)), Some(0)),
        // D3 = 3 2^64 + 1 from rows (1, 0, 0), (0, 3 2^32, 1), (0, -1, 2^32).
        (v3(1, 0, 0), v3(0, 3 * U, 1), v3(0, -1, U), Some((3 << 64) + 1), Some(1)),
        (v3(1, 0, 0), v3(0, -1, U), v3(0, 3 * U, 1), Some(-(3 << 64) - 1), Some(-1)),
        // D3 = 9 2^64 and 15 2^64: ties at 1.5 and 2.5 raw, both to 2.
        (v3(9 * U, 0, 0), v3(0, U, 0), v3(0, 0, 1), Some(9 << 64), Some(2)),
        (v3(15 * U, 0, 0), v3(0, U, 0), v3(0, 0, 1), Some(15 << 64), Some(2)),
        // Rows (MAX, 0, 1), (0, 6 2^32, 0), (u, 0, 2^32): D3 = 6 2^64 MAX - 6 2^32 u, which does
        // not fit i128 (the oracle checks it). At u = 0
        // exactly MAX; u = 1 - 2^31 rounds to MAX; u = -2^31 is a tie to 2^63, refused.
        (v3(MAX, 0, 1), v3(0, 6 * U, 0), v3(0, 0, U), None, Some(MAX)),
        (v3(MAX, 0, 1), v3(0, 6 * U, 0), v3(1 - half, 0, U), None, Some(MAX)),
        (v3(MAX, 0, 1), v3(0, 6 * U, 0), v3(-half, 0, U), None, None),
        // With MIN: u = 2^31 is a tie at MIN - 1/2, which goes to MIN (even).
        (v3(MIN, 0, 1), v3(0, 6 * U, 0), v3(half, 0, U), None, Some(MIN)),
    ];
    for (b, c, d, d3, rounded) in cases {
        let context = format!("{b:?} {c:?} {d:?}");
        let exact = sextupled_signed_volume3(o, b, c, d);
        assert!(same(exact, &det3(o, b, c, d)), "{context}");
        if d3.is_some() {
            assert_eq!(exact.to_i128(), d3, "{context}");
        }
        let got = signed_volume3_rounded(o, b, c, d).map(FixedQ32_32::raw);
        assert_eq!(got, rounded, "{context}");
        let six = Big::from_i128(6 << 64);
        assert!(rounds_to(&det3(o, b, c, d), &six, got), "{context}");
    }
    // One raw unit past the MIN tie rounds to MIN - 1, outside i64.
    let (b, c, d) = (v3(MIN, 0, 1), v3(0, 6 * U, 0), v3(half + 1, 0, U));
    assert_eq!(signed_volume3_rounded(o, b, c, d), None);
    assert!(rounds_to(&det3(o, b, c, d), &Big::from_i128(6 << 64), None));
}
