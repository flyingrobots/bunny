//! Contract tests for `bunny-curve/1` (`docs/topics/curve/test-plan.md`).
//!
//! Two references, each sharing no code with `bunny-linalg` or `bunny-num`:
//!
//! - the rule port: `port_lerp` rounds each step by `i128` Euclidean division, and
//!   `port_levels` and `port_derivative` run de Casteljau on raw coordinates in the stated
//!   order, so the kernel must equal them bit for bit;
//! - the exact Bernstein form: `exact_within` evaluates `sum C(n, i) c_i s^i (1 - s)^(n - i)`
//!   in a 384-bit two's-complement integer and decides `|result - exact| <= bound` exactly.
//!
//! `U = 2^32` is `1.0` and `H = 2^31` is `0.5` in raw units.

// The 384-bit integer splits and joins `u128` values into `u64` limbs.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
// The port names its de Casteljau points `a` to `m`, as `docs/topics/curve/README.md` does.
#![allow(clippy::many_single_char_names)]

use bunny_linalg::{CubicBezier3, FixedVec3, CURVE_CONTRACT_ID};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const SEED: u64 = 0xB0_0308_C0B3_0001;

type Raw3 = [i64; 3];
type Polygon = [Raw3; 4];

fn port_lerp(a: i64, b: i64, t: i64) -> i64 {
    let unit = 1_i128 << 32;
    let exact = (i128::from(b) - i128::from(a)) * i128::from(t);
    let (floor, twice_rest) = (exact.div_euclid(unit), 2 * exact.rem_euclid(unit));
    let up = i128::from(twice_rest > unit || (twice_rest == unit && floor.rem_euclid(2) == 1));
    i64::try_from(i128::from(a) + floor + up).expect("a lerp with t in [0, 1] stays in range")
}

/// One axis's de Casteljau points `[a, b, c, d, e, m]` at `t`.
fn port_levels(p: [i64; 4], t: i64) -> [i64; 6] {
    let (a, b, c) = (port_lerp(p[0], p[1], t), port_lerp(p[1], p[2], t), port_lerp(p[2], p[3], t));
    let (d, e) = (port_lerp(a, b, t), port_lerp(b, c, t));
    [a, b, c, d, e, port_lerp(d, e, t)]
}

fn differences(p: [i64; 4]) -> [i128; 3] {
    [1, 2, 3].map(|i| i128::from(p[i]) - i128::from(p[i - 1]))
}

/// One axis's derivative: `None` when a difference or the final product leaves `i64`.
fn port_derivative(p: [i64; 4], t: i64) -> Option<i64> {
    let [d0, d1, d2] = differences(p).map(i64::try_from);
    let (d0, d1, d2) = (d0.ok()?, d1.ok()?, d2.ok()?);
    let (f, g) = (port_lerp(d0, d1, t), port_lerp(d1, d2, t));
    i64::try_from(3 * i128::from(port_lerp(f, g, t))).ok()
}

fn axis(p: &Polygon, k: usize) -> [i64; 4] {
    p.map(|point| point[k])
}

fn per_axis<T: Copy>(f: impl Fn(usize) -> T) -> [T; 3] {
    [f(0), f(1), f(2)]
}

fn all3(v: [Option<i64>; 3]) -> Option<Raw3> {
    Some([v[0]?, v[1]?, v[2]?])
}

fn fx(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fx(raw[0]), fx(raw[1]), fx(raw[2]))
}

fn raw_of(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn curve(p: &Polygon) -> CubicBezier3 {
    CubicBezier3::new(vec3(p[0]), vec3(p[1]), vec3(p[2]), vec3(p[3]))
}

fn polygon_of(c: CubicBezier3) -> Polygon {
    [raw_of(c.p0), raw_of(c.p1), raw_of(c.p2), raw_of(c.p3)]
}

fn at(p: &Polygon, t: i64) -> Option<Raw3> {
    curve(p).at(fx(t)).map(raw_of)
}

fn derivative(p: &Polygon, t: i64) -> Option<Raw3> {
    curve(p).derivative_at(fx(t)).map(raw_of)
}

fn split(p: &Polygon, t: i64) -> Option<(Polygon, Polygon)> {
    curve(p).split(fx(t)).map(|(left, right)| (polygon_of(left), polygon_of(right)))
}

/// A 384-bit two's-complement integer; every exact numerator here is below `2^260`.
#[derive(Clone, Copy)]
struct W([u64; 6]);

impl W {
    fn new(value: i128) -> Self {
        let extension = if value < 0 { u64::MAX } else { 0 };
        let bits = value as u128;
        Self([bits as u64, (bits >> 64) as u64, extension, extension, extension, extension])
    }

    fn add(self, other: Self) -> Self {
        let (mut out, mut carry) = ([0_u64; 6], 0_u128);
        for k in 0..6 {
            let sum = u128::from(self.0[k]) + u128::from(other.0[k]) + carry;
            (out[k], carry) = (sum as u64, sum >> 64);
        }
        Self(out)
    }

    fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::new(1))
    }

    fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }

    fn mul(self, other: Self) -> Self {
        let mut out = [0_u64; 6];
        for i in 0..6 {
            let mut carry = 0_u128;
            for j in 0..6 - i {
                let sum =
                    u128::from(out[i + j]) + u128::from(self.0[i]) * u128::from(other.0[j]) + carry;
                (out[i + j], carry) = (sum as u64, sum >> 64);
            }
        }
        Self(out)
    }

    fn at_most(self, bound: Self) -> bool {
        bound.sub(self).0[5] >> 63 == 0
    }
}

/// The parameter `s / one`, held exactly.
#[derive(Clone, Copy)]
struct Param {
    s: W,
    one: W,
}

fn param32(t: i64) -> Param {
    Param { s: W::new(i128::from(t)), one: W::new(1 << 32) }
}

/// Whether `|result - B(s)| <= halves / 2` raw units, where `B` is the Bernstein polynomial
/// with these coefficients, decided on the exact numerator over `one^n`.
fn exact_within(result: i64, coefficients: &[i128], at: Param, halves: i128) -> bool {
    let n = coefficients.len() - 1;
    let binomial = [[1, 0, 0, 0], [1, 1, 0, 0], [1, 2, 1, 0], [1, 3, 3, 1]][n];
    let rest = at.one.sub(at.s);
    let mut numerator = W::new(0);
    for (i, &c) in coefficients.iter().enumerate() {
        let mut term = W::new(c * binomial[i]);
        for _ in 0..i {
            term = term.mul(at.s);
        }
        for _ in i..n {
            term = term.mul(rest);
        }
        numerator = numerator.add(term);
    }
    let denominator = (0..n).fold(W::new(1), |d, _| d.mul(at.one));
    let twice_error = W::new(2 * i128::from(result)).mul(denominator).sub(numerator.mul(W::new(2)));
    let bound = denominator.mul(W::new(halves));
    twice_error.at_most(bound) && twice_error.neg().at_most(bound)
}

fn coordinates(p: [i64; 4]) -> [i128; 4] {
    p.map(i128::from)
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A draw below `n` from the high half of the state: an LCG's low bits have short periods.
fn pick(state: &mut u64, n: u64) -> u64 {
    (next_seed(state) >> 32) % n
}

/// A coordinate of every magnitude, with the ends of the raw range and zero often.
fn coordinate(state: &mut u64) -> i64 {
    match pick(state, 16) {
        0 => i64::MIN,
        1 => i64::MAX,
        2 => 0,
        _ => next_seed(state).cast_signed() >> pick(state, 64),
    }
}

fn polygon(state: &mut u64) -> Polygon {
    let mut point = || [coordinate(state), coordinate(state), coordinate(state)];
    [point(), point(), point(), point()]
}

/// A parameter in `[0, 1]`: the ends and one raw unit inside them, multiples of 1/8 (whose
/// steps tie often), else uniform.
fn parameter(state: &mut u64) -> i64 {
    match pick(state, 8) {
        0 => 0,
        1 => U,
        2 => 1,
        3 => U - 1,
        4 | 5 => pick(state, 9).cast_signed() * (U / 8),
        _ => ((next_seed(state) >> 16) % (1 << 32 | 1)).cast_signed(),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_001_hand_derived_curves() {
    assert_eq!(CURVE_CONTRACT_ID, "bunny-curve/1");
    // (0, 1, 2, 3) on x: every lerp has a step of exactly t, so at(t) = 3t with no rounding.
    let straight = [[0, 0, 0], [U, 0, 0], [2 * U, 0, 0], [3 * U, 0, 0]];
    for k in 0..=8 {
        let t = k * U / 8;
        assert_eq!(at(&straight, t), Some([3 * t, 0, 0]), "k={k}");
        assert_eq!(derivative(&straight, t), Some([3 * U, 0, 0]), "k={k}");
    }
    // Split at 0.5: (0, 0.5, 1.0, 1.5) and (1.5, 2.0, 2.5, 3.0).
    let halves = [0, 1, 2, 3].map(|k| [k * H, 0, 0]);
    let upper = [3, 4, 5, 6].map(|k| [k * H, 0, 0]);
    assert_eq!(split(&straight, H), Some((halves, upper)));
    // A flat curve: y and z stay zero at every t, whatever x does.
    let flat = [[5, 0, 0], [-3 * U, 0, 0], [7 * U + 1, 0, 0], [i64::MAX, 0, 0]];
    for t in [0, 1, H - 1, H, H + 1, U - 1, U].into_iter().chain((0..=8).map(|k| k * U / 8)) {
        let [_, y, z] = at(&flat, t).expect("t in [0, 1]");
        assert_eq!((y, z), (0, 0), "t={t}");
    }
    // Equal control points: a constant, a zero derivative, and constant halves.
    let point = [-7 * U + 3, i64::MIN, i64::MAX];
    let constant = [point; 4];
    for t in [0, 1, H, U / 3, U - 1, U] {
        assert_eq!(at(&constant, t), Some(point), "t={t}");
        assert_eq!(derivative(&constant, t), Some([0; 3]), "t={t}");
        assert_eq!(split(&constant, t), Some((constant, constant)), "t={t}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_002_golden_vectors() {
    // From the Python model in the pull request (integer rule port plus `fractions` check).
    // p1 and p2 differ on every axis; the odd raw values force ties.
    let asymmetric = [[0, 0, 0], [U, 2 * U, -H], [3 * U + 1, -U, 7], [4 * U, U, -3]];
    let rows: [(i64, Raw3, Raw3); 4] = [
        (
            H,
            [8_589_934_592, 2_147_483_648, -805_306_366],
            [19_327_352_832, -6_442_450_944, 1_610_612_742],
        ),
        (
            1_431_655_765,
            [5_408_477_334, 3_022_384_394, -954_437_175],
            [18_611_524_950, -2_863_311_528, 3],
        ),
        // z is 1.4999999993 raw from the exact value: the 3/2 bound is nearly reached.
        (1, [3, 6, 0], [12_884_901_894, 25_769_803_746, -6_442_450_938]),
        (U - 1, [17_179_869_181, 4_294_967_290, -3], [12_884_901_891, 25_769_803_746, -24]),
    ];
    for (t, point, slope) in rows {
        assert_eq!(at(&asymmetric, t), Some(point), "t={t}");
        assert_eq!(derivative(&asymmetric, t), Some(slope), "t={t}");
    }
    let swapped = [asymmetric[0], asymmetric[2], asymmetric[1], asymmetric[3]];
    assert_ne!(at(&swapped, 1_431_655_765), Some(rows[1].1), "p1 and p2 are not interchangeable");
    let left = [
        [0, 0, 0],
        [2_147_483_648, 4_294_967_296, -1_073_741_824],
        [5_368_709_120, 3_221_225_472, -1_073_741_822],
        [8_589_934_592, 2_147_483_648, -805_306_366],
    ];
    let right = [
        [8_589_934_592, 2_147_483_648, -805_306_366],
        [11_811_160_064, 1_073_741_824, -536_870_909],
        [15_032_385_537, 0, 2],
        [17_179_869_184, 4_294_967_296, -3],
    ];
    assert_eq!(split(&asymmetric, H), Some((left, right)));
    // The ends of the raw range: evaluation is defined, the derivative's differences are not.
    let (lo, hi) = (i64::MIN, i64::MAX);
    let extreme = [[lo, hi, 0], [hi, lo, 1], [lo, hi, -1], [hi, lo, 0]];
    assert_eq!(at(&extreme, H), Some([0, -1, 0]));
    assert_eq!(at(&extreme, U / 3), Some([-341_606_372_212_580_656, 341_606_372_212_580_655, 0]));
    assert_eq!(derivative(&extreme, H), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_003_t_outside_zero_to_one_is_refused() {
    let p = [[0, 0, 0], [U, 2 * U, -H], [3 * U + 1, -U, 7], [4 * U, U, -3]];
    for t in [i64::MIN, -U, -1, U + 1, 2 * U, i64::MAX] {
        assert_eq!(at(&p, t), None, "t={t}");
        assert_eq!(derivative(&p, t), None, "t={t}");
        assert_eq!(split(&p, t), None, "t={t}");
    }
    for t in [0, U] {
        assert!(at(&p, t).is_some() && derivative(&p, t).is_some() && split(&p, t).is_some());
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_004_derivative_overflow_on_and_past_each_threshold() {
    // 3x fits i64 exactly when |x| <= 3_074_457_345_618_258_602 (i64::MAX = 3 * that + 1).
    let third = i64::MAX / 3;
    for (x, expected) in [
        (third, Some(3 * third)),
        (third + 1, None),
        (-third, Some(-3 * third)),
        (-third - 1, None),
    ] {
        let p = [[0, 0, 0], [x, 0, 0], [x, 0, 0], [x, 0, 0]];
        assert_eq!(derivative(&p, 0).map(|d| d[0]), expected, "x={x}");
        assert_eq!(derivative(&p, U), Some([0; 3]), "x={x}: the end slope is 3 (p3 - p2) = 0");
    }
    // A difference that leaves i64 refuses every t, even where the result would be zero.
    for (p0, p1, defined) in [
        (-1, i64::MAX - 1, true),
        (-1, i64::MAX, false),
        (1, i64::MIN + 1, true),
        (1, i64::MIN, false),
    ] {
        let p = [[0, p0, 0], [0, p1, 0], [0, p1, 0], [0, p1, 0]];
        assert_eq!(derivative(&p, U), defined.then_some([0; 3]), "p0={p0} p1={p1}");
        // Evaluation and split never refuse inside [0, 1].
        assert!(at(&p, H).is_some() && split(&p, H).is_some());
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_005_seeded_evaluation_and_derivative_match_both_oracles() {
    println!("curve_tp_005 seed={SEED:#018x} cases=100000");
    let mut state = SEED;
    let (mut defined, mut refused, mut past_one, mut past_two) = (0_u32, 0_u32, 0_u32, 0_u32);
    for case in 0..100_000_u32 {
        let p = polygon(&mut state);
        let t = parameter(&mut state);
        let context = format!("seed={SEED:#018x} case={case} p={p:?} t={t}");
        let expected = per_axis(|k| port_levels(axis(&p, k), t)[5]);
        assert_eq!(at(&p, t), Some(expected), "{context}");
        for (k, &value) in expected.iter().enumerate() {
            let c = coordinates(axis(&p, k));
            assert!(exact_within(value, &c, param32(t), 3), "{context} axis={k}");
            past_one += u32::from(!exact_within(value, &c, param32(t), 2));
        }
        let slope = all3(per_axis(|k| port_derivative(axis(&p, k), t)));
        assert_eq!(derivative(&p, t), slope, "{context}");
        if let Some(slope) = slope {
            defined += 1;
            for (k, &value) in slope.iter().enumerate() {
                let d = differences(axis(&p, k)).map(|d| 3 * d);
                assert!(exact_within(value, &d, param32(t), 6), "{context} axis={k}");
                past_two += u32::from(!exact_within(value, &d, param32(t), 4));
            }
        } else {
            refused += 1;
        }
        // The ends, for every sampled polygon.
        assert_eq!(at(&p, 0), Some(p[0]), "{context}");
        assert_eq!(at(&p, U), Some(p[3]), "{context}");
        let fits = differences(axis(&p, 0)).iter().all(|d| i64::try_from(*d).is_ok())
            && differences(axis(&p, 1)).iter().all(|d| i64::try_from(*d).is_ok())
            && differences(axis(&p, 2)).iter().all(|d| i64::try_from(*d).is_ok());
        for (t_end, i) in [(0, 0), (U, 2)] {
            let end = all3(per_axis(|k| i64::try_from(3 * differences(axis(&p, k))[i]).ok()));
            assert_eq!(derivative(&p, t_end), end.filter(|_| fits), "{context} end={t_end}");
        }
    }
    println!(
        "curve_tp_005 defined={defined} refused={refused} past_one={past_one} past_two={past_two}"
    );
    assert!(defined >= 20_000 && refused >= 10_000, "both classes are reached");
    assert!(past_one > 0 && past_two > 0, "errors above 1 and 2 raw units are reached");
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_006_seeded_splits_share_their_points_and_track_the_original() {
    println!("curve_tp_006 seed={SEED:#018x} cases=20000");
    let mut state = SEED ^ 0x0006;
    let mut past_evaluation_bound = 0_u32;
    for case in 0..20_000_u32 {
        let p = polygon(&mut state);
        let (t, u) = (parameter(&mut state), parameter(&mut state));
        let context = format!("seed={SEED:#018x} case={case} p={p:?} t={t} u={u}");
        let levels = per_axis(|k| port_levels(axis(&p, k), t));
        let point = |i: usize| per_axis(|k| levels[k][i]);
        let expected = ([p[0], point(0), point(3), point(5)], [point(5), point(4), point(2), p[3]]);
        let (left, right) = split(&p, t).expect("t in [0, 1]");
        assert_eq!((left, right), expected, "{context}");
        assert_eq!(at(&left, U), at(&p, t), "{context}");
        assert_eq!(at(&right, 0), at(&p, t), "{context}");
        assert_eq!(split(&p, 0), Some(([p[0]; 4], p)), "{context}");
        assert_eq!(split(&p, U), Some((p, [p[3]; 4])), "{context}");
        // Each half at u against the exact original at t u and at t + (1 - t) u, over 2^64.
        let one = W::new(1 << 64);
        let (wt, wu) = (W::new(i128::from(t)), W::new(i128::from(u)));
        let inner = Param { s: wt.mul(wu), one };
        let outer = Param { s: wt.mul(W::new(1 << 32)).add(W::new(1 << 32).sub(wt).mul(wu)), one };
        for (half, at_half) in [(&left, inner), (&right, outer)] {
            let point = at(half, u).expect("u in [0, 1]");
            for (k, &value) in point.iter().enumerate() {
                let c = coordinates(axis(&p, k));
                assert!(exact_within(value, &c, at_half, 6), "{context} axis={k}");
                past_evaluation_bound += u32::from(!exact_within(value, &c, at_half, 3));
            }
        }
    }
    println!("curve_tp_006 past_evaluation_bound={past_evaluation_bound}");
    // A half is less exact than evaluating the original: its inner points are rounded.
    assert!(past_evaluation_bound > 0, "halves exceed the 3/2 raw unit bound of at");
}
