//! Look rotation contract tests (`bunny-look-rotation/1`): a hand-derived
//! quarter turn, handedness, a pinned diagonal golden, typed refusals one raw
//! unit either side of each threshold, and a seeded orthonormality bound.
//!
//! Every oracle here is exact `i128` arithmetic written in this file. The
//! diagonal golden and the seeded digest come from a separate Python model of
//! the documented rule, which also checked every result against the exact
//! geometry in `Decimal` at 80 digits (see `docs/topics/look-rotation/test-plan.md`).

use bunny_linalg::{FixedMat3, FixedVec3, LookAtError};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const SEED: u64 = 0xB002_0310_0000_0001;
const CASES: usize = 10_000;
/// Results the Python model gives for the seeded corpus.
const EXPECTED_OK: usize = 9_798;
const EXPECTED_PARALLEL_UP: usize = 202;
const EXPECTED_DIGEST: u64 = 0xF8FC_3439_149D_2CF2;
/// Documented bounds, in raw units.
const COLUMN_LENGTH_BOUND: i128 = 2;
const COLUMN_DOT_BOUND: i128 = 2;
const ROW_LENGTH_BOUND: i128 = 3;
const ROW_DOT_BOUND: i128 = 4;

fn raw_vec(x: i64, y: i64, z: i64) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn v(x: i64, y: i64, z: i64) -> FixedVec3 {
    raw_vec(x * ONE, y * ONE, z * ONE)
}

fn rotation(forward: FixedVec3, up: FixedVec3) -> FixedMat3 {
    FixedMat3::try_look_rotation(forward, up).expect("valid look rotation")
}

fn rows(m: FixedMat3) -> [[i64; 3]; 3] {
    let row = |r: FixedVec3| [r.x.raw(), r.y.raw(), r.z.raw()];
    [row(m.row0()), row(m.row1()), row(m.row2())]
}

fn columns(m: FixedMat3) -> [[i64; 3]; 3] {
    rows(m.transpose())
}

fn dot(a: [i64; 3], b: [i64; 3]) -> i128 {
    (0..3).map(|i| i128::from(a[i]) * i128::from(b[i])).sum()
}

/// `| |a| - 1 | <= bound` raw, decided exactly on squares.
fn length_within(a: [i64; 3], bound: i128) -> bool {
    let one = i128::from(ONE);
    let square = dot(a, a);
    (one - bound) * (one - bound) <= square && square <= (one + bound) * (one + bound)
}

/// `|a . b| <= bound` raw, decided exactly: `|sum a_i b_i| <= bound * 2^32`.
fn dot_within(a: [i64; 3], b: [i64; 3], bound: i128) -> bool {
    dot(a, b).abs() <= bound * i128::from(ONE)
}

#[wasm_bindgen_test(unsupported = test)]
fn lr_tp_001_forward_x_with_up_y_is_the_hand_derived_quarter_turn() {
    // right = forward x up = (1,0,0) x (0,1,0) = (0,0,1); true up = right x forward
    // = (0,1,0); back = (-1,0,0). Columns (right, up, back), so rows below:
    // a -90 degree turn about +Y.
    let quarter = [[0, 0, -ONE], [0, ONE, 0], [ONE, 0, 0]];
    assert_eq!(rows(rotation(v(1, 0, 0), v(0, 1, 0))), quarter);
    // Only the direction matters: a longer forward, a longer up, a 1-raw forward.
    assert_eq!(rows(rotation(v(5, 0, 0), v(0, 7, 0))), quarter);
    assert_eq!(rows(rotation(raw_vec(1, 0, 0), v(0, 1, 0))), quarter);
    // Forward -Z with up +Y is the identity: local -Z already points forward.
    assert_eq!(rotation(v(0, 0, -1), v(0, 1, 0)), FixedMat3::IDENTITY);
    // Forward +Z turns half way about +Y.
    assert_eq!(rows(rotation(v(0, 0, 1), v(0, 1, 0))), [[-ONE, 0, 0], [0, ONE, 0], [0, 0, -ONE]]);
}

#[wasm_bindgen_test(unsupported = test)]
fn lr_tp_002_local_axes_map_onto_right_up_and_forward() {
    let m = rotation(v(1, 0, 0), v(0, 1, 0));
    // Local -Z goes to forward, +Y to up, +X to right (forward x up).
    assert_eq!(m.checked_mul_vec3(v(0, 0, -1)), Some(v(1, 0, 0)));
    assert_eq!(m.checked_mul_vec3(v(0, 1, 0)), Some(v(0, 1, 0)));
    assert_eq!(m.checked_mul_vec3(v(1, 0, 0)), Some(v(0, 0, 1)));
    // A proper rotation, not a reflection.
    assert_eq!(m.determinant(), Some(FixedQ32_32::ONE));
    // Looking straight down with up -Z: right = (0,-1,0) x (0,0,-1) = (1,0,0),
    // true up = (1,0,0) x (0,-1,0) = (0,0,-1). A -90 degree turn about +X.
    let down = rotation(v(0, -1, 0), v(0, 0, -1));
    assert_eq!(rows(down), [[ONE, 0, 0], [0, 0, ONE], [0, -ONE, 0]]);
    assert_eq!(down.determinant(), Some(FixedQ32_32::ONE));
}

#[wasm_bindgen_test(unsupported = test)]
fn lr_tp_003_diagonal_forward_has_the_pinned_golden_matrix() {
    // forward (1, 2, 3), up +Y. Ideal columns (Decimal reference, raw units):
    // forward 1147878293.502, 2295756587.004, 3443634880.507;
    // right -4074563739.270, 0, 1358187913.484. Each entry is within 1/2 raw.
    assert_eq!(
        rows(rotation(v(1, 2, 3), v(0, 1, 0))),
        [
            [-4_074_563_739, -725_981_977, -1_147_878_294],
            [0, 3_629_909_884, -2_295_756_587],
            [1_358_187_913, -2_177_945_930, -3_443_634_881],
        ]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn lr_tp_004_degenerate_inputs_are_refused_one_raw_unit_from_the_threshold() {
    let look = FixedMat3::try_look_rotation;
    let y = v(0, 1, 0);
    // Forward is refused only when it is exactly zero; one raw unit is a direction.
    assert_eq!(look(v(0, 0, 0), y), Err(LookAtError::CoincidentEyeTarget));
    assert_eq!(look(raw_vec(0, 0, -1), y), Ok(FixedMat3::IDENTITY));
    assert!(look(raw_vec(0, 0, 1), y).is_ok());
    // A zero up hint is refused; one raw unit is a direction.
    assert_eq!(look(v(0, 0, -1), v(0, 0, 0)), Err(LookAtError::ParallelUp));
    assert_eq!(look(v(0, 0, -1), raw_vec(0, 1, 0)), Ok(FixedMat3::IDENTITY));
    // Up exactly parallel or antiparallel to forward, at any scale, is refused.
    let forward = raw_vec(3, 6, 9);
    for up in [raw_vec(1, 2, 3), raw_vec(-1, -2, -3), v(1, 2, 3), raw_vec(-7, -14, -21)] {
        assert_eq!(look(forward, up), Err(LookAtError::ParallelUp));
    }
    let low = raw_vec(i64::MIN, i64::MIN, i64::MIN);
    assert_eq!(look(low, low), Err(LookAtError::ParallelUp));
    assert_eq!(look(low, raw_vec(1, 1, 1)), Err(LookAtError::ParallelUp));
    // One raw unit off parallel is accepted.
    assert!(look(forward, raw_vec(1, 2, 4)).is_ok());
    let hint = raw_vec(1, 2, 3);
    assert_eq!(look(raw_vec(1_000_000, 2_000_000, 3_000_000), hint), Err(LookAtError::ParallelUp));
    assert!(look(raw_vec(1_000_000, 2_000_000, 3_000_001), hint).is_ok());
    // The rounded forward can be exactly parallel to up when the input is not:
    // (2^40, 2^40 + 1, 0) rounds to (s, s, 0), s = 3037000500. That is refused,
    // and an up one raw unit away from (s, s, 0)'s direction is accepted.
    let n = 1_i64 << 40;
    let near_diagonal = raw_vec(n, n + 1, 0);
    assert_ne!(dot([n, n + 1, 0], [-ONE, ONE, 0]), 0, "input is not parallel to (1, 1, 0)");
    assert_eq!(look(near_diagonal, raw_vec(ONE, ONE, 0)), Err(LookAtError::ParallelUp));
    assert!(look(near_diagonal, raw_vec(ONE, ONE + 1, 0)).is_ok());
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn next64(state: &mut u64) -> u64 {
    let (a, b, c) = (next(state), next(state), next(state));
    (a << 33) ^ (b << 2) ^ (c & 3)
}

/// A signed raw value with a random bit width from 1 to 63.
fn wide(state: &mut u64) -> i64 {
    let bits = 1 + next(state) % 63;
    let magnitude = i64::try_from(next64(state) >> (64 - bits)).expect("63 bits fit");
    if next(state).is_multiple_of(2) {
        magnitude
    } else {
        -magnitude
    }
}

fn random_vec(state: &mut u64) -> FixedVec3 {
    let (x, y, z) = (wide(state), wide(state), wide(state));
    raw_vec(x, y, z)
}

/// FNV-1a over the result stream, matching the Python model byte for byte.
fn fnv(mut digest: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(0x100_0000_01B3);
    }
    digest
}

#[wasm_bindgen_test(unsupported = test)]
fn lr_tp_005_seeded_directions_are_orthonormal_within_the_documented_bound() {
    let mut state = SEED;
    let mut digest = 0xCBF2_9CE4_8422_2325;
    let (mut ok, mut parallel) = (0, 0);
    let pairs = [(0, 1), (0, 2), (1, 2)];
    for case in 0..CASES {
        let forward = random_vec(&mut state);
        let up = if case % 2 == 0 { random_vec(&mut state) } else { v(0, 1, 0) };
        let context = format!("seed={SEED:#x} case={case} forward={forward:?} up={up:?}");
        let m = match FixedMat3::try_look_rotation(forward, up) {
            Ok(m) => m,
            Err(LookAtError::ParallelUp) => {
                parallel += 1;
                digest = fnv(digest, &[2]);
                continue;
            }
            Err(other) => panic!("unexpected {other:?}; {context}"),
        };
        ok += 1;
        digest = fnv(digest, &[0]);
        for entry in rows(m).into_iter().flatten() {
            digest = fnv(digest, &entry.to_le_bytes());
        }
        let (cols, rws) = (columns(m), rows(m));
        for i in 0..3 {
            assert!(length_within(cols[i], COLUMN_LENGTH_BOUND), "column {i}; {context}");
            assert!(length_within(rws[i], ROW_LENGTH_BOUND), "row {i}; {context}");
        }
        for (a, b) in pairs {
            assert!(dot_within(cols[a], cols[b], COLUMN_DOT_BOUND), "columns {a}.{b}; {context}");
            assert!(dot_within(rws[a], rws[b], ROW_DOT_BOUND), "rows {a}.{b}; {context}");
        }
    }
    assert_eq!((ok, parallel), (EXPECTED_OK, EXPECTED_PARALLEL_UP), "seed={SEED:#x}");
    assert_eq!(digest, EXPECTED_DIGEST, "seed={SEED:#x}: results differ from the Python model");
}
