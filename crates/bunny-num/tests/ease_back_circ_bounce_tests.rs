//! Contract tests for `bunny-ease/1`, part 2: back, circ and bounce
//! (`docs/topics/ease/test-plan.md`, EASE-TP-020 to EASE-TP-030).
//!
//! Oracle: `oracle` below evaluates each ease from GSAP's definitions in
//! flyingrobots/bunny#305, sharing no helper with the kernel. Back is one signed numerator over
//! `2^96` built from the unexpanded forms `t^2 ((s + 1) t - s)` and
//! `(t - 1)^2 ((s + 1)(t - 1) + s) + 1` in base-`2^32` digits (`product`), and rounded to
//! nearest, ties to even, on those digits. Bounce is one `u128` fraction per piece, chosen by
//! comparing `275 t` with `100`, `200` and `250` (the breakpoints `1/2.75`, `2/2.75` and
//! `2.5/2.75`), and rounded by a division. Circ is the issue's rule, the floor of the exact
//! root, with its own bit-by-bit integer square root (`root`); the bound is proved separately
//! by squaring (`ease_tp_026`). The kernel instead uses expanded forms, reflections, a shift
//! and a mask, and core's `u128::isqrt`. The pinned digests and the golden raw values come from
//! a separate Python `fractions`/`decimal` model posted on the pull request. `U = 2^32` is one
//! unit (`1.0`), `H = 2^31`.

// Base-2^32 digits are taken by truncating `as` casts on purpose, and every raw value cast
// between signed and unsigned here is in range first.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use bunny_num::ease::{
    back_in, back_in_out, back_out, bounce_in, bounce_in_out, bounce_out, circ_in, circ_in_out,
    circ_out, BACK_DEFAULT_OVERSHOOT,
};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
/// GSAP's 1.70158 as the contract rounds it.
const S0: i64 = 7_308_230_452;
/// The issue's named input: rounding `1 - t^2` first errs by 54.58 raw units here.
const NAMED: i64 = 4_294_920_955;
const SEED: u64 = 0xB0_0305_EA5E_0002;
const PAIR_SEED: u64 = 0xB0_0305_BAC4_0003;
/// FNV-1a of every grid output, function by function, from the Python model.
const GRID_DIGEST: u64 = 0x209A_0C47_0F21_B5E3;
/// FNV-1a of every seeded output, function by function, from the Python model.
const SEEDED_DIGEST: u64 = 0x2417_EE12_F1CC_09DC;
/// FNV-1a of the three back eases at every seeded `(s, t)` pair, from the Python model.
const PAIR_DIGEST: u64 = 0xAF16_23D9_3C23_DD44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Back,
    Circ,
    Bounce,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    In,
    Out,
    InOut,
}

/// The nine eases, in the order the digests use; back takes `S0` there.
const FUNCTIONS: [(Family, Kind); 9] = [
    (Family::Back, Kind::In),
    (Family::Back, Kind::Out),
    (Family::Back, Kind::InOut),
    (Family::Circ, Kind::In),
    (Family::Circ, Kind::Out),
    (Family::Circ, Kind::InOut),
    (Family::Bounce, Kind::In),
    (Family::Bounce, Kind::Out),
    (Family::Bounce, Kind::InOut),
];

fn kernel(family: Family, kind: Kind, s: i64, r: i64) -> Option<i64> {
    let (s, t) = (FixedQ32_32::from_raw(s), FixedQ32_32::from_raw(r));
    let value = match (family, kind) {
        (Family::Back, Kind::In) => back_in(s, t),
        (Family::Back, Kind::Out) => back_out(s, t),
        (Family::Back, Kind::InOut) => back_in_out(s, t),
        (Family::Circ, Kind::In) => circ_in(t),
        (Family::Circ, Kind::Out) => circ_out(t),
        (Family::Circ, Kind::InOut) => circ_in_out(t),
        (Family::Bounce, Kind::In) => bounce_in(t),
        (Family::Bounce, Kind::Out) => bounce_out(t),
        (Family::Bounce, Kind::InOut) => bounce_in_out(t),
    };
    value.map(FixedQ32_32::raw)
}

/// `m k` in base-`2^32` digits, least significant first.
fn product(m: u128, k: u128) -> [u32; 8] {
    let digits = |v: u128| [v as u32, (v >> 32) as u32, (v >> 64) as u32, (v >> 96) as u32];
    let (a, b) = (digits(m), digits(k));
    let mut out = [0_u32; 8];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0_u64;
        for (j, &y) in b.iter().enumerate() {
            let sum = u64::from(x) * u64::from(y) + u64::from(out[i + j]) + carry;
            out[i + j] = sum as u32;
            carry = sum >> 32;
        }
        out[i + 4] = carry as u32;
    }
    out
}

/// `round_half_even(digits / 2^96)`: digits 3 to 4 are the quotient, 0 to 2 the remainder.
fn nearest_over_2_96(digits: [u32; 8]) -> i128 {
    assert!(digits[5..].iter().all(|&d| d == 0), "the oracle's quotient is too wide");
    let quotient = i128::from(digits[3]) | i128::from(digits[4]) << 32;
    let (top, rest_zero) = (digits[2], digits[0] == 0 && digits[1] == 0);
    let above = top > 1 << 31 || (top == 1 << 31 && !rest_zero);
    let tie = top == 1 << 31 && rest_zero;
    quotient + i128::from(above || (tie && quotient % 2 == 1))
}

/// GSAP's back at `s = s_raw / 2^32`, `t = r / 2^32`, as `base + k inner / 2^96` rounded once.
fn oracle_back(kind: Kind, s: i64, r: i64) -> i64 {
    let (s, r, u) = (i128::from(s), i128::from(r), i128::from(U));
    let (k, inner, base) = match kind {
        Kind::In => (r * r, (s + u) * r - s * u, 0),
        Kind::Out => ((r - u) * (r - u), (s + u) * (r - u) + s * u, u),
        Kind::InOut if r < i128::from(H) => (2 * r * r, 2 * (s + u) * r - s * u, 0),
        Kind::InOut => (2 * (r - u) * (r - u), 2 * (s + u) * (r - u) + s * u, u),
    };
    let magnitude = nearest_over_2_96(product(inner.unsigned_abs(), k as u128));
    let value = base + if inner < 0 { -magnitude } else { magnitude };
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// `floor(sqrt(n))`, one result bit per step from the top.
fn root(n: u128) -> u128 {
    let (mut rest, mut result, mut bit) = (n, 0_u128, 1_u128 << 126);
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if rest >= result + bit {
            rest -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}

/// The issue's circ rule: the floor of the exact root, then an exact subtraction.
fn oracle_circ(kind: Kind, r: i64) -> i64 {
    let (r, u, h) = (r as u128, U as u128, H as u128);
    let value = match kind {
        Kind::In => u - root(u * u - r * r),
        Kind::Out => root(2 * u * r - r * r),
        Kind::InOut if r < h => h - root(u * u - 4 * r * r) / 2,
        Kind::InOut => h + root(u * u - (2 * u - 2 * r) * (2 * u - 2 * r)) / 2,
    };
    value as i64
}

/// `bounce_out(r / 2^32)` in raw units as `(numerator, denominator)`:
/// `121/16 (t - top/bottom)^2 + level/64`.
fn bounce_out_fraction(r: i64) -> (u128, u128) {
    let (r, u) = (i128::from(r), i128::from(U));
    // 1.5/2.75 = 6/11, 2.25/2.75 = 9/11, 2.625/2.75 = 21/22; .75, .9375, .984375 over 64.
    let (top, bottom, level) = match 275 * r {
        scaled if scaled < 100 * u => (0, 1, 0),
        scaled if scaled < 200 * u => (6, 11, 48),
        scaled if scaled < 250 * u => (9, 11, 60),
        _ => (21, 22, 63),
    };
    let offset = bottom * r - top * u;
    let numerator = 4 * 121 * offset * offset + level * bottom * bottom * u * u;
    (numerator as u128, (64 * bottom * bottom * u) as u128)
}

/// `round_half_even(numerator / denominator)`.
fn nearest(numerator: u128, denominator: u128) -> i64 {
    let (quotient, twice_rest) = (numerator / denominator, 2 * (numerator % denominator));
    let up = twice_rest > denominator || (twice_rest == denominator && quotient % 2 == 1);
    (quotient + u128::from(up)) as i64
}

fn oracle_bounce(kind: Kind, r: i64) -> i64 {
    let one = |den: u128| U as u128 * den;
    match kind {
        Kind::Out => {
            let (num, den) = bounce_out_fraction(r);
            nearest(num, den)
        }
        Kind::In => {
            let (num, den) = bounce_out_fraction(U - r);
            nearest(one(den) - num, den)
        }
        Kind::InOut if r < H => {
            let (num, den) = bounce_out_fraction(U - 2 * r);
            nearest(one(den) - num, 2 * den)
        }
        Kind::InOut => {
            let (num, den) = bounce_out_fraction(2 * r - U);
            nearest(one(den) + num, 2 * den)
        }
    }
}

fn oracle(family: Family, kind: Kind, s: i64, r: i64) -> i64 {
    match family {
        Family::Back => oracle_back(kind, s, r),
        Family::Circ => oracle_circ(kind, r),
        Family::Bounce => oracle_bounce(kind, r),
    }
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// The issue's named input, then `2^20` seeded raw `t` in `[0, 2^32]`.
fn seeded_t() -> Vec<i64> {
    let mut state = SEED;
    let drawn = (0..1 << 20).map(|_| (next_seed(&mut state) % (U as u64 + 1)) as i64);
    std::iter::once(NAMED).chain(drawn).collect()
}

/// `2^16` seeded `(s, t)`: `s` a seeded `i64` shifted right by 0 to 47 bits, so every
/// magnitude from `2^16` to `2^63` occurs, and `t` in `[0, 2^32]`.
fn seeded_pairs() -> Vec<(i64, i64)> {
    let mut state = PAIR_SEED;
    let mut draw = || next_seed(&mut state);
    (0..1 << 16)
        .map(|_| {
            let s = (draw() as i64) >> (draw() % 48);
            (s, (draw() % (U as u64 + 1)) as i64)
        })
        .collect()
}

fn grid_t() -> Vec<i64> {
    (0..=65_536).map(|k| k << 16).collect()
}

/// Every raw `t` within `2^16` of 0 and of 1.
fn ends_t() -> Vec<i64> {
    (0..=1 << 16).chain(U - (1 << 16)..=U).collect()
}

/// Every raw `t` within 4 096 of each join and breakpoint: `1/2`, the bounce.out breakpoints
/// `b` of `4/11`, `8/11` and `10/11`, their reflections `1 - b` for bounce.in, and
/// `(1 - b) / 2` and `(1 + b) / 2` for bounce.inOut.
fn joins_t() -> Vec<i64> {
    let breaks = [4 * U / 11, 8 * U / 11, 10 * U / 11];
    let centres = breaks.iter().flat_map(|&b| [b, U - b, H - b / 2, H + b / 2]).chain([H]);
    centres.flat_map(|c| c - 4096..=c + 4096).filter(|r| (0..=U).contains(r)).collect()
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

fn assert_matches_oracle(points: &[i64], label: &str) {
    for (family, kind) in FUNCTIONS {
        for &r in points {
            let expected = oracle(family, kind, S0, r);
            assert_eq!(
                kernel(family, kind, S0, r),
                Some(expected),
                "{label} {family:?} {kind:?} at {r}"
            );
        }
    }
}

/// `(family, kind, raw t, expected raw)`, back at `S0`, each derived in its comment.
const GOLDEN: &[(Family, Kind, i64, i64)] = &[
    // 1/2.75 <= 1/2 < 2/2.75: 121/16 (1/2 - 6/11)^2 + 3/4 = 121/16 / 484 + 3/4 = 49/64 exactly.
    (Family::Bounce, Kind::Out, H, 3_288_334_336),
    // 1 - sqrt(3)/2: 2^32 - isqrt(2^64 - 2^62) = 2^32 - 3 719 550 786 (squares checked below).
    (Family::Circ, Kind::In, H, 575_416_510),
    // 2^64 - r^2 = 398 064 011 439 591, isqrt 19 951 541 (exact root 19 951 541.58).
    (Family::Circ, Kind::In, NAMED, 4_275_015_755),
    // One raw unit before 1: 2^64 - (2^32 - 1)^2 = 2^33 - 1, isqrt 92 681.
    (Family::Circ, Kind::In, U - 1, U - 92_681),
    (Family::Circ, Kind::Out, 1, 92_681),
    // The join: 2^62 - (2^31 - 1)^2 = 2^32 - 1, isqrt 65 535, so 1/2 - 65 535 raw.
    (Family::Circ, Kind::InOut, H - 1, H - 65_535),
    // u = 1 - 2t = 2^18 raw, piece 1: (22 2^18)^2 / 2^39 = 121 / 2 = 60.5, a tie: 1/2 - 60.
    (Family::Bounce, Kind::InOut, H - (1 << 17), H - 60),
    // (1/2)^2 ((s + 1)/2 - s) = (1 - s)/8 = (2^32 - S0)/8 = -376 657 894.5, a tie: even.
    (Family::Back, Kind::In, H, -376_657_894),
    (Family::Back, Kind::Out, H, U + 376_657_894),
    // back_in(s, 1/2) / 2 = (2^32 - S0)/16 = -188 328 947.25.
    (Family::Back, Kind::InOut, U / 4, -188_328_947),
    (Family::Bounce, Kind::In, 0x1234_5678, 45_728_146),
    (Family::Back, Kind::Out, 0xDEAD_BEEF, 4_393_195_904),
    (Family::Circ, Kind::Out, 0x9ABC_DEF0, 3_944_678_568),
];

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_020_hand_derived_eases_and_the_default_overshoot() {
    // 1.70158 = 85 079 / 50 000; S0 is its nearest raw value: |50 000 S0 - 85 079 2^32| < 25 000.
    assert_eq!(BACK_DEFAULT_OVERSHOOT.raw(), S0);
    assert!((50_000 * i128::from(S0) - 85_079 * i128::from(U)).abs() < 25_000);
    for &(family, kind, r, expected) in GOLDEN {
        assert_eq!(oracle(family, kind, S0, r), expected, "oracle {family:?} {kind:?} at {r}");
        assert_eq!(kernel(family, kind, S0, r), Some(expected), "{family:?} {kind:?} at {r}");
    }
    let squares = |n: u128, radicand: u128| n * n <= radicand && radicand < (n + 1) * (n + 1);
    assert!(squares(3_719_550_786, 3 << 62), "circ_in(1/2) root");
    assert!(squares(19_951_541, 398_064_011_439_591), "circ_in(NAMED) root");
    assert!(squares(92_681, (1 << 33) - 1) && squares(65_535, (1 << 32) - 1));
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_021_the_domain_is_refused_outside_and_the_ends_are_exact() {
    for s in [S0, 0, 1, -1, -U, 10 * U, i64::MIN, i64::MAX] {
        for (family, kind) in FUNCTIONS {
            assert_eq!(kernel(family, kind, s, 0), Some(0), "{family:?} {kind:?} s={s} at 0");
            assert_eq!(kernel(family, kind, s, U), Some(U), "{family:?} {kind:?} s={s} at 1");
            for r in [-1, U + 1, -U, i64::MIN, i64::MAX] {
                assert_eq!(kernel(family, kind, s, r), None, "{family:?} {kind:?} s={s} at {r}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_022_the_grid_matches_the_oracle_exhaustively() {
    let grid = grid_t();
    assert_matches_oracle(&grid, "grid");
    let outputs =
        FUNCTIONS.iter().flat_map(|&(f, k)| grid.iter().map(move |&r| kernel(f, k, S0, r)));
    assert_eq!(digest(outputs), GRID_DIGEST, "the Python model's grid digest");
    println!("ease_tp_022 functions=9 points={} cases={}", grid.len(), 9 * grid.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_023_every_raw_t_near_the_ends_and_joins_matches_the_oracle() {
    let points: Vec<i64> = ends_t().into_iter().chain(joins_t()).collect();
    assert_matches_oracle(&points, "near an end or join");
    println!("ease_tp_023 points={} cases={}", points.len(), 9 * points.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_024_seeded_full_resolution_t_match_the_oracle() {
    let seeded = seeded_t();
    println!("ease_tp_024 seed={SEED:#018x} seeded={} named={NAMED}", seeded.len());
    assert_matches_oracle(&seeded, "seeded");
    let outputs =
        FUNCTIONS.iter().flat_map(|&(f, k)| seeded.iter().map(move |&r| kernel(f, k, S0, r)));
    assert_eq!(digest(outputs), SEEDED_DIGEST, "the Python model's seeded digest");
    println!("ease_tp_024 cases={}", 9 * seeded.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_025_back_matches_the_oracle_for_every_overshoot() {
    let pairs = seeded_pairs();
    println!("ease_tp_025 seed={PAIR_SEED:#018x} pairs={}", pairs.len());
    let kinds = [Kind::In, Kind::Out, Kind::InOut];
    let extremes = [i64::MIN, i64::MIN + 1, -U - 1, -U, -1, 0, 1, U, i64::MAX];
    let corners = extremes.iter().flat_map(|&s| [0, 1, H - 1, H, H + 1, U - 1, U].map(|r| (s, r)));
    for (s, r) in pairs.iter().copied().chain(corners) {
        for kind in kinds {
            let value = kernel(Family::Back, kind, s, r);
            assert_eq!(value, Some(oracle_back(kind, s, r)), "back {kind:?} s={s} at {r}");
            // |f| <= 1 + 4 |s| / 27: every overshoot fits, with room to spare.
            let fits = value.is_some_and(|v| {
                27 * i128::from(v).abs() <= 27 * i128::from(U) + 4 * i128::from(s).abs()
            });
            assert!(fits, "back {kind:?} s={s} at {r} is {value:?}");
        }
    }
    let outputs = pairs.iter().flat_map(|&(s, r)| kinds.map(|k| kernel(Family::Back, k, s, r)));
    assert_eq!(digest(outputs), PAIR_DIGEST, "the Python model's pair digest");
    println!("ease_tp_025 cases={}", 3 * (pairs.len() + 7 * extremes.len()));
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_026_circ_is_the_floor_root_within_one_raw_unit() {
    // Each check squares the kernel's root and so needs no square root: `y^2 <= X < (y+1)^2`
    // is `y = floor(sqrt(X))`, which puts the result within one raw unit, on the stated side.
    let floor_root = |y: u128, radicand: u128| y * y <= radicand && radicand < (y + 1) * (y + 1);
    let (u, h) = (U as u128, H as u128);
    let points: Vec<i64> = grid_t().into_iter().chain(ends_t()).chain(seeded_t()).collect();
    for &r in &points {
        let raw = |kind| kernel(Family::Circ, kind, S0, r).map(|v| v as u128);
        let (inn, out, in_out, t) = (raw(Kind::In), raw(Kind::Out), raw(Kind::InOut), r as u128);
        assert!(inn.is_some_and(|v| floor_root(u - v, u * u - t * t)), "circ_in at {r}");
        assert!(out.is_some_and(|v| floor_root(v, 2 * u * t - t * t)), "circ_out at {r}");
        // in-out: y = floor(sqrt(X) / 2) is (2y)^2 <= X < (2y + 2)^2.
        let (y, radicand) = match in_out {
            Some(v) if r < H => (h - v, u * u - 4 * t * t),
            Some(v) => (v - h, u * u - 4 * (u - t) * (u - t)),
            None => (0, u128::MAX),
        };
        let half = 4 * y * y <= radicand && radicand < 4 * (y + 1) * (y + 1);
        assert!(half, "circ_in_out at {r}");
    }
    println!("ease_tp_026 points={}", points.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_027_reflections_and_the_middle_are_exact() {
    let points: Vec<i64> =
        grid_t().into_iter().chain(seeded_t().into_iter().take(1 << 16)).collect();
    let back = [S0, 0, U, -U / 2, 1 << 40, i64::MIN, i64::MAX].map(|s| (Family::Back, s));
    for (family, s) in back.into_iter().chain([(Family::Circ, 0), (Family::Bounce, 0)]) {
        let f = |kind, r| kernel(family, kind, s, r);
        for &r in &points {
            assert_eq!(
                f(Kind::Out, r),
                f(Kind::In, U - r).map(|v| U - v),
                "{family:?} out s={s} at {r}"
            );
            let mirrored = f(Kind::InOut, U - r).map(|v| U - v);
            assert_eq!(f(Kind::InOut, r), mirrored, "{family:?} in-out s={s} at {r}");
        }
        assert_eq!(f(Kind::InOut, H), Some(H), "{family:?} in-out s={s} at 1/2");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_028_circ_is_monotone_and_back_and_bounce_are_not() {
    let points: Vec<i64> = grid_t()
        .iter()
        .flat_map(|&r| [r - 1, r, r + 1])
        .chain(ends_t())
        .filter(|r| (0..=U).contains(r))
        .collect();
    let mut sorted = points.clone();
    sorted.sort_unstable();
    sorted.dedup();
    for kind in [Kind::In, Kind::Out, Kind::InOut] {
        let values: Vec<Option<i64>> =
            sorted.iter().map(|&r| kernel(Family::Circ, kind, S0, r)).collect();
        for pair in values.windows(2) {
            assert!(pair[0].is_some() && pair[0] <= pair[1], "circ {kind:?}: {pair:?}");
        }
    }
    // Back dips below 0 and comes back; bounce falls after each bounce.
    let back = |r| kernel(Family::Back, Kind::In, S0, r);
    assert!(back(U / 4) < back(0) && back(0) < back(U));
    let bounce = |r| kernel(Family::Bounce, Kind::Out, S0, r);
    assert!(bounce(4 * U / 11) > bounce(H) && bounce(H) < bounce(8 * U / 11));
    println!("ease_tp_028 points={}", sorted.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_029_bounce_is_continuous_at_each_breakpoint() {
    // The oracle's pieces meet exactly at t = n/22 for n of 8, 16, 20 (and give 1 at t = 1):
    // 121/16 (n/22 - a/b)^2 + c/64 = ((n - 22a/b)^2 + c) / 64 = 1 on both sides.
    let pieces: [(i64, i64, i64); 4] = [(0, 1, 0), (6, 11, 48), (9, 11, 60), (21, 22, 63)];
    let at = |n: i64, (a, b, c): (i64, i64, i64)| (n - 22 * a / b).pow(2) + c;
    for (n, pair) in [8, 16, 20].into_iter().zip(pieces.windows(2)) {
        assert_eq!((at(n, pair[0]), at(n, pair[1])), (64, 64), "pieces at {n}/22");
    }
    assert_eq!(at(22, pieces[3]), 64);
    // Either side of a breakpoint k/11 the kernel is within 1 raw unit of the common value 1
    // carried along that side's slope m/16: |176 (f - 2^32) - m (11 r - k 2^32)| <= 176.
    for (k, left, right) in [(4, 88, -44), (8, 44, -22), (10, 22, -11)] {
        let below = k * U / 11;
        for (r, m) in [(below, left), (below + 1, right)] {
            let f = kernel(Family::Bounce, Kind::Out, S0, r).map_or(i128::MAX, i128::from);
            let gap = 176 * (f - i128::from(U)) - i128::from(m) * i128::from(11 * r - k * U);
            assert!(gap.abs() <= 176, "bounce_out at {r} beside {k}/11: {gap}");
        }
    }
    // No jump anywhere: the slope is at most 5.5, so one raw unit moves a value by at most 6
    // raw units, and one grid step of 2^16 by at most 5.5 2^16 + 1.
    for kind in [Kind::In, Kind::Out, Kind::InOut] {
        let f = |r| kernel(Family::Bounce, kind, S0, r).unwrap_or(i64::MIN);
        let near: Vec<i64> = joins_t().into_iter().chain(ends_t()).collect();
        for r in near.into_iter().filter(|&r| r < U) {
            assert!((f(r + 1) - f(r)).abs() <= 6, "bounce {kind:?} at {r}");
        }
        for r in grid_t().into_iter().filter(|&r| r < U) {
            assert!((f(r + (1 << 16)) - f(r)).abs() <= 360_449, "bounce {kind:?} grid at {r}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn ease_tp_030_back_overshoots_by_the_closed_form() {
    // For s > 0, back_in is least at t* = 2s / (3(s + 1)), where it is -4 s^3 / (27 (s + 1)^2);
    // back_out is greatest at 1 - t* by the same amount above 1, and back_in_out halves both.
    // The least raw value near t* is within 1/2 + 2^-16 raw units of that: one rounding, plus
    // the curve's rise over half a raw unit of t, s 2^-34 raw units.
    for s in [S0, U, 3 * U, 10 * U] {
        let (si, ui) = (i128::from(s), i128::from(U));
        let scale = 27 * (si + ui) * (si + ui);
        let t_star = (2 * si * ui / (3 * (si + ui))) as i64;
        let window = |c: i64| c - 4..=c + 4;
        let low = window(t_star).filter_map(|r| kernel(Family::Back, Kind::In, s, r)).min();
        let high = window(U - t_star).filter_map(|r| kernel(Family::Back, Kind::Out, s, r)).max();
        let half = window(t_star / 2).filter_map(|r| kernel(Family::Back, Kind::InOut, s, r)).min();
        let within = |error: i128| (error << 17).abs() <= scale * ((1 << 16) + 2);
        let low = low.map_or(i128::MAX, i128::from);
        let high = high.map_or(i128::MAX, i128::from);
        let half = half.map_or(i128::MAX, i128::from);
        assert!(within(scale * low + 4 * si.pow(3)), "back_in minimum, s={s}: {low}");
        assert!(within(scale * (high - ui) - 4 * si.pow(3)), "back_out maximum, s={s}: {high}");
        assert!(within(scale * half + 2 * si.pow(3)), "back_in_out minimum, s={s}: {half}");
        assert!(low < 0 && high > ui, "back leaves [0, 1], s={s}");
    }
    // back_out(s, 1) is exactly 1 for every overshoot.
    for s in [S0, 0, U, i64::MIN, i64::MAX] {
        assert_eq!(kernel(Family::Back, Kind::Out, s, U), Some(U), "back_out(s={s}, 1)");
    }
}
