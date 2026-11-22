//! Equivalence tests: the table-seeded integer square root returns the same bits as the
//! bit-by-bit reference it replaced.

use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const SQRT_EQUIVALENCE_SEED: u64 = 0xB002_5051_0000_0001;
const SQRT_EQUIVALENCE_CASES: usize = 20_000;

/// The digit-by-digit square root Bunny used through v0.6.0, kept as the reference oracle.
fn reference_sqrt_u128(val: u128) -> u128 {
    if val == 0 {
        return 0;
    }
    let mut op = val;
    let mut res = 0_u128;
    let mut one = 1_u128 << 126;
    one >>= val.leading_zeros() & !1;
    while one != 0 {
        if op >= res + one {
            op -= res + one;
            res = (res >> 1) + one;
        } else {
            res >>= 1;
        }
        one >>= 2;
    }
    res
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Inputs spread across every magnitude: a random value shifted right by a random amount.
fn spread_u128(state: &mut u64) -> u128 {
    let wide = (u128::from(next_seed(state)) << 64) | u128::from(next_seed(state));
    wide >> (next_seed(state) % 128)
}

#[wasm_bindgen_test(unsupported = test)]
fn sqrt_u128_matches_reference_on_boundaries() {
    let mut cases = vec![0_u128, 1, 2, 3, 4, 5, u128::MAX, u128::MAX - 1, 3 * (1_u128 << 126)];
    for bit in 0..128 {
        let power = 1_u128 << bit;
        cases.extend([power, power - 1, power.saturating_add(1)]);
    }
    for root in [1_u128, 2, 3, 0xFFFF_FFFF, 0x1_0000_0000, u128::from(u64::MAX)] {
        let square = root * root;
        cases.extend([square - 1, square, square + 1]);
    }
    for value in cases {
        assert_eq!(FixedQ32_32::sqrt_u128(value), reference_sqrt_u128(value), "value={value}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sqrt_u128_matches_reference_on_seeded_inputs() {
    let mut state = SQRT_EQUIVALENCE_SEED;
    for case in 0..SQRT_EQUIVALENCE_CASES {
        let value = spread_u128(&mut state);
        assert_eq!(
            FixedQ32_32::sqrt_u128(value),
            reference_sqrt_u128(value),
            "seed={SQRT_EQUIVALENCE_SEED:#018x}; case={case}; value={value}"
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn sqrt_matches_reference_for_every_small_raw_value() {
    for raw in 0_i64..=65_536 {
        let expected = reference_sqrt_u128(u128::from(raw.unsigned_abs()) << 32);
        let actual = FixedQ32_32::from_raw(raw).sqrt().map(FixedQ32_32::raw);
        assert_eq!(actual, i64::try_from(expected).ok(), "raw={raw}");
    }
}
