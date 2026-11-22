//! Differential fidelity test for `try_look_at`: the 0.7.0 implementation is kept
//! verbatim in `look_at_reference` and compared on a large seeded corpus.
//!
//! Wherever the reference returns `Ok` and the inputs are not exactly parallel,
//! the new result must be bit-identical. The two corrected classes change on
//! purpose: exactly parallel inputs become `ParallelUp`, and a translation entry
//! whose finished value is representable no longer overflows.

mod look_at_reference;

use bunny_linalg::{FixedMat4, FixedVec3, LookAtError};
use bunny_num::FixedQ32_32;
use look_at_reference::reference_try_look_at;
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB002_0200_0000_0273;
const CASES: usize = 60_000;

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn next64(state: &mut u64) -> u64 {
    (next(state) << 33) ^ (next(state) << 2) ^ (next(state) & 3)
}

/// A value with a random bit width from 1 to `max_bits` and a random sign.
fn wide(state: &mut u64, max_bits: u64) -> i64 {
    let bits = 1 + next(state) % max_bits;
    let magnitude = i128::from(next64(state) >> (64 - bits));
    let signed = if next(state).is_multiple_of(2) { magnitude } else { -magnitude };
    i64::try_from(signed.clamp(i128::from(i64::MIN), i128::from(i64::MAX))).expect("clamped")
}

fn vec3(x: i64, y: i64, z: i64) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn random_vec(state: &mut u64, max_bits: u64) -> FixedVec3 {
    vec3(wide(state, max_bits), wide(state, max_bits), wide(state, max_bits))
}

fn axis_vec(state: &mut u64) -> FixedVec3 {
    let value = (1_i64 << 32) * [-3, -1, 1, 2][usize::try_from(next(state) % 4).expect("small")];
    match next(state) % 4 {
        0 => vec3(value, 0, 0),
        1 => vec3(0, value, 0),
        2 => vec3(0, 0, value),
        _ => vec3(0, 0, 0),
    }
}

fn components(v: FixedVec3) -> [i128; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()].map(i128::from)
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Exact parallelism, decided independently of the cross product: reduce each
/// vector by the gcd of its components and compare the primitive directions.
fn exactly_parallel(a: [i128; 3], b: [i128; 3]) -> bool {
    let primitive = |v: [i128; 3]| {
        let g = v.iter().fold(0, |acc, c| gcd(acc, c.unsigned_abs()));
        let g = i128::try_from(g).expect("fits");
        if g == 0 {
            [0; 3]
        } else {
            v.map(|c| c / g)
        }
    };
    let (pa, pb) = (primitive(a), primitive(b));
    pa == [0; 3] || pb == [0; 3] || pa == pb || pa == pb.map(|c| -c)
}

fn difference(target: FixedVec3, eye: FixedVec3) -> [i128; 3] {
    let (t, e) = (components(target), components(eye));
    [t[0] - e[0], t[1] - e[1], t[2] - e[2]]
}

fn corpus_case(state: &mut u64, index: usize) -> (FixedVec3, FixedVec3, FixedVec3) {
    match index % 5 {
        0 => (random_vec(state, 36), random_vec(state, 36), random_vec(state, 36)),
        1 => (axis_vec(state), axis_vec(state), axis_vec(state)),
        2 => (random_vec(state, 6), random_vec(state, 6), random_vec(state, 6)),
        3 => (random_vec(state, 64), random_vec(state, 64), random_vec(state, 64)),
        _ => {
            // Mostly parallel or nearly parallel up vectors.
            let (eye, target) = (random_vec(state, 40), random_vec(state, 40));
            let d = difference(target, eye);
            let k = i128::from(wide(state, 3));
            let nudge = i128::from(next(state) % 3) - 1;
            let up = d.map(|c| i64::try_from((c * k / 2 + nudge).clamp(-(1 << 62), 1 << 62)));
            (eye, target, vec3(up[0].unwrap_or(0), up[1].unwrap_or(0), up[2].unwrap_or(0)))
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_032_new_results_are_bit_identical_to_the_shipped_reference() {
    let mut state = SEED;
    let (mut identical, mut now_parallel, mut now_ok, mut errors_kept) = (0, 0, 0, 0);
    for index in 0..CASES {
        let (eye, target, up) = corpus_case(&mut state, index);
        let reference = reference_try_look_at(eye, target, up);
        let current = FixedMat4::try_look_at(eye, target, up);
        let parallel = exactly_parallel(difference(target, eye), components(up));
        let context = format!("case={index} eye={eye:?} target={target:?} up={up:?}");
        match (reference, current) {
            (Ok(old), Ok(new)) if !parallel => {
                assert_eq!(old, new, "{context}");
                identical += 1;
            }
            (Ok(_), Err(LookAtError::ParallelUp)) => {
                assert!(parallel, "{context}");
                now_parallel += 1;
            }
            (Err(LookAtError::Overflow), Ok(_)) => {
                assert!(!parallel, "{context}");
                now_ok += 1;
            }
            (old, new) => {
                assert!(old.is_err() && old == new || parallel, "{context}: {old:?} vs {new:?}");
                errors_kept += 1;
            }
        }
        // An exactly parallel input is never accepted.
        if parallel && difference(target, eye) != [0; 3] {
            assert_eq!(current, Err(LookAtError::ParallelUp), "{context}");
        }
    }
    assert!(identical > CASES / 3, "identical={identical}");
    assert!(now_parallel > 100, "now_parallel={now_parallel}");
    assert!(errors_kept > 0, "errors_kept={errors_kept}");
    println!(
        "identical={identical} now_parallel={now_parallel} now_ok={now_ok} errors={errors_kept}"
    );
}
