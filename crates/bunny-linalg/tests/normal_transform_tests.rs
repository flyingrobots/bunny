//! Normal transform tests (flyingrobots/bunny#108): `FixedAffine3::checked_transform_normal`
//! applies the inverse transpose of the linear part. Hand-derived perpendicularity under a
//! non-uniform scale, refusals one raw unit either side of each threshold, a seeded oracle and
//! pinned goldens.
//!
//! The oracle here is exact `i128` arithmetic written in this file. It takes the rounded inverse
//! from `FixedMat3::try_inverse` as given (that is the documented rule), and multiplies, rounds
//! by `div_euclid`, `rem_euclid` and a comparison of twice the remainder, and range-checks with
//! `i64::try_from` on its own. The goldens come from a separate Python model of the documented
//! rule, including `try_inverse` (see `docs/topics/transform/test-plan.md`).

use bunny_linalg::{FixedAffine3, FixedMat3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const SEED: u64 = 0xB002_0108_0000_0002;
const CASES: usize = 10_000;

type Rows = [[i64; 3]; 3];

fn v3(v: [i64; 3]) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(v[0]), raw(v[1]), raw(v[2]))
}

fn r3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn matrix(m: Rows) -> FixedMat3 {
    FixedMat3::from_rows(v3(m[0]), v3(m[1]), v3(m[2]))
}

fn rows(m: FixedMat3) -> Rows {
    [r3(m.row0()), r3(m.row1()), r3(m.row2())]
}

fn linear(m: Rows) -> FixedAffine3 {
    FixedAffine3::from_parts(matrix(m), v3([0; 3]))
}

fn diagonal(s: [i64; 3]) -> Rows {
    [[s[0], 0, 0], [0, s[1], 0], [0, 0, s[2]]]
}

fn normal(a: FixedAffine3, n: [i64; 3]) -> Option<[i64; 3]> {
    a.checked_transform_normal(v3(n)).map(r3)
}

fn vector(a: FixedAffine3, v: [i64; 3]) -> Option<[i64; 3]> {
    a.checked_transform_vector(v3(v)).map(r3)
}

/// The exact dot product of two raw vectors, in units of `2^-64`.
fn dot(a: [i64; 3], b: [i64; 3]) -> i128 {
    a.into_iter().zip(b).map(|(x, y)| i128::from(x) * i128::from(y)).sum()
}

/// `numerator / 2^32` rounded to nearest, ties to even.
fn round32(numerator: i128) -> i128 {
    let unit = 1_i128 << 32;
    let (floor, remainder) = (numerator.div_euclid(unit), numerator.rem_euclid(unit));
    floor + i128::from(2 * remainder > unit || (2 * remainder == unit && floor % 2 != 0))
}

/// The rule applied to the rounded inverse `inv`: column `i` of `inv` (row `i` of its
/// transpose) times `n`, each product rounded once, every product and partial sum in range.
fn oracle(inv: Rows, n: [i64; 3]) -> Option<[i64; 3]> {
    let mut out = [0_i64; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        let mut sum = 0_i64;
        for (row, coordinate) in inv.iter().zip(n) {
            let product =
                i64::try_from(round32(i128::from(row[i]) * i128::from(coordinate))).ok()?;
            sum = i64::try_from(i128::from(sum) + i128::from(product)).ok()?;
        }
        *slot = sum;
    }
    Some(out)
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_026_non_uniform_scale_keeps_normals_perpendicular() {
    // Scale (2, 1, 1). The surface through the origin with normal (1, 1, 0) holds the tangent
    // (1, -1, 0). The tangent maps to (2, -1, 0); the inverse transpose maps the normal to
    // (1/2, 1, 0), and (1/2)(2) + (1)(-1) = 0. The linear part alone gives (2, 1, 0), whose
    // dot product with the tangent image is 4 - 1 = 3: no longer perpendicular.
    let stretch = linear(diagonal([2 * ONE, ONE, ONE]));
    let n2 = normal(stretch, [ONE, ONE, 0]).expect("invertible");
    let t2 = vector(stretch, [ONE, -ONE, 0]).expect("in range");
    assert_eq!((n2, t2), ([ONE / 2, ONE, 0], [2 * ONE, -ONE, 0]));
    assert_eq!(dot(n2, t2), 0);
    let naive = vector(stretch, [ONE, ONE, 0]).expect("in range");
    assert_eq!(dot(naive, t2), 3_i128 << 64);
    // The translation is ignored.
    let moved = FixedAffine3::from_parts(matrix(diagonal([2 * ONE, ONE, ONE])), v3([ONE; 3]));
    assert_eq!(normal(moved, [ONE, ONE, 0]), Some(n2));
    // A rotation (the quarter turn about +Z, rows (0, -1, 0), (1, 0, 0), (0, 0, 1)) is its own
    // inverse transpose: normal and vector transforms agree.
    let quarter = linear([[0, -ONE, 0], [ONE, 0, 0], [0, 0, ONE]]);
    assert_eq!(normal(quarter, [3, 5, -7]), vector(quarter, [3, 5, -7]));
    // A uniform scale 2 gives n / 2, parallel to the linear image 2 n.
    let twice = linear(diagonal([2 * ONE; 3]));
    assert_eq!(normal(twice, [2, -4, 6]), Some([1, -2, 3]));
    assert_eq!(vector(twice, [2, -4, 6]), Some([4, -8, 12]));
    // A reflection keeps the outward side: scale (-1, 1, 1) maps the normal (1, 0, 0) to
    // (-1, 0, 0), the same as the point image of the outward direction.
    let mirror = linear(diagonal([-ONE, ONE, ONE]));
    assert_eq!(normal(mirror, [ONE, 0, 0]), Some([-ONE, 0, 0]));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_027_normal_refusals_one_raw_unit_either_side() {
    // Singular: a zero scale has determinant zero.
    assert_eq!(normal(linear(diagonal([0, ONE, ONE])), [ONE, 0, 0]), None);
    // Nonsingular but the rounded determinant is zero: the minor 1 raw * 1 raw rounds to 0.
    assert_eq!(normal(linear(diagonal([1, 1, 1])), [ONE, 0, 0]), None);
    // The inverse entry 1 / (2 raw) is 2^63 raw, out of range; 1 / (3 raw) rounds to
    // 0x5555_5555_5555_5555 raw.
    assert_eq!(normal(linear(diagonal([2, ONE, ONE])), [ONE, 0, 0]), None);
    let thin = linear(diagonal([3, ONE, ONE]));
    let inverse_x = 0x5555_5555_5555_5555;
    assert_eq!(normal(thin, [ONE, 0, 0]), Some([inverse_x, 0, 0]));
    // inverse_x * n_x: n_x = 1.5 rounds to 2^63 (None); 1.5 - 1 raw fits; -1.5 is i64::MIN.
    let three_halves = 3 << 31;
    assert_eq!(normal(thin, [three_halves - 1, 0, 0]), Some([9_223_372_035_423_120_042, 0, 0]));
    assert_eq!(normal(thin, [three_halves, 0, 0]), None);
    assert_eq!(normal(thin, [three_halves + 1, 0, 0]), None);
    assert_eq!(normal(thin, [-three_halves, 0, 0]), Some([i64::MIN, 0, 0]));
    assert_eq!(normal(thin, [-three_halves - 1, 0, 0]), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_028_seeded_normals_match_the_oracle() {
    let mut state = SEED;
    let mut counts = [0_usize; 3];
    for case in 0..CASES {
        let m = [vec(&mut state), vec(&mut state), vec(&mut state)];
        let n = vec(&mut state);
        let got = normal(linear(m), n);
        let want = matrix(m).try_inverse().and_then(|inv| oracle(rows(inv), n));
        assert_eq!(got, want, "seed {SEED:#x}, case {case}");
        counts[match (want, matrix(m).try_inverse()) {
            (Some(_), _) => 0,
            (None, None) => 1,
            (None, Some(_)) => 2,
        }] += 1;
        let (Some(got), Some(inv)) = (got, matrix(m).try_inverse()) else { continue };
        for (i, value) in got.into_iter().enumerate() {
            let column = [rows(inv)[0][i], rows(inv)[1][i], rows(inv)[2][i]];
            let error = (i128::from(value) << 32) - dot(column, n);
            assert!(error.abs() <= 3 << 31, "seed {SEED:#x}, case {case}, axis {i}");
        }
    }
    assert_eq!(counts, EXPECTED_NORMAL_COUNTS, "seed {SEED:#x}");
    // Exactly invertible: power-of-two scales and a signed permutation. Every product is
    // exact, so n' . (M t) equals n . t exactly, for every n and t.
    let mut state = SEED ^ 1;
    for case in 0..CASES {
        let scale = [power(&mut state), power(&mut state), power(&mut state)];
        let m = permuted(diagonal(scale), next(&mut state));
        let (n, t) = (aligned(&mut state), aligned(&mut state));
        let n2 = normal(linear(m), n).expect("exactly invertible");
        let t2 = vector(linear(m), t).expect("in range");
        assert_eq!(dot(n2, t2), dot(n, t), "seed {:#x}, case {case}", SEED ^ 1);
    }
}

/// How many seeded normals are in range, `None` from `try_inverse`, and `None` from the product.
const EXPECTED_NORMAL_COUNTS: [usize; 3] = [6_662, 3_335, 3];

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_030_pinned_normal_golden() {
    let m = [
        [0x1_8000_0001, -0x4000_0003, 0x7],
        [0x2000_0005, 0x2_0000_0000, -0xC000_0001],
        [-0x11, 0x8000_0000, 0x5555_5555],
    ];
    let inverse = matrix(m).try_inverse().expect("invertible");
    assert_eq!(
        rows(inverse),
        [
            [0xA989_5313, 0x0D90_1B23, 0x1E84_3D02],
            [-0x06C8_0D89, 0x5160_A2C2, 0xB719_6E36],
            [0x0A2C_146F, -0x7A10_F420, 0x1_ED59_DABA],
        ]
    );
    let n = [0x1_0000_0003, -0x2_8000_0001, 0x5];
    let n2 = normal(linear(m), n).expect("in range");
    assert_eq!(n2, [0xBA7D_74EC, -0xBDE1_7BC4, -0x1_AB3B_567C]);
    // The tangent (n_y, -n_x, 0) is perpendicular to n; its image stays perpendicular to n2 to
    // within 783 529 307 * 2^-60, while the linear image of n is not.
    let t2 = vector(linear(m), [n[1], -n[0], 0]).expect("in range");
    assert_eq!(t2, [-0x3_8000_0000, -0x2_5000_0013, -0x7FFF_FFD7]);
    assert_eq!(dot(n2, t2), 783_529_307 << 4);
    let naive = vector(linear(m), n).expect("in range");
    assert_eq!(dot(naive, t2), 20_572_443_100_177_235_819 << 2);
}

/// Raw components below 2^36 in magnitude (16 units): a uniform 64-bit pattern shifted right
/// by 28 to 63 bits, so tiny entries (singular, nearly singular and overflowing inverses) occur.
fn vec(state: &mut u64) -> [i64; 3] {
    [0; 3].map(|_| {
        let shift = 28 + next(state) % 36;
        signed(state) >> shift
    })
}

/// A raw component, a multiple of 2^12, below 2^40 in magnitude.
fn aligned(state: &mut u64) -> [i64; 3] {
    [signed(state) >> 24 & !0xFFF, signed(state) >> 24 & !0xFFF, signed(state) >> 24 & !0xFFF]
}

/// `+-2^k` units for `k` in `-8..=8`.
fn power(state: &mut u64) -> i64 {
    let k = next(state) % 17;
    let magnitude = 1_i64 << (24 + k);
    if next(state) & 1 == 0 {
        magnitude
    } else {
        -magnitude
    }
}

/// The diagonal matrix with its rows permuted by `pick` (one of six).
fn permuted(m: Rows, pick: u64) -> Rows {
    let orders = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    let order = orders[usize::try_from(pick % 6).unwrap_or(0)];
    [m[order[0]], m[order[1]], m[order[2]]]
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// A uniform 64-bit pattern from three 31-bit draws.
fn signed(state: &mut u64) -> i64 {
    let (a, b, c) = (next(state), next(state), next(state));
    i64::from_ne_bytes(((a << 33) ^ (b << 2) ^ (c & 3)).to_ne_bytes())
}
