//! Rotation contract tests (`bunny-rotation/1`): hand-derived quarter turns from the
//! coordinate law, exact cardinal angles, agreement between the axis rotations and the
//! axis-angle rotation, pinned goldens, ties, the axis-length extremes of `FixedUnitVec3`, and
//! seeded orthonormality and determinant bounds.
//!
//! Every oracle here is exact `i128` arithmetic written in this file: the Rodrigues entries are
//! rebuilt from `FixedAngle::sin_cos` (the kernel's input, not the kernel) with a Levi-Civita
//! sign and rounded by `div_euclid` and `rem_euclid`. The goldens and the seeded digest come
//! from a separate Python model of the documented rule, which also checked every result in
//! `Decimal` at 40 digits against the exact rotation (see `docs/topics/rotation/test-plan.md`).

use bunny_linalg::{FixedMat3, FixedUnitVec3, FixedVec3, ROTATION_CONTRACT_ID};
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const QUARTER: u64 = 1 << 62;
const SEED: u64 = 0xB002_0311_0000_0001;
const CASES: usize = 10_000;
/// FNV-1a over the seeded results, from the Python model.
const EXPECTED_DIGEST: u64 = 0xB74D_1C11_AF2A_0E65;
/// `from_degrees(60)` and `from_degrees(30)`: cosine and sine are exactly `2^31` there.
const SIXTY_DEGREES: u64 = 0x2AAA_AAAA_AAAA_AAAB;
const THIRTY_DEGREES: u64 = 0x1555_5555_5555_5555;
/// Documented bounds for `rotation_axis_angle`, in raw units.
const LENGTH_BOUND: i128 = 11;
const DOT_BOUND: i128 = 12;
const DETERMINANT_BOUND: i128 = 15;

type Rows = [[i64; 3]; 3];

fn raw_vec(x: i64, y: i64, z: i64) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn unit(u: [i64; 3]) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(raw_vec(u[0], u[1], u[2])).expect("a FixedUnitVec3 axis")
}

fn angle(raw: u64) -> FixedAngle {
    FixedAngle::from_raw(raw)
}

fn rows(m: FixedMat3) -> Rows {
    let row = |r: FixedVec3| [r.x.raw(), r.y.raw(), r.z.raw()];
    [row(m.row0()), row(m.row1()), row(m.row2())]
}

/// `numerator / 2^64` rounded to nearest, ties to even.
fn oracle_round(numerator: i128) -> i64 {
    let unit = 1_i128 << 64;
    let (floor, remainder) = (numerator.div_euclid(unit), numerator.rem_euclid(unit));
    let up = 2 * remainder > unit || (2 * remainder == unit && floor % 2 != 0);
    i64::try_from(floor + i128::from(up)).expect("entries are below 4.0")
}

/// The Rodrigues matrix `c I + s [u]x + (1 - c) u u^T` on the raw axis, each entry the exact
/// value over `2^64` rounded once. `[u]x` entry `(i, j)` is `-epsilon(i, j, k) u_k`.
fn oracle_axis_angle(axis: [i64; 3], turn: FixedAngle) -> Rows {
    let (sin, cos) = turn.sin_cos();
    let (sine, cosine) = (i128::from(sin.raw()), i128::from(cos.raw()));
    let versine = i128::from(ONE) - cosine;
    let wide = axis.map(i128::from);
    let mut out = [[0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, entry) in row.iter_mut().enumerate() {
            let mut numerator = versine * wide[i] * wide[j];
            if i == j {
                numerator += cosine << 64;
            } else {
                let k = 3 - i - j;
                let epsilon = if (j + 3 - i) % 3 == 1 { 1 } else { -1 };
                numerator -= epsilon * ((sine * wide[k]) << 32);
            }
            *entry = oracle_round(numerator);
        }
    }
    out
}

fn transpose(m: Rows) -> Rows {
    let mut t = [[0; 3]; 3];
    for (i, row) in m.iter().enumerate() {
        for (j, &e) in row.iter().enumerate() {
            t[j][i] = e;
        }
    }
    t
}

fn dot(a: [i64; 3], b: [i64; 3]) -> i128 {
    (0..3).map(|i| i128::from(a[i]) * i128::from(b[i])).sum()
}

/// `| |a| - 1 | <= numerator / denominator` raw, decided exactly on squares.
fn length_within(a: [i64; 3], numerator: i128, denominator: i128) -> bool {
    let one = i128::from(ONE) * denominator;
    let square = dot(a, a) * denominator * denominator;
    (one - numerator).pow(2) <= square && square <= (one + numerator).pow(2)
}

/// `|a . b| <= bound` raw: `|sum a_i b_i| <= bound * 2^32`.
fn dot_within(a: [i64; 3], b: [i64; 3], bound: i128) -> bool {
    dot(a, b).abs() <= bound * i128::from(ONE)
}

/// The exact determinant of the raw entries, a value with 96 fractional bits.
fn determinant(m: Rows) -> i128 {
    let e = |i: usize, j: usize| i128::from(m[i][j]);
    e(0, 0) * (e(1, 1) * e(2, 2) - e(1, 2) * e(2, 1))
        - e(0, 1) * (e(1, 0) * e(2, 2) - e(1, 2) * e(2, 0))
        + e(0, 2) * (e(1, 0) * e(2, 1) - e(1, 1) * e(2, 0))
}

/// `|det - 1| <= bound` raw.
fn determinant_within(m: Rows, bound: i128) -> bool {
    (determinant(m) - (1_i128 << 96)).abs() <= bound << 64
}

/// The orthonormality and determinant bounds of the axis-angle rotation.
fn assert_bounds(m: Rows, context: &str) {
    let cols = transpose(m);
    for i in 0..3 {
        assert!(length_within(m[i], LENGTH_BOUND, 1), "row {i}; {context}");
        assert!(length_within(cols[i], LENGTH_BOUND, 1), "column {i}; {context}");
    }
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        assert!(dot_within(m[a], m[b], DOT_BOUND), "rows {a}.{b}; {context}");
        assert!(dot_within(cols[a], cols[b], DOT_BOUND), "columns {a}.{b}; {context}");
    }
    assert!(determinant_within(m, DETERMINANT_BOUND), "determinant; {context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_001_quarter_turns_follow_the_right_hand_rule() {
    assert_eq!(ROTATION_CONTRACT_ID, "bunny-rotation/1");
    // A positive quarter turn about a unit axis a maps each v perpendicular to a onto a x v.
    // The coordinate law fixes X x Y = Z, Y x Z = X and Z x X = Y, so about +Y: Z -> Y x Z = X
    // and X -> Y x X = -Z. The images are the columns.
    let q = angle(QUARTER);
    let y = FixedMat3::rotation_y(q);
    assert_eq!(y.checked_mul_vec3(raw_vec(ONE, 0, 0)), Some(raw_vec(0, 0, -ONE)));
    assert_eq!(y.checked_mul_vec3(raw_vec(0, 0, ONE)), Some(raw_vec(ONE, 0, 0)));
    assert_eq!(rows(y), [[0, 0, ONE], [0, ONE, 0], [-ONE, 0, 0]]);
    // About +X: Y -> X x Y = Z and Z -> X x Z = -Y.
    assert_eq!(rows(FixedMat3::rotation_x(q)), [[ONE, 0, 0], [0, 0, -ONE], [0, ONE, 0]]);
    // About +Z: X -> Z x X = Y and Y -> Z x Y = -X.
    assert_eq!(rows(FixedMat3::rotation_z(q)), [[0, -ONE, 0], [ONE, 0, 0], [0, 0, ONE]]);
    // The look rotation documents forward +X with up +Y as -90 degrees about +Y.
    let look = FixedMat3::try_look_rotation(raw_vec(ONE, 0, 0), raw_vec(0, ONE, 0));
    assert_eq!(look, Ok(FixedMat3::rotation_y(angle(3 * QUARTER))));
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_002_cardinal_angles_give_exact_zeros_and_ones() {
    let builders: [fn(FixedAngle) -> FixedMat3; 3] =
        [FixedMat3::rotation_x, FixedMat3::rotation_y, FixedMat3::rotation_z];
    for build in builders {
        let quarter = build(angle(QUARTER));
        let mut power = FixedMat3::IDENTITY;
        for k in 0..4_u64 {
            // sin_cos is exact at the cardinal angles and one angle unit either side of them.
            for raw in [(k * QUARTER).wrapping_sub(1), k * QUARTER, k * QUARTER + 1] {
                let m = build(angle(raw));
                assert_eq!(m, power, "k={k} raw={raw:#x}");
                assert!(rows(m).iter().flatten().all(|e| [0, ONE, -ONE].contains(e)));
                assert_eq!(m.determinant(), Some(FixedQ32_32::ONE));
            }
            power = quarter.checked_mul_mat3(power).expect("entries are 0 and 1");
        }
        assert_eq!(power, FixedMat3::IDENTITY, "four quarter turns");
        assert_eq!(build(FixedAngle::ZERO), FixedMat3::IDENTITY);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_003_axis_angle_about_the_coordinate_axes_matches_the_axis_rotations() {
    let mut state = SEED;
    let cardinal = (0..8).map(|k| k * (QUARTER / 2));
    let raws: Vec<u64> = cardinal.chain((0..1_000).map(|_| next64(&mut state))).collect();
    for raw in raws {
        let (a, neg) = (angle(raw), angle(raw).wrapping_neg());
        let about = FixedMat3::rotation_axis_angle;
        let context = format!("seed={SEED:#x} angle={raw:#x}");
        assert_eq!(about(FixedUnitVec3::UNIT_X, a), FixedMat3::rotation_x(a), "{context}");
        assert_eq!(about(FixedUnitVec3::UNIT_Y, a), FixedMat3::rotation_y(a), "{context}");
        assert_eq!(about(FixedUnitVec3::UNIT_Z, a), FixedMat3::rotation_z(a), "{context}");
        assert_eq!(about(FixedUnitVec3::NEG_UNIT_X, a), FixedMat3::rotation_x(neg), "{context}");
        assert_eq!(about(FixedUnitVec3::NEG_UNIT_Y, a), FixedMat3::rotation_y(neg), "{context}");
        assert_eq!(about(FixedUnitVec3::NEG_UNIT_Z, a), FixedMat3::rotation_z(neg), "{context}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_004_zero_is_the_identity_and_a_negated_angle_is_the_transpose() {
    let mut state = SEED;
    for case in 0..1_000 {
        let a = angle(next64(&mut state));
        let u = random_axis(&mut state, case);
        let context = format!("seed={SEED:#x} case={case} axis={u:?}");
        let m = FixedMat3::rotation_axis_angle(unit(u), a);
        assert_eq!(FixedMat3::rotation_axis_angle(unit(u), FixedAngle::ZERO), FixedMat3::IDENTITY);
        assert_eq!(
            FixedMat3::rotation_axis_angle(unit(u), a.wrapping_neg()),
            m.transpose(),
            "{context}"
        );
        for build in [FixedMat3::rotation_x, FixedMat3::rotation_y, FixedMat3::rotation_z] {
            assert_eq!(build(a.wrapping_neg()), build(a).transpose(), "{context}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_005_pinned_goldens() {
    // 30 degrees about +Z: sin is exactly 2^31 and cos 3719550787 raw (the angle goldens);
    // the entries are those values, copied.
    let (c, s) = (3_719_550_787, 2_147_483_648);
    assert_eq!(
        rows(FixedMat3::rotation_z(angle(THIRTY_DEGREES))),
        [[c, -s, 0], [s, c, 0], [0, 0, ONE]]
    );
    // 60 degrees about (2/7, 3/7, 6/7), each component rounded to nearest raw. The Decimal
    // ideal on that axis is, row by row, 2322788435.551 -2925229206.882 2120007557.226 /
    // 3451143569.678 2541919420.204 -273857251.899 / -1068178831.920 1851600340.287
    // 3725226735.959: every entry within 1/2 raw.
    let axis = [1_227_133_513, 1_840_700_270, 3_681_400_539];
    let golden = [
        [2_322_788_436, -2_925_229_207, 2_120_007_557],
        [3_451_143_570, 2_541_919_420, -273_857_252],
        [-1_068_178_832, 1_851_600_340, 3_725_226_736],
    ];
    assert_eq!(rows(FixedMat3::rotation_axis_angle(unit(axis), angle(SIXTY_DEGREES))), golden);
    assert_eq!(oracle_axis_angle(axis, angle(SIXTY_DEGREES)), golden);
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_006_entries_on_a_half_round_to_even() {
    // At 60 degrees cos is exactly 2^31, so 1 - cos is 2^31 raw. About (1, y 2^-32, 0) the
    // entries m01 and m10 are exactly (1 - cos) y = y / 2 raw: a tie for odd y.
    for (y, expected) in [(1, 0), (-1, 0), (3, 2), (-3, -2), (5, 2), (-5, -2)] {
        let m = rows(FixedMat3::rotation_axis_angle(unit([ONE, y, 0]), angle(SIXTY_DEGREES)));
        assert_eq!((m[0][1], m[1][0]), (expected, expected), "y={y}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_007_axis_rotations_are_orthogonal_within_the_sine_and_cosine_error() {
    let mut state = SEED;
    for case in 0..CASES {
        let a = angle(next64(&mut state));
        let (sine, cosine) = a.sin_cos();
        let sum = dot([sine.raw(), cosine.raw(), 0], [sine.raw(), cosine.raw(), 0]);
        for build in [FixedMat3::rotation_x, FixedMat3::rotation_y, FixedMat3::rotation_z] {
            let m = rows(build(a));
            let cols = transpose(m);
            let context = format!("seed={SEED:#x} case={case} angle={:#x}", a.raw());
            for i in 0..3 {
                assert!(length_within(m[i], 3, 2), "row {i}; {context}");
                assert!(length_within(cols[i], 3, 2), "column {i}; {context}");
            }
            for (i, j) in [(0, 1), (0, 2), (1, 2)] {
                assert_eq!((dot(m[i], m[j]), dot(cols[i], cols[j])), (0, 0), "{i}.{j}; {context}");
            }
            // The determinant is exactly c^2 + s^2.
            assert_eq!(determinant(m), sum << 32, "{context}");
            assert!(determinant_within(m, 3), "{context}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_008_seeded_axis_angle_rotations_match_the_oracle_and_the_bounds() {
    let mut state = SEED;
    let mut digest = 0xCBF2_9CE4_8422_2325;
    for case in 0..CASES {
        let a = angle(next64(&mut state));
        let u = random_axis(&mut state, case);
        let context = format!("seed={SEED:#x} case={case} angle={:#x} axis={u:?}", a.raw());
        let m = rows(FixedMat3::rotation_axis_angle(unit(u), a));
        assert_eq!(m, oracle_axis_angle(u, a), "{context}");
        assert_bounds(m, &context);
        for entry in m.into_iter().flatten() {
            digest = fnv(digest, &entry.to_le_bytes());
        }
    }
    assert_eq!(digest, EXPECTED_DIGEST, "seed={SEED:#x}: results differ from the Python model");
}

#[wasm_bindgen_test(unsupported = test)]
fn rot_tp_009_the_longest_and_shortest_axes_bound_the_error() {
    let valid = |u: [i64; 3]| FixedUnitVec3::try_from_unit(raw_vec(u[0], u[1], u[2])).is_some();
    // The longest FixedUnitVec3 along x: floor(sqrt) of the raw sum of squares is 2^32 + 1.
    let long = [ONE + 1, 92_681, 0];
    assert!(valid(long) && !valid([ONE + 1, 92_682, 0]) && !valid([ONE + 2, 0, 0]));
    let half = angle(1 << 63);
    let m = rows(FixedMat3::rotation_axis_angle(unit(long), half));
    // m00 = -1 + 2 x^2 = 1 + 4 raw: the axis length, not the rounding, sets the error.
    assert_eq!(m, [[ONE + 4, 185_362, 0], [185_362, -ONE + 4, 0], [0, 0, -ONE]]);
    assert!(!length_within(m[0], 7, 1), "row 0 is more than 7 raw longer than one");
    assert_bounds(m, "longest axis, half turn");
    // The shortest FixedUnitVec3 along x.
    let short = [ONE - 1, 0, 0];
    assert!(valid(short) && !valid([ONE - 2, 0, 0]));
    let m = rows(FixedMat3::rotation_axis_angle(unit(short), half));
    assert_eq!(m, [[ONE - 4, 0, 0], [0, -ONE, 0], [0, 0, -ONE]]);
    assert_bounds(m, "shortest axis, half turn");
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn next64(state: &mut u64) -> u64 {
    let (a, b, c) = (next(state), next(state), next(state));
    (a << 33) ^ (b << 2) ^ (c & 3)
}

/// Uniform in `[-m, m]`.
fn signed_within(state: &mut u64, m: u128) -> i128 {
    let span = u64::try_from(2 * m + 1).expect("m is below 2^33");
    i128::from(next64(state) % span) - i128::try_from(m).expect("m is below 2^33")
}

fn isqrt(n: i128) -> u128 {
    u128::try_from(n).expect("non-negative").isqrt()
}

/// An axis anywhere in the `FixedUnitVec3` band: the raw sum of squares is aimed at the top of
/// the band (`(2^32 + 2)^2 - 1`), its bottom (`(2^32 - 1)^2`) or a uniform point between.
fn random_axis(state: &mut u64, case: usize) -> [i64; 3] {
    let one = i128::from(ONE);
    let (low, high) = ((one - 1) * (one - 1), (one + 2) * (one + 2) - 1);
    let span = u64::try_from(high - low + 1).expect("the band is below 2^35");
    let target = match case % 4 {
        0 => high,
        1 => low,
        _ => low + i128::from(next64(state) % span),
    };
    let a = signed_within(state, isqrt(target));
    let b = signed_within(state, isqrt(target - a * a));
    let mut c = i128::try_from(isqrt(target - a * a - b * b)).expect("below 2^33");
    if (i128::try_from(isqrt(a * a + b * b + c * c)).expect("below 2^33") - one).abs() > 1 {
        c += 1;
    }
    if next(state) % 2 == 1 {
        c = -c;
    }
    let mut u = [a, b, c].map(|v| i64::try_from(v).expect("a unit component"));
    u.rotate_left(usize::try_from(next(state) % 3).expect("below 3"));
    u
}

/// FNV-1a over the result stream, matching the Python model byte for byte.
fn fnv(mut digest: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(0x100_0000_01B3);
    }
    digest
}
