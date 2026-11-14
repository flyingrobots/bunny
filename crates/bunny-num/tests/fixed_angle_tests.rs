//! Angle and trigonometry contract tests (`docs/topics/angle-trigonometry/test-plan.md`).

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const QUARTER: u64 = 1 << 62;
const ANGLE_PROPERTY_SEED: u64 = 0xB002_A461_0000_0001;
const ANGLE_PROPERTY_CASES: usize = 20_000;

/// `(angle raw, sin raw, cos raw)`. Each value is the correctly rounded Q32.32 of the exact
/// result, checked against 60-digit decimal arithmetic when the vectors were recorded.
const GOLDEN: &[(u64, i64, i64)] = &[
    (0x0000_0000_0000_0000, 0, 4_294_967_296),              // 0
    (0x4000_0000_0000_0000, 4_294_967_296, 0),              // 90 degrees
    (0x8000_0000_0000_0000, 0, -4_294_967_296),             // 180 degrees
    (0xc000_0000_0000_0000, -4_294_967_296, 0),             // 270 degrees
    (0x2000_0000_0000_0000, 3_037_000_500, 3_037_000_500),  // 45 degrees
    (0x6000_0000_0000_0000, 3_037_000_500, -3_037_000_500), // 135 degrees
    (0xa000_0000_0000_0000, -3_037_000_500, -3_037_000_500), // 225 degrees
    (0xe000_0000_0000_0000, -3_037_000_500, 3_037_000_500), // 315 degrees
    (0x0000_0000_0000_0001, 0, 4_294_967_296),              // one unit past zero
    (0xffff_ffff_ffff_ffff, 0, 4_294_967_296),              // one unit before a full turn
    (0x3fff_ffff_ffff_ffff, 4_294_967_296, 0),              // one unit before 90 degrees
    (0x4000_0000_0000_0001, 4_294_967_296, 0),              // one unit past 90 degrees
    (0x7fff_ffff_ffff_ffff, 0, -4_294_967_296),             // one unit before 180 degrees
    (0x8000_0000_0000_0001, 0, -4_294_967_296),             // one unit past 180 degrees
    (0x0010_0000_0000_0000, 6_588_395, 4_294_962_243),      // first table segment boundary
    (0x0018_0000_0000_0000, 9_882_587, 4_294_955_926),      // middle of the second segment
    (0x1555_5555_5555_5555, 2_147_483_648, 3_719_550_787),  // from_degrees(30)
    (0x2aaa_aaaa_aaaa_aaab, 3_719_550_787, 2_147_483_648),  // from_degrees(60)
    (0x28be_60db_9391_054a, 3_614_090_360, 2_320_580_734),  // from_radians(1)
    (0xd741_9f24_6c6e_fab6, -3_614_090_360, 2_320_580_734), // from_radians(-1)
];

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn whole(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value * ONE_RAW)
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_001_golden_vectors_are_bit_exact() {
    for &(raw, sin, cos) in GOLDEN {
        let (actual_sin, actual_cos) = FixedAngle::from_raw(raw).sin_cos();
        assert_eq!((actual_sin.raw(), actual_cos.raw()), (sin, cos), "angle={raw:#018x}");
        assert_eq!(FixedAngle::from_raw(raw).sin().raw(), sin, "sin angle={raw:#018x}");
        assert_eq!(FixedAngle::from_raw(raw).cos().raw(), cos, "cos angle={raw:#018x}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_002_cardinal_angles_are_exact() {
    for (quadrant, sin, cos) in
        [(0_u64, 0, ONE_RAW), (1, ONE_RAW, 0), (2, 0, -ONE_RAW), (3, -ONE_RAW, 0)]
    {
        let (actual_sin, actual_cos) = FixedAngle::from_raw(quadrant * QUARTER).sin_cos();
        assert_eq!((actual_sin.raw(), actual_cos.raw()), (sin, cos), "quadrant={quadrant}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_003_negation_symmetry_is_bit_exact() {
    let mut state = ANGLE_PROPERTY_SEED;
    for case in 0..ANGLE_PROPERTY_CASES {
        let angle = FixedAngle::from_raw(next_seed(&mut state));
        let (sin, cos) = angle.sin_cos();
        let (neg_sin, neg_cos) = angle.wrapping_neg().sin_cos();
        assert_eq!(neg_sin.raw(), -sin.raw(), "sin(-a) == -sin(a); case={case}");
        assert_eq!(neg_cos.raw(), cos.raw(), "cos(-a) == cos(a); case={case}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_004_error_is_within_one_raw_unit() {
    let mut state = ANGLE_PROPERTY_SEED ^ 0x5EED_0000_0000_0002;
    let mut angles: Vec<u64> = (0..ANGLE_PROPERTY_CASES).map(|_| next_seed(&mut state)).collect();
    angles.extend((0..=1_u64 << 12).map(|k| k << 50));
    // Hermite error peaks inside a segment: probe every segment at a quarter, half and
    // three quarters of its width, plus the units next to each boundary.
    for k in 0..1_u64 << 12 {
        angles.extend([1, 1 << 50, 1 << 51, 3 << 50, (1 << 52) - 1].map(|offset| (k << 52) + offset));
    }
    for raw in angles {
        let (sin, cos) = FixedAngle::from_raw(raw).sin_cos();
        // Oracle: f64 trigonometry, whose error (~1e-16) is far below one raw unit (~2.3e-10).
        #[allow(clippy::cast_precision_loss)]
        let radians = raw as f64 / 18_446_744_073_709_551_616.0 * std::f64::consts::TAU;
        #[allow(clippy::cast_precision_loss)]
        let scale = ONE_RAW as f64;
        #[allow(clippy::cast_precision_loss)]
        let errors = [
            (sin.raw() as f64 - radians.sin() * scale).abs(),
            (cos.raw() as f64 - radians.cos() * scale).abs(),
        ];
        for error in errors {
            assert!(error <= 1.0, "angle={raw:#018x} error={error} raw units");
        }
    }
}

/// A float-free cross-check of the bound: `sin^2 + cos^2` stays within what two errors of at
/// most one raw unit can explain, so a bad table entry is caught without any oracle at all.
#[wasm_bindgen_test(unsupported = test)]
fn at_tp_004b_pythagorean_identity_holds_within_the_bound() {
    let mut state = ANGLE_PROPERTY_SEED ^ 0x5EED_0000_0000_0004;
    for case in 0..ANGLE_PROPERTY_CASES {
        let (sin, cos) = FixedAngle::from_raw(next_seed(&mut state)).sin_cos();
        let (s, c) = (i128::from(sin.raw()), i128::from(cos.raw()));
        let one = i128::from(ONE_RAW);
        let drift = (s * s + c * c - one * one).abs();
        // d(s^2) = 2 s ds with |ds| <= 1, plus the ds^2 terms.
        let allowed = 2 * (s.abs() + c.abs()) + 2;
        assert!(drift <= allowed, "case={case} drift={drift} allowed={allowed}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_005_turns_wrap_exactly() {
    let quarter = FixedQ32_32::from_raw(ONE_RAW / 4);
    let five_quarters = FixedQ32_32::from_raw(ONE_RAW + ONE_RAW / 4);
    assert_eq!(FixedAngle::from_turns(quarter), FixedAngle::QUARTER_TURN);
    assert_eq!(FixedAngle::from_turns(five_quarters), FixedAngle::QUARTER_TURN);
    assert_eq!(FixedAngle::from_turns(FixedQ32_32::from_raw(-ONE_RAW / 4)).raw(), 3 * QUARTER);
    assert_eq!(FixedAngle::from_turns(whole(-3)), FixedAngle::ZERO);
    assert_eq!(
        FixedAngle::QUARTER_TURN.wrapping_add(FixedAngle::from_raw(3 * QUARTER)),
        FixedAngle::ZERO
    );
    assert_eq!(FixedAngle::ZERO.wrapping_sub(FixedAngle::QUARTER_TURN).raw(), 3 * QUARTER);
    assert_eq!(FixedAngle::HALF_TURN.wrapping_neg(), FixedAngle::HALF_TURN);
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_006_whole_quarter_degrees_convert_exactly() {
    for (degrees, raw) in [
        (0, 0),
        (90, QUARTER),
        (180, 2 * QUARTER),
        (270, 3 * QUARTER),
        (360, 0),
        (-90, 3 * QUARTER),
        (450, QUARTER),
    ] {
        assert_eq!(FixedAngle::from_degrees(whole(degrees)).raw(), raw, "degrees={degrees}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn at_tp_007_radians_round_to_the_nearest_unit_symmetrically() {
    let mut state = ANGLE_PROPERTY_SEED ^ 0x5EED_0000_0000_0003;
    for case in 0..ANGLE_PROPERTY_CASES {
        let raw = i64::from_ne_bytes(next_seed(&mut state).to_ne_bytes()) >> 20;
        let positive = FixedAngle::from_radians(FixedQ32_32::from_raw(raw));
        let negative = FixedAngle::from_radians(FixedQ32_32::from_raw(-raw));
        assert_eq!(negative, positive.wrapping_neg(), "case={case}");
    }
    // pi (rounded to Q32.32) lands within one table segment of the half turn.
    let pi = FixedAngle::from_radians(FixedQ32_32::from_raw(13_493_037_705));
    assert!(pi.raw().abs_diff(FixedAngle::HALF_TURN.raw()) < 1 << 32, "pi={:#018x}", pi.raw());
}
