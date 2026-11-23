//! Contract tests for `bunny-lerp/1` in `bunny-linalg` (`docs/topics/lerp/test-plan.md`).
//!
//! Oracle: `oracle_lerp` computes each component's exact `a + (b - a) * t / 2^32` step in
//! `i128` and rounds the step to nearest with ties to even by Euclidean division; it shares no
//! code with `bunny-num`. `U = 2^32` is one unit (`1.0`) and `H = 2^31` is half of it.

use bunny_linalg::{lerp_channels, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0303_1E4F_0003;

fn oracle_lerp(a: i64, b: i64, t: i64) -> Option<i64> {
    let unit = 1_i128 << 32;
    let exact = (i128::from(b) - i128::from(a)) * i128::from(t);
    let (floor, twice_rest) = (exact.div_euclid(unit), 2 * exact.rem_euclid(unit));
    let up = i128::from(twice_rest > unit || (twice_rest == unit && floor.rem_euclid(2) == 1));
    i64::try_from(i128::from(a) + floor + up).ok()
}

fn fixed(raw: [i64; 3]) -> [FixedQ32_32; 3] {
    raw.map(FixedQ32_32::from_raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = fixed(raw);
    FixedVec3::new(x, y, z)
}

fn raw_of(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// The component-wise oracle: `None` when any component is `None`.
fn oracle3(a: [i64; 3], b: [i64; 3], t: i64) -> Option<[i64; 3]> {
    Some([oracle_lerp(a[0], b[0], t)?, oracle_lerp(a[1], b[1], t)?, oracle_lerp(a[2], b[2], t)?])
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn spread(state: &mut u64) -> i64 {
    let value = next_seed(state).cast_signed();
    value >> (next_seed(state) % 64)
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_008_vector_and_channel_lerps_are_component_wise() {
    // (0, -1.0, 1 raw) to (1.0, 1.0, 2 raw) at 0.5: 0.5, 0, and 1 + (1/2 -> 0) = 1 raw.
    let (a, b) = ([0, -U, 1], [U, U, 2]);
    let expected = [H, 0, 1];
    let t = FixedQ32_32::from_raw(H);
    assert_eq!(FixedVec3::lerp(vec3(a), vec3(b), t).map(raw_of), Some(expected));
    assert_eq!(lerp_channels(fixed(a), fixed(b), t), Some(fixed(expected)));
    // The ends are exact, and t = 2.0 extrapolates.
    for (t, expected) in [(0, a), (U, b), (2 * U, [2 * U, 3 * U, 3])] {
        let t = FixedQ32_32::from_raw(t);
        assert_eq!(FixedVec3::lerp(vec3(a), vec3(b), t).map(raw_of), Some(expected));
        assert_eq!(lerp_channels(fixed(a), fixed(b), t), Some(fixed(expected)));
    }
    // One overflowing component refuses the whole result, in each position.
    for position in 0..3 {
        let mut far = [U; 3];
        far[position] = i64::MAX;
        let t = FixedQ32_32::from_raw(2 * U);
        assert_eq!(FixedVec3::lerp(vec3([0; 3]), vec3(far), t), None, "position {position}");
        assert_eq!(lerp_channels(fixed([0; 3]), fixed(far), t), None, "position {position}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn lerp_tp_009_seeded_vectors_and_channels_match_the_oracle() {
    println!("lerp_tp_009 seed={SEED:#018x} cases=20000");
    let mut state = SEED;
    let (mut defined, mut refused) = (0_u32, 0_u32);
    for case in 0..20_000_u32 {
        let a = [spread(&mut state), spread(&mut state), spread(&mut state)];
        let b = [spread(&mut state), spread(&mut state), spread(&mut state)];
        let raw = next_seed(&mut state);
        let t =
            if case % 2 == 0 { (raw % (1 << 32 | 1)).cast_signed() } else { spread(&mut state) };
        let expected = oracle3(a, b, t);
        let ft = FixedQ32_32::from_raw(t);
        let context = format!("seed={SEED:#018x} case={case} a={a:?} b={b:?} t={t}");
        assert_eq!(FixedVec3::lerp(vec3(a), vec3(b), ft).map(raw_of), expected, "{context}");
        assert_eq!(lerp_channels(fixed(a), fixed(b), ft), expected.map(fixed), "{context}");
        if expected.is_some() {
            defined += 1;
        } else {
            refused += 1;
        }
    }
    println!("lerp_tp_009 defined={defined} refused={refused}");
    assert!(defined >= 5_000 && refused >= 1_000, "both classes are reached");
}
