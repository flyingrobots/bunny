//! Contract tests for `bunny-ease/1` (`docs/topics/ease/test-plan.md`).
//!
//! Oracle: `oracle_ease` below evaluates each ease from the definition in
//! flyingrobots/bunny#304 as one exact fraction over `2^(32(n-1))`, in a base-`2^32` natural
//! number written here (`Nat`), and rounds it to nearest with ties to even by
//! `floor((2v + 2^s) / 2^(s+1))`, moving an exact tie that landed on an odd value back by one.
//! It shares no helper with the kernel, which takes a reflection for "out" and the upper half
//! of "in-out", divides "in-out" by `2^(31(n-1))`, and rounds by a shift and a mask.
//! The two pinned digests and the golden raw values come from a separate Python
//! `fractions.Fraction` model (posted on the pull request), so the kernel, this oracle and that
//! model agree on every grid and seeded point. `U = 2^32` is one unit (`1.0`), `H = 2^31`.

// Base-2^32 digits are taken by truncating `as` casts on purpose, and raw `t` is in
// `[0, 2^32]` (or a seeded draw reduced below `2^32 + 1`) before every signed conversion.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use bunny_num::ease::{power_in, power_in_out, power_out, steps};
use bunny_num::{FixedQ32_32, EASE_CONTRACT_ID};
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0304_EA5E_0001;
/// FNV-1a of every grid output, function by function, from the Python model.
const GRID_DIGEST: u64 = 0xDCC5_EEFB_F191_7BEC;
/// FNV-1a of every seeded output, function by function, from the Python model.
const SEEDED_DIGEST: u64 = 0xA93B_D20B_D794_88FF;

#[derive(Clone, Copy, Debug)]
enum Kind {
    In,
    Out,
    InOut,
}

/// The twelve power eases, in the order the digests use.
const FUNCTIONS: [(Kind, u32); 12] = [
    (Kind::In, 2),
    (Kind::In, 3),
    (Kind::In, 4),
    (Kind::In, 5),
    (Kind::Out, 2),
    (Kind::Out, 3),
    (Kind::Out, 4),
    (Kind::Out, 5),
    (Kind::InOut, 2),
    (Kind::InOut, 3),
    (Kind::InOut, 4),
    (Kind::InOut, 5),
];

/// A natural number in base `2^32`, least significant digit first.
#[derive(Clone)]
struct Nat(Vec<u32>);

impl Nat {
    fn small(value: u64) -> Self {
        Self(vec![value as u32, (value >> 32) as u32])
    }

    fn power_of_two(bits: u32) -> Self {
        let mut digits = vec![0; bits as usize / 32 + 1];
        digits[bits as usize / 32] = 1 << (bits % 32);
        Self(digits)
    }

    fn mul(&self, other: &Self) -> Self {
        let mut digits = vec![0_u32; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.0.iter().enumerate() {
                let sum = u64::from(a) * u64::from(b) + u64::from(digits[i + j]) + carry;
                digits[i + j] = sum as u32;
                carry = sum >> 32;
            }
            digits[i + other.0.len()] = carry as u32;
        }
        Self(digits)
    }

    fn pow(&self, exponent: u32) -> Self {
        (0..exponent).fold(Self::small(1), |product, _| product.mul(self))
    }

    /// `self + other` or `self - other`, for `self >= other` when subtracting.
    fn combine(&self, other: &Self, subtract: bool) -> Self {
        let width = self.0.len().max(other.0.len()) + 1;
        let digit = |n: &Self, i: usize| i64::from(n.0.get(i).copied().unwrap_or(0));
        let (mut digits, mut carry) = (Vec::with_capacity(width), 0_i64);
        for i in 0..width {
            let other_digit = if subtract { -digit(other, i) } else { digit(other, i) };
            let sum = digit(self, i) + other_digit + carry;
            digits.push(sum.rem_euclid(1 << 32) as u32);
            carry = sum.div_euclid(1 << 32);
        }
        assert_eq!(carry, 0, "the oracle subtracted a larger number");
        Self(digits)
    }

    /// `(floor(self / 2^bits), self mod 2^bits == 0)`, the quotient below `2^64`.
    fn split(&self, bits: u32) -> (u64, bool) {
        let (whole, part) = (bits as usize / 32, bits % 32);
        let digit = |i: usize| u128::from(self.0.get(i).copied().unwrap_or(0));
        let window = digit(whole) | digit(whole + 1) << 32 | digit(whole + 2) << 64;
        let quotient = window >> part;
        assert!(quotient < 1 << 64 && self.0.iter().skip(whole + 3).all(|&d| d == 0), "wide");
        let low_zero =
            self.0.iter().take(whole).all(|&d| d == 0) && digit(whole) % (1 << part) == 0;
        (quotient as u64, low_zero)
    }

    /// `round_half_even(self / 2^shift)` as `floor((2v + 2^shift) / 2^(shift+1))`, a tie
    /// that landed on an odd value moved back by one.
    fn round_down_by(&self, shift: u32) -> i64 {
        let sum = self.combine(self, false).combine(&Self::power_of_two(shift), false);
        let (quotient, tie) = sum.split(shift + 1);
        let rounded = if tie && quotient % 2 == 1 { quotient - 1 } else { quotient };
        i64::try_from(rounded).unwrap_or(i64::MAX)
    }
}

/// The issue's definition at `t = r / 2^32`, rounded once; `r` in `[0, 2^32]`.
fn oracle_ease(kind: Kind, n: u32, r: i64) -> i64 {
    let (t, rest) = (Nat::small(r as u64), Nat::small((U - r) as u64));
    let whole = Nat::power_of_two(32 * n);
    let scale = Nat::power_of_two(n - 1);
    let numerator = match kind {
        Kind::In => t.pow(n),
        Kind::Out => whole.combine(&rest.pow(n), true),
        Kind::InOut if r < H => scale.mul(&t.pow(n)),
        Kind::InOut => whole.combine(&scale.mul(&rest.pow(n)), true),
    };
    numerator.round_down_by(32 * (n - 1))
}

/// `round_half_even(k / n)` for `k = floor(r n / 2^32)`, in `u128`.
fn oracle_steps(n: u32, r: i64) -> i64 {
    let (n, r) = (u128::from(n), r as u128);
    let k = r * n / (1 << 32);
    let sum = (k << 33) + n;
    let mut nearest = sum / (2 * n);
    if sum % (2 * n) == 0 && nearest % 2 == 1 {
        nearest -= 1;
    }
    nearest as i64
}

fn kernel(kind: Kind, n: u32, r: i64) -> Option<i64> {
    let t = FixedQ32_32::from_raw(r);
    match kind {
        Kind::In => power_in(n, t),
        Kind::Out => power_out(n, t),
        Kind::InOut => power_in_out(n, t),
    }
    .map(FixedQ32_32::raw)
}

fn step(n: u32, r: i64) -> Option<i64> {
    steps(n, FixedQ32_32::from_raw(r)).map(FixedQ32_32::raw)
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn seeded_t() -> Vec<i64> {
    let mut state = SEED;
    (0..100_000).map(|_| (next_seed(&mut state) % (U as u64 + 1)) as i64).collect()
}

fn grid_t() -> Vec<i64> {
    (0..=65_536).map(|k| k << 16).collect()
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

/// `(kind, n, raw t, expected raw)`: golden vectors for the WebAssembly job.
const EASE_GOLDEN: &[(Kind, u32, i64, i64)] = &[
    (Kind::In, 2, H, 1 << 30),            // 0.5^2 = 0.25
    (Kind::Out, 2, U / 4, 1_879_048_192), // 1 - 0.75^2 = 0.4375
    (Kind::InOut, 3, H, H),               // 4 (1/2)^3 = 1 - 4 (1/2)^3 = 0.5
    (Kind::In, 3, 3 << 21, 14),           // 27 2^63 / 2^64 = 13.5 -> 14 (even), not 13
    (Kind::In, 3, 1 << 21, 0),            // 2^63 / 2^64 = 0.5 -> 0 (even)
    (Kind::Out, 3, 3 << 21, 18_846_734),  // 2^32 - 2045^3 / 2 = 18 846 733.5 -> even
    (Kind::InOut, 2, 3 << 15, 4),         // 2 (9 2^30) / 2^32 = 4.5 -> 4 (even)
    (Kind::In, 5, U - 1, U - 5),          // (1 - e)^5 = 1 - 5e + 10e^2 - ..., e = 2^-32
    (Kind::Out, 5, 1, 5),                 // 1 - (1 - e)^5 = 5e - 10e^2 + ... -> 5 raw
    (Kind::InOut, 4, H - 1, H - 4),       // slope 4 at 1/2: 1/2 - 4e + 24e^2 - ... -> H - 4
    (Kind::InOut, 5, H + 1, H + 5),       // slope 5 at 1/2: 1/2 + 5e - 40e^2 + ... -> H + 5
    (Kind::In, 4, 0x1234_5678, 109_827),
    (Kind::Out, 4, 0xDEAD_BEEF, 4_293_734_509),
    (Kind::InOut, 5, 0x9ABC_DEF0, 3_629_514_417),
];

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_001_hand_derived_eases_and_ties_round_to_even() {
    assert_eq!(EASE_CONTRACT_ID, "bunny-ease/1");
    for &(kind, n, r, expected) in EASE_GOLDEN {
        assert_eq!(oracle_ease(kind, n, r), expected, "oracle {kind:?} {n} at {r}");
        assert_eq!(kernel(kind, n, r), Some(expected), "{kind:?} {n} at {r}");
    }
    // Both in-out branches give exactly one half at t = 1/2: 4 (1/2)^3 and 1 - 4 (1/2)^3.
    let half_cubed = Nat::small(H as u64).pow(3);
    let lower = Nat::power_of_two(2).mul(&half_cubed).round_down_by(64);
    let upper = Nat::power_of_two(96).combine(&Nat::power_of_two(2).mul(&half_cubed), true);
    assert_eq!((lower, upper.round_down_by(64)), (H, H));
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_002_the_domain_is_refused_outside_and_the_ends_are_exact() {
    for (kind, n) in FUNCTIONS {
        assert_eq!(kernel(kind, n, 0), Some(0), "{kind:?} {n} at 0");
        assert_eq!(kernel(kind, n, U), Some(U), "{kind:?} {n} at 1");
        for r in [-1, U + 1, i64::MIN, i64::MAX, -U] {
            assert_eq!(kernel(kind, n, r), None, "{kind:?} {n} at {r}");
        }
        for bad in [0, 1, 6, u32::MAX] {
            for r in [0, H, U] {
                assert_eq!(kernel(kind, bad, r), None, "{kind:?} n={bad} at {r}");
            }
        }
    }
    for n in [1, 2, 3, 7, u32::MAX] {
        assert_eq!((step(n, 0), step(n, U)), (Some(0), Some(U)), "steps({n}) ends");
        assert_eq!((step(n, -1), step(n, U + 1)), (None, None), "steps({n}) outside");
    }
    assert_eq!((step(0, 0), step(0, H), step(0, U)), (None, None, None));
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_003_the_grid_matches_the_oracle_exhaustively() {
    let grid = grid_t();
    for (kind, n) in FUNCTIONS {
        for &r in &grid {
            assert_eq!(kernel(kind, n, r), Some(oracle_ease(kind, n, r)), "{kind:?} {n} at {r}");
        }
    }
    let outputs = FUNCTIONS.iter().flat_map(|&(k, n)| grid.iter().map(move |&r| kernel(k, n, r)));
    assert_eq!(digest(outputs), GRID_DIGEST, "the Python model's grid digest");
    println!("ease_tp_003 functions=12 points=65537 cases={}", 12 * grid.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_004_seeded_and_boundary_points_match_the_oracle() {
    let seeded = seeded_t();
    println!("ease_tp_004 seed={SEED:#018x} seeded={}", seeded.len());
    let window = |centre: i64| (centre - 4096..=centre + 4096).filter(|r| (0..=U).contains(r));
    let boundary: Vec<i64> = window(0).chain(window(H)).chain(window(U)).collect();
    for (kind, n) in FUNCTIONS {
        for &r in seeded.iter().chain(&boundary) {
            assert_eq!(kernel(kind, n, r), Some(oracle_ease(kind, n, r)), "{kind:?} {n} at {r}");
        }
    }
    let outputs = FUNCTIONS.iter().flat_map(|&(k, n)| seeded.iter().map(move |&r| kernel(k, n, r)));
    assert_eq!(digest(outputs), SEEDED_DIGEST, "the Python model's seeded digest");
    println!(
        "ease_tp_004 boundary={} cases={}",
        boundary.len(),
        12 * (seeded.len() + boundary.len())
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_005_every_ease_is_monotone_non_decreasing() {
    // Each grid point and one raw unit either side, in order.
    let points: Vec<i64> =
        grid_t().iter().flat_map(|&r| [r - 1, r, r + 1]).filter(|r| (0..=U).contains(r)).collect();
    let mut equal_neighbours = 0;
    for (kind, n) in FUNCTIONS {
        let values: Vec<Option<i64>> = points.iter().map(|&r| kernel(kind, n, r)).collect();
        for pair in values.windows(2) {
            assert!(pair[0].is_some() && pair[0] <= pair[1], "{kind:?} {n}: {pair:?}");
            equal_neighbours += usize::from(pair[0] == pair[1]);
        }
    }
    // Not strict: near a flat end the rounding repeats values.
    assert_eq!((kernel(Kind::In, 5, 0), kernel(Kind::In, 5, 1)), (Some(0), Some(0)));
    assert_eq!((kernel(Kind::Out, 2, U - 1), kernel(Kind::Out, 2, U)), (Some(U), Some(U)));
    println!("ease_tp_005 points={} equal_neighbours={equal_neighbours}", points.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_006_reflections_symmetry_and_the_join_are_exact() {
    let points: Vec<i64> = grid_t().into_iter().chain(seeded_t()).collect();
    for n in 2..=5 {
        for &r in &points {
            let reflected = kernel(Kind::In, n, U - r).map(|v| U - v);
            assert_eq!(kernel(Kind::Out, n, r), reflected, "out({n}) at {r}");
            let mirrored = kernel(Kind::InOut, n, U - r).map(|v| U - v);
            assert_eq!(kernel(Kind::InOut, n, r), mirrored, "in-out({n}) at {r}");
        }
        assert_eq!(kernel(Kind::InOut, n, H), Some(H), "in-out({n}) at 1/2");
        // The join: one raw unit either side moves by at most the slope there, n raw units.
        for r in [H - 1, H + 1] {
            let moved = kernel(Kind::InOut, n, r).map(|v| (v - H).unsigned_abs());
            assert!(moved.is_some_and(|m| (1..=u64::from(n)).contains(&m)), "{n} at {r}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_007_steps_are_k_over_n_on_each_interval() {
    let third_up = 1_431_655_766; // the least raw t with 3 t >= 2^32
    let golden: &[(u32, i64, i64)] = &[
        (4, U / 4 - 1, 0),
        (4, U / 4, U / 4),
        (4, U - 1, 3 * U / 4),
        (2, H, H),
        (3, third_up - 1, 0),
        (3, third_up, 1_431_655_765), // round(2^32 / 3), just below t
        (1, U - 1, 0),
        (u32::MAX, U - 1, U - 1), // (2^32 - 2) / (2^32 - 1) = 1 - 1/(2^32 - 1) -> U - 1
        (u32::MAX, 1, 0),
        (u32::MAX, 2, 1), // 1 / (2^32 - 1) -> 1 raw
    ];
    for &(n, r, expected) in golden {
        assert_eq!(oracle_steps(n, r), expected, "oracle steps({n}) at {r}");
        assert_eq!(step(n, r), Some(expected), "steps({n}) at {r}");
    }
    let mut state = SEED ^ 0x5_7E95;
    println!("ease_tp_007 seed={state:#018x} cases=100000");
    for case in 0..100_000 {
        let n = ((next_seed(&mut state) as u32) >> (next_seed(&mut state) % 32)).max(1);
        let r = (next_seed(&mut state) % (U as u64 + 1)) as i64;
        assert_eq!(step(n, r), Some(oracle_steps(n, r)), "case {case}: steps({n}) at {r}");
        let next = (r + 1).min(U);
        assert!(step(n, r) <= step(n, next), "case {case}: steps({n}) at {r} and {next}");
    }
}
