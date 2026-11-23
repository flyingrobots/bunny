//! Contract tests for the `bunny-ease/1` sine eases (`docs/topics/ease/test-plan.md`).
//!
//! Oracle: `oracle_sin` below is the exact sine of `pi r / 2^33` (that is, `sin(t pi/2)` for
//! `t = r / 2^32`) in unsigned fixed point with 124 fractional bits, from `pi` by Machin's
//! formula and the Taylor series of `sin`. Every exact ease follows from it:
//! `1 - cos(t pi/2) = 1 - sin((1 - t) pi/2)`, `sin(t pi/2)`, and
//! `(1 - cos(t pi)) / 2 = sin(t pi/2)^2`. It shares no table, constant or helper with the
//! kernel, which reads `FixedAngle::sin_cos` (a compile-time table and cubic Hermite
//! interpolation). Each oracle value is within about `2^-90` raw units of the real number.
//! The two digests and the golden raw values come from a separate Python model (a port of
//! `sin_cos` and `decimal` sine at 40 digits, posted on the pull request), which also measured
//! the same worst cases. `U = 2^32` is one unit (`1.0`), `H = 2^31`.

// Raw `t` is in `[0, 2^32]` (or a seeded draw reduced below `2^32 + 1`) before every signed
// conversion, and every oracle quotient fits its target.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use bunny_num::ease::{sine_in, sine_in_out, sine_out};
use bunny_num::{FixedAngle, FixedQ32_32, EASE_CONTRACT_ID};
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0306_51E5_0001;
/// FNV-1a of every output at `t = k / 2^20`, function by function, from the Python model.
const GRID_DIGEST: u64 = 0x8EFC_4FCB_0CE6_8578;
/// FNV-1a of every seeded output, function by function, from the Python model.
const SEEDED_DIGEST: u64 = 0xEAA4_D12A_FCBD_499F;

/// The contract bound, derived from `sin_cos`'s one raw unit: one raw unit, in units of
/// `2^-92` raw (the oracle's resolution).
const BOUND: u128 = 1 << 92;

#[derive(Clone, Copy, Debug)]
enum Kind {
    In,
    Out,
    InOut,
}

const KINDS: [Kind; 3] = [Kind::In, Kind::Out, Kind::InOut];

// ---- Oracle: unsigned fixed point with 124 fractional bits -------------------------------

const FRAC: u32 = 124;
const ONE: u128 = 1 << FRAC;
const LOW: u128 = (1 << 64) - 1;

/// `floor(a b / 2^124)` for `a, b < 2^126`.
fn fx_mul(a: u128, b: u128) -> u128 {
    let (a1, a0, b1, b0) = (a >> 64, a & LOW, b >> 64, b & LOW);
    let low = a0 * b0;
    let middle = a1 * b0 + a0 * b1 + (low >> 64);
    ((a1 * b1) << (128 - FRAC)) + (middle >> (FRAC - 64))
}

/// `pi` by Machin's formula, `16 atan(1/5) - 4 atan(1/239)`, each by its series in `1/m`.
fn machin_pi() -> u128 {
    let atan_inverse = |m: u128| {
        let (mut power, mut sum, mut k) = (ONE / m, 0_i128, 0_u128);
        while power != 0 {
            let term = (power / (2 * k + 1)) as i128;
            sum += if k % 2 == 0 { term } else { -term };
            power /= m * m;
            k += 1;
        }
        sum as u128
    };
    16 * atan_inverse(5) - 4 * atan_inverse(239)
}

/// `sin(pi r / 2^33)` for `r` in `[0, 2^32]`, by the Taylor series at `x <= pi / 2`.
fn oracle_sin(pi: u128, r: i64) -> u128 {
    let x = (pi >> 33) * r as u128;
    let x2 = fx_mul(x, x);
    let (mut term, mut positive, mut negative, mut n) = (x, 0_u128, 0_u128, 1_u128);
    while term != 0 {
        if n % 4 == 1 {
            positive += term;
        } else {
            negative += term;
        }
        term = fx_mul(term, x2) / ((n + 1) * (n + 2));
        n += 2;
    }
    positive - negative
}

/// The exact ease from `s = sin(t pi/2)` and `c = cos(t pi/2)`, both with 124 fractional
/// bits, in units of `2^-92` raw (so `1.0` is `2^124`).
fn exact(kind: Kind, s: u128, c: u128) -> u128 {
    match kind {
        Kind::In => ONE.abs_diff(c), // c can exceed 1.0 by the series' last bit
        Kind::Out => s,
        Kind::InOut => fx_mul(s, s),
    }
}

/// `|kernel - exact|` in units of `2^-92` raw.
fn error(value: i64, exact: u128) -> u128 {
    ((value as u128) << 92).abs_diff(exact)
}

/// A distance in units of `2^-92` raw, as millionths of a raw unit.
fn micro(distance: u128) -> u128 {
    distance * 1_000_000 >> 92
}

// ---- Kernel access, digests and points ---------------------------------------------------

fn kernel(kind: Kind, r: i64) -> Option<i64> {
    let t = FixedQ32_32::from_raw(r);
    match kind {
        Kind::In => sine_in(t),
        Kind::Out => sine_out(t),
        Kind::InOut => sine_in_out(t),
    }
    .map(FixedQ32_32::raw)
}

fn value(kind: Kind, r: i64) -> i64 {
    kernel(kind, r).expect("inside the domain")
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn seeded_t() -> Vec<i64> {
    let mut state = SEED;
    (0..100_000).map(|_| (next_seed(&mut state) % (U as u64 + 1)) as i64).collect()
}

/// Every `t = k / 2^20`: 1 048 577 points.
fn grid_t() -> impl Iterator<Item = i64> {
    (0..=1 << 20).map(|k: i64| k << 12)
}

/// FNV-1a over little-endian raw values; a refused value hashes as `-1`.
fn digest(values: impl Iterator<Item = Option<i64>>) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for value in values {
        for byte in value.unwrap_or(-1).to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01B3);
        }
    }
    hash
}

/// The kernel's form without the reflections: `sin(r 2^30)` and
/// `half_even(U - cos(r 2^31))` read straight from `sin_cos`, for every `r`.
fn direct(kind: Kind, r: i64) -> i64 {
    let quarter = FixedAngle::from_raw((r as u64) << 30).sin_cos();
    let half = FixedAngle::from_raw((r as u64) << 31).cos().raw();
    match kind {
        Kind::In => U - quarter.1.raw(),
        Kind::Out => quarter.0.raw(),
        Kind::InOut => {
            let m = U - half;
            (m >> 1) + (m & (m >> 1) & 1)
        }
    }
}

/// `(raw t, sine_in, sine_out, sine_in_out)` raw: golden vectors for the WebAssembly job,
/// from the Python model.
const GOLDEN: &[(i64, i64, i64, i64)] = &[
    (0, 0, 0, 0), // the ends are exact
    (U, U, U, U),
    (H, 1_257_966_796, 3_037_000_500, H), // 1 - cos(pi/4), sin(pi/4), exactly 1/2
    (H - 1, 1_257_966_795, 3_037_000_499, H - 2), // in-out slope pi/2 at 1/2
    (H + 1, 1_257_966_797, 3_037_000_501, H + 2), //
    (1, 0, 2, 0),                         // 1 raw: sin(pi/2^33) = 1.57 raw
    (U - 1, U - 2, U, U),                 // 1 - cos(pi/2 - e) = 1 - 1.57 raw
    (U / 4, 326_934_918, 1_643_612_827, 628_983_398), // 1 - cos(pi/8), sin(pi/8), ...
    (3 * U / 4, 2_651_354_469, 3_968_032_378, 3_665_983_898),
    (0x1234_5678, 26_766_583, 478_755_417, 53_366_355),
    (0xDEAD_BEEF, 3_422_936_595, 4_205_509_069, 4_117_914_134),
    (0x9ABC_DEF0, 1_794_768_155, 3_492_241_161, 2_839_543_934),
];

/// `(raw t, U - cos(t pi), sine_in_out)`: the halving meets an exact tie and goes to even.
const TIES: &[(i64, i64, i64)] = &[
    (24_576, 1, 0), // 1/2 -> 0, not 1 (half up)
    (49_152, 3, 2), // 3/2 -> 2, not 1 (truncation)
    (77_824, 7, 4), // 7/2 -> 4
    (65_536, 5, 2), // 5/2 -> 2
];

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_040_golden_vectors_and_the_halving_tie_go_to_even() {
    assert_eq!(EASE_CONTRACT_ID, "bunny-ease/1");
    let pi = machin_pi();
    for &(r, expected_in, expected_out, expected_in_out) in GOLDEN {
        let (s, c) = (oracle_sin(pi, r), oracle_sin(pi, U - r));
        for (kind, expected) in KINDS.into_iter().zip([expected_in, expected_out, expected_in_out])
        {
            assert_eq!(kernel(kind, r), Some(expected), "{kind:?} at {r}");
            assert!(error(expected, exact(kind, s, c)) <= BOUND, "oracle {kind:?} at {r}");
        }
    }
    for &(r, twice, expected) in TIES {
        assert_eq!(U - FixedAngle::from_raw((r as u64) << 31).cos().raw(), twice, "at {r}");
        assert_eq!(kernel(Kind::InOut, r), Some(expected), "in-out tie at {r}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_041_the_domain_is_refused_outside_and_the_ends_and_middle_are_exact() {
    // sin_cos is exact at every multiple of a quarter turn (AT-TP-002); the eases read it there.
    let cardinal = |raw: u64| {
        let (s, c) = FixedAngle::from_raw(raw).sin_cos();
        (s.raw(), c.raw())
    };
    assert_eq!((cardinal(0), cardinal(1 << 62), cardinal(1 << 63)), ((0, U), (U, 0), (0, -U)));
    for kind in KINDS {
        assert_eq!((kernel(kind, 0), kernel(kind, U)), (Some(0), Some(U)), "{kind:?} ends");
        for r in [-1, U + 1, i64::MIN, i64::MAX, -U, 2 * U] {
            assert_eq!(kernel(kind, r), None, "{kind:?} at {r}");
        }
    }
    assert_eq!(kernel(Kind::InOut, H), Some(H), "sine_in_out(1/2)");
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_042_the_sweep_over_every_k_over_2_20_is_within_one_raw_unit() {
    let pi = machin_pi();
    let mut worst = [(0_u128, 0_i64); 3];
    // Point k needs sin(k) and sin(2^20 - k); point 2^20 - k needs the same two.
    for k in 0..=1_i64 << 19 {
        let (a, b) = (oracle_sin(pi, k << 12), oracle_sin(pi, U - (k << 12)));
        for (r, s, c) in [(k << 12, a, b), (U - (k << 12), b, a)] {
            for (slot, kind) in worst.iter_mut().zip(KINDS) {
                let distance = error(value(kind, r), exact(kind, s, c));
                assert!(distance <= BOUND, "{kind:?} at {r}: {} micro-raw", micro(distance));
                if distance > slot.0 {
                    *slot = (distance, r);
                }
            }
        }
    }
    for (kind, (distance, r)) in KINDS.into_iter().zip(worst) {
        println!(
            "ease_tp_042 {kind:?} points=1048577 max_error_micro_raw={} at_raw_t={r} k={}",
            micro(distance),
            r >> 12
        );
    }
    let outputs = KINDS.into_iter().flat_map(|kind| grid_t().map(move |r| kernel(kind, r)));
    assert_eq!(digest(outputs), GRID_DIGEST, "the Python model's grid digest");
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_043_seeded_and_boundary_points_are_within_one_raw_unit() {
    let pi = machin_pi();
    let seeded = seeded_t();
    println!("ease_tp_043 seed={SEED:#018x} seeded={}", seeded.len());
    let window = |centre: i64| (centre - 4096..=centre + 4096).filter(|r| (0..=U).contains(r));
    let boundary: Vec<i64> = window(0).chain(window(H)).chain(window(U)).collect();
    let mut worst = [0_u128; 3];
    for &r in seeded.iter().chain(&boundary) {
        let (s, c) = (oracle_sin(pi, r), oracle_sin(pi, U - r));
        for (slot, kind) in worst.iter_mut().zip(KINDS) {
            let distance = error(value(kind, r), exact(kind, s, c));
            assert!(distance <= BOUND, "{kind:?} at {r}: {} micro-raw", micro(distance));
            *slot = (*slot).max(distance);
        }
    }
    let outputs = KINDS.into_iter().flat_map(|kind| seeded.iter().map(move |&r| kernel(kind, r)));
    assert_eq!(digest(outputs), SEEDED_DIGEST, "the Python model's seeded digest");
    println!(
        "ease_tp_043 boundary={} cases={} max_error_micro_raw={:?}",
        boundary.len(),
        3 * (seeded.len() + boundary.len()),
        worst.map(micro)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_044_every_sine_ease_is_monotone_non_decreasing() {
    // Every t = k / 2^20, then every t = k / 2^16 and one raw unit either side.
    let fine: Vec<i64> = grid_t().collect();
    let near: Vec<i64> = (0..=1_i64 << 16)
        .flat_map(|k| [(k << 16) - 1, k << 16, (k << 16) + 1])
        .filter(|r| (0..=U).contains(r))
        .collect();
    for kind in KINDS {
        let values: Vec<i64> = fine.iter().map(|&r| value(kind, r)).collect();
        let mut equal = Vec::new();
        for (k, pair) in values.windows(2).enumerate() {
            assert!(pair[0] <= pair[1], "{kind:?} at k={k}: {pair:?}");
            if pair[0] == pair[1] {
                equal.push(k);
            }
        }
        // Equal grid neighbours occur only where the slope is under 2^-12 per raw unit of t:
        // the flat start of in, the flat end of out, both ends of in-out.
        let flat = |k: &usize| *k < 128 || *k >= (1 << 20) - 128;
        assert!(equal.iter().all(flat), "{kind:?}: equal neighbours {equal:?}");
        let near_values: Vec<i64> = near.iter().map(|&r| value(kind, r)).collect();
        assert!(near_values.windows(2).all(|p| p[0] <= p[1]), "{kind:?} near grid");
        println!(
            "ease_tp_044 {kind:?} grid_points={} equal_grid_neighbours={} first={:?} last={:?} \
             near_points={}",
            fine.len(),
            equal.len(),
            equal.iter().filter(|&&k| k < 1 << 19).max(),
            equal.iter().filter(|&&k| k >= 1 << 19).min(),
            near.len()
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_045_reflections_and_the_direct_formulas_agree_bit_for_bit() {
    let points: Vec<i64> = grid_t().step_by(16).chain(seeded_t()).chain(0..=4096).collect();
    for &r in &points {
        assert_eq!(value(Kind::Out, r), U - value(Kind::In, U - r), "out reflection at {r}");
        assert_eq!(value(Kind::InOut, U - r), U - value(Kind::InOut, r), "in-out at {r}");
        for kind in KINDS {
            assert_eq!(value(kind, r), direct(kind, r), "{kind:?} direct formula at {r}");
        }
    }
    println!("ease_tp_045 points={}", points.len());
}
