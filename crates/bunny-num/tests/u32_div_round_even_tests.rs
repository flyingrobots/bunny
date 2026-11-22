//! Contract tests for `bunny-u32-div-round-even/1` (`docs/topics/gpu-u32-div-round-even/test-plan.md`).

use bunny_num::{
    div_round_even_u32, div_round_even_u32_bytes, div_round_even_u32_words, DivisionByZero,
    U32_DIV_ROUND_EVEN_CONTRACT_ID, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO,
    U32_DIV_ROUND_EVEN_STATUS_OK,
};
use wasm_bindgen_test::wasm_bindgen_test;

/// 15 core golden vectors from Amendment 1 of `docs/design/0005-gpu-compute-backend.md`:
/// `(n, d, expected_value, expected_status)`.
const GOLDEN_VECTORS: &[(u32, u32, u32, u32)] = &[
    (0, 1, 0, U32_DIV_ROUND_EVEN_STATUS_OK),
    (u32::MAX, 1, u32::MAX, U32_DIV_ROUND_EVEN_STATUS_OK),
    (u32::MAX, u32::MAX, 1, U32_DIV_ROUND_EVEN_STATUS_OK),
    (1, 4, 0, U32_DIV_ROUND_EVEN_STATUS_OK),
    (3, 4, 1, U32_DIV_ROUND_EVEN_STATUS_OK),
    (1, 2, 0, U32_DIV_ROUND_EVEN_STATUS_OK),
    (3, 2, 2, U32_DIV_ROUND_EVEN_STATUS_OK),
    (5, 2, 2, U32_DIV_ROUND_EVEN_STATUS_OK),
    (7, 2, 4, U32_DIV_ROUND_EVEN_STATUS_OK),
    (u32::MAX, 2, 2_147_483_648, U32_DIV_ROUND_EVEN_STATUS_OK),
    (3_000_000_000, 4_000_000_000, 1, U32_DIV_ROUND_EVEN_STATUS_OK),
    (2_147_483_647, u32::MAX, 0, U32_DIV_ROUND_EVEN_STATUS_OK),
    (2_147_483_648, u32::MAX, 1, U32_DIV_ROUND_EVEN_STATUS_OK),
    (0, 0, 0, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO),
    (u32::MAX, 0, 0, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO),
];

/// Independent wide-integer (`u128`) oracle computing exact rational ties-to-even rounding.
fn u128_oracle_words(n: u32, d: u32) -> [u32; 2] {
    if d == 0 {
        return [0, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO];
    }
    let n128 = u128::from(n);
    let d128 = u128::from(d);
    let q = n128 / d128;
    let r = n128 % d128;
    let twice_r = r * 2;
    let rounded = if twice_r > d128 || (twice_r == d128 && (q & 1) != 0) { q + 1 } else { q };
    let value = u32::try_from(rounded).expect("rounded u32/u32 quotient must fit in u32");
    [value, U32_DIV_ROUND_EVEN_STATUS_OK]
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[wasm_bindgen_test(unsupported = test)]
fn udre_tp_001_golden_vectors_match_contract() {
    assert_eq!(U32_DIV_ROUND_EVEN_CONTRACT_ID, "bunny-u32-div-round-even/1");
    for &(n, d, exp_val, exp_status) in GOLDEN_VECTORS {
        assert_eq!(u128_oracle_words(n, d), [exp_val, exp_status], "oracle mismatch for {n}/{d}");
        assert_eq!(div_round_even_u32_words(n, d), [exp_val, exp_status], "words mismatch {n}/{d}");

        let mut expected_bytes = [0_u8; 8];
        expected_bytes[0..4].copy_from_slice(&exp_val.to_le_bytes());
        expected_bytes[4..8].copy_from_slice(&exp_status.to_le_bytes());
        assert_eq!(div_round_even_u32_bytes(n, d), expected_bytes, "bytes mismatch {n}/{d}");

        if exp_status == U32_DIV_ROUND_EVEN_STATUS_OK {
            assert_eq!(div_round_even_u32(n, d), Ok(exp_val), "Ok mismatch {n}/{d}");
        } else {
            assert_eq!(div_round_even_u32(n, d), Err(DivisionByZero), "Err mismatch {n}/{d}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn udre_tp_002_division_by_zero_returns_canonical_error() {
    for n in [0_u32, 1, 2, 42, i32::MAX as u32, (i32::MAX as u32) + 1, u32::MAX - 1, u32::MAX] {
        assert_eq!(div_round_even_u32(n, 0), Err(DivisionByZero));
        assert_eq!(div_round_even_u32_words(n, 0), [0, U32_DIV_ROUND_EVEN_STATUS_DIV_BY_ZERO]);
        assert_eq!(div_round_even_u32_bytes(n, 0), [0, 0, 0, 0, 1, 0, 0, 0]);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn udre_tp_003_boundary_grid_and_large_ties_match_u128_oracle() {
    let edges: &[u32] = &[
        0,
        1,
        2,
        3,
        4,
        5,
        7,
        8,
        15,
        16,
        255,
        65_535,
        65_536,
        i32::MAX as u32,
        (i32::MAX as u32) + 1,
        3_000_000_000,
        4_000_000_000,
        u32::MAX - 1,
        u32::MAX,
    ];
    for &n in edges {
        for &d in edges {
            assert_eq!(div_round_even_u32_words(n, d), u128_oracle_words(n, d), "edge {n}/{d}");
        }
    }

    // Large even-denominator exact ties where 2 * r overflows u32
    for step in 0_u32..64 {
        let d = u32::MAX - 1 - step * 2;
        let half = d / 2;
        for q in 0_u32..=3 {
            if let Some(n) = q.checked_mul(d).and_then(|v| v.checked_add(half)) {
                assert_eq!(div_round_even_u32_words(n, d), u128_oracle_words(n, d), "tie {n}/{d}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn udre_tp_004_seeded_property_sweep_matches_u128_oracle() {
    let mut state = 0xB002_0005_0428_0001_u64;
    for _ in 0..50_000 {
        let raw = next_seed(&mut state);
        let n = (raw & 0xFFFF_FFFF) as u32;
        let d = (raw >> 32) as u32;
        assert_eq!(div_round_even_u32_words(n, d), u128_oracle_words(n, d), "seeded {n}/{d}");
    }
}
