//! `FixedDivisor` must return exactly what `FixedQ32_32::checked_div` returns.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedDivisor, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const DIVISOR_SEED: u64 = 0xB002_D1F1_0000_0001;
const DIVISOR_CASES: usize = 100_000;
const HALF: i64 = ONE_RAW / 2;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A raw value of random magnitude (shifted right by up to 63 bits) and sign.
fn spread_raw(state: &mut u64) -> i64 {
    let raw = i64::from_ne_bytes(next_seed(state).to_ne_bytes());
    raw >> (next_seed(state) % 64)
}

fn assert_matches(dividend: i64, divisor: i64) {
    let (a, b) = (FixedQ32_32::from_raw(dividend), FixedQ32_32::from_raw(divisor));
    let prepared = FixedDivisor::new(b);
    assert_eq!(prepared.is_none(), divisor == 0, "divisor={divisor}");
    if let Some(prepared) = prepared {
        assert_eq!(
            prepared.checked_div(a),
            a.checked_div(b),
            "dividend={dividend} divisor={divisor}"
        );
        assert_eq!(prepared.divisor(), b, "divisor round trip {divisor}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn divisor_matches_checked_div_on_boundaries() {
    let edges = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        HALF - 1,
        HALF,
        HALF + 1,
        ONE_RAW - 1,
        ONE_RAW,
        ONE_RAW + 1,
        -ONE_RAW,
        2 * ONE_RAW,
        2 * ONE_RAW + 1,
        2 * ONE_RAW - 1,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        1 << 62,
        -(1 << 62),
        (1 << 32) - 1,
        1 << 31,
    ];
    for &dividend in &edges {
        for &divisor in &edges {
            assert_matches(dividend, divisor);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn divisor_matches_checked_div_for_every_power_of_two_divisor() {
    let mut state = DIVISOR_SEED ^ 0x0002;
    for bit in 0..63 {
        let divisor = 1_i64 << bit;
        for _ in 0..64 {
            let dividend = spread_raw(&mut state);
            assert_matches(dividend, divisor);
            assert_matches(dividend, -divisor);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn divisor_matches_checked_div_on_seeded_pairs() {
    let mut state = DIVISOR_SEED;
    for _ in 0..DIVISOR_CASES {
        let (dividend, divisor) = (spread_raw(&mut state), spread_raw(&mut state));
        assert_matches(dividend, divisor);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn divisor_matches_checked_div_on_exact_ties() {
    // With divisor = c * 2^33 and dividend = (2k + 1) * c, the exact quotient in raw units
    // is dividend * 2^32 / divisor = (2k + 1) / 2: always exactly halfway.
    let mut state = DIVISOR_SEED ^ 0x0003;
    let mut rounded_up = 0;
    for _ in 0..DIVISOR_CASES / 10 {
        let c = i64::try_from((next_seed(&mut state) >> 32) % (1 << 29)).expect("bounded") | 1;
        let k = i64::try_from((next_seed(&mut state) >> 32) % (1 << 20)).expect("bounded");
        let (dividend, divisor) = ((2 * k + 1) * c, c << 33);
        let quotient = FixedQ32_32::from_raw(dividend).checked_div(FixedQ32_32::from_raw(divisor));
        assert!(
            quotient.is_some_and(|q| q.raw() == k || q.raw() == k + 1),
            "a tie rounds to a neighbour"
        );
        rounded_up += i32::from(quotient.is_some_and(|q| q.raw() == k + 1));
        assert_matches(dividend, divisor);
        assert_matches(-dividend, divisor);
        assert_matches(dividend, -divisor);
    }
    assert!(rounded_up > 0, "ties to even round some halves up");
}
