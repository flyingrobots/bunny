//! Contract tests for the conversions out of `FixedAngle` and for `FixedAngle::atan2`
//! (`docs/topics/angle-trigonometry/test-plan.md`, AT-TP-010 to AT-TP-016).
//!
//! Oracles: `to_turns` and `to_degrees` are checked against the exact rational rounded by
//! `u128` quotient and remainder (`nearest_even` below); `to_radians` and `atan2` against
//! `angle_oracle`, which derives `pi` by Machin's formula and `atan` by its Taylor series
//! with 124 fractional bits. None of them shares a helper or constant with the kernel.

mod angle_oracle;

use angle_oracle::{Oracle, EXTRA_BITS};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const QUARTER: u64 = 1 << 62;
const HALF: u64 = 1 << 63;
const EIGHTH: u64 = 1 << 61;
const SEED: u64 = 0xB002_A461_0244_0001;
const CASES: usize = 20_000;
/// The stated `to_radians` bound, `1/2 + 2^-30` raw units, in units of `2^-EXTRA_BITS` raw.
const RADIANS_BOUND: u128 = (1 << (EXTRA_BITS - 1)) + (1 << (EXTRA_BITS - 30));
/// The stated `atan2` bound, `1/2 + 2^-9` angle units, in units of `2^-EXTRA_BITS` angle unit.
const ATAN2_BOUND: u128 = (1 << (EXTRA_BITS - 1)) + (1 << (EXTRA_BITS - 9));
/// Allowance for the oracle's own error (below `2^-40` of a unit here), in the same units.
const ORACLE_SLACK: u128 = 2;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A seeded `i64` of any magnitude: a random value shifted right by a random amount.
fn spread_i64(state: &mut u64) -> i64 {
    let value = i64::from_ne_bytes(next_seed(state).to_ne_bytes());
    value >> (next_seed(state) % 64)
}

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

/// `numerator / 2^shift` rounded to nearest with ties to even, by quotient and remainder.
fn nearest_even(numerator: u128, shift: u32) -> i64 {
    let divisor = 1_u128 << shift;
    let (quotient, remainder) = (numerator / divisor, numerator % divisor);
    let up = 2 * remainder > divisor || (2 * remainder == divisor && quotient % 2 == 1);
    i64::try_from(quotient + u128::from(up)).expect("result fits i64")
}

/// Angles on and one unit either side of every rounding boundary the conversions have.
fn boundary_angles() -> Vec<u64> {
    let mut angles = vec![0, 1, u64::MAX, u64::MAX - 1, QUARTER, HALF, 3 * QUARTER];
    // Ties of to_turns (raw = k 2^32 + 2^31) for even and odd k, and the last one.
    for tie in [1 << 31, 3 << 31, 5 << 31, u64::MAX - (1 << 31) + 1] {
        angles.extend([tie - 1, tie, tie + 1]);
    }
    // Ties of to_degrees (raw * 45 = 2^28 mod 2^29): 2^28 and 3 2^28 give 22.5 and 67.5 raw.
    for tie in [1 << 28, 3 << 28, (1 << 28) + (1 << 40), u64::MAX - (1 << 28) + 1] {
        angles.extend([tie - 1, tie, tie + 1]);
    }
    angles
}

fn seeded_angles() -> Vec<u64> {
    let mut state = SEED;
    let mut angles = boundary_angles();
    angles.extend((0..CASES).map(|_| next_seed(&mut state)));
    angles
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_010_turns_and_degrees_are_rounded_once_to_nearest_even() {
    for raw in seeded_angles() {
        let angle = FixedAngle::from_raw(raw);
        let turns = nearest_even(u128::from(raw), 32);
        let degrees = nearest_even(u128::from(raw) * 360, 32);
        assert_eq!(angle.to_turns().raw(), turns, "to_turns angle={raw:#018x}");
        assert_eq!(angle.to_degrees().raw(), degrees, "to_degrees angle={raw:#018x}");
        assert!((0..=ONE_RAW).contains(&turns) && (0..=360 * ONE_RAW).contains(&degrees));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_011_radians_are_within_half_a_unit_and_two_to_the_minus_thirty() {
    let oracle = Oracle::new();
    for raw in seeded_angles() {
        let actual = FixedAngle::from_raw(raw).to_radians().raw();
        let actual = u128::try_from(actual).expect("radians are non-negative") << EXTRA_BITS;
        let exact = oracle.radians(raw);
        let error = actual.abs_diff(exact);
        assert!(error <= RADIANS_BOUND + ORACLE_SLACK, "angle={raw:#018x} error={error}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_012_turn_round_trips_are_exact() {
    let mut state = SEED ^ 0x0012;
    for case in 0..CASES {
        let turns = spread_i64(&mut state);
        let angle = FixedAngle::from_turns(q(turns));
        assert_eq!(angle.to_turns().raw(), turns.rem_euclid(ONE_RAW), "case={case}");
        let raw = next_seed(&mut state);
        let back = FixedAngle::from_turns(FixedAngle::from_raw(raw).to_turns()).raw();
        // The angle rounded to the nearest multiple of 2^32 units, ties to even, wrapping.
        let multiple = u64::try_from(nearest_even(u128::from(raw), 32)).expect("non-negative");
        assert_eq!(back, multiple.wrapping_shl(32), "case={case} angle={raw:#018x}");
    }
}

/// Directions on and one raw unit either side of the axes, the diagonals and the extremes.
fn boundary_directions() -> Vec<(i64, i64)> {
    let (one, max, min) = (ONE_RAW, i64::MAX, i64::MIN);
    let mut pairs = vec![(1, max), (max, 1), (max, max - 1), (max - 1, max), (max, min)];
    for x in [one, -one, max, min, 1, -1] {
        pairs.extend([(0, x), (1, x), (-1, x), (x, 0), (x, 1), (x, -1)]);
    }
    for x in [one, -one, 3, max - 1, min + 1] {
        pairs.extend([(x, x), (x + 1, x), (x - 1, x), (-x, x), (-x - 1, x)]);
    }
    pairs
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_013_atan2_is_within_half_a_unit_and_two_to_the_minus_nine() {
    let oracle = Oracle::new();
    let mut state = SEED ^ 0x0013;
    let mut pairs = boundary_directions();
    pairs.extend((0..CASES).map(|_| (spread_i64(&mut state), spread_i64(&mut state))));
    let full = 1_u128 << (64 + EXTRA_BITS);
    for (y, x) in pairs {
        let (Some(actual), Some(exact)) = (FixedAngle::atan2(q(y), q(x)), oracle.atan2(y, x))
        else {
            assert_eq!((y, x), (0, 0), "only the origin has no angle");
            continue;
        };
        let actual = u128::from(actual.raw()) << EXTRA_BITS;
        // The distance on the circle: the angle wraps at a full turn.
        let gap = actual.abs_diff(exact);
        let error = gap.min(full - gap);
        assert!(error <= ATAN2_BOUND + ORACLE_SLACK, "y={y} x={x} error={error}");
    }
}

fn atan2_raw(y: i64, x: i64) -> Option<u64> {
    FixedAngle::atan2(q(y), q(x)).map(FixedAngle::raw)
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_014_atan2_axes_diagonals_branch_and_origin() {
    let (one, min) = (ONE_RAW, i64::MIN);
    assert_eq!(atan2_raw(0, 0), None);
    for (y, x, raw) in [
        (0, one, 0),
        (one, one, EIGHTH),
        (one, 0, QUARTER),
        (one, -one, 3 * EIGHTH),
        (0, -one, HALF),
        (-one, -one, 5 * EIGHTH),
        (-one, 0, 3 * QUARTER),
        (-one, one, 7 * EIGHTH),
        (0, min, HALF),
        (min, 0, 3 * QUARTER),
        (min, min, 5 * EIGHTH),
        (0, 1, 0),
        (-1, 0, 3 * QUARTER),
    ] {
        assert_eq!(atan2_raw(y, x), Some(raw), "y={y} x={x}");
    }
    // The negative x axis is exactly a half turn; one raw unit above it is just below a half
    // turn and one raw unit below it just above, so the result never jumps there.
    let above = atan2_raw(1, -one).expect("not the origin");
    let below = atan2_raw(-1, -one).expect("not the origin");
    assert!(above < HALF && HALF - above < 1 << 30, "above={above:#018x}");
    assert!(below > HALF && below - HALF < 1 << 30, "below={below:#018x}");
    // One raw unit below the positive x axis wraps to just below a full turn.
    let under = atan2_raw(-1, one).expect("not the origin");
    assert!(under.wrapping_neg() < 1 << 30, "under={under:#018x}");
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_015_atan2_symmetries_and_scale_are_bit_exact() {
    let mut state = SEED ^ 0x0015;
    for case in 0..CASES {
        // Keep away from i64::MIN so that negation and doubling stay in range.
        let (y, x) = (spread_i64(&mut state) >> 1, spread_i64(&mut state) >> 1);
        let Some(angle) = FixedAngle::atan2(q(y), q(x)) else {
            continue;
        };
        let raw = angle.raw();
        assert_eq!(atan2_raw(-y, x), Some(raw.wrapping_neg()), "conjugate case={case}");
        assert_eq!(atan2_raw(y, -x), Some(HALF.wrapping_sub(raw)), "mirror case={case}");
        assert_eq!(atan2_raw(x, y), Some(QUARTER.wrapping_sub(raw)), "swap case={case}");
        assert_eq!(atan2_raw(2 * y, 2 * x), Some(raw), "scale case={case}");
    }
}

/// `(angle raw, to_turns raw, to_degrees raw, to_radians raw)`, each the correctly rounded
/// value from a 60-digit decimal model, with the exact value's offset from it noted.
const CONVERSION_GOLDEN: &[(u64, i64, i64, i64)] = &[
    (0, 0, 0, 0),
    (1 << 31, 0, 180, 3), // a turns tie goes to even 0
    (3 << 31, 2, 540, 9), // a turns tie goes to even 2
    (1 << 28, 0, 22, 0),  // a degrees tie (22.5) to 22
    (3 << 28, 0, 68, 1),  // a degrees tie (67.5) to 68
    (QUARTER, 1_073_741_824, 386_547_056_640, 6_746_518_852), // radians +0.261
    (HALF, 2_147_483_648, 773_094_113_280, 13_493_037_705), // radians -0.478
    (3 * QUARTER, 3_221_225_472, 1_159_641_169_920, 20_239_556_557), // radians -0.217
    (u64::MAX, 4_294_967_296, 1_546_188_226_560, 26_986_075_409), // radians +0.044
    (0x1555_5555_5555_5555, 357_913_941, 128_849_018_880, 2_248_839_617), // 30 degrees
    (0x28be_60db_9391_054a, 683_565_276, 246_083_499_208, 4_294_967_296), // one radian
];

/// `(y raw, x raw, angle raw)`, each the correctly rounded angle from the same model; the
/// exact value is at least 0.07 units from a rounding boundary in every case.
const ATAN2_GOLDEN: &[(i64, i64, u64)] = &[
    (ONE_RAW, 2 * ONE_RAW, 1_361_218_612_134_873_190), // atan(1/2), +0.338
    (3 * ONE_RAW, 4 * ONE_RAW, 1_889_248_794_157_641_523), // 3-4-5, +0.324
    (-4 * ONE_RAW, 3 * ONE_RAW, 15_724_306_849_439_805_235), // +0.324
    (7_439_101_574, ONE_RAW, 3_074_457_345_700_505_648), // near 60 degrees, -0.256
    (5, -12, 8_064_308_537_748_178_180),               // 5-12-13, -0.327
    (1, ONE_RAW, 683_565_276),                         // one raw unit above +x, -0.424
    (-1, ONE_RAW, 18_446_744_073_025_986_340),         // one raw unit below +x, +0.424
    (1, -ONE_RAW, 9_223_372_036_171_210_532),          // just above -x, +0.424
    (-1, -ONE_RAW, 9_223_372_037_538_341_084),         // just below -x, -0.424
    (i64::MAX, i64::MIN, 6_917_529_027_641_081_856),   // +0.159
    (i64::MAX, i64::MAX - 1, 2_305_843_009_213_693_952), // +0.159
    (1, i64::MAX, 0),                                  // +0.318
];

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_016_golden_vectors_are_bit_exact() {
    for &(raw, turns, degrees, radians) in CONVERSION_GOLDEN {
        let angle = FixedAngle::from_raw(raw);
        let actual = (angle.to_turns().raw(), angle.to_degrees().raw(), angle.to_radians().raw());
        assert_eq!(actual, (turns, degrees, radians), "angle={raw:#018x}");
    }
    for &(y, x, raw) in ATAN2_GOLDEN {
        assert_eq!(atan2_raw(y, x), Some(raw), "y={y} x={x}");
    }
}
