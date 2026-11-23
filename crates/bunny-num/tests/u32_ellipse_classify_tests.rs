//! Contract tests for `bunny-u32-ellipse-classify/1`
//! (`docs/topics/gpu-u32-ellipse-classify/test-plan.md`).

use core::cmp::Ordering;

use bunny_num::{
    classify_ellipse_u32, classify_ellipse_u32_bytes, classify_ellipse_u32_words, EllipseClass,
    EllipseDomainError, U32_ELLIPSE_CLASSIFY_CONTRACT_ID, U32_ELLIPSE_MAX_COORD,
    U32_ELLIPSE_STATUS_OK, U32_ELLIPSE_STATUS_OUT_OF_DOMAIN,
};
use wasm_bindgen_test::wasm_bindgen_test;

/// Independent wide-integer (`u128`) oracle for exact ellipse classification.
fn u128_oracle_words(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> [u32; 2] {
    if dx > U32_ELLIPSE_MAX_COORD
        || dy > U32_ELLIPSE_MAX_COORD
        || semi_x > U32_ELLIPSE_MAX_COORD
        || semi_y > U32_ELLIPSE_MAX_COORD
    {
        return [0, U32_ELLIPSE_STATUS_OUT_OF_DOMAIN];
    }
    if semi_x == 0 || semi_y == 0 {
        let code = if dx == 0 && dy == 0 {
            EllipseClass::OnBoundary.code()
        } else {
            EllipseClass::Outside.code()
        };
        return [code, U32_ELLIPSE_STATUS_OK];
    }
    let p = u128::from(dx) * u128::from(semi_y);
    let q = u128::from(dy) * u128::from(semi_x);
    let r = u128::from(semi_x) * u128::from(semi_y);
    let lhs = p * p + q * q;
    let rhs = r * r;
    let class = match lhs.cmp(&rhs) {
        Ordering::Less => EllipseClass::Inside,
        Ordering::Equal => EllipseClass::OnBoundary,
        Ordering::Greater => EllipseClass::Outside,
    };
    [class.code(), U32_ELLIPSE_STATUS_OK]
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[wasm_bindgen_test(unsupported = test)]
fn uec_tp_001_golden_pythagorean_ties_and_neighbors_are_exact() {
    assert_eq!(U32_ELLIPSE_CLASSIFY_CONTRACT_ID, "bunny-u32-ellipse-classify/1");
    assert_eq!(U32_ELLIPSE_MAX_COORD, 46_340);

    let cases: &[(u32, u32, u32, u32, EllipseClass)] = &[
        // Pythagorean exact boundary ties & off-by-1 neighbors
        (3, 4, 5, 5, EllipseClass::OnBoundary),
        (3, 3, 5, 5, EllipseClass::Inside),
        (3, 5, 5, 5, EllipseClass::Outside),
        (5, 12, 13, 13, EllipseClass::OnBoundary),
        (15, 20, 25, 25, EllipseClass::OnBoundary),
        // Non-circular ellipse exact boundary tie: (6/10)^2 + (16/20)^2 = 0.36 + 0.64 = 1
        (6, 16, 10, 20, EllipseClass::OnBoundary),
        (6, 15, 10, 20, EllipseClass::Inside),
        (6, 17, 10, 20, EllipseClass::Outside),
        // Cardinal axis endpoints
        (0, 0, 10, 20, EllipseClass::Inside),
        (10, 0, 10, 20, EllipseClass::OnBoundary),
        (0, 20, 10, 20, EllipseClass::OnBoundary),
        (11, 0, 10, 20, EllipseClass::Outside),
        (0, 21, 10, 20, EllipseClass::Outside),
        // Max domain corner (p^2 + q^2 = 9,222,615,725,798,720,000 < 2^63)
        (46_340, 46_340, 46_340, 46_340, EllipseClass::Outside),
        (46_340, 0, 46_340, 46_340, EllipseClass::OnBoundary),
        (0, 46_340, 46_340, 46_340, EllipseClass::OnBoundary),
        // Scaled Pythagorean tie near max domain: (27804/46340)^2 + (37072/46340)^2 = (3/5)^2+(4/5)^2 = 1
        (27_804, 37_072, 46_340, 46_340, EllipseClass::OnBoundary),
        (27_803, 37_072, 46_340, 46_340, EllipseClass::Inside),
        (27_805, 37_072, 46_340, 46_340, EllipseClass::Outside),
    ];

    for &(dx, dy, a, b, expected) in cases {
        assert_eq!(u128_oracle_words(dx, dy, a, b), [expected.code(), U32_ELLIPSE_STATUS_OK]);
        assert_eq!(classify_ellipse_u32(dx, dy, a, b), Ok(expected), "({dx},{dy},{a},{b})");
        assert_eq!(
            classify_ellipse_u32_words(dx, dy, a, b),
            [expected.code(), U32_ELLIPSE_STATUS_OK]
        );
        let mut expected_bytes = [0_u8; 8];
        expected_bytes[0..4].copy_from_slice(&expected.code().to_le_bytes());
        expected_bytes[4..8].copy_from_slice(&U32_ELLIPSE_STATUS_OK.to_le_bytes());
        assert_eq!(classify_ellipse_u32_bytes(dx, dy, a, b), expected_bytes);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn uec_tp_002_zero_semiaxis_branch_precedes_raw_polynomial() {
    // Raw polynomial gives 0 == 0 (OnBoundary) on (0, 1, 0, 5) and (1, 0, 5, 0);
    // explicit branch MUST classify only (0, 0) as OnBoundary and all others as Outside.
    assert_eq!(classify_ellipse_u32(0, 0, 0, 5), Ok(EllipseClass::OnBoundary));
    assert_eq!(classify_ellipse_u32(0, 0, 5, 0), Ok(EllipseClass::OnBoundary));
    assert_eq!(classify_ellipse_u32(0, 0, 0, 0), Ok(EllipseClass::OnBoundary));
    assert_eq!(classify_ellipse_u32(0, 1, 0, 5), Ok(EllipseClass::Outside));
    assert_eq!(classify_ellipse_u32(1, 0, 5, 0), Ok(EllipseClass::Outside));
    assert_eq!(classify_ellipse_u32(1, 1, 0, 0), Ok(EllipseClass::Outside));
    assert_eq!(classify_ellipse_u32(0, 46_340, 0, 46_340), Ok(EllipseClass::Outside));
    assert_eq!(classify_ellipse_u32(46_340, 0, 46_340, 0), Ok(EllipseClass::Outside));
}

#[wasm_bindgen_test(unsupported = test)]
fn uec_tp_003_out_of_domain_returns_canonical_error() {
    let over = U32_ELLIPSE_MAX_COORD + 1;
    let bad_inputs = [
        (over, 0, 10, 10),
        (0, over, 10, 10),
        (0, 0, over, 10),
        (0, 0, 10, over),
        (over, 0, 0, 5),
        (0, 0, 0, over),
        (u32::MAX, u32::MAX, u32::MAX, u32::MAX),
    ];
    for (dx, dy, a, b) in bad_inputs {
        assert_eq!(classify_ellipse_u32(dx, dy, a, b), Err(EllipseDomainError));
        assert_eq!(classify_ellipse_u32_words(dx, dy, a, b), [0, U32_ELLIPSE_STATUS_OUT_OF_DOMAIN]);
        assert_eq!(classify_ellipse_u32_bytes(dx, dy, a, b), [0, 0, 0, 0, 1, 0, 0, 0]);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn uec_tp_004_seeded_property_sweep_matches_u128_oracle() {
    let mut state = 0xB002_0005_0430_0001_u64;
    let span = u64::from(U32_ELLIPSE_MAX_COORD) + 1;
    for _ in 0..50_000 {
        let r1 = next_seed(&mut state);
        let r2 = next_seed(&mut state);
        let dx = u32::try_from(r1 % span).expect("bounded by span");
        let dy = u32::try_from((r1 >> 32) % span).expect("bounded by span");
        let a = u32::try_from(r2 % span).expect("bounded by span");
        let b = u32::try_from((r2 >> 32) % span).expect("bounded by span");
        assert_eq!(
            classify_ellipse_u32_words(dx, dy, a, b),
            u128_oracle_words(dx, dy, a, b),
            "seeded ({dx},{dy},{a},{b})",
        );
    }
}
