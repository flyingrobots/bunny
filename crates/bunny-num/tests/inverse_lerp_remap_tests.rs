//! Contract tests for `checked_clamp`, `inverse_lerp` and `remap` under `bunny-lerp/1`
//! (`docs/topics/lerp/test-plan.md`, LERP-TP-020 to LERP-TP-030).
//!
//! Oracles, sharing no helper with the kernel (which multiplies and divides magnitudes in
//! `u128` and rounds by comparing the remainder with its complement):
//! - `oracle_inverse` rounds `(x - a) 2^32 / (b - a)` in `i128` by Euclidean division of the
//!   signed value and a comparison of twice the remainder with the divisor.
//! - `oracle_remap` never forms the product: `mul_div` walks the bits of one factor,
//!   doubling and adding the other modulo the divisor, so it keeps `floor(m n / d)` and its
//!   remainder below `2^128` without a wide multiply.
//!
//! `U = 2^32` is one unit (`1.0`) and `H = 2^31` is half of it. Hand-derived values are
//! written as `exact -> neighbours -> result`.

use std::cmp::Ordering;

use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0111_4E3A_0001;

/// `round_half_even((x - a) 2^32 / (b - a))` on the signed value; `None` for `a == b` or
/// outside `i64`.
fn oracle_inverse(a: i64, b: i64, x: i64) -> Option<i64> {
    let run = i128::from(b) - i128::from(a);
    if run == 0 {
        return None;
    }
    let numerator = (i128::from(x) - i128::from(a)) << 32;
    let (numerator, run) = if run < 0 { (-numerator, -run) } else { (numerator, run) };
    let floor = numerator.div_euclid(run);
    let up = match (2 * numerator.rem_euclid(run)).cmp(&run) {
        Ordering::Less => 0,
        Ordering::Greater => 1,
        Ordering::Equal => floor.rem_euclid(2),
    };
    i64::try_from(floor + up).ok()
}

/// `(floor(m n / d), m n mod d)` for `m, n < 2^64` and `0 < d < 2^64`, by shift and add over
/// the bits of `n`, keeping `m n_k = q d + r` with `0 <= r < d` for each prefix `n_k` of `n`.
fn mul_div(m: u128, n: u128, d: u128) -> (u128, u128) {
    let (m_quotient, m_remainder) = (m / d, m % d);
    let (mut quotient, mut remainder) = (0_u128, 0_u128);
    for bit in (0..64).rev() {
        quotient *= 2;
        remainder *= 2;
        if remainder >= d {
            quotient += 1;
            remainder -= d;
        }
        if (n >> bit) & 1 == 1 {
            quotient += m_quotient;
            remainder += m_remainder;
            if remainder >= d {
                quotient += 1;
                remainder -= d;
            }
        }
    }
    (quotient, remainder)
}

/// `out_lo + round_half_even((x - in_lo)(out_hi - out_lo) / (in_hi - in_lo))`, the step
/// rounded and then added; `None` for `in_lo == in_hi` or outside `i64`.
fn oracle_remap(x: i64, in_lo: i64, in_hi: i64, out_lo: i64, out_hi: i64) -> Option<i64> {
    let run = i128::from(in_hi) - i128::from(in_lo);
    if run == 0 {
        return None;
    }
    let offset = i128::from(x) - i128::from(in_lo);
    let rise = i128::from(out_hi) - i128::from(out_lo);
    let divisor = run.unsigned_abs();
    let (quotient, remainder) = mul_div(offset.unsigned_abs(), rise.unsigned_abs(), divisor);
    let up = match (2 * remainder).cmp(&divisor) {
        Ordering::Less => 0,
        Ordering::Greater => 1,
        Ordering::Equal => quotient % 2,
    };
    let magnitude = i128::try_from(quotient + up).ok()?;
    let negative = (offset < 0) ^ (rise < 0) ^ (run < 0);
    let step = if negative { -magnitude } else { magnitude };
    i64::try_from(i128::from(out_lo).checked_add(step)?).ok()
}

/// Whether the exact remap step is a tie, from the oracle's own quotient and remainder.
fn remap_is_tie(x: i64, in_lo: i64, in_hi: i64, out_lo: i64, out_hi: i64) -> bool {
    let run = (i128::from(in_hi) - i128::from(in_lo)).unsigned_abs();
    let offset = (i128::from(x) - i128::from(in_lo)).unsigned_abs();
    let rise = (i128::from(out_hi) - i128::from(out_lo)).unsigned_abs();
    run != 0 && 2 * mul_div(offset, rise, run).1 == run
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn clamp(x: i64, lo: i64, hi: i64) -> Option<i64> {
    fixed(x).checked_clamp(fixed(lo), fixed(hi)).map(FixedQ32_32::raw)
}

fn inverse(a: i64, b: i64, x: i64) -> Option<i64> {
    FixedQ32_32::inverse_lerp(fixed(a), fixed(b), fixed(x)).map(FixedQ32_32::raw)
}

fn lerp(a: i64, b: i64, t: i64) -> Option<i64> {
    FixedQ32_32::lerp(fixed(a), fixed(b), fixed(t)).map(FixedQ32_32::raw)
}

fn remap(x: i64, in_lo: i64, in_hi: i64, out_lo: i64, out_hi: i64) -> Option<i64> {
    FixedQ32_32::remap(fixed(x), [fixed(in_lo), fixed(in_hi)], [fixed(out_lo), fixed(out_hi)])
        .map(FixedQ32_32::raw)
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
/// either side, and the eight lowest and seven highest raw values.
fn grid() -> Vec<i64> {
    let mut values = Vec::new();
    for centre in [0, H, -H, U, -U, 1 << 62, -(1 << 62)] {
        values.extend((-3..=3).map(|offset| centre + offset));
    }
    values.extend((0..8).map(|offset| i64::MIN + offset));
    values.extend((0..7).map(|offset| i64::MAX - offset));
    values
}

/// `(x, lo, hi, expected raw)`, derived by hand.
const CLAMP_GOLDEN: &[(i64, i64, i64, Option<i64>)] = &[
    (5, 5, 5, Some(5)), // lo == hi, x on it
    (4, 5, 5, Some(5)), // lo == hi, one raw unit below
    (6, 5, 5, Some(5)), // lo == hi, one raw unit above
    (-U, -U, U, Some(-U)),
    (-U - 1, -U, U, Some(-U)), // one raw unit below lo
    (-U + 1, -U, U, Some(-U + 1)),
    (U, -U, U, Some(U)),
    (U + 1, -U, U, Some(U)), // one raw unit above hi
    (U - 1, -U, U, Some(U - 1)),
    (0, U, -U, None),   // lo > hi
    (0, 1, 0, None),    // lo one raw unit above hi
    (0, 0, 0, Some(0)), // lo == hi == x
    (i64::MIN, i64::MIN, i64::MAX, Some(i64::MIN)),
    (i64::MAX, i64::MIN, i64::MAX, Some(i64::MAX)),
    (i64::MIN, i64::MIN + 1, i64::MAX, Some(i64::MIN + 1)),
    (i64::MAX, i64::MIN, i64::MAX - 1, Some(i64::MAX - 1)),
    (0, i64::MAX, i64::MIN, None),
];

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_020_checked_clamp_is_exact_and_refuses_an_inverted_range() {
    for &(x, lo, hi, expected) in CLAMP_GOLDEN {
        assert_eq!(clamp(x, lo, hi), expected, "checked_clamp({x}, {lo}, {hi})");
    }
    let values = grid();
    for &x in &values {
        for &lo in &values {
            for &hi in &values {
                let expected = (lo <= hi).then(|| x.max(lo).min(hi));
                assert_eq!(clamp(x, lo, hi), expected, "checked_clamp({x}, {lo}, {hi})");
                if lo <= hi {
                    assert_eq!(Ord::clamp(fixed(x), fixed(lo), fixed(hi)).raw(), x.clamp(lo, hi));
                }
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_021_min_and_max_follow_the_raw_order() {
    let values = grid();
    for &a in &values {
        for &b in &values {
            assert_eq!(fixed(a).min(fixed(b)).raw(), a.min(b), "min({a}, {b})");
            assert_eq!(fixed(a).max(fixed(b)).raw(), a.max(b), "max({a}, {b})");
            assert_eq!(fixed(a).cmp(&fixed(b)), a.cmp(&b), "cmp({a}, {b})");
        }
    }
}

/// `(a, b, x, expected raw t)`, derived by hand.
const INVERSE_GOLDEN: &[(i64, i64, i64, Option<i64>)] = &[
    (0, U, H, Some(H)),                     // inverse_lerp(0, 1.0, 0.5) = 0.5
    (-U, U, 0, Some(H)),                    // inverse_lerp(-1.0, 1.0, 0) = 0.5
    (U, 0, U / 4, Some(3 * U / 4)),         // a reversed range: 0.75
    (0, 3 * U, U, Some(1_431_655_765)),     // 2^32 / 3 = 1 431 655 765 + 1/3
    (0, 3 * U, 2 * U, Some(2_863_311_531)), // 2^33 / 3 = 2 863 311 530 + 2/3
    (0, 2 * U, 1, Some(0)),                 // 1/2 -> 0, 1 -> 0 (even)
    (0, 2 * U, 3, Some(2)),                 // 3/2 -> 1, 2 -> 2 (even)
    (0, 2 * U, -1, Some(0)),                // -1/2 -> -1, 0 -> 0 (even)
    (0, 2 * U, -3, Some(-2)),               // -3/2 -> -2, -1 -> -2 (even)
    (0, 4 * U, 1, Some(0)),                 // 1/4, below the tie
    (0, 4 * U, 2, Some(0)),                 // 1/2, the tie, to even 0
    (0, 4 * U, 3, Some(1)),                 // 3/4, above the tie
    (0, U, 2 * U, Some(2 * U)),             // extrapolated past b: 2.0
    (0, U, -U, Some(-U)),                   // extrapolated before a: -1.0
    (7, 7, 7, None),                        // a == b: degenerate
    (0, 0, 0, None),
    (i64::MAX, i64::MAX, 0, None),
    (i64::MIN, i64::MIN, i64::MAX, None),
    (i64::MIN, i64::MAX, i64::MIN, Some(0)), // x == a, the widest range
    (i64::MIN, i64::MAX, i64::MAX, Some(U)), // x == b, the widest range
    (i64::MIN, i64::MAX, 0, Some(H)),        // 2^95 / (2^64 - 1) = 2^31 + 2^31 / (2^64 - 1)
    (0, U, i64::MAX, Some(i64::MAX)),        // t = x - a exactly, on the maximum
    (-1, U - 1, i64::MAX, None),             // one raw unit past it
    (0, U, i64::MIN, Some(i64::MIN)),        // on the minimum
    (1, U + 1, i64::MIN, None),              // one raw unit below it
    (i64::MIN, i64::MIN + 2 * U, i64::MAX - 1, Some(i64::MAX)), // (2^64 - 2) / 2 = 2^63 - 1
    (i64::MIN, i64::MIN + 2 * U, i64::MAX - 2, Some(i64::MAX - 1)), // 2^63 - 3/2 -> even
    (i64::MIN, i64::MIN + 2 * U, i64::MAX, None), // 2^63 - 1/2 -> 2^63 (even), outside
    (0, 1, (1 << 31) - 1, Some((H - 1) * U)), // b - a = 1 raw: t = x 2^32
    (0, 1, 1 << 31, None),                   // 2^31 * 2^32 = 2^63, outside
];

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_022_hand_derived_inverse_lerps() {
    for &(a, b, x, expected) in INVERSE_GOLDEN {
        assert_eq!(oracle_inverse(a, b, x), expected, "oracle inverse_lerp({a}, {b}, {x})");
        assert_eq!(inverse(a, b, x), expected, "inverse_lerp({a}, {b}, {x})");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_023_inverse_lerp_matches_the_oracle_on_the_grid() {
    let values = grid();
    let mut checked = 0_u32;
    for &a in &values {
        for &b in &values {
            assert_eq!(inverse(a, b, a), (a != b).then_some(0), "inverse_lerp({a}, {b}, a)");
            assert_eq!(inverse(a, b, b), (a != b).then_some(U), "inverse_lerp({a}, {b}, b)");
            for &x in &values {
                let expected = oracle_inverse(a, b, x);
                assert_eq!(inverse(a, b, x), expected, "inverse_lerp({a}, {b}, {x})");
                assert_eq!(oracle_remap(x, a, b, 0, U), expected, "oracles agree ({a}, {b}, {x})");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 64 * 64 * 64);
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_024_seeded_inverse_lerps_match_the_oracle() {
    println!("lerp_tp_024 seed={SEED:#018x} cases=100000");
    let mut state = SEED;
    let (mut defined, mut overflowed, mut ties, mut degenerate) = (0_u32, 0_u32, 0_u32, 0_u32);
    for case in 0..100_000_u32 {
        let a = spread(&mut state);
        let b = match case % 4 {
            0 => a.wrapping_add((2 * U) << (next_seed(&mut state) % 4)), // ties are common
            1 => a,                                                      // degenerate
            _ => spread(&mut state),
        };
        let x = if case % 4 == 0 {
            a.wrapping_add(spread(&mut state) >> 20)
        } else {
            spread(&mut state)
        };
        let expected = oracle_inverse(a, b, x);
        assert_eq!(inverse(a, b, x), expected, "seed={SEED:#018x} case={case} ({a}, {b}, {x})");
        ties += u32::from(remap_is_tie(x, a, b, 0, U));
        degenerate += u32::from(a == b);
        defined += u32::from(expected.is_some());
        overflowed += u32::from(a != b && expected.is_none());
    }
    println!(
        "lerp_tp_024 defined={defined} overflowed={overflowed} ties={ties} degenerate={degenerate}"
    );
    assert!(defined >= 10_000 && overflowed >= 1_000 && ties >= 1_000, "every class is reached");
    assert_eq!(degenerate, 25_000, "every fourth case is a degenerate range");
}

/// The largest `|lerp(a, b, inverse_lerp(a, b, x)) - x|`: 0 for `|b - a| <= 2^32` raw, else
/// `floor(1/2 + |b - a| / 2^33)`.
fn forward_bound(run: u128) -> u128 {
    if run <= 1 << 32 {
        0
    } else {
        ((1 << 32) + run) >> 33
    }
}

/// The largest `|inverse_lerp(a, b, lerp(a, b, t)) - t|`: 0 for `|b - a| >= 2^32` raw, else
/// `floor(1/2 + 2^31 / |b - a|)`.
fn backward_bound(run: u128) -> u128 {
    if run >= 1 << 32 {
        0
    } else {
        (run + (1 << 32)) / (2 * run)
    }
}

fn check_round_trips(a: i64, b: i64, value: i64, worst: &mut [u128; 2]) -> [bool; 2] {
    let run = (i128::from(b) - i128::from(a)).unsigned_abs();
    let forward = inverse(a, b, value).and_then(|t| lerp(a, b, t));
    if let Some(y) = forward {
        let error = (i128::from(y) - i128::from(value)).unsigned_abs();
        assert!(error <= forward_bound(run), "lerp(inverse_lerp({a}, {b}, {value})) = {y}");
        worst[0] = worst[0].max(error);
    }
    let backward = lerp(a, b, value).and_then(|x| inverse(a, b, x));
    if let Some(t) = backward {
        let error = (i128::from(t) - i128::from(value)).unsigned_abs();
        assert!(error <= backward_bound(run), "inverse_lerp(lerp({a}, {b}, {value})) = {t}");
        worst[1] = worst[1].max(error);
    }
    [forward.is_some(), backward.is_some()]
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_025_round_trips_hold_within_their_bounds() {
    // Not an exact round trip below one unit: two t one raw unit apart give one x.
    assert_eq!(lerp(0, 1, 0), lerp(0, 1, 1));
    assert_eq!(inverse(0, 1, 0), Some(0));
    assert_eq!(backward_bound(1), 1 << 31);
    assert_eq!(forward_bound(u128::from(u64::MAX)), 1 << 31);
    let mut worst = [0_u128; 2];
    let values = grid();
    for &a in &values {
        for &b in &values {
            if a != b {
                for &v in &values {
                    check_round_trips(a, b, v, &mut worst);
                }
            }
        }
    }
    println!("lerp_tp_025 seed={SEED:#018x} cases=100000");
    let mut state = SEED ^ 0x25;
    let mut both = 0_u32;
    for _ in 0..100_000_u32 {
        let (a, b) = (spread(&mut state), spread(&mut state));
        let value = if next_seed(&mut state) % 2 == 0 {
            spread(&mut state)
        } else {
            spread(&mut state) >> 31
        };
        if a != b {
            both += u32::from(check_round_trips(a, b, value, &mut worst) == [true, true]);
        }
    }
    println!(
        "lerp_tp_025 both_defined={both} worst_forward={} worst_backward={}",
        worst[0], worst[1]
    );
    assert!(both >= 10_000, "both round trips are reached");
    assert!(worst[0] > 0 && worst[1] > 0, "neither round trip is exact in general");
}

/// `(x, in_lo, in_hi, out_lo, out_hi, expected raw)`, derived by hand.
const REMAP_GOLDEN: &[(i64, i64, i64, i64, i64, Option<i64>)] = &[
    (H, 0, U, 10 * U, 20 * U, Some(15 * U)), // 0.5 in [0, 1] to [10, 20] = 15.0
    (U, 0, 3 * U, 0, U, Some(1_431_655_765)), // 1/3
    (U / 4, 0, U, U, 0, Some(3 * U / 4)),    // a reversed output range: 0.75
    (2 * U, 0, U, 0, 10 * U, Some(20 * U)),  // extrapolated: 20.0
    (-U, 0, U, 0, 10 * U, Some(-10 * U)),    // extrapolated: -10.0
    (1, 0, 2, 0, 1, Some(0)),                // 1/2 -> 0, 1 -> 0 (even)
    (3, 0, 2, 0, 1, Some(2)),                // 3/2 -> 1, 2 -> 2 (even)
    (-1, 0, 2, 0, 1, Some(0)),               // -1/2 -> 0 (even)
    (-3, 0, 2, 0, 1, Some(-2)),              // -3/2 -> -2 (even)
    (1, 0, 2, 5, 6, Some(5)),                // 5 + (1/2 -> 0): the step is rounded, not 5.5
    (1, 0, 4, 0, 1, Some(0)),                // 1/4, below a tie
    (3, 0, 4, 0, 1, Some(1)),                // 3/4, above it
    (5, 3, 3, 0, U, None),                   // in_lo == in_hi: degenerate
    (3, 3, 3, 0, U, None),
    (i64::MIN, i64::MIN, i64::MIN, i64::MIN, i64::MAX, None),
    (9, 0, U, 7, 7, Some(7)), // out_lo == out_hi is allowed
    (i64::MIN, i64::MIN, i64::MAX, i64::MIN, i64::MAX, Some(i64::MIN)), // x == in_lo
    (i64::MAX, i64::MIN, i64::MAX, i64::MIN, i64::MAX, Some(i64::MAX)), // x == in_hi, step 2^64 - 1
    (i64::MAX, i64::MIN, i64::MAX, i64::MAX, i64::MIN, Some(i64::MIN)), // reversed, widest
    (i64::MIN, i64::MAX, i64::MIN, i64::MAX, i64::MIN, Some(i64::MIN)), // reversed input range
    (i64::MAX, 0, 1, 0, 1, Some(i64::MAX)), // the identity, on the maximum
    (i64::MAX, 0, 1, 1, 2, None), // one raw unit past it
    (i64::MIN, 0, 1, 0, 1, Some(i64::MIN)),
    (i64::MIN, 0, 1, -1, 0, None), // one raw unit below the minimum
    (i64::MAX, i64::MIN, i64::MIN + 2, -1, 0, Some(i64::MAX)), // -1 + (2^63 - 1/2 -> 2^63)
    (i64::MAX, i64::MIN, i64::MIN + 2, 0, 1, None), // 2^63, outside
    (i64::MAX - 1, i64::MIN, i64::MIN + 2, 0, 1, Some(i64::MAX)), // 2^63 - 1
    (i64::MAX, i64::MIN, i64::MIN + 2, 0, -1, Some(i64::MIN)), // -(2^63 - 1/2) -> -2^63
    (i64::MAX, i64::MIN, i64::MIN + 2, -1, -2, None), // one raw unit below
    (i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN, i64::MAX, None), // 2^64 + 1/(2^64 - 2) -> 2^64
    (i64::MAX, i64::MIN, i64::MIN + 1, i64::MIN, i64::MAX, None), // the product, ~2^128
];

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_026_hand_derived_remaps() {
    for &(x, in_lo, in_hi, out_lo, out_hi, expected) in REMAP_GOLDEN {
        let args = format!("({x}, [{in_lo}, {in_hi}], [{out_lo}, {out_hi}])");
        assert_eq!(oracle_remap(x, in_lo, in_hi, out_lo, out_hi), expected, "oracle remap{args}");
        assert_eq!(remap(x, in_lo, in_hi, out_lo, out_hi), expected, "remap{args}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_027_remap_matches_the_oracle_on_the_grid() {
    let values: Vec<i64> = grid().into_iter().step_by(4).collect();
    assert_eq!(values.len(), 16);
    let mut checked = 0_u32;
    for &x in &values {
        for &in_lo in &values {
            for &in_hi in &values {
                for &out_lo in &values {
                    for &out_hi in &values {
                        let expected = oracle_remap(x, in_lo, in_hi, out_lo, out_hi);
                        let actual = remap(x, in_lo, in_hi, out_lo, out_hi);
                        assert_eq!(
                            actual, expected,
                            "remap({x}, [{in_lo}, {in_hi}], [{out_lo}, {out_hi}])"
                        );
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 16 * 16 * 16 * 16 * 16);
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_028_seeded_remaps_match_the_oracle_and_lerp() {
    println!("lerp_tp_028 seed={SEED:#018x} cases=100000");
    let mut state = SEED ^ 0x28;
    let (mut defined, mut overflowed, mut ties) = (0_u32, 0_u32, 0_u32);
    for case in 0..100_000_u32 {
        let [x, in_lo, out_lo, out_hi] = [0; 4].map(|_| spread(&mut state));
        let in_hi = match case % 4 {
            0 => in_lo.wrapping_add(2 << (next_seed(&mut state) % 8)), // small even runs: ties
            1 => in_lo,                                                // degenerate
            _ => spread(&mut state),
        };
        let expected = oracle_remap(x, in_lo, in_hi, out_lo, out_hi);
        let args = format!(
            "seed={SEED:#018x} case={case} ({x}, [{in_lo}, {in_hi}], [{out_lo}, {out_hi}])"
        );
        assert_eq!(remap(x, in_lo, in_hi, out_lo, out_hi), expected, "{args}");
        assert_eq!(remap(x, 0, U, out_lo, out_hi), lerp(out_lo, out_hi, x), "lerp {args}");
        assert_eq!(remap(x, in_lo, in_hi, 0, U), inverse(in_lo, in_hi, x), "inverse {args}");
        ties += u32::from(remap_is_tie(x, in_lo, in_hi, out_lo, out_hi));
        defined += u32::from(expected.is_some());
        overflowed += u32::from(in_lo != in_hi && expected.is_none());
    }
    println!("lerp_tp_028 defined={defined} overflowed={overflowed} ties={ties}");
    assert!(defined >= 10_000 && overflowed >= 1_000 && ties >= 1_000, "every class is reached");
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_029_remap_ends_are_exact_and_the_map_is_monotone() {
    let mut values = grid();
    values.sort_unstable();
    let small: Vec<i64> = values.iter().copied().step_by(8).collect();
    for &in_lo in &values {
        for &in_hi in &small {
            for &out_lo in &small {
                for &out_hi in &values {
                    let live = in_lo != in_hi;
                    let ends = [
                        remap(in_lo, in_lo, in_hi, out_lo, out_hi),
                        remap(in_hi, in_lo, in_hi, out_lo, out_hi),
                    ];
                    assert_eq!(
                        ends,
                        [live.then_some(out_lo), live.then_some(out_hi)],
                        "ends ({in_lo}, {in_hi})"
                    );
                }
            }
        }
    }
    for &in_lo in &small {
        for &in_hi in &small {
            for &out_lo in &small {
                for &out_hi in &small {
                    let outputs: Vec<i64> = values
                        .iter()
                        .filter_map(|&x| remap(x, in_lo, in_hi, out_lo, out_hi))
                        .collect();
                    let rising = (out_hi >= out_lo) == (in_hi >= in_lo);
                    for pair in outputs.windows(2) {
                        let ordered = if rising { pair[0] <= pair[1] } else { pair[0] >= pair[1] };
                        assert!(
                            ordered,
                            "remap(x, [{in_lo}, {in_hi}], [{out_lo}, {out_hi}]) not monotone"
                        );
                    }
                }
            }
        }
    }
}
