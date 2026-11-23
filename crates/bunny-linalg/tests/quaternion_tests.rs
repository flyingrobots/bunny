//! Quaternion contract tests (`bunny-quaternion/1`): storage order, the length band and its
//! refusals one raw unit either side, the normalisation at the extremes of the raw range, the
//! Hamilton product and its composition order, drift over a chain, vector rotation and its
//! overflow boundary, the rotation matrix and its orthonormality bounds, axis-angle input and
//! its agreement with `bunny-rotation/1`, the sign operations, and pinned goldens.
//!
//! Every oracle is in `quaternion_oracle`, which shares no code with the kernel: exact
//! arbitrary-precision integers, the Hamilton product from the unit rules, rotation as the
//! sandwich `q v q*`, and each rounding checked by its defining inequality. The goldens and the
//! seeded digest come from a separate Python model of the documented rules (see
//! `docs/topics/quaternion/test-plan.md`).

// Quaternion components read best as x, y, z, w and a, b, q, v.
#![allow(clippy::many_single_char_names)]

mod quaternion_oracle;

use bunny_linalg::{
    FixedMat3, FixedUnitQuat, FixedUnitVec3, FixedVec3, QuatError, QUATERNION_CONTRACT_ID,
    QUAT_LENGTH_SQUARED_MAX_RAW, QUAT_LENGTH_SQUARED_MIN_RAW,
};
use bunny_num::{FixedAngle, FixedQ32_32};
use quaternion_oracle::{
    big, hamilton, is_nearest, is_normalisation, norm_squared, quat, sandwich,
};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF: i64 = ONE / 2;
const QUARTER: u64 = 1 << 62;
const SEED: u64 = 0xB002_0109_0000_0001;
const CASES: usize = 10_000;
/// Cases of the pinned digest, shared with the Python model.
const DIGEST_CASES: usize = 2_000;
/// FNV-1a over the digest corpus, from the Python model.
const EXPECTED_DIGEST: u64 = 0xA1BB_1276_9435_F52F;
/// `round(2^32 / sqrt(2))`: both components of the quarter turn about a coordinate axis.
const EIGHTH: i64 = 3_037_000_500;
/// Pairs `(b, b*)` in the drift chain: 2 000 products.
const DRIFT_PAIRS: usize = 1_000;
/// The derived agreement bound with `rotation_axis_angle`, per entry, in raw units.
const AGREEMENT_BOUND: i64 = 18;

type Raw4 = [i64; 4];

/// The seed of case `QUAT-TP-0nn`: `SEED` with `nn` mixed into its low bits.
const fn seed(case: u64) -> u64 {
    SEED ^ case
}
type Rows = [[i64; 3]; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn try_unit(r: Raw4) -> Result<FixedUnitQuat, QuatError> {
    FixedUnitQuat::try_from_unit(fixed(r[0]), fixed(r[1]), fixed(r[2]), fixed(r[3]))
}

fn unit(r: Raw4) -> FixedUnitQuat {
    try_unit(r).expect("a value in the band")
}

fn normalize(r: Raw4) -> Result<FixedUnitQuat, QuatError> {
    FixedUnitQuat::try_normalize(fixed(r[0]), fixed(r[1]), fixed(r[2]), fixed(r[3]))
}

fn raw(q: FixedUnitQuat) -> Raw4 {
    q.to_array().map(FixedQ32_32::raw)
}

fn vec(v: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(v[0]), fixed(v[1]), fixed(v[2]))
}

fn unvec(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn rows(m: FixedMat3) -> Rows {
    [unvec(m.row0()), unvec(m.row1()), unvec(m.row2())]
}

fn angle(raw: u64) -> FixedAngle {
    FixedAngle::from_raw(raw)
}

fn axis(u: [i64; 3]) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(vec(u)).expect("a FixedUnitVec3 axis")
}

fn length_squared(r: Raw4) -> u128 {
    r.iter().map(|&c| u128::from(c.unsigned_abs()).pow(2)).sum()
}

fn in_band(r: Raw4) -> bool {
    let s = length_squared(r);
    let one = u128::from(ONE.unsigned_abs());
    (one - 1).pow(2) <= s && s <= (one + 1).pow(2)
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_001_storage_order_and_representation_equality() {
    assert_eq!(QUATERNION_CONTRACT_ID, "bunny-quaternion/1");
    let q = unit([HALF, -HALF, HALF, HALF]);
    assert_eq!([q.x().raw(), q.y().raw(), q.z().raw(), q.w().raw()], [HALF, -HALF, HALF, HALF]);
    assert_eq!(raw(q), [HALF, -HALF, HALF, HALF], "to_array is (x, y, z, w)");
    assert_eq!(raw(FixedUnitQuat::IDENTITY), [0, 0, 0, ONE], "w is the scalar part");
    // q and -q: different values, the same rotation.
    assert_ne!(q, -q);
    assert_eq!(raw(-q), [-HALF, HALF, -HALF, -HALF]);
    assert_eq!(q.to_mat3(), (-q).to_mat3());
    let v = vec([3 * ONE, -5, 7 << 40]);
    assert_eq!(q.rotate_vec3(v), (-q).rotate_vec3(v));
    // The unit rules i^2 = j^2 = k^2 = ijk = -1, exactly: each product is exactly unit.
    let (i, j, k) = (unit([ONE, 0, 0, 0]), unit([0, ONE, 0, 0]), unit([0, 0, ONE, 0]));
    assert_eq!(i * j, k);
    assert_eq!(j * k, i);
    assert_eq!(k * i, j);
    assert_eq!(j * i, -k);
    assert_eq!(i * i, -FixedUnitQuat::IDENTITY);
    assert_eq!((i * j) * k, -FixedUnitQuat::IDENTITY);
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_002_the_length_band_one_raw_unit_either_side() {
    let one = u128::from(ONE.unsigned_abs());
    assert_eq!(QUAT_LENGTH_SQUARED_MIN_RAW, (one - 1).pow(2));
    assert_eq!(QUAT_LENGTH_SQUARED_MAX_RAW, (one + 1).pow(2));
    // S = (2^32 - 1)^2 - 1, by Euler's four-square identity on 2^33 (2^31 - 1).
    let below = [3_009_019_904, 3_064_725_504, 720_896, -589_824];
    assert_eq!(length_squared(below), QUAT_LENGTH_SQUARED_MIN_RAW - 1);
    assert_eq!(try_unit(below), Err(QuatError::NotUnit));
    let bottom = [ONE - 1, 0, 0, 0];
    assert_eq!(length_squared(bottom), QUAT_LENGTH_SQUARED_MIN_RAW);
    assert_eq!(raw(unit(bottom)), bottom);
    let top = [0, 0, 0, ONE + 1];
    assert_eq!(length_squared(top), QUAT_LENGTH_SQUARED_MAX_RAW);
    assert_eq!(raw(unit(top)), top);
    let above = [1, 0, 0, ONE + 1];
    assert_eq!(length_squared(above), QUAT_LENGTH_SQUARED_MAX_RAW + 1);
    assert_eq!(try_unit(above), Err(QuatError::NotUnit));
    // A FixedUnitVec3 may be up to 2^-31 long; the quaternion band is not.
    assert_eq!(try_unit([ONE + 2, 0, 0, 0]), Err(QuatError::NotUnit));
    // Sums that pass u128, and zero.
    assert_eq!(try_unit([i64::MIN; 4]), Err(QuatError::NotUnit));
    assert_eq!(try_unit([i64::MAX, i64::MAX, i64::MAX, i64::MAX]), Err(QuatError::NotUnit));
    assert_eq!(try_unit([0; 4]), Err(QuatError::NotUnit));
    assert!(QuatError::NotUnit.to_string().contains("length"));
    assert!(QuatError::Zero.to_string().contains("zero"));
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_003_normalisation_refuses_only_zero_and_accepts_the_extremes() {
    assert_eq!(normalize([0; 4]), Err(QuatError::Zero));
    let cases: [(Raw4, Raw4); 12] = [
        ([1, 0, 0, 0], [ONE, 0, 0, 0]),
        ([0, 0, 0, -1], [0, 0, 0, -ONE]),
        ([0, i64::MIN, 0, 0], [0, -ONE, 0, 0]),
        ([0, 0, i64::MAX, 0], [0, 0, ONE, 0]),
        ([1, 1, 1, 1], [HALF, HALF, HALF, HALF]),
        ([i64::MIN; 4], [-HALF; 4]),
        ([i64::MAX; 4], [HALF; 4]),
        ([i64::MIN, i64::MAX, i64::MIN, i64::MAX], [-HALF, HALF, -HALF, HALF]),
        // The small component is 2^-31 of the large one: it rounds to zero.
        ([i64::MAX, 1, 0, 0], [ONE, 0, 0, 0]),
        // (3, 4) / 5: 0.6 and 0.8 rounded to the nearest raw unit, at either scale.
        ([3, 4, 0, 0], [2_576_980_378, 3_435_973_837, 0, 0]),
        ([3 << 60, 4 << 60, 0, 0], [2_576_980_378, 3_435_973_837, 0, 0]),
        // 2^32 / sqrt(2), rounded: where FixedUnitVec3::new of (1, 1, 0) has no result.
        ([1, 0, 0, 1], [EIGHTH, 0, 0, EIGHTH]),
    ];
    for (input, expected) in cases {
        let q = normalize(input).expect("every nonzero input is accepted");
        assert_eq!(raw(q), expected, "input {input:?}");
        assert!(is_normalisation(&quat(input), raw(q)), "input {input:?}");
        assert!(in_band(raw(q)), "input {input:?}");
    }
    assert_eq!(FixedUnitVec3::new(vec([1, 1, 0])), None, "the vector rule refuses this input");
    // Exactly unit values are fixed points.
    for r in [[HALF, -HALF, HALF, HALF], [0, 0, 0, ONE], [0, -ONE, 0, 0]] {
        assert_eq!(raw(normalize(r).expect("unit")), r);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_004_seeded_normalisation_matches_the_oracle() {
    let mut state = SEED;
    let mut zeros = 0;
    for case in 0..CASES {
        let input = [0; 4].map(|_| random_raw(&mut state));
        let context = format!("seed={SEED:#x} case={case} input={input:?}");
        let Ok(q) = normalize(input) else {
            assert_eq!(input, [0; 4], "{context}");
            zeros += 1;
            continue;
        };
        assert!(is_normalisation(&quat(input), raw(q)), "{context}");
        assert!(in_band(raw(q)), "{context}");
        // Odd: the negated input gives the negated result.
        if let Some(negated) = checked_neg4(input) {
            assert_eq!(normalize(negated), Ok(-q), "{context}");
        }
        // Only the direction matters: a positive multiple gives the same bits.
        if let Some(scaled) = scale4(input, 3) {
            assert_eq!(normalize(scaled), Ok(q), "{context}");
        }
        // A value of the type renormalises to within one raw unit per component.
        let again = raw(normalize(raw(q)).expect("nonzero"));
        assert!(again.iter().zip(raw(q)).all(|(a, b)| (a - b).abs() <= 1), "{context}");
    }
    assert!(zeros < CASES / 100, "the corpus is not mostly zero");
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_005_products_compose_right_to_left() {
    let x_quarter = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_X, angle(QUARTER));
    let y_quarter = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Y, angle(QUARTER));
    assert_eq!(raw(x_quarter), [EIGHTH, 0, 0, EIGHTH]);
    assert_eq!(raw(y_quarter), [0, EIGHTH, 0, EIGHTH]);
    // The exact product has four equal components: (1/2, 1/2, 1/2, 1/2), exactly.
    let both = x_quarter * y_quarter;
    assert_eq!(raw(both), [HALF; 4]);
    let x = FixedMat3::rotation_x(angle(QUARTER));
    let y = FixedMat3::rotation_y(angle(QUARTER));
    // a * b applies b first, as the matrix product does on column vectors.
    assert_eq!(Some(both.to_mat3()), x.checked_mul_mat3(y));
    assert_eq!(rows(both.to_mat3()), [[0, 0, ONE], [ONE, 0, 0], [0, ONE, 0]]);
    for e in [[ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]] {
        let first_y = y_quarter.rotate_vec3(vec(e)).and_then(|v| x_quarter.rotate_vec3(v));
        assert_eq!(both.rotate_vec3(vec(e)), first_y, "basis {e:?}");
    }
    assert_ne!(y_quarter * x_quarter, both, "the product does not commute");
    let mut state = SEED;
    for case in 0..1_000 {
        let (a, b) = (unit(band_quat(&mut state, case)), unit(band_quat(&mut state, case + 1)));
        let context = format!("seed={SEED:#x} case={case}");
        assert_eq!((-a) * b, -(a * b), "{context}");
        assert_eq!(a * (-b), -(a * b), "{context}");
        // a a* = |a|^2, a positive scalar: exactly the identity after normalisation.
        assert_eq!(a * a.conjugate(), FixedUnitQuat::IDENTITY, "{context}");
        // The identity factor renormalises.
        assert_eq!(Ok(FixedUnitQuat::IDENTITY * a), normalize(raw(a)), "{context}");
        assert_eq!(Ok(a * FixedUnitQuat::IDENTITY), normalize(raw(a)), "{context}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_006_seeded_products_match_the_oracle_and_the_bounds() {
    let mut state = seed(6);
    for case in 0..CASES {
        let (ra, rb) = (band_quat(&mut state, case), band_quat(&mut state, case / 4));
        let product = raw(unit(ra) * unit(rb));
        let exact = hamilton(&quat(ra), &quat(rb));
        let context = format!("seed={:#x} case={case} a={ra:?} b={rb:?}", seed(6));
        assert!(is_normalisation(&exact, product), "{context}");
        assert!(in_band(product), "{context}");
        // Within 5/2 + 2^-32 raw of the exact (unnormalised) product: on raw squared units,
        // |2 (n 2^32 - p)| <= 5 2^32 + 2.
        for (n, p) in product.iter().zip(&exact) {
            let error = big(i128::from(*n) << 32).sub(p).mul(&big(2)).abs();
            assert!(error.le(&big(5 * i128::from(ONE) + 2)), "{context}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_007_drift_over_a_chain_stays_within_the_derived_bound() {
    // c0 is exactly unit and each pair multiplies by b b* = |b|^2, so the exact normalised
    // product of every prefix that ends a pair is c0 itself.
    let c0 = [HALF, -HALF, HALF, HALF];
    let mut c = unit(c0);
    let mut state = seed(7);
    let mut worst = 0_i128;
    for pair in 1..=DRIFT_PAIRS {
        let b = unit(band_quat(&mut state, pair));
        c = (c * b) * b.conjugate();
        let error: i128 = raw(c).iter().zip(c0).map(|(a, e)| i128::from(a - e).pow(2)).sum();
        // After M = 2 pair products the bound is M (1 + 2^-11) raw, Euclidean.
        let m = 2 * i128::try_from(pair).expect("small");
        assert!(error << 22 <= (m * ((1 << 11) + 1)).pow(2), "seed={:#x} pair={pair}", seed(7));
        worst = worst.max(error);
        assert!(in_band(raw(c)));
    }
    // Measured: the squared Euclidean error after 2 000 products, in raw units squared.
    assert!(worst > 0, "rounding is visible over a long chain");
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_008_quarter_and_half_turns_map_the_basis_exactly() {
    let (ex, ey, ez) = ([ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]);
    let neg = |v: [i64; 3]| v.map(|c| -c);
    let quarter = |u| FixedUnitQuat::from_axis_angle(u, angle(QUARTER));
    let half = |u| FixedUnitQuat::from_axis_angle(u, angle(2 * QUARTER));
    let cases: [(FixedUnitQuat, [[i64; 3]; 3]); 6] = [
        (quarter(FixedUnitVec3::UNIT_X), [ex, ez, neg(ey)]),
        (quarter(FixedUnitVec3::UNIT_Y), [neg(ez), ey, ex]),
        (quarter(FixedUnitVec3::UNIT_Z), [ey, neg(ex), ez]),
        (half(FixedUnitVec3::UNIT_X), [ex, neg(ey), neg(ez)]),
        (half(FixedUnitVec3::UNIT_Y), [neg(ex), ey, neg(ez)]),
        (half(FixedUnitVec3::UNIT_Z), [neg(ex), neg(ey), ez]),
    ];
    for (q, images) in cases {
        for (e, image) in [ex, ey, ez].into_iter().zip(images) {
            assert_eq!(q.rotate_vec3(vec(e)).map(unvec), Some(image), "{q:?} {e:?}");
        }
    }
    assert_eq!(raw(half(FixedUnitVec3::UNIT_X)), [-ONE, 0, 0, 0], "the half turn is (-u, 0)");
    assert_eq!(quarter(FixedUnitVec3::UNIT_Z).to_mat3(), FixedMat3::rotation_z(angle(QUARTER)));
    // The identity leaves every vector exactly.
    let v = vec([i64::MIN, i64::MAX, -12_345]);
    assert_eq!(FixedUnitQuat::IDENTITY.rotate_vec3(v), Some(v));
    // q and -q: the same bits, from a formula even in q.
    let mut state = seed(8);
    for case in 0..1_000 {
        let q = unit(band_quat(&mut state, case));
        let v = vec([0; 3].map(|_| random_raw(&mut state) >> 1));
        assert_eq!(q.rotate_vec3(v), (-q).rotate_vec3(v), "seed={:#x} case={case}", seed(8));
        assert_eq!(q.to_mat3(), (-q).to_mat3(), "seed={:#x} case={case}", seed(8));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_009_seeded_rotations_match_the_oracle_and_overflow_at_the_range() {
    let mut state = seed(9);
    let mut refused = 0;
    for case in 0..CASES {
        let r = band_quat(&mut state, case);
        let v = [0; 3].map(|_| random_raw(&mut state));
        let context = format!("seed={:#x} case={case} q={r:?} v={v:?}", seed(9));
        let exact = sandwich(r, v);
        let s = norm_squared(&quat(r));
        if let Some(out) = unit(r).rotate_vec3(vec(v)) {
            for (n, e) in unvec(out).iter().zip(&exact) {
                assert!(is_nearest(e, &s, *n), "{context}");
            }
        } else {
            assert!(exact.iter().any(|e| out_of_range(e, &s)), "{context}");
            refused += 1;
        }
        // Components of at most 2^62 never overflow.
        let small = v.map(|c| c >> 1);
        assert!(unit(r).rotate_vec3(vec(small)).is_some(), "{context}");
    }
    assert!(refused > 0 && refused < CASES / 2, "both outcomes occur: {refused}");
    // The half turn about z negates x: one raw unit inside the range, and at its end.
    let half_z = unit([0, 0, ONE, 0]);
    let max = i64::MAX;
    assert_eq!(half_z.rotate_vec3(vec([-max, 0, 0])).map(unvec), Some([max, 0, 0]));
    assert_eq!(half_z.rotate_vec3(vec([i64::MIN, 0, 0])), None);
    let quarter_z = unit([0, 0, EIGHTH, EIGHTH]);
    assert_eq!(quarter_z.rotate_vec3(vec([i64::MIN, 0, 0])).map(unvec), Some([0, i64::MIN, 0]));
    assert_eq!(quarter_z.rotate_vec3(vec([0, -max, 0])).map(unvec), Some([max, 0, 0]));
    assert_eq!(quarter_z.rotate_vec3(vec([0, i64::MIN, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_010_no_rounding_meets_an_exact_half() {
    // The oracle comparisons of QUAT-TP-004, -006, -009, -011 and -013 accept an exact half
    // only for an even result. This case checks the proof's premise directly: over a corpus
    // whose components share many factors of two, twice each error is never the divisor.
    let mut state = seed(10);
    for case in 0..2_000 {
        let r = scaled_band_quat(&mut state, case);
        let q = unit(r);
        let s = norm_squared(&quat(r));
        let v = [0; 3].map(|_| random_raw(&mut state) >> 2);
        let rotated = unvec(q.rotate_vec3(vec(v)).expect("small components"));
        for (n, e) in rotated.iter().zip(sandwich(r, v)) {
            let twice = e.sub(&s.mul(&big(i128::from(*n)))).mul(&big(2)).abs();
            assert!(twice.lt(&s), "an exact half: case={case} q={r:?} v={v:?}");
        }
        let m = rows(q.to_mat3());
        for (j, column) in [[1, 0, 0], [0, 1, 0], [0, 0, 1]].into_iter().enumerate() {
            for (i, p) in sandwich(r, column).iter().enumerate() {
                let numerator = p.shl(32);
                let twice = numerator.sub(&s.mul(&big(i128::from(m[i][j])))).mul(&big(2)).abs();
                assert!(twice.lt(&s), "an exact half in the matrix: case={case} q={r:?}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_011_seeded_matrices_match_the_oracle_and_the_bounds() {
    let mut state = seed(11);
    for case in 0..CASES {
        let r = band_quat(&mut state, case);
        let m = rows(unit(r).to_mat3());
        let s = norm_squared(&quat(r));
        let context = format!("seed={:#x} case={case} q={r:?}", seed(11));
        for (j, column) in [[1, 0, 0], [0, 1, 0], [0, 0, 1]].into_iter().enumerate() {
            for (i, p) in sandwich(r, column).iter().enumerate() {
                assert!(is_nearest(&p.shl(32), &s, m[i][j]), "entry {i}{j}; {context}");
            }
        }
        assert_orthonormal(m, (1, 2, 3), &context);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_012_axis_angle_exact_cases() {
    let mut state = seed(12);
    for case in 0..1_000 {
        let u = axis(random_axis(&mut state, case));
        let a = angle(next64(&mut state));
        let context = format!("seed={:#x} case={case}", seed(12));
        assert_eq!(FixedUnitQuat::from_axis_angle(u, FixedAngle::ZERO), FixedUnitQuat::IDENTITY);
        let q = FixedUnitQuat::from_axis_angle(u, a);
        assert!(q.w().raw() >= 0, "{context}");
        if a != FixedAngle::HALF_TURN {
            let back = FixedUnitQuat::from_axis_angle(u, a.wrapping_neg());
            assert_eq!(back, q.conjugate(), "{context}");
        }
        let half = FixedUnitQuat::from_axis_angle(u, FixedAngle::HALF_TURN);
        let d = u.into_inner();
        let minus_u = normalize([-d.x.raw(), -d.y.raw(), -d.z.raw(), 0]).expect("nonzero");
        assert_eq!(half, minus_u, "{context}");
    }
    // The quarter turn about z: sin and cos of an eighth turn are equal bit for bit.
    let q = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Z, angle(QUARTER));
    assert_eq!(raw(q), [0, 0, EIGHTH, EIGHTH]);
    // An odd raw angle: halved toward zero, so +1 and -1 give conjugates.
    let plus = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Y, angle(1));
    assert_eq!(plus, FixedUnitQuat::IDENTITY, "half of one angle unit is zero");
    let minus = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Y, angle(u64::MAX));
    assert_eq!(minus, FixedUnitQuat::IDENTITY);
    // sin_cos steps from 0 to 1 raw between the half angles 0x145F_306E and 0x145F_306F, so
    // the odd angle 0x28BE_60DD and its negation have sine 0 at the half toward zero; halving
    // the negation by floor would reach 0x145F_306F below zero, with sine -1 raw.
    let step: u64 = 0x28BE_60DD;
    let plus = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Z, angle(step));
    let minus = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Z, angle(step.wrapping_neg()));
    assert_eq!((plus, minus), (FixedUnitQuat::IDENTITY, FixedUnitQuat::IDENTITY));
    let next = FixedUnitQuat::from_axis_angle(FixedUnitVec3::UNIT_Z, angle(step + 1));
    assert_eq!(raw(next), [0, 0, 1, ONE], "the half angle 0x145F_306F has sine 1 raw");
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_013_seeded_axis_angle_matches_the_oracle_and_rotation_axis_angle() {
    let mut state = seed(13);
    let mut worst = 0;
    for case in 0..CASES {
        let u = random_axis(&mut state, case);
        let a = angle(next64(&mut state));
        let q = FixedUnitQuat::from_axis_angle(axis(u), a);
        let context = format!("seed={:#x} case={case} u={u:?} angle={:#x}", seed(13), a.raw());
        // The documented input: sin and cos of the signed angle halved toward zero.
        let half = angle((a.raw().cast_signed() / 2).cast_unsigned());
        let (s, c) = half.sin_cos();
        let s = i128::from(s.raw());
        let [x, y, z] = u.map(i128::from);
        let p = [s * x, s * y, s * z, i128::from(c.raw()) << 32].map(big);
        assert!(is_normalisation(&p, raw(q)), "{context}");
        assert!(in_band(raw(q)), "{context}");
        let from_quat = rows(q.to_mat3());
        let rodrigues = rows(FixedMat3::rotation_axis_angle(axis(u), a));
        for (row, other) in from_quat.iter().zip(rodrigues) {
            for (e, f) in row.iter().zip(other) {
                worst = worst.max((e - f).abs());
            }
        }
        assert!(worst <= AGREEMENT_BOUND, "{context}");
    }
    assert!(worst > 0, "the two paths round differently");
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_014_pinned_goldens_and_digest() {
    // Case 0 of the digest corpus, from the Python model.
    let a = unit([-1_258_418_262, -827_554_770, -4_001_651_733, -406_280_778]);
    let b = unit([4_260_264_693, 145_396_284, -37_422_028, 523_782_707]);
    assert_eq!(raw(a * b), [-413_788_321, -4_094_959_876, 293_795_230, 1_191_852_035]);
    let v = vec([3, -53_287_043, -44_924_578]);
    assert_eq!(a.rotate_vec3(v).map(unvec), Some([-22_789_221, 34_737_190, -55_961_645]));
    assert_eq!(
        rows(a.to_mat3()),
        [
            [-3_480_674_440, -272_125_073, 2_501_519_030],
            [1_242_013_748, -3_899_196_661, 1_303_998_208],
            [2_188_390_131, 1_780_156_572, 3_238_631_700],
        ]
    );
    let u = axis([3_484_598_525, -2_324_832_238, -948_405_122]);
    let q = FixedUnitQuat::from_axis_angle(u, angle(0x0EBC_85AB_AD79_128F));
    assert_eq!(raw(q), [626_736_076, -418_141_781, -170_579_107, 4_224_926_767]);
    let input = [160_720_203_838, -16_254_302_004_955, -20_809_308, -3_535_175_881_646_691];
    let n = normalize(input).expect("nonzero");
    assert_eq!(raw(n), [195_261, -19_747_520, -25, -4_294_921_893]);
    let mut state = seed(14);
    let mut digest = 0xCBF2_9CE4_8422_2325;
    for case in 0..DIGEST_CASES {
        let (a, b) = (band_quat(&mut state, case), band_quat(&mut state, case + 2));
        let v = [0; 3].map(|_| random_raw(&mut state));
        let u = random_axis(&mut state, case);
        let turn = next64(&mut state);
        let input = [0; 4].map(|_| random_raw(&mut state));
        let (a, b) = (unit(a), unit(b));
        digest = match normalize(input) {
            Ok(n) => fnv_raw(digest, &raw(n)),
            Err(_) => fnv(digest, b"Z"),
        };
        digest = fnv_raw(digest, &raw(a * b));
        digest = match a.rotate_vec3(vec(v)) {
            Some(out) => fnv_raw(digest, &unvec(out)),
            None => fnv(digest, b"N"),
        };
        digest = fnv_raw(digest, rows(a.to_mat3()).as_flattened());
        digest = fnv_raw(digest, &raw(FixedUnitQuat::from_axis_angle(axis(u), angle(turn))));
    }
    assert_eq!(digest, EXPECTED_DIGEST, "seed={:#x} cases={DIGEST_CASES}", seed(14));
}

#[wasm_bindgen_test(unsupported = test)]
fn quat_tp_015_conjugate_negation_and_canonical_sign() {
    let q = unit([HALF, -HALF, HALF, -HALF]);
    assert_eq!(raw(q.conjugate()), [-HALF, HALF, -HALF, -HALF]);
    assert_eq!(raw(q.canonical()), [-HALF, HALF, -HALF, HALF], "w < 0 is negated");
    // At w = 0 the sign follows x, then y, then z.
    let ties: [(Raw4, Raw4); 4] = [
        ([-ONE, 0, 0, 0], [ONE, 0, 0, 0]),
        ([0, -ONE, 0, 0], [0, ONE, 0, 0]),
        ([0, 0, -ONE, 0], [0, 0, ONE, 0]),
        ([-EIGHTH, EIGHTH, 0, 0], [EIGHTH, -EIGHTH, 0, 0]),
    ];
    for (input, expected) in ties {
        assert_eq!(raw(unit(input).canonical()), expected, "{input:?}");
        assert_eq!(raw(unit(expected).canonical()), expected, "{input:?}");
    }
    let mut state = seed(15);
    for case in 0..1_000 {
        let q = unit(band_quat(&mut state, case));
        let context = format!("seed={:#x} case={case}", seed(15));
        assert_eq!(q.conjugate().conjugate(), q, "{context}");
        assert_eq!(-(-q), q, "{context}");
        assert_eq!(q.canonical(), (-q).canonical(), "{context}");
        assert_eq!(q.canonical().canonical(), q.canonical(), "{context}");
        assert!(q.canonical() == q || q.canonical() == -q, "{context}");
        assert_eq!(q.conjugate().to_mat3(), q.to_mat3().transpose(), "{context}");
    }
}

/// Lengths within `bounds.0`, dot products within `bounds.1` and the determinant within
/// `bounds.2` raw units, decided exactly on the raw entries.
fn assert_orthonormal(m: Rows, bounds: (i128, i128, i128), context: &str) {
    let one = i128::from(ONE);
    let dot = |a: [i64; 3], b: [i64; 3]| -> i128 {
        a.iter().zip(b).map(|(x, y)| i128::from(*x) * i128::from(y)).sum()
    };
    let cols = [0, 1, 2].map(|j| [m[0][j], m[1][j], m[2][j]]);
    for line in m.iter().chain(&cols) {
        let square = dot(*line, *line);
        assert!(
            (one - bounds.0).pow(2) <= square && square <= (one + bounds.0).pow(2),
            "{context}"
        );
    }
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        assert!(dot(m[a], m[b]).abs() <= bounds.1 * one, "rows {a}.{b}; {context}");
        assert!(dot(cols[a], cols[b]).abs() <= bounds.1 * one, "columns {a}.{b}; {context}");
    }
    let e = |i: usize, j: usize| big(i128::from(m[i][j]));
    let det = e(0, 0)
        .mul(&e(1, 1).mul(&e(2, 2)).sub(&e(1, 2).mul(&e(2, 1))))
        .sub(&e(0, 1).mul(&e(1, 0).mul(&e(2, 2)).sub(&e(1, 2).mul(&e(2, 0)))))
        .add(&e(0, 2).mul(&e(1, 0).mul(&e(2, 1)).sub(&e(1, 1).mul(&e(2, 0)))));
    let error = det.sub(&big(1).shl(96)).abs();
    assert!(error.le(&big(bounds.2).shl(64)), "determinant; {context}");
}

/// `n` would round outside `i64`: `2e >= (2^64 - 1) s` or `2e < -(2^64 + 1) s`.
fn out_of_range(e: &quaternion_oracle::Big, s: &quaternion_oracle::Big) -> bool {
    let twice = e.mul(&big(2));
    let high = s.mul(&big((1_i128 << 64) - 1));
    let low = s.mul(&big(-(1_i128 << 64) - 1));
    high.le(&twice) || twice.lt(&low)
}

fn checked_neg4(r: Raw4) -> Option<Raw4> {
    Some([r[0].checked_neg()?, r[1].checked_neg()?, r[2].checked_neg()?, r[3].checked_neg()?])
}

fn scale4(r: Raw4, k: i64) -> Option<Raw4> {
    Some([r[0].checked_mul(k)?, r[1].checked_mul(k)?, r[2].checked_mul(k)?, r[3].checked_mul(k)?])
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn next64(state: &mut u64) -> u64 {
    let (a, b, c) = (next(state), next(state), next(state));
    (a << 33) ^ (b << 2) ^ (c & 3)
}

/// A raw value of random width: `bits` uniform in `0..=64`, then that many random low bits and
/// a random sign, clamped to the `i64` range (so both extremes occur).
fn random_raw(state: &mut u64) -> i64 {
    let bits = next(state) % 65;
    let magnitude = if bits == 0 { 0 } else { next64(state) >> (64 - bits) };
    let negative = next(state) % 2 == 1;
    let value = if negative { -i128::from(magnitude) } else { i128::from(magnitude) };
    i64::try_from(value).unwrap_or(if negative { i64::MIN } else { i64::MAX })
}

/// Uniform in `[-m, m]`.
fn signed_within(state: &mut u64, m: u128) -> i128 {
    let span = u64::try_from(2 * m + 1).expect("m is below 2^33");
    i128::from(next64(state) % span) - i128::try_from(m).expect("m is below 2^33")
}

fn isqrt(n: i128) -> i128 {
    i128::try_from(u128::try_from(n).expect("non-negative").isqrt()).expect("below 2^64")
}

/// A quaternion anywhere in the band, built without the kernel: the raw sum of squares is
/// aimed at the top of the band, its bottom or a uniform point between; three components are
/// uniform within the remaining length, the fourth its floor square root (raised by one if that
/// leaves the band), with a random sign and a random rotation of the four.
fn band_quat(state: &mut u64, case: usize) -> Raw4 {
    let one = i128::from(ONE);
    let r = quat_between(state, case, (one - 1).pow(2), (one + 1).pow(2));
    r.map(|v| i64::try_from(v).expect("a band component"))
}

/// A band quaternion whose components share `case % 17` factors of two: drawn at the band
/// divided by `4^k`, redrawn until it lands inside, and scaled by `2^k`.
fn scaled_band_quat(state: &mut u64, case: usize) -> Raw4 {
    let k = u32::try_from(case % 17).expect("small");
    let one = i128::from(ONE);
    let (low, high) = ((one - 1).pow(2), (one + 1).pow(2));
    let scale = 1_i128 << (2 * k);
    let (low_k, high_k) = ((low + scale - 1) / scale, high / scale);
    loop {
        let r = quat_between(state, case, low_k, high_k);
        let s: i128 = r.iter().map(|v| v * v).sum();
        if low_k <= s && s <= high_k {
            return r.map(|v| i64::try_from(v << k).expect("a band component"));
        }
    }
}

/// Four integers whose sum of squares is aimed at `[low, high]` (see [`band_quat`]); inside
/// it whenever `high - low` is at least twice the largest component.
fn quat_between(state: &mut u64, case: usize, low: i128, high: i128) -> [i128; 4] {
    let span = u64::try_from(high - low + 1).expect("the band is below 2^35");
    let target = match case % 4 {
        0 => high,
        1 => low,
        _ => low + i128::from(next64(state) % span),
    };
    let a = signed_within(state, isqrt(target).unsigned_abs());
    let b = signed_within(state, isqrt(target - a * a).unsigned_abs());
    let c = signed_within(state, isqrt(target - a * a - b * b).unsigned_abs());
    let mut d = isqrt(target - a * a - b * b - c * c);
    if a * a + b * b + c * c + d * d < low {
        d += 1;
    }
    if next(state) % 2 == 1 {
        d = -d;
    }
    let mut r = [a, b, c, d];
    r.rotate_left(usize::try_from(next(state) % 4).expect("below 4"));
    r
}

/// An axis anywhere in the `FixedUnitVec3` band, as `bunny-rotation/1`'s tests draw them.
fn random_axis(state: &mut u64, case: usize) -> [i64; 3] {
    let one = i128::from(ONE);
    let (low, high) = ((one - 1).pow(2), (one + 2).pow(2) - 1);
    let span = u64::try_from(high - low + 1).expect("the band is below 2^35");
    let target = match case % 4 {
        0 => high,
        1 => low,
        _ => low + i128::from(next64(state) % span),
    };
    let a = signed_within(state, isqrt(target).unsigned_abs());
    let b = signed_within(state, isqrt(target - a * a).unsigned_abs());
    let mut c = isqrt(target - a * a - b * b);
    if (isqrt(a * a + b * b + c * c) - one).abs() > 1 {
        c += 1;
    }
    if next(state) % 2 == 1 {
        c = -c;
    }
    let mut u = [a, b, c].map(|v| i64::try_from(v).expect("a unit component"));
    u.rotate_left(usize::try_from(next(state) % 3).expect("below 3"));
    u
}

fn fnv_raw(digest: u64, values: &[i64]) -> u64 {
    values.iter().fold(digest, |d, v| fnv(d, &v.to_le_bytes()))
}

/// FNV-1a over the result stream, matching the Python model byte for byte.
fn fnv(mut digest: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(0x100_0000_01B3);
    }
    digest
}
