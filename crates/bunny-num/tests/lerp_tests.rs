//! Contract tests for `bunny-lerp/1` in `bunny-num` (`docs/topics/lerp/test-plan.md`).
//!
//! Oracle: `oracle_lerp` and `oracle_ratio` below compute the exact rational in `i128` and
//! `u128` and round it to nearest with ties to even by Euclidean division and a comparison of
//! twice the remainder with the divisor. They share no helper with the kernel, which rounds by
//! an arithmetic shift and a mask. `U = 2^32` is one unit (`1.0`) and `H = 2^31` is half of it.
//! Hand-derived values are written as `exact -> neighbours -> result`.

use std::cmp::Ordering;

use bunny_num::{FixedQ32_32, LERP_CONTRACT_ID};
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0303_1E4F_0001;

/// `a + round_half_even((b - a) * t / 2^32)`, exact in `i128`; `None` outside `i64`.
fn oracle_lerp(a: i64, b: i64, t: i64) -> Option<i64> {
    let unit = 1_i128 << 32;
    let exact = (i128::from(b) - i128::from(a)) * i128::from(t);
    let floor = exact.div_euclid(unit);
    let up = match (2 * exact.rem_euclid(unit)).cmp(&unit) {
        Ordering::Less => 0,
        Ordering::Greater => 1,
        Ordering::Equal => floor.rem_euclid(2),
    };
    i64::try_from(i128::from(a) + floor + up).ok()
}

/// `round_half_even(n * 2^32 / d)` as `floor((2N + d) / 2d)`, a tie moved back to even.
fn oracle_ratio(n: u32, d: u32) -> Option<i64> {
    if d == 0 {
        return None;
    }
    let (twice_numerator, d) = (u128::from(n) << 33, u128::from(d));
    let sum = twice_numerator + d;
    let mut nearest = sum / (2 * d);
    if sum % (2 * d) == 0 && nearest % 2 == 1 {
        nearest -= 1;
    }
    i64::try_from(nearest).ok()
}

fn lerp(a: i64, b: i64, t: i64) -> Option<i64> {
    FixedQ32_32::lerp(FixedQ32_32::from_raw(a), FixedQ32_32::from_raw(b), FixedQ32_32::from_raw(t))
        .map(FixedQ32_32::raw)
}

fn ratio(n: u32, d: u32) -> Option<i64> {
    FixedQ32_32::from_ratio(n, d).map(FixedQ32_32::raw)
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A raw value of any magnitude: a random `i64` shifted right (arithmetically) by 0 to 63.
fn spread(state: &mut u64) -> i64 {
    let value = next_seed(state).cast_signed();
    value >> (next_seed(state) % 64)
}

/// 64 raw values: 0, `+-0.5`, `+-1.0` and `+-2^30` (raw `+-2^62`), each with three raw units
/// either side, and the eight lowest and seven highest raw values (real values near `+-2^31`).
fn grid() -> Vec<i64> {
    let mut values = Vec::new();
    for centre in [0, H, -H, U, -U, 1 << 62, -(1 << 62)] {
        values.extend((-3..=3).map(|offset| centre + offset));
    }
    values.extend((0..8).map(|offset| i64::MIN + offset));
    values.extend((0..7).map(|offset| i64::MAX - offset));
    values
}

/// `(a, b, t, expected raw)`: golden vectors for the WebAssembly job, derived by hand.
const LERP_GOLDEN: &[(i64, i64, i64, Option<i64>)] = &[
    (0, U, H, Some(H)),                          // lerp(0, 1.0, 0.5) = 0.5
    (-U, U, U / 4, Some(-H)),                    // lerp(-1.0, 1.0, 0.25) = -0.5
    (0, 1, H, Some(0)),                          // 1/2 -> 0, 1 -> 0 (even)
    (0, 1, H - 1, Some(0)),                      // 1/2 - 2^-32 -> 0
    (0, 1, H + 1, Some(1)),                      // 1/2 + 2^-32 -> 1
    (0, 3, H, Some(2)),                          // 3/2 -> 1, 2 -> 2 (even)
    (0, 3, H - 1, Some(1)),                      // 3/2 - 3 * 2^-32 -> 1
    (0, -1, H, Some(0)),                         // -1/2 -> -1, 0 -> 0 (even)
    (0, -3, H, Some(-2)),                        // -3/2 -> -2, -1 -> -2 (even)
    (5, 6, H, Some(5)), // 5 + (1/2 -> 0, 1 -> 0): the step is rounded, not 5.5
    (4, 5, H, Some(4)), // 4 + (1/2 -> 0): the same step at every a
    (0, U, 2 * U, Some(2 * U)), // lerp(0, 1.0, 2.0) = 2.0, extrapolated
    (0, U, -U, Some(-U)), // lerp(0, 1.0, -1.0) = -1.0, extrapolated
    (0, i64::MAX, 2 * U, None), // 2^64 - 2 overflows
    (i64::MAX - 1, i64::MAX, U, Some(i64::MAX)), // ends exactly on the maximum
    (i64::MAX - 1, i64::MAX, 2 * U, None), // one raw unit past it
    (i64::MIN + 1, i64::MIN, U, Some(i64::MIN)), // ends exactly on the minimum
    (i64::MIN + 1, i64::MIN, 2 * U, None), // one raw unit below it
    (i64::MIN, i64::MAX, U, Some(i64::MAX)), // b - a = 2^64 - 1 does not fit i64
    (i64::MIN, i64::MAX, H, Some(0)), // -2^63 + (2^63 - 1/2) -> 2^63 - 1 (odd), 2^63
    (i64::MAX, i64::MIN, H, Some(-1)), // 2^63 - 1 - (2^63 - 1/2) -> -2^63 (even), so -1
    (i64::MIN, i64::MAX, i64::MIN, None), // the widest product, -(2^64 - 1) * 2^31
    (i64::MAX, i64::MIN, i64::MAX, None),
];

/// `(n, d, expected raw)`: golden vectors for the WebAssembly job, derived by hand.
const RATIO_GOLDEN: &[(u32, u32, Option<i64>)] = &[
    (1, 3, Some(1_431_655_765)),   // 2^32 / 3 = 1 431 655 765 + 1/3
    (2, 3, Some(2_863_311_531)),   // 2^33 / 3 = 2 863 311 530 + 2/3
    (7, 7, Some(U)),               // exactly 1.0
    (u32::MAX, u32::MAX, Some(U)), // exactly 1.0
    (1, 2, Some(H)),               // 0.5
    (0, 1, Some(0)),
    (0, 0, None), // division by zero
    (1, 0, None),
    (u32::MAX, 0, None),
    (2_147_483_647, 1, Some(i64::MAX - (U - 1))), // 2^31 - 1, the largest whole value
    (2_147_483_648, 1, None),                     // 2^31 is raw 2^63
    (u32::MAX, 2, Some(i64::MAX - (H - 1))),      // 2^31 - 1/2, the largest reachable value
    (u32::MAX, 1, None),
    (1, u32::MAX, Some(1)), // 2^32 / (2^32 - 1) = 1 + 1/(2^32 - 1)
    (2_147_483_647, u32::MAX, Some(H - 1)), // (2^31 - 1) + (2^31 - 1) / (2^32 - 1), under a half
    (2_147_483_648, u32::MAX, Some(H + 1)), // 2^31 + 2^31 / (2^32 - 1), over a half
];

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_001_hand_derived_lerps_and_ties_round_to_even() {
    assert_eq!(LERP_CONTRACT_ID, "bunny-lerp/1");
    for &(a, b, t, expected) in LERP_GOLDEN {
        assert_eq!(oracle_lerp(a, b, t), expected, "oracle lerp({a}, {b}, {t})");
        assert_eq!(lerp(a, b, t), expected, "lerp({a}, {b}, {t})");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_002_the_ends_are_exact_for_every_grid_pair() {
    let values = grid();
    for &a in &values {
        for &b in &values {
            assert_eq!(lerp(a, b, 0), Some(a), "lerp({a}, {b}, 0)");
            assert_eq!(lerp(a, b, U), Some(b), "lerp({a}, {b}, 1.0)");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_003_the_grid_matches_the_oracle_exhaustively() {
    let values = grid();
    assert_eq!(values.len(), 64);
    let mut sorted = values.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 64, "grid values are distinct");
    let mut checked = 0_u64;
    for &a in &values {
        for &b in &values {
            for k in 0..=4096_i64 {
                let t = k << 20;
                assert_eq!(lerp(a, b, t), oracle_lerp(a, b, t), "lerp({a}, {b}, {t})");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 64 * 64 * 4097);
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_004_seeded_triples_match_the_oracle() {
    println!("lerp_tp_004 seed={SEED:#018x} cases=100000");
    let mut state = SEED;
    let (mut defined, mut refused, mut ties) = (0_u32, 0_u32, 0_u32);
    for case in 0..100_000_u32 {
        let (a, b) = (spread(&mut state), spread(&mut state));
        let raw = next_seed(&mut state);
        let t = match case % 4 {
            0 => (raw % (1 << 32 | 1)).cast_signed(), // [0, 1.0]
            1 => (raw % (1 << 35)).cast_signed() - (1 << 34), // [-4.0, 4.0)
            2 => spread(&mut state),
            _ => ((raw % 9).cast_signed() - 4) * H, // multiples of 1/2: ties when b - a is odd
        };
        let expected = oracle_lerp(a, b, t);
        assert_eq!(lerp(a, b, t), expected, "seed={SEED:#018x} case={case} lerp({a}, {b}, {t})");
        let product = (i128::from(b) - i128::from(a)) * i128::from(t);
        ties += u32::from(product.rem_euclid(1 << 32) == 1 << 31);
        if expected.is_some() {
            defined += 1;
        } else {
            refused += 1;
        }
    }
    println!("lerp_tp_004 defined={defined} refused={refused} ties={ties}");
    assert!(defined >= 10_000 && refused >= 10_000 && ties >= 1_000, "every class is reached");
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_005_lerp_is_monotone_in_t() {
    let values: Vec<i64> = grid().into_iter().step_by(4).collect();
    let mut ts = Vec::new();
    for k in -2048..=2048_i64 {
        ts.extend([(k << 22) - 1, k << 22, (k << 22) + 1]);
    }
    for &a in &values {
        for &b in &values {
            let strict = (i128::from(b) - i128::from(a)).abs() >= i128::from(U);
            let outputs: Vec<i64> = ts.iter().filter_map(|&t| lerp(a, b, t)).collect();
            for pair in outputs.windows(2) {
                let (earlier, later) = (pair[0], pair[1]);
                let ordered = if b >= a { earlier <= later } else { earlier >= later };
                assert!(ordered, "lerp({a}, {b}, t) not monotone: {earlier} then {later}");
                assert!(!strict || earlier != later, "lerp({a}, {b}, t) repeats {earlier}");
            }
        }
    }
    // Below one unit apart, neighbouring t can give the same value: not strictly monotone.
    assert_eq!(lerp(0, 1, 0), lerp(0, 1, 1));
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_006_hand_derived_ratios() {
    for &(n, d, expected) in RATIO_GOLDEN {
        assert_eq!(oracle_ratio(n, d), expected, "oracle from_ratio({n}, {d})");
        assert_eq!(ratio(n, d), expected, "from_ratio({n}, {d})");
    }
    assert_eq!(FixedQ32_32::from_ratio(7, 7), Some(FixedQ32_32::ONE));
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_007_ratios_match_the_oracle_on_a_grid_and_seeded_pairs() {
    let edges: Vec<u32> = [0_u32, 1, 2, 3, 7, 1 << 16, (1 << 31) - 1, 1 << 31, (1 << 31) + 1]
        .into_iter()
        .chain([3_000_000_000, u32::MAX - 2, u32::MAX - 1, u32::MAX])
        .collect();
    for &n in &edges {
        for &d in &edges {
            assert_eq!(ratio(n, d), oracle_ratio(n, d), "from_ratio({n}, {d})");
        }
    }
    for d in 1..=4096_u32 {
        assert_eq!(FixedQ32_32::from_ratio(d, d), Some(FixedQ32_32::ONE), "from_ratio({d}, {d})");
    }
    println!("lerp_tp_007 seed={SEED:#018x} cases=100000");
    let mut state = SEED;
    let (mut defined, mut refused) = (0_u32, 0_u32);
    for case in 0..100_000_u32 {
        let raw = next_seed(&mut state);
        let n = (raw >> 32) as u32;
        let d = ((raw & 0xFFFF_FFFF) as u32) >> (next_seed(&mut state) % 32);
        let expected = oracle_ratio(n, d);
        assert_eq!(ratio(n, d), expected, "seed={SEED:#018x} case={case} from_ratio({n}, {d})");
        if expected.is_some() {
            defined += 1;
        } else {
            refused += 1;
        }
    }
    println!("lerp_tp_007 defined={defined} refused={refused}");
    assert!(defined >= 10_000 && refused >= 1_000, "both classes are reached");
}
