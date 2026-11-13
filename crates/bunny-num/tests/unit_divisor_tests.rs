//! `UnitDivisor` must return exactly what `FixedQ32_32::checked_div` returns for dividends no
//! larger than the divisor, and `None` for larger ones.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedQ32_32, UnitDivisor};
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT_SEED: u64 = 0xB002_0D17_0000_0001;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn spread_raw(state: &mut u64) -> i64 {
    i64::from_ne_bytes(next_seed(state).to_ne_bytes()) >> ((next_seed(state) >> 32) % 64)
}

fn assert_matches(dividend: i64, divisor: i64) {
    let (a, b) = (FixedQ32_32::from_raw(dividend), FixedQ32_32::from_raw(divisor));
    let Some(prepared) = UnitDivisor::new(b) else {
        assert_eq!(divisor, 0);
        return;
    };
    let expected =
        if dividend.unsigned_abs() <= divisor.unsigned_abs() { a.checked_div(b) } else { None };
    assert_eq!(prepared.checked_div(a), expected, "dividend={dividend} divisor={divisor}");
}

#[wasm_bindgen_test(unsupported = test)]
fn unit_divisor_matches_checked_div_on_boundaries() {
    let edges = [
        0,
        1,
        -1,
        2,
        3,
        ONE_RAW / 2,
        ONE_RAW - 1,
        ONE_RAW,
        ONE_RAW + 1,
        -ONE_RAW,
        3 * ONE_RAW,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        1 << 62,
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
fn unit_divisor_matches_checked_div_on_seeded_pairs() {
    let mut state = UNIT_SEED;
    for _ in 0..200_000 {
        let (a, b) = (spread_raw(&mut state), spread_raw(&mut state));
        // Order them so most pairs are in range; out-of-range pairs check the `None` case.
        if next_seed(&mut state) >> 63 == 0 && a.unsigned_abs() > b.unsigned_abs() {
            assert_matches(b, a);
        } else {
            assert_matches(a, b);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn unit_divisor_matches_checked_div_for_every_divisor_bit_length() {
    let mut state = UNIT_SEED ^ 0x0002;
    for bits in 1..=63 {
        let top = 1_i64.checked_shl(bits - 1).unwrap_or(i64::MIN);
        for _ in 0..256 {
            let divisor = top | (spread_raw(&mut state) & (top - 1).max(0));
            let dividend = spread_raw(&mut state) % divisor.max(1);
            assert_matches(dividend, divisor);
            assert_matches(-dividend, divisor);
            assert_matches(divisor, divisor);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn unit_divisor_matches_checked_div_on_exact_ties() {
    // dividend = (2k + 1) * c and divisor = c * 2^33 give a quotient of exactly (2k + 1) / 2.
    let mut state = UNIT_SEED ^ 0x0003;
    for _ in 0..20_000 {
        let c = i64::try_from((next_seed(&mut state) >> 32) % (1 << 29)).expect("bounded") | 1;
        let k = i64::try_from((next_seed(&mut state) >> 32) % (1 << 20)).expect("bounded");
        assert_matches((2 * k + 1) * c, c << 33);
        assert_matches(-(2 * k + 1) * c, -(c << 33));
    }
}
