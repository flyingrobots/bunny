//! Contract tests for `FixedAngle::atan`, `asin`, `acos` and the signed conversions out
//! (`docs/topics/angle-trigonometry/test-plan.md`, AT-TP-030 to AT-TP-039).
//!
//! Oracles: `atan` and the signed radians against `angle_oracle` (Machin `pi`, Taylor
//! `atan`, 124 fractional bits); `asin` and `acos` against `inverse_oracle`, which adds an
//! integer Newton square root of a 248-bit number; the signed turns and degrees against
//! the exact signed rational rounded by `i128` Euclidean quotient and remainder. None of
//! them shares a helper or constant with the kernel.

mod angle_oracle;
mod inverse_oracle;

use angle_oracle::{Oracle, EXTRA_BITS};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const QUARTER: u64 = 1 << 62;
const HALF: u64 = 1 << 63;
const EIGHTH: u64 = 1 << 61;
const SEED: u64 = 0xB002_A461_0244_0002;
const CASES: usize = 4_000;
/// A full turn in the oracle's units, `2^-EXTRA_BITS` angle unit.
const FULL: u128 = 1 << (64 + EXTRA_BITS);
/// The stated `atan` bound, `1/2 + 2^-9` angle units, in units of `2^-EXTRA_BITS` angle unit.
const ATAN_BOUND: u128 = (1 << (EXTRA_BITS - 1)) + (1 << (EXTRA_BITS - 9));
/// Three times the stated `asin` and `acos` bound of `2/3` angle unit, in the same units.
const THRICE_INVERSE_BOUND: u128 = 2 << EXTRA_BITS;
/// The stated radians bound, `1/2 + 2^-30` raw units, in units of `2^-EXTRA_BITS` raw.
const RADIANS_BOUND: u128 = (1 << (EXTRA_BITS - 1)) + (1 << (EXTRA_BITS - 30));
/// Allowance for the oracles' own error (below `2^-40` of a unit here), in the same units.
const ORACLE_SLACK: u128 = 2;
/// `round(2 pi 2^61) / 2^29`, the kernel's full turn in raw radians, rounded down.
const TAU_RAW: i64 = 26_986_075_409;

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

/// The distance on the circle between a kernel angle and an oracle angle.
fn circle_error(actual: FixedAngle, exact: u128) -> u128 {
    let gap = (u128::from(actual.raw()) << EXTRA_BITS).abs_diff(exact);
    gap.min(FULL - gap)
}

/// Ratios on and one raw unit either side of zero, one and the extremes, then seeded ones.
fn atan_ratios() -> Vec<i64> {
    let (one, max, min) = (ONE_RAW, i64::MAX, i64::MIN);
    let mut ratios = vec![0, 1, -1, max, max - 1, min, min + 1];
    for x in [one, -one, 2 * one, -2 * one] {
        ratios.extend([x - 1, x, x + 1]);
    }
    let mut state = SEED ^ 0x0030;
    ratios.extend((0..CASES).map(|_| spread_i64(&mut state)));
    ratios
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_030_atan_is_atan2_over_one_and_within_its_bound() {
    let oracle = Oracle::new();
    for x in atan_ratios() {
        let actual = FixedAngle::atan(q(x));
        assert_eq!(Some(actual), FixedAngle::atan2(q(x), FixedQ32_32::ONE), "x={x}");
        let exact = oracle.atan2(x, ONE_RAW).expect("(1, x) is not the origin");
        let error = circle_error(actual, exact);
        assert!(error <= ATAN_BOUND + ORACLE_SLACK, "x={x} error={error}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_031_atan_exact_values_range_and_odd_symmetry() {
    let atan = |x: i64| FixedAngle::atan(q(x)).raw();
    assert_eq!(atan(0), 0);
    assert_eq!(atan(ONE_RAW), EIGHTH);
    assert_eq!(atan(-ONE_RAW), 7 * EIGHTH);
    for x in atan_ratios() {
        // Read as a signed i64 the angle is strictly inside a quarter turn either side.
        let signed = i64::from_ne_bytes(atan(x).to_ne_bytes());
        assert!(signed.unsigned_abs() < QUARTER, "x={x} angle={signed}");
        assert_eq!(signed < 0, x < 0, "x={x} angle={signed}");
        if x != i64::MIN {
            assert_eq!(atan(-x), atan(x).wrapping_neg(), "x={x}");
        }
    }
    // The extremes stay about 2^33 / (2 pi) units inside the quarter turns.
    assert!(QUARTER - atan(i64::MAX) > 1 << 30);
    assert!(atan(i64::MIN) - 3 * QUARTER > 1 << 30);
}

fn asin_raw(x: i64) -> Option<u64> {
    FixedAngle::asin(q(x)).map(FixedAngle::raw)
}

fn acos_raw(x: i64) -> Option<u64> {
    FixedAngle::acos(q(x)).map(FixedAngle::raw)
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_032_asin_and_acos_refuse_ratios_outside_minus_one_to_one() {
    for x in [ONE_RAW + 1, -ONE_RAW - 1, 2 * ONE_RAW, i64::MAX, i64::MIN, i64::MIN + 1] {
        assert_eq!(asin_raw(x), None, "asin x={x}");
        assert_eq!(acos_raw(x), None, "acos x={x}");
    }
    for x in [ONE_RAW, -ONE_RAW, ONE_RAW - 1, -ONE_RAW + 1, 0] {
        assert!(asin_raw(x).is_some() && acos_raw(x).is_some(), "x={x}");
    }
}

/// Every `2^18`-th ratio of the domain, the 2 048 ratios at each end, the 64 either side of
/// zero, and seeded ratios.
fn domain_ratios() -> Vec<i64> {
    let mut ratios: Vec<i64> = (-(1 << 14)..=(1 << 14)).map(|k| k << 18).collect();
    for j in 0..2_048 {
        ratios.extend([ONE_RAW - j, -ONE_RAW + j]);
    }
    ratios.extend(-64..64);
    let mut state = SEED ^ 0x0033;
    let seeded = (0..CASES).map(|_| spread_i64(&mut state) % (ONE_RAW + 1));
    ratios.extend(seeded);
    ratios
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_033_asin_and_acos_are_within_two_thirds_of_a_unit() {
    let oracle = Oracle::new();
    let (mut worst, mut worst_at) = (0, 0);
    for x in domain_ratios() {
        let asin = FixedAngle::asin(q(x)).expect("in the domain");
        let acos = FixedAngle::acos(q(x)).expect("in the domain");
        let asin_error = circle_error(asin, inverse_oracle::asin(&oracle, x));
        let acos_error = circle_error(acos, inverse_oracle::acos(&oracle, x));
        let error = asin_error.max(acos_error);
        assert!(3 * error <= THRICE_INVERSE_BOUND + 3 * ORACLE_SLACK, "x={x} error={error}");
        if error > worst {
            (worst, worst_at) = (error, x);
        }
    }
    // Reported, not asserted: the largest error this set reaches, in angle units.
    let units = worst * 1_000_000 >> EXTRA_BITS;
    println!("asin/acos worst error {units} micro-units at x raw {worst_at}");
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_034_asin_and_acos_exact_cases_and_one_unit_inside_each_end() {
    let one = ONE_RAW;
    for (x, asin, acos) in [(0, 0, QUARTER), (one, QUARTER, 0), (-one, 3 * QUARTER, HALF)] {
        assert_eq!((asin_raw(x), acos_raw(x)), (Some(asin), Some(acos)), "x={x}");
    }
    let near = |x: i64| (asin_raw(x).expect("domain"), acos_raw(x).expect("domain"));
    // One raw unit inside +1 and -1: off the poles by about 2^-15.5 radians.
    let (asin, acos) = near(one - 1);
    assert!(asin < QUARTER && QUARTER - asin > 1 << 40 && acos > 1 << 40, "{asin} {acos}");
    let (asin, acos) = near(-one + 1);
    assert!(asin > 3 * QUARTER && asin - 3 * QUARTER > 1 << 40, "asin={asin}");
    assert!(acos < HALF && HALF - acos > 1 << 40, "acos={acos}");
    // One raw unit either side of zero: off zero and the quarter turn by about 2^32 / 2 pi.
    let (asin, acos) = near(1);
    assert!(asin > 0 && asin < 1 << 30 && acos < QUARTER && QUARTER - acos < 1 << 30);
    let (asin, acos) = near(-1);
    assert!(asin.wrapping_neg() < 1 << 30 && acos > QUARTER && acos - QUARTER < 1 << 30);
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_035_asin_and_acos_identities_are_bit_exact() {
    for x in domain_ratios() {
        let (asin, acos) = (asin_raw(x).expect("domain"), acos_raw(x).expect("domain"));
        assert_eq!(asin.wrapping_add(acos), QUARTER, "complement x={x}");
        assert_eq!(asin_raw(-x), Some(asin.wrapping_neg()), "odd asin x={x}");
        assert_eq!(acos_raw(-x), Some(HALF - acos), "mirrored acos x={x}");
        // asin is in [-1/4, 1/4] turn read as signed, acos in [0, 1/2] turn.
        let signed = i64::from_ne_bytes(asin.to_ne_bytes());
        assert!(signed.unsigned_abs() <= QUARTER && acos <= HALF, "range x={x}");
    }
}

/// `numerator / 2^shift` rounded to nearest with ties to even, for a signed numerator, by
/// Euclidean quotient and remainder.
fn nearest_even_signed(numerator: i128, shift: u32) -> i64 {
    let divisor = 1_i128 << shift;
    let (quotient, remainder) = (numerator.div_euclid(divisor), numerator.rem_euclid(divisor));
    let up = 2 * remainder > divisor || (2 * remainder == divisor && quotient % 2 != 0);
    i64::try_from(quotient + i128::from(up)).expect("result fits i64")
}

/// Angles on and one unit either side of zero, the half turn, the turns and degrees ties of
/// both signs, then seeded angles.
fn signed_angles() -> Vec<u64> {
    let mut angles = vec![0, 1, u64::MAX, HALF - 1, HALF, HALF + 1, QUARTER, 3 * QUARTER];
    for tie in [1 << 31, 3 << 31, 1 << 28, 3 << 28, HALF - (1 << 31), HALF - (1 << 28)] {
        angles.extend([tie - 1, tie, tie + 1, (tie - 1).wrapping_neg(), tie.wrapping_neg()]);
    }
    let mut state = SEED ^ 0x0036;
    angles.extend((0..5 * CASES).map(|_| next_seed(&mut state)));
    angles
}

fn signed(raw: u64) -> i64 {
    i64::from_ne_bytes(raw.to_ne_bytes())
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_036_signed_conversions_round_once_within_their_bounds() {
    let oracle = Oracle::new();
    for raw in signed_angles() {
        let (angle, s) = (FixedAngle::from_raw(raw), i128::from(signed(raw)));
        assert_eq!(angle.to_turns_signed().raw(), nearest_even_signed(s, 32), "{raw:#018x}");
        let degrees = nearest_even_signed(s * 360, 32);
        assert_eq!(angle.to_degrees_signed().raw(), degrees, "{raw:#018x}");
        assert!((-180 * ONE_RAW..=180 * ONE_RAW).contains(&degrees), "{raw:#018x}");
        // The exact signed radians, scaled by 2^EXTRA_BITS: negative below a full turn.
        let exact = if s < 0 {
            -i128::try_from(oracle.radians(raw.wrapping_neg())).expect("fits")
        } else {
            i128::try_from(oracle.radians(raw)).expect("fits")
        };
        let actual = i128::from(angle.to_radians_signed().raw()) << EXTRA_BITS;
        let error = actual.abs_diff(exact);
        assert!(error <= RADIANS_BOUND + ORACLE_SLACK, "{raw:#018x} error={error}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_037_signed_conversions_match_the_unsigned_ones_and_negation() {
    let neg = |v: FixedQ32_32| -v.raw();
    for raw in signed_angles() {
        let a = FixedAngle::from_raw(raw);
        let signed_forms = (a.to_turns_signed(), a.to_degrees_signed(), a.to_radians_signed());
        let (turns, degrees, radians) = (a.to_turns(), a.to_degrees(), a.to_radians());
        if raw < HALF {
            assert_eq!(signed_forms, (turns, degrees, radians), "{raw:#018x}");
        } else {
            assert_eq!(signed_forms.0.raw(), turns.raw() - ONE_RAW, "{raw:#018x}");
            assert_eq!(signed_forms.1.raw(), degrees.raw() - 360 * ONE_RAW, "{raw:#018x}");
            let gap = radians.raw() - signed_forms.2.raw();
            assert!(gap == TAU_RAW || gap == TAU_RAW + 1, "{raw:#018x} gap={gap}");
        }
        if raw != HALF {
            let n = a.wrapping_neg();
            assert_eq!(n.to_turns_signed().raw(), neg(signed_forms.0), "{raw:#018x}");
            assert_eq!(n.to_degrees_signed().raw(), neg(signed_forms.1), "{raw:#018x}");
            assert_eq!(n.to_radians_signed().raw(), neg(signed_forms.2), "{raw:#018x}");
        }
    }
    // The half turn reads as -1/2; one unit below it rounds up to +1/2.
    let half = FixedAngle::HALF_TURN;
    assert_eq!(half.to_turns_signed().raw(), -ONE_RAW / 2);
    assert_eq!(half.to_degrees_signed().raw(), -180 * ONE_RAW);
    let below = FixedAngle::from_raw(HALF - 1);
    assert_eq!(below.to_turns_signed().raw(), ONE_RAW / 2);
    assert_eq!(below.to_degrees_signed().raw(), 180 * ONE_RAW);
    assert_eq!(below.to_radians_signed().raw(), -half.to_radians_signed().raw());
}
