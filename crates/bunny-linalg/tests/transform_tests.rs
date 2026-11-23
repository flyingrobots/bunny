//! Transform composition tests (`bunny-transform/1`): the scale, rotate, translate order of
//! `FixedAffine3::from_trs`, the pivot formulas of `rotate_about_point` and
//! `scale_about_point`, ties, pinned goldens, overflow one raw unit either side of each
//! threshold, zero and negative scales, and the composition bound of `checked_mul_affine`.
//!
//! Every oracle here is exact `i128` arithmetic written in this file: products are rounded by
//! `div_euclid`, `rem_euclid` and a comparison of twice the remainder, and range checks are
//! explicit comparisons with `i64::MIN` and `i64::MAX`. The goldens come from a separate Python
//! model of the documented rule (see `docs/topics/transform/test-plan.md`).

use bunny_linalg::{FixedAffine3, FixedMat3, FixedUnitVec3, FixedVec3, TRANSFORM_CONTRACT_ID};
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const QUARTER: u64 = 1 << 62;
const HALF: u64 = 1 << 63;
const SEED: u64 = 0xB002_0312_0000_0001;
const CASES: usize = 10_000;
/// How many of the 10 000 seeded `from_trs` inputs are in range.
const EXPECTED_TRS_SOME: usize = 4_682;
/// `from_degrees(30)` and `from_degrees(60)`.
const THIRTY_DEGREES: u64 = 0x1555_5555_5555_5555;
const SIXTY_DEGREES: u64 = 0x2AAA_AAAA_AAAA_AAAB;

type Rows = [[i64; 3]; 3];
type Affine = (Rows, [i64; 3]);

fn raw_vec(v: [i64; 3]) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(v[0]), raw(v[1]), raw(v[2]))
}

fn units(x: i64, y: i64, z: i64) -> FixedVec3 {
    raw_vec([x * ONE, y * ONE, z * ONE])
}

fn matrix(m: Rows) -> FixedMat3 {
    FixedMat3::from_rows(raw_vec(m[0]), raw_vec(m[1]), raw_vec(m[2]))
}

fn vec_raw(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn rows(m: FixedMat3) -> Rows {
    [vec_raw(m.row0()), vec_raw(m.row1()), vec_raw(m.row2())]
}

fn parts(a: FixedAffine3) -> Affine {
    (rows(a.linear), vec_raw(a.translation))
}

fn angle(raw: u64) -> FixedAngle {
    FixedAngle::from_raw(raw)
}

fn translate(t: FixedVec3) -> FixedAffine3 {
    FixedAffine3::from_parts(FixedMat3::IDENTITY, t)
}

fn linear(m: FixedMat3) -> FixedAffine3 {
    FixedAffine3::from_parts(m, units(0, 0, 0))
}

fn diagonal(s: [i64; 3]) -> Rows {
    [[s[0], 0, 0], [0, s[1], 0], [0, 0, s[2]]]
}

/// `numerator / 2^32` rounded to nearest, ties to even.
fn round32(numerator: i128) -> i128 {
    let unit = 1_i128 << 32;
    let (floor, remainder) = (numerator.div_euclid(unit), numerator.rem_euclid(unit));
    floor + i128::from(2 * remainder > unit || (2 * remainder == unit && floor % 2 != 0))
}

fn in_range(value: i128) -> Option<i64> {
    i64::try_from(value).ok()
}

fn oracle_mul(a: i64, b: i64) -> Option<i64> {
    in_range(round32(i128::from(a) * i128::from(b)))
}

/// `R * diag(s)` with each entry rounded once; the translation copied.
fn oracle_trs(t: [i64; 3], r: Rows, s: [i64; 3]) -> Option<Affine> {
    let mut out = [[0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = oracle_mul(r[i][j], s[j])?;
        }
    }
    Some((out, t))
}

/// `p - R p`, where `R p` sums three rounded products, each partial sum in range.
fn oracle_about(p: [i64; 3], r: Rows) -> Option<Affine> {
    let mut t = [0; 3];
    for i in 0..3 {
        let mut sum = 0_i128;
        for j in 0..3 {
            sum = i128::from(in_range(sum + i128::from(oracle_mul(r[i][j], p[j])?))?);
        }
        t[i] = in_range(i128::from(p[i]) - sum)?;
    }
    Some((r, t))
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_001_scale_then_rotate_then_translate() {
    assert_eq!(TRANSFORM_CONTRACT_ID, "bunny-transform/1");
    // rotation_y(quarter) has rows (0, 0, 1), (0, 1, 0), (-1, 0, 0): +X -> -Z, +Z -> +X.
    // Scale first: (1, 0, 0) -> (2, 0, 0). Rotate: (2, 0, 0) -> (0, 0, -2). Translate by
    // (1, 0, 0): (1, 0, -2). The wrong order rotates first, (1, 0, 0) -> (0, 0, -1), then
    // scales z by 4, (0, 0, -4), then translates: (1, 0, -4). The scale is non-uniform so the
    // two orders differ.
    let rotation = FixedMat3::rotation_y(angle(QUARTER));
    let (t, s) = (units(1, 0, 0), units(2, 3, 4));
    let a = FixedAffine3::from_trs(t, rotation, s).expect("small entries");
    assert_eq!(a.checked_transform_point(units(1, 0, 0)), Some(units(1, 0, -2)));
    assert_eq!(rows(a.linear), [[0, 0, 4 * ONE], [0, 3 * ONE, 0], [-2 * ONE, 0, 0]]);
    assert_eq!(a.checked_transform_vector(units(1, 0, 0)), Some(units(0, 0, -2)));
    let swapped = matrix(diagonal([2 * ONE, 3 * ONE, 4 * ONE])).checked_mul_mat3(rotation);
    let wrong = FixedAffine3::from_parts(swapped.expect("small entries"), t);
    assert_eq!(wrong.checked_transform_point(units(1, 0, 0)), Some(units(1, 0, -4)));
    // The same transform as T * R * S composed right to left.
    let scale = linear(matrix(diagonal([2 * ONE, 3 * ONE, 4 * ONE])));
    let composed = linear(rotation)
        .checked_mul_affine(scale)
        .and_then(|rs| translate(t).checked_mul_affine(rs));
    assert_eq!(composed, Some(a));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_002_rotation_and_scale_about_a_point_fix_the_pivot() {
    // rotation_y(half) has rows (-1, 0, 0), (0, 1, 0), (0, 0, -1). About (1, 0, 0): (2, 0, 0)
    // is one unit past the pivot along +X, so it lands one unit before it, at the origin.
    let pivot = units(1, 0, 0);
    let half = FixedMat3::rotation_y(angle(HALF));
    let a = FixedAffine3::rotate_about_point(pivot, half).expect("small entries");
    assert_eq!(parts(a), (rows(half), [2 * ONE, 0, 0]));
    assert_eq!(a.checked_transform_point(units(2, 0, 0)), Some(units(0, 0, 0)));
    assert_eq!(a.checked_transform_point(pivot), Some(pivot));
    let away = translate(units(-1, 0, 0));
    let literal = linear(half)
        .checked_mul_affine(away)
        .and_then(|ra| translate(pivot).checked_mul_affine(ra));
    assert_eq!(literal, Some(a));
    // Scale (2, -1, 0) about (1, 2, 3): q -> p + s (q - p), so (2, 2, 2) -> (3, 2, 3).
    let pivot = units(1, 2, 3);
    let s = units(2, -1, 0);
    let b = FixedAffine3::scale_about_point(pivot, s).expect("small entries");
    assert_eq!(parts(b), (diagonal([2 * ONE, -ONE, 0]), [-ONE, 4 * ONE, 3 * ONE]));
    assert_eq!(b.checked_transform_point(units(2, 2, 2)), Some(units(3, 2, 3)));
    assert_eq!(b.checked_transform_point(pivot), Some(pivot));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_003_products_on_a_half_round_to_even() {
    // At 60 degrees the cosine is exactly 2^31 raw, so m00 of rotation_z is one half and
    // m00 * s is exactly s / 2 raw: a tie for odd s.
    let sixty = FixedMat3::rotation_z(angle(SIXTY_DEGREES));
    assert_eq!(sixty.m00.raw(), 1 << 31);
    for (s, half, rest) in [(1, 0, 1), (-1, 0, -1), (3, 2, 1), (-3, -2, -1), (5, 2, 3)] {
        let v = raw_vec([s, 0, 0]);
        let a = FixedAffine3::from_trs(units(0, 0, 0), sixty, raw_vec([s, ONE, ONE]));
        assert_eq!(a.map(|a| a.linear.m00.raw()), Some(half), "from_trs s={s}");
        // About (s, 0, 0) raw: t_x = s - round(s / 2).
        let b = FixedAffine3::rotate_about_point(v, sixty).expect("small entries");
        assert_eq!(b.translation.x.raw(), rest, "rotate_about_point s={s}");
        let c = FixedAffine3::scale_about_point(v, raw_vec([1 << 31, ONE, ONE]));
        assert_eq!(c.map(|c| c.translation.x.raw()), Some(rest), "scale_about_point s={s}");
        assert_eq!(b.checked_transform_point(v), Some(v));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_004_pinned_goldens() {
    let r30 = FixedMat3::rotation_z(angle(THIRTY_DEGREES));
    assert_eq!(rows(r30)[0], [3_719_550_787, -2_147_483_648, 0]);
    // The 60 degree rotation about the rounded (2/7, 3/7, 6/7) pinned by ROT-TP-005.
    let axis = raw_vec([1_227_133_513, 1_840_700_270, 3_681_400_539]);
    let unit = FixedUnitVec3::try_from_unit(axis).expect("a unit axis");
    let r60 = FixedMat3::rotation_axis_angle(unit, angle(SIXTY_DEGREES));
    assert_eq!(rows(r60)[0], [2_322_788_436, -2_925_229_207, 2_120_007_557]);
    let scale = raw_vec([5_583_457_485, -2_147_483_649, 7 * ONE]);
    let g1 =
        FixedAffine3::from_trs(raw_vec([6_442_450_944, -9_663_676_416, 1024 * ONE]), r30, scale);
    let g1 = g1.expect("in range");
    assert_eq!(
        rows(g1.linear),
        [[4_835_416_023, 1_073_741_824, 0], [2_791_728_742, -1_859_775_394, 0], [0, 0, 7 * ONE]]
    );
    let image = g1.checked_transform_point(units(2, 3, -1)).map(vec_raw);
    assert_eq!(image, Some([19_334_508_462, -9_659_545_114, 4_367_981_740_032]));
    let s2 = raw_vec([5_368_709_120, 10_737_418_240, -2_147_483_648]);
    let g2 = FixedAffine3::from_trs(raw_vec([-3_221_225_472, 3 * ONE, 536_870_912]), r60, s2);
    let g2 = g2.expect("in range");
    assert_eq!(
        rows(g2.linear),
        [
            [2_903_485_545, -7_313_073_018, -1_060_003_778],
            [4_313_929_462, 6_354_798_550, 136_928_626],
            [-1_335_223_540, 4_629_000_850, -1_862_613_368],
        ]
    );
    let image = g2.checked_transform_point(units(1, -2, 3)).map(vec_raw);
    assert_eq!(image, Some([11_128_394_775, 4_900_020_128, -15_644_194_432]));
    let goldens = [
        (
            FixedAffine3::rotate_about_point(
                raw_vec([45_097_156_608, -13_958_643_712, 7 * ONE]),
                r30,
            ),
            [-937_448_512, -24_418_681_958, 0],
        ),
        (
            FixedAffine3::rotate_about_point(
                raw_vec([-6_442_450_944, 2 * ONE, 3_221_225_472]),
                r60,
            ),
            [1_302_184_456, 8_888_204_046, -4_878_163_508],
        ),
        (
            FixedAffine3::scale_about_point(
                raw_vec([6_442_450_944, 10_737_418_240, -16_106_127_360]),
                scale,
            ),
            [-1_932_735_284, 16_106_127_362, 96_636_764_160],
        ),
    ];
    for (index, (affine, translation)) in goldens.into_iter().enumerate() {
        let affine = affine.expect("in range");
        assert_eq!(vec_raw(affine.translation), translation, "golden {index}");
        let origin = affine.checked_transform_point(units(0, 0, 0)).map(vec_raw);
        assert_eq!(origin, Some(translation), "golden {index}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_005_seeded_constructors_match_the_oracle_and_fix_the_pivot() {
    let mut state = SEED;
    let (mut some, mut none, mut trs_some) = (0, 0, 0);
    for case in 0..CASES {
        let m = [0; 3].map(|_| [0; 3].map(|_| raw_any(&mut state)));
        let (shift, scale, pivot) = (triple(&mut state), triple(&mut state), triple(&mut state));
        let context =
            format!("seed={SEED:#x} case={case} m={m:?} t={shift:?} s={scale:?} p={pivot:?}");
        let trs = FixedAffine3::from_trs(raw_vec(shift), matrix(m), raw_vec(scale));
        assert_eq!(trs.map(parts), oracle_trs(shift, m, scale), "from_trs; {context}");
        let about = FixedAffine3::rotate_about_point(raw_vec(pivot), matrix(m));
        assert_eq!(about.map(parts), oracle_about(pivot, m), "rotate; {context}");
        let scaled = FixedAffine3::scale_about_point(raw_vec(pivot), raw_vec(scale));
        assert_eq!(scaled.map(parts), oracle_about(pivot, diagonal(scale)), "scale; {context}");
        if let Some(a) = trs {
            trs_some += 1;
            assert_trs_bound(parts(a), m, scale, &context);
        }
        for (result, linear) in [(about, m), (scaled, diagonal(scale))] {
            match result {
                Some(a) => {
                    some += 1;
                    assert_eq!(
                        a.checked_transform_point(raw_vec(pivot)),
                        Some(raw_vec(pivot)),
                        "{context}"
                    );
                    assert_about_bound(parts(a).1, pivot, linear, &context);
                }
                None => none += 1,
            }
        }
    }
    // Both outcomes are exercised; the counts are fixed by the seed.
    let counts =
        format!("seed={SEED:#x}: from_trs {trs_some} Some; about {some} Some, {none} None");
    assert_eq!((trs_some, some, none), (EXPECTED_TRS_SOME, 11_717, 8_283), "{counts}");
}

/// Each linear entry within 1/2 raw of the exact `R_ij s_j`.
fn assert_trs_bound(a: Affine, r: Rows, s: [i64; 3], context: &str) {
    for i in 0..3 {
        for j in 0..3 {
            let exact = i128::from(r[i][j]) * i128::from(s[j]);
            let error = (i128::from(a.0[i][j]) << 32) - exact;
            assert!(2 * error.abs() <= 1 << 32, "entry {i}{j}; {context}");
        }
    }
}

/// Each translation component within 3/2 raw of the exact `p - R p`. Every product fits when
/// the constructor succeeds, so each is below `2^96` raw over `2^32`.
fn assert_about_bound(t: [i64; 3], p: [i64; 3], m: Rows, context: &str) {
    for i in 0..3 {
        let product: i128 = (0..3).map(|j| i128::from(m[i][j]) * i128::from(p[j])).sum();
        let error = (i128::from(t[i]) << 32) - ((i128::from(p[i]) << 32) - product);
        assert!(2 * error.abs() <= 3 << 32, "translation {i}; {context}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_006_overflow_is_none_one_raw_unit_past_each_threshold() {
    let (min, max) = (i64::MIN, i64::MAX);
    let half = FixedMat3::rotation_y(angle(HALF));
    let origin = units(0, 0, 0);
    // m00 = -1: (-1) * s is in range for s = MIN + 1 and not for s = MIN.
    let trs = |r: FixedMat3, x: i64| FixedAffine3::from_trs(origin, r, raw_vec([x, ONE, ONE]));
    assert_eq!(trs(half, min + 1).map(|a| a.linear.m00.raw()), Some(max));
    assert_eq!(trs(half, min), None);
    // The longest-axis half turn of ROT-TP-009 has m00 = 1 + 4 raw.
    let long = FixedUnitVec3::try_from_unit(raw_vec([ONE + 1, 92_681, 0])).expect("unit");
    let long = FixedMat3::rotation_axis_angle(long, angle(HALF));
    assert_eq!(long.m00.raw(), ONE + 4);
    assert_eq!(trs(long, 0x7FFF_FFFE_0000_0007).map(|a| a.linear.m00.raw()), Some(max));
    assert_eq!(trs(long, 0x7FFF_FFFE_0000_0008), None);
    assert_eq!(trs(long, -0x7FFF_FFFE_0000_0008).map(|a| a.linear.m00.raw()), Some(min));
    assert_eq!(trs(long, -0x7FFF_FFFE_0000_0009), None);
    // The translation is copied: the extremes are accepted.
    let ends = raw_vec([min, max, 0]);
    let copied = FixedAffine3::from_trs(ends, FixedMat3::IDENTITY, units(1, 1, 1));
    assert_eq!(copied, Some(FixedAffine3::from_parts(FixedMat3::IDENTITY, ends)));
    // About p with m00 = -1, t_x = p_x - (-p_x) = 2 p_x.
    let about = |r: FixedMat3, x: i64| FixedAffine3::rotate_about_point(raw_vec([x, 0, 0]), r);
    let scale = |s: i64, x: i64| {
        FixedAffine3::scale_about_point(raw_vec([x, 0, 0]), raw_vec([s, ONE, ONE]))
    };
    for pivot_formula in [true, false] {
        let build = |x| if pivot_formula { about(half, x) } else { scale(-ONE, x) };
        assert_eq!(build((1 << 62) - 1).map(|a| a.translation.x.raw()), Some(max - 1));
        assert_eq!(build(1 << 62), None);
        assert_eq!(build(-(1 << 62)).map(|a| a.translation.x.raw()), Some(min));
        assert_eq!(build(-(1 << 62) - 1), None);
    }
    // Scale 2: the product 2 p_x leaves the range first; t_x = -p_x.
    assert_eq!(scale(2 * ONE, (1 << 62) - 1).map(|a| a.translation.x.raw()), Some(1 - (1 << 62)));
    assert_eq!(scale(2 * ONE, 1 << 62), None);
    assert_eq!(scale(2 * ONE, -(1 << 62)).map(|a| a.translation.x.raw()), Some(1 << 62));
    assert_eq!(scale(2 * ONE, -(1 << 62) - 1), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_007_zero_and_negative_scales_are_accepted() {
    let origin = units(0, 0, 0);
    let quarter = FixedMat3::rotation_z(angle(QUARTER));
    let flat = FixedAffine3::from_trs(origin, quarter, units(0, 1, 1)).expect("accepted");
    assert_eq!(flat.linear.determinant(), Some(FixedQ32_32::ZERO));
    assert_eq!(flat.try_inverse(), None);
    let mirror = FixedAffine3::from_trs(origin, FixedMat3::IDENTITY, units(-1, 1, 1));
    let mirror = mirror.expect("accepted");
    assert_eq!(mirror.linear.determinant(), Some(FixedQ32_32::from_raw(-ONE)));
    assert_eq!(mirror.checked_transform_point(units(1, 2, 3)), Some(units(-1, 2, 3)));
    let collapse = FixedAffine3::scale_about_point(units(1, 1, 1), origin).expect("accepted");
    assert_eq!(collapse.checked_transform_point(units(5, -3, 2)), Some(units(1, 1, 1)));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_008_composition_is_associative_within_the_rounding_bound() {
    let mut state = SEED ^ 0x0008;
    for case in 0..CASES {
        let [a, b, c] = [0; 3].map(|_| random_trs(&mut state));
        let context = format!("seed={SEED:#x}^8 case={case} a={a:?} b={b:?} c={c:?}");
        let left = a.checked_mul_affine(b).and_then(|ab| ab.checked_mul_affine(c));
        let right = b.checked_mul_affine(c).and_then(|bc| a.checked_mul_affine(bc));
        let (left, right) = (parts(left.expect("in range")), parts(right.expect("in range")));
        let (a, c) = (parts(a), parts(c));
        let (lin, trans) = exact_product(a, parts(b), c);
        let unit = 1_i128 << 64;
        for i in 0..3 {
            let a_row: i128 = a.0[i].iter().map(|&e| i128::from(e).abs()).sum();
            let right_bound = 3 * unit + 3 * (a_row << 32);
            for j in 0..3 {
                let c_col: i128 = (0..3).map(|k| i128::from(c.0[k][j]).abs()).sum();
                let left_err = (i128::from(left.0[i][j]) << 64) - lin[i][j];
                let right_err = (i128::from(right.0[i][j]) << 64) - lin[i][j];
                assert!(2 * left_err.abs() <= 3 * unit + 3 * (c_col << 32), "L{i}{j}; {context}");
                assert!(2 * right_err.abs() <= right_bound, "R{i}{j}; {context}");
            }
            let c_t: i128 = c.1.iter().map(|&e| i128::from(e).abs()).sum();
            let left_err = (i128::from(left.1[i]) << 64) - trans[i];
            let right_err = (i128::from(right.1[i]) << 64) - trans[i];
            assert!(2 * left_err.abs() <= 6 * unit + 3 * (c_t << 32), "Lt{i}; {context}");
            assert!(2 * right_err.abs() <= right_bound, "Rt{i}; {context}");
        }
    }
}

/// The exact `A B C` over `2^64` raw. Inputs from `random_trs` have linear entries below
/// `2^35` raw and translations at most `2^44`, so every term is below `2^114` and every sum
/// below `2^118`; checked `i128` arithmetic proves that per case.
fn exact_product(a: Affine, b: Affine, c: Affine) -> ([[i128; 3]; 3], [i128; 3]) {
    let wide = i128::from;
    let mut lin = [[0_i128; 3]; 3];
    let mut trans = [0_i128; 3];
    for i in 0..3 {
        let mut total = wide(a.1[i]).checked_mul(1 << 64).expect("fits");
        for k in 0..3 {
            let first = wide(a.0[i][k]).checked_mul(wide(b.1[k]) << 32).expect("fits");
            total = total.checked_add(first).expect("fits");
            for l in 0..3 {
                let pair = wide(a.0[i][k]).checked_mul(wide(b.0[k][l])).expect("fits");
                let second = pair.checked_mul(wide(c.1[l])).expect("fits");
                total = total.checked_add(second).expect("fits");
                for (j, entry) in lin[i].iter_mut().enumerate() {
                    let entry_term = pair.checked_mul(wide(c.0[l][j])).expect("fits");
                    *entry = entry.checked_add(entry_term).expect("fits");
                }
            }
        }
        trans[i] = total;
    }
    (lin, trans)
}

/// A rotation (a product of two axis rotations), a scale within 4 and a translation within
/// 4096 in each component.
fn random_trs(state: &mut u64) -> FixedAffine3 {
    let rotation = FixedMat3::rotation_x(angle(next64(state)))
        .checked_mul_mat3(FixedMat3::rotation_z(angle(next64(state))))
        .expect("entries within one");
    let scale = [0; 3].map(|_| signed_within(state, 1 << 34));
    let translation = [0; 3].map(|_| signed_within(state, 1 << 44));
    FixedAffine3::from_trs(raw_vec(translation), rotation, raw_vec(scale)).expect("small")
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn next64(state: &mut u64) -> u64 {
    let (a, b, c) = (next(state), next(state), next(state));
    (a << 33) ^ (b << 2) ^ (c & 3)
}

/// A raw value of any magnitude: a uniform 64-bit pattern shifted right by 0 to 63 bits.
fn raw_any(state: &mut u64) -> i64 {
    let shift = next(state) % 64;
    i64::from_ne_bytes(next64(state).to_ne_bytes()) >> shift
}

fn triple(state: &mut u64) -> [i64; 3] {
    [0; 3].map(|_| raw_any(state))
}

/// Uniform in `[-m, m]`.
fn signed_within(state: &mut u64, m: i64) -> i64 {
    let span = u64::try_from(2 * m + 1).expect("m is below 2^62");
    i64::try_from(next64(state) % span).expect("below 2^63") - m
}
