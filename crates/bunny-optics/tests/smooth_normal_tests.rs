//! Evidence for `bunny-smooth-normal/1`: hand-derived points, a golden input on or beside the
//! boundary of every rounding rule, an exact big-integer oracle over seeded weights and
//! normals, and golden vectors.
//!
//! Golden values marked "model" come from a separate Python model written from the chapter's
//! definition (unbounded integers, each rounding rule switchable); the oracle shares no code
//! with the crate.

// Seeded draws are reduced before each `as` conversion, so every value is in range.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

mod exact;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{smooth_normal, SMOOTH_NORMAL_ID};
use bunny_query::BarycentricNumerators;
use wasm_bindgen_test::wasm_bindgen_test;

use exact::{squared_length, within, Big, Signed};

const ONE: i64 = ONE_RAW;
const SEED: u64 = 0xB002_0289_0000_0002;

type Raw3 = [i64; 3];

fn normal(raw: Raw3) -> FixedUnitVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedUnitVec3::try_from_unit(FixedVec3::new(x, y, z)).expect("a unit vector")
}

fn raw(n: FixedUnitVec3) -> Raw3 {
    let v = n.into_inner();
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// `smooth_normal` of three raw unit normals at the weights `(det - u - v, u, v) / det`.
fn at(normals: [Raw3; 3], u: i128, v: i128, det: i128) -> Option<Raw3> {
    let weights = BarycentricNumerators::new(u, v, det).expect("a point on the triangle");
    smooth_normal(normals.map(normal), weights).map(raw)
}

const X: Raw3 = [ONE, 0, 0];
const NEG_X: Raw3 = [-ONE, 0, 0];
const Y: Raw3 = [0, ONE, 0];
const Z: Raw3 = [0, 0, ONE];
/// A unit vector that normalizing again changes: its `y` becomes `-3 548 619 166` (model).
const R: Raw3 = [-1_816_015_699, -3_548_619_165, 1_598_791_128];
/// `(0.8, 0.6, 0)` rounded: both components above `2^31`.
const G: Raw3 = [3_435_973_837, 2_576_980_378, 0];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_074_hand_derived_points_give_exact_shading_normals() {
    assert_eq!(SMOOTH_NORMAL_ID, "bunny-smooth-normal/1");
    // Corners return their vertex normal unchanged, R included.
    assert_eq!(at([R, G, X], 0, 0, 7), Some(R));
    assert_eq!(at([R, G, X], 7, 0, 7), Some(G));
    assert_eq!(at([R, G, X], 0, 7, 7), Some(X));
    // Equal normals (a flat face, or an edge between equal normals) give that normal exactly.
    assert_eq!(at([R, R, R], 2, 3, 9), Some(R));
    assert_eq!(at([R, R, X], 4, 0, 9), Some(R));
    // The midpoint of an X-Y edge: (1, 1, 0) / sqrt(2); 2^32 / sqrt(2) = 3 037 000 499.976.
    let diagonal = Some([3_037_000_500, 3_037_000_500, 0]);
    assert_eq!(at([X, Y, Z], 5, 0, 10), diagonal);
    assert_eq!(at([X, Y, Z], 1, 0, 2), diagonal);
    // Weights (6, 1, 3) / 10: the second and third weights are not interchangeable (model).
    assert_eq!(at([X, Y, Z], 1, 3, 10), Some([3_799_550_278, 633_258_380, 1_899_775_139]));
    // Opposite normals cancel exactly at the midpoint: no normal.
    assert_eq!(at([X, NEG_X, Z], 4, 0, 8), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_075_every_rounding_rule_is_pinned_at_its_boundary() {
    // Weight shift, floored. det = 2^62 + 11, so k = 1; w1 = B = 2 * odd, w2 = 6 (3 after the
    // shift), normals (X, -X, Y): m = ((w0 >> 1) - (B >> 1), 3, 0) * 2^32. With w0 = B + 1 (a
    // tie) the floor gives x = 0, where rounding to nearest even would give 1, (1, 3) / sqrt(10).
    let b: i128 = (1 << 61) + 2;
    let weight = |w0: i128| at([X, NEG_X, Y], b, 6, w0 + b + 6);
    let tilted = Some([1_358_187_913, 4_074_563_739, 0]);
    assert_eq!(weight(b), Some(Y));
    assert_eq!(weight(b + 1), Some(Y));
    assert_eq!(weight(b + 2), tilted);
    // Division, ties to even. Weights (px, py, 0) on (X, Y, Z) normalize p = (px, py, 0) itself;
    // L = floor(sqrt(px^2 + py^2)) is 2^33 L' and px = (2q + 1) L' with q = 2 576 980 378, so
    // x = px 2^32 / L is exactly q + 1/2 and rounds to the even q (model; ties away give q + 1).
    let (px, py) = (1_763_362_027_009_388_091_i128, 2_351_149_368_062_836_052_i128);
    let tie = |px: i128| at([X, Y, Z], py, 0, px + py);
    assert_eq!(tie(px), Some([2_576_980_378, 3_435_973_836, 0]));
    assert_eq!(tie(px - 1), Some([2_576_980_378, 3_435_973_836, 0]));
    assert_eq!(tie(px + 1), Some([2_576_980_379, 3_435_973_836, 0]));
    // The length is floored. Here x is again an exact tie, at an odd q = 2 576 980 377, and the
    // squared length is above (L + 1/2)^2: the floor keeps the tie (rounded up to the even
    // ...378); a length rounded to nearest would be L + 1, giving ...377 (model).
    let (sx, sy) = (1_763_362_026_325_113_565_i128, 2_351_149_368_576_041_947_i128);
    assert_eq!(at([X, Y, Z], sy, 0, sx + sy), Some([2_576_980_378, 3_435_973_837, 0]));
    // The prescale is floored. A third weight of 1 on G adds G's components (each above 2^31,
    // below 2^32) to m = (px, py, 0) 2^32, and the prescale shifts right by 32: the floor gives
    // the division tie above; rounding would give (px + 1, py + 1), and x = ...379 (model).
    assert_eq!(at([X, Y, G], py, 1, px + py + 1), Some([2_576_980_378, 3_435_973_836, 0]));
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 11
}

fn below(state: &mut u64, limit: u128) -> u128 {
    let wide = u128::from(next(state)) << 64 | u128::from(next(state));
    wide % limit
}

/// A seeded unit normal: a random vector normalized, an axis, or the negation of `other`.
fn seeded_normal(state: &mut u64, other: Raw3) -> Raw3 {
    match next(state) % 6 {
        0 => other,
        1 => other.map(|c| -c),
        2 => Z,
        _ => {
            let mut c = || FixedQ32_32::from_raw((next(state) % (1 << 33)) as i64 - ONE);
            FixedUnitVec3::new(FixedVec3::new(c(), c(), c())).map_or(X, raw)
        }
    }
}

/// What the oracle concluded about one case.
#[derive(Debug, PartialEq, Eq)]
enum Checked {
    Exact,
    Bounded,
    Cancelled,
    Zero,
}

/// Checks one case against the exact interpolation `S = sum w_i n_i`, computed with big integers
/// for any `det` below `2^127`: shared normals exactly; `None` only for `S = 0` or, after the
/// shift, for `rho < 2^-59` (where `|S| / 2^k` is below the floors' error); otherwise within
/// `1 + 64 det / |S|` raw units (`1 + 2^-26 / rho`), or 1 when `det < 2^62`.
fn check(normals: [Raw3; 3], u: i128, v: i128, det: i128) -> Result<Checked, String> {
    let weights = [det - u - v, u, v];
    let exact: [Signed; 3] = [0, 1, 2].map(|a| {
        (0..3).fold(Signed::new(0), |s, i| {
            s.add(&Signed::product(weights[i], i128::from(normals[i][a]), 0))
        })
    });
    let found = at(normals, u, v, det);
    let present: Vec<Raw3> = (0..3).filter(|&i| weights[i] > 0).map(|i| normals[i]).collect();
    if present.iter().all(|n| *n == present[0]) {
        return (found == Some(present[0])).then_some(Checked::Exact).ok_or("shared".into());
    }
    let zero = Signed::new(0);
    if exact.iter().all(|c| *c == zero) {
        return found.is_none().then_some(Checked::Zero).ok_or("a normal for zero".into());
    }
    let shifted = det >= 1 << 62;
    let slack = if shifted { Signed::product(det, 64, 0) } else { zero };
    match found {
        Some(r) if within(r, &exact, 1, &slack) => Ok(Checked::Bounded),
        None if shifted && tiny(&exact, det) => Ok(Checked::Cancelled),
        other => Err(format!("{other:?} for the exact {exact:?}")),
    }
}

/// Whether `rho < 2^-59`: `|S|^2 2^118 < det^2 2^64`, that is `|S|^2 2^54 < det^2`.
fn tiny(exact: &[Signed; 3], det: i128) -> bool {
    let scaled = squared_length(exact).mul(&Big::new(1 << 54));
    scaled.compare(&Signed::new(det).square()) == std::cmp::Ordering::Less
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_076_shading_normals_are_within_the_bound_of_the_exact_oracle() {
    let mut state = SEED;
    let mut counts = [0_usize; 4];
    for case in 0..3000 {
        let bits = [1, 8, 40, 61, 62, 63, 80, 93, 110, 116][(next(&mut state) % 10) as usize];
        let det = 1 + below(&mut state, 1 << bits) as i128;
        let u = below(&mut state, det.unsigned_abs() + 1) as i128;
        let v = below(&mut state, (det - u).unsigned_abs() + 1) as i128;
        let first = seeded_normal(&mut state, X);
        let second = seeded_normal(&mut state, first);
        let normals = [first, second, seeded_normal(&mut state, second)];
        let outcome = check(normals, u, v, det).unwrap_or_else(|why| {
            panic!("seed {SEED:#x}, case {case}: {normals:?} {u} {v} {det}: {why}")
        });
        counts[outcome as usize] += 1;
    }
    let [exact, bounded, cancelled, zero] = counts;
    assert!(exact > 100 && bounded > 1000 && zero >= 5, "{counts:?}");
    assert!(cancelled < 30, "{counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_077_golden_shading_normals_lock_the_rule() {
    // (model) No shift, a 20-bit shift and a 55-bit shift, the last with u + v far below det.
    assert_eq!(at([R, G, X], 3, 4, 9), Some([4_255_422_680, 113_040_765, 570_389_057]));
    let (u, v, det) = ((1 << 80) + 12_345, (1 << 79) + 999, (1 << 81) + 77);
    assert_eq!(at([R, G, X], u, v, det), Some([4_174_188_673, 716_615_428, 713_691_338]));
    let negated = R.map(|c| -c);
    let (u, v, det) = ((1 << 100) + 1, 3, (1 << 116) + 5);
    assert_eq!(at([G, negated, Y], u, v, det), Some([3_435_957_821, 2_577_001_731, -24_396]));
    // Each is also within the bound of the exact oracle.
    let twenty = ((1 << 80) + 12_345, (1 << 79) + 999, (1 << 81) + 77);
    for (u, v, det) in [(3, 4, 9), twenty] {
        assert_eq!(check([R, G, X], u, v, det), Ok(Checked::Bounded));
    }
    assert_eq!(check([G, negated, Y], (1 << 100) + 1, 3, (1 << 116) + 5), Ok(Checked::Bounded));
}
