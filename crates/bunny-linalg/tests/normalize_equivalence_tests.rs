//! The division-free `normalize` must return exactly what the previous implementation did:
//! the length, then each component divided by it with saturating division.

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const NORMALIZE_SEED: u64 = 0xB002_A0E3_0000_0001;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn spread(state: &mut u64) -> FixedQ32_32 {
    let raw = i64::from_ne_bytes(next_seed(state).to_ne_bytes());
    FixedQ32_32::from_raw(raw >> ((next_seed(state) >> 32) % 64))
}

fn reference_normalize3(v: FixedVec3) -> Option<FixedVec3> {
    let len = v.length()?;
    if len == FixedQ32_32::ZERO {
        None
    } else {
        Some(FixedVec3::new(v.x / len, v.y / len, v.z / len))
    }
}

fn reference_normalize2(v: FixedVec2) -> Option<FixedVec2> {
    let len = v.length()?;
    if len == FixedQ32_32::ZERO {
        None
    } else {
        Some(FixedVec2::new(v.x / len, v.y / len))
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn normalize3_matches_reference_on_seeded_vectors() {
    let mut state = NORMALIZE_SEED;
    let mut normalized = 0;
    for case in 0..100_000 {
        // Independent magnitudes per component, so lengths and ratios cover every scale.
        let v = FixedVec3::new(spread(&mut state), spread(&mut state), spread(&mut state));
        let expected = reference_normalize3(v);
        normalized += i32::from(expected.is_some());
        assert_eq!(v.normalize(), expected, "case={case}");
    }
    assert!(normalized > 90_000, "most cases normalize rather than overflow: {normalized}");
}

#[wasm_bindgen_test(unsupported = test)]
fn normalize2_matches_reference_on_seeded_vectors() {
    let mut state = NORMALIZE_SEED ^ 0x0002;
    for case in 0..100_000 {
        let v = FixedVec2::new(spread(&mut state), spread(&mut state));
        assert_eq!(v.normalize(), reference_normalize2(v), "case={case}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn normalize_matches_reference_on_axis_and_extreme_vectors() {
    let (zero, one) = (FixedQ32_32::ZERO, FixedQ32_32::ONE);
    let values = [
        zero,
        one,
        -one,
        FixedQ32_32::from_raw(1),
        FixedQ32_32::from_raw(-1),
        FixedQ32_32::from_raw(i64::MAX),
        FixedQ32_32::from_raw(i64::MIN),
        FixedQ32_32::from_raw(1 << 40),
    ];
    for &x in &values {
        for &y in &values {
            for &z in &values {
                let v = FixedVec3::new(x, y, z);
                assert_eq!(v.normalize(), reference_normalize3(v), "{v:?}");
            }
            let w = FixedVec2::new(x, y);
            assert_eq!(w.normalize(), reference_normalize2(w), "{w:?}");
        }
    }
}
