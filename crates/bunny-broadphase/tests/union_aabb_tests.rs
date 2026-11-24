//! Dedicated tests for the existing `utils::union_aabb`, pinned as it behaves today
//! (flyingrobots/bunny#151). It takes each axis's minimum of the minima and maximum of the
//! maxima and never checks order, so an inverted input gives a box that is neither the other
//! input nor the hull of both inputs' corners. That behaviour is recorded here, not changed;
//! `FixedAabb3::merge` in `bunny-geom` is the checked helper that refuses inverted boxes.

use bunny_broadphase::utils::union_aabb;
use bunny_geom::{BoundsError, FixedAabb3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB015_1000_0000_0004;
const CASES: usize = 5_000;

type Raw3 = [i64; 3];

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn aabb(min: Raw3, max: Raw3) -> FixedAabb3 {
    FixedAabb3::new(v(min), v(max))
}

fn corners(b: &FixedAabb3) -> (Raw3, Raw3) {
    (raw(b.min), raw(b.max))
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_012_union_aabb_as_it_behaves_today() {
    let a = aabb([0, 0, 0], [2, 2, 2]);
    let b = aabb([1, -3, 5], [4, 1, 6]);
    assert_eq!(corners(&union_aabb(a, b)), ([0, -3, 0], [4, 2, 6]));
    assert_eq!(union_aabb(a, b), union_aabb(b, a));
    assert_eq!(union_aabb(a, a), a);
    let full = aabb([i64::MIN; 3], [i64::MAX; 3]);
    assert_eq!(union_aabb(full, a), full, "exact at the raw range");

    // Finding: inverted inputs are not refused, and the result is neither set union nor hull.
    // x of the inverted box: min -10 above max -20. The set of points it holds is empty, so a
    // set union would be `a`'s x range [0, 2]; the hull of its corners would be [-20, 2].
    let inverted = aabb([-10, 0, 0], [-20, 2, 2]);
    assert_eq!(corners(&union_aabb(a, inverted)), ([-10, 0, 0], [2, 2, 2]));
    // Two inverted boxes on x ([2, 0] and [-5, -7]) give an ordered box [-5, 0] on x.
    let p = aabb([2, 0, 0], [0, 1, 1]);
    let q = aabb([-5, 0, 0], [-7, 1, 1]);
    let both = union_aabb(p, q);
    assert_eq!(corners(&both), ([-5, 0, 0], [0, 1, 1]));
    assert!(FixedAabb3::try_new(both.min, both.max).is_ok(), "looks like real bounds");
    // The checked helper refuses both.
    assert_eq!(a.merge(&inverted), Err(BoundsError::InvertedBox));
    assert_eq!(p.merge(&q), Err(BoundsError::InvertedBox));
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(0x1405_7B7E_F767_814F);
    *state >> 11 ^ *state << 53 ^ *state
}

fn raw_any(state: &mut u64) -> i64 {
    let shift = next(state) % 64;
    i64::from_ne_bytes(next(state).to_ne_bytes()) >> shift
}

fn ordered_box(state: &mut u64) -> (Raw3, Raw3) {
    let (a, b) = ([0; 3].map(|_| raw_any(state)), [0; 3].map(|_| raw_any(state)));
    ([0, 1, 2].map(|i| a[i].min(b[i])), [0, 1, 2].map(|i| a[i].max(b[i])))
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_013_seeded_union_aabb_equals_merge_on_ordered_boxes() {
    let mut state = SEED;
    for case in 0..CASES {
        let ((alo, ahi), (blo, bhi)) = (ordered_box(&mut state), ordered_box(&mut state));
        let ctx = format!("seed={SEED:#x} case={case}");
        let got = corners(&union_aabb(aabb(alo, ahi), aabb(blo, bhi)));
        let want = ([0, 1, 2].map(|i| alo[i].min(blo[i])), [0, 1, 2].map(|i| ahi[i].max(bhi[i])));
        assert_eq!(got, want, "{ctx}");
        let merged = aabb(alo, ahi).merge(&aabb(blo, bhi)).map(|m| corners(&m));
        assert_eq!(merged, Ok(got), "merge agrees on ordered boxes; {ctx}");
    }
    println!("seed={SEED:#x}: {CASES} ordered pairs");
}
