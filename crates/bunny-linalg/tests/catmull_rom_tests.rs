//! Contract tests for `bunny-curve/1`, part 2: uniform Catmull-Rom splines
//! (`docs/topics/curve/test-plan.md`, CURVE-TP-020 to -025).
//!
//! Two references, each sharing no code with `bunny-linalg` or `bunny-num`:
//!
//! - the rule port: `port_segment` converts a segment with `i128` Euclidean division by 6
//!   and ties to even, and `port_at` and `port_derivative` run de Casteljau on raw
//!   coordinates with `port_lerp`, so the kernel must equal them bit for bit;
//! - the exact Catmull-Rom cubic in its power form, not the Bezier form,
//!   `2 C(t) = 2 P1 + (P2 - P0) t + (2 P0 - 5 P1 + 4 P2 - P3) t^2 + (3 P1 - P0 - 3 P2 + P3) t^3`;
//!   `within` evaluates it as an exact numerator in a 384-bit two's-complement integer and
//!   decides `|result - exact| <= bound` exactly.
//!
//! `U = 2^32` is `1.0` and `H = 2^31` is `0.5` in raw units.

// The 384-bit integer splits and joins `u128` values into `u64` limbs.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
// The ports name the four neighbours `a` to `d` and the de Casteljau points as the chapter does.
#![allow(clippy::many_single_char_names)]

use bunny_linalg::{CatmullRom3, CatmullRomError, CubicBezier3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const SEED: u64 = 0xB0_0309_C47E_0001;

type Raw3 = [i64; 3];
/// One segment's Bezier control points, per axis.
type Axes = [[i64; 4]; 3];

fn fx(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fx(raw[0]), fx(raw[1]), fx(raw[2]))
}

fn raw_of(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn axes_of(c: CubicBezier3) -> Axes {
    let p = [c.p0, c.p1, c.p2, c.p3].map(raw_of);
    [0, 1, 2].map(|k| p.map(|point| point[k]))
}

fn spline(points: &[FixedVec3], closed: bool) -> CatmullRom3<'_> {
    CatmullRom3::new(points, closed).expect("a valid point list")
}

fn at(s: CatmullRom3<'_>, u: i64) -> Option<Raw3> {
    s.at(fx(u)).map(raw_of)
}

fn derivative(s: CatmullRom3<'_>, u: i64) -> Option<Raw3> {
    s.derivative_at(fx(u)).map(raw_of)
}

// ---- The rule port ----

fn port_round_sixth(n: i128) -> i128 {
    let (q, r) = (n.div_euclid(6), n.rem_euclid(6));
    q + i128::from(r > 3 || (r == 3 && q % 2 != 0))
}

fn port_lerp(a: i64, b: i64, t: i64) -> i64 {
    let unit = 1_i128 << 32;
    let exact = (i128::from(b) - i128::from(a)) * i128::from(t);
    let (floor, twice_rest) = (exact.div_euclid(unit), 2 * exact.rem_euclid(unit));
    let up = i128::from(twice_rest > unit || (twice_rest == unit && floor.rem_euclid(2) == 1));
    i64::try_from(i128::from(a) + floor + up).expect("a lerp with t in [0, 1] stays in range")
}

fn port_at(p: [i64; 4], t: i64) -> i64 {
    let (a, b, c) = (port_lerp(p[0], p[1], t), port_lerp(p[1], p[2], t), port_lerp(p[2], p[3], t));
    port_lerp(port_lerp(a, b, t), port_lerp(b, c, t), t)
}

fn port_derivative(p: [i64; 4], t: i64) -> Option<i64> {
    let d = [1, 2, 3].map(|i| i64::try_from(i128::from(p[i]) - i128::from(p[i - 1])).ok());
    let (d0, d1, d2) = (d[0]?, d[1]?, d[2]?);
    let q = port_lerp(port_lerp(d0, d1, t), port_lerp(d1, d2, t), t);
    i64::try_from(3 * i128::from(q)).ok()
}

/// `P0` to `P3` of segment `i`: wrapped when closed, end points duplicated when open.
fn port_neighbours(points: &[Raw3], closed: bool, i: usize) -> [Raw3; 4] {
    let n = points.len();
    // Point `j - 1`, so that segment 0's `P0` is point -1.
    let before =
        |j: usize| if closed { points[(j + n - 1) % n] } else { points[j.clamp(1, n) - 1] };
    [before(i), before(i + 1), before(i + 2), before(i + 3)]
}

/// The segment and local parameter of `u`, the last segment owning `u = count`.
fn port_locate(count: usize, u: i64) -> (usize, i64) {
    let (whole, t) = (usize::try_from(u / U).expect("u >= 0"), u % U);
    if whole == count {
        (count - 1, U)
    } else {
        (whole, t)
    }
}

fn port_segment(points: &[Raw3], closed: bool, i: usize) -> Option<Axes> {
    let q = port_neighbours(points, closed, i);
    let axis = |k: usize| {
        let [a, b, c, d] = q.map(|point| i128::from(point[k]));
        let b1 = i64::try_from(port_round_sixth(6 * b + c - a)).ok()?;
        let b2 = i64::try_from(port_round_sixth(6 * c - d + b)).ok()?;
        Some([q[1][k], b1, b2, q[2][k]])
    };
    Some([axis(0)?, axis(1)?, axis(2)?])
}

fn port_count(points: &[Raw3], closed: bool) -> usize {
    if closed {
        points.len()
    } else {
        points.len() - 1
    }
}

fn port_new(points: &[Raw3], closed: bool) -> Result<(), CatmullRomError> {
    if points.len() < if closed { 3 } else { 2 } {
        return Err(CatmullRomError::TooFewPoints);
    }
    if points.iter().all(|p| *p == points[0]) {
        return Err(CatmullRomError::AllPointsEqual);
    }
    match (0..port_count(points, closed)).find(|&i| port_segment(points, closed, i).is_none()) {
        Some(segment) => Err(CatmullRomError::ControlPointOutOfRange { segment }),
        None => Ok(()),
    }
}

// ---- The exact power form ----

/// A 384-bit two's-complement integer; every exact numerator here is below `2^180`.
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
        for (limb, (&left, &right)) in out.iter_mut().zip(self.0.iter().zip(&other.0)) {
            let sum = u128::from(left) + u128::from(right) + carry;
            (*limb, carry) = (sum as u64, sum >> 64);
        }
        Self(out)
    }

    fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::new(1))
    }

    fn mul(self, other: Self) -> Self {
        let mut out = [0_u64; 6];
        for (i, &left) in self.0.iter().enumerate() {
            let mut carry = 0_u128;
            for (j, &right) in other.0.iter().take(6 - i).enumerate() {
                let sum = u128::from(out[i + j]) + u128::from(left) * u128::from(right) + carry;
                (out[i + j], carry) = (sum as u64, sum >> 64);
            }
        }
        Self(out)
    }

    fn is_negative(self) -> bool {
        self.0[5] >> 63 == 1
    }
}

/// The power-form coefficients `[2 P1, P2 - P0, 2 P0 - 5 P1 + 4 P2 - P3, 3 P1 - P0 - 3 P2 + P3]`
/// of twice the exact Catmull-Rom cubic on one axis.
fn power_form(q: [Raw3; 4], k: usize) -> [i128; 4] {
    let [a, b, c, d] = q.map(|point| i128::from(point[k]));
    [2 * b, c - a, 2 * a - 5 * b + 4 * c - d, 3 * b - a - 3 * c + d]
}

/// Whether `|result - n / d| <= num / den` (with `d > 0`), decided exactly.
fn within(result: i64, (n, d): (W, W), (num, den): (i128, i128)) -> bool {
    let error = W::new(i128::from(result)).mul(d).add(n.neg()).mul(W::new(den));
    let magnitude = if error.is_negative() { error.neg() } else { error };
    !d.mul(W::new(num)).add(magnitude.neg()).is_negative()
}

/// Position and derivative of the exact cubic at `t = s / 2^32`, as `(numerator, denominator)`:
/// `C = (c0 U^3 + c1 s U^2 + c2 s^2 U + c3 s^3) / (2 U^3)` and
/// `C' = (c1 U^2 + 2 c2 s U + 3 c3 s^2) / (2 U^2)`.
fn exact(c: [i128; 4], s: i64) -> ((W, W), (W, W)) {
    let (s, one) = (W::new(i128::from(s)), W::new(1 << 32));
    let term = |coefficient: i128, powers_of_s: usize, powers_of_one: usize| {
        let mut value = W::new(coefficient);
        for _ in 0..powers_of_s {
            value = value.mul(s);
        }
        for _ in 0..powers_of_one {
            value = value.mul(one);
        }
        value
    };
    let position =
        term(c[0], 0, 3).add(term(c[1], 1, 2)).add(term(c[2], 2, 1)).add(term(c[3], 3, 0));
    let velocity = term(c[1], 0, 2).add(term(2 * c[2], 1, 1)).add(term(3 * c[3], 2, 0));
    ((position, term(2, 0, 3)), (velocity, term(2, 0, 2)))
}

// ---- Seeded inputs ----

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn pick(state: &mut u64, n: u64) -> u64 {
    (next_seed(state) >> 32) % n
}

/// A coordinate of every magnitude, with the ends of the raw range, zero and repeats often.
fn coordinate(state: &mut u64) -> i64 {
    match pick(state, 16) {
        0 => MIN,
        1 => MAX,
        2 => 0,
        3 => 3 * U,
        _ => next_seed(state).cast_signed() >> (2 + pick(state, 62)),
    }
}

/// A local parameter: the ends and one raw unit inside them, multiples of 1/8, else uniform.
fn local(state: &mut u64) -> i64 {
    match pick(state, 8) {
        0 => 0,
        1 => U,
        2 => 1,
        3 => U - 1,
        4 | 5 => pick(state, 9).cast_signed() * (U / 8),
        _ => ((next_seed(state) >> 16) % (1 << 32 | 1)).cast_signed(),
    }
}

// ---- Cases ----

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_020_hand_derived_collinear_and_through_points() {
    // P_i = i d with d = (6, 12, -6): interior control steps are d / 3 = (2, 4, -2) whole
    // units, so every lerp at u = k/8 is exact and at(u) = u d with no rounding.
    let d = [6 * U, 12 * U, -6 * U];
    let raw: Vec<Raw3> = (0..6).map(|i| d.map(|c| i * c)).collect();
    let points: Vec<FixedVec3> = raw.iter().copied().map(vec3).collect();
    let s = spline(&points, false);
    assert_eq!(s.segment_count(), 5);
    for k in 8..=32 {
        let u = k * U / 8;
        assert_eq!(at(s, u), Some(d.map(|c| c / 8 * k)), "interior k={k}");
        assert_eq!(derivative(s, u), Some(d), "interior k={k}");
    }
    // The end segments see a duplicated end point: control points 0, d/6, 2d/3, d, so the
    // exact cubic is d g(t) with g(t) = (t + 2 t^2 - t^3) / 2, and every lerp is still exact.
    let g = |k: i64| d.map(|c| c / 6 * 3 * (64 * k + 16 * k * k - k * k * k) / 512);
    for k in 0..=8 {
        assert_eq!(at(s, k * U / 8), Some(g(k)), "first k={k}");
        let mirrored = g(8 - k).map(|c| -c);
        let last = raw[5];
        let expected = [0, 1, 2].map(|i| last[i] + mirrored[i]);
        assert_eq!(at(s, 4 * U + k * U / 8), Some(expected), "last k={k}");
    }
    // The ends' tangents are (P1 - P0) / 2 exactly; the joint tangents are d.
    assert_eq!(derivative(s, 0), Some(d.map(|c| c / 2)));
    assert_eq!(derivative(s, 5 * U), Some(d.map(|c| c / 2)));
    for k in 0..=5 {
        assert_eq!(at(s, k * U), Some(raw[k as usize]), "through point {k}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_021_golden_vectors() {
    // From the Python model in the pull request (integer rule port plus a `fractions` check).
    let open =
        [[0, 0, 0], [U, 2 * U, -H], [3 * U + 1, -U, 7], [4 * U, U, -3], [5 * U + 3, 3 * U, 1]];
    let closed = [[0, 0, 0], [6 * U + 5, 0, U], [3 * U, -6 * U - 1, -U]];
    let (open_points, closed_points) = (open.map(vec3), closed.map(vec3));
    let (o, c) = (spline(&open_points, false), spline(&closed_points, true));
    let open_inner: [[Raw3; 2]; 4] = [
        [
            [715_827_883, 1_431_655_765, -357_913_941],
            [2_147_483_648, 9_305_762_475, -2_147_483_649],
        ],
        [
            [6_442_450_944, 7_874_106_709, -2_147_483_647],
            [10_737_418_241, -3_579_139_413, -357_913_934],
        ],
        [[15_032_385_537, -5_010_795_179, 357_913_948], [15_748_213_418, 1_431_655_765, -2]],
        [[18_611_524_950, 7_158_278_827, -4], [20_759_008_600, 11_453_246_123, 0]],
    ];
    let closed_inner: [[Raw3; 2]; 3] = [
        [[2_147_483_649, U, 1_431_655_765], [23_622_320_133, U, 5_010_795_179]],
        [[27_917_287_429, -U, 3_579_139_413], [17_179_869_185, -25_769_803_777, -3_579_139_413]],
        [[8_589_934_591, -25_769_803_777, -5_010_795_179], [-2_147_483_649, -U, -1_431_655_765]],
    ];
    for (s, raw, inner) in [(o, &open[..], &open_inner[..]), (c, &closed[..], &closed_inner[..])] {
        let beziers: Vec<CubicBezier3> = s.to_beziers().collect();
        assert_eq!(beziers.len(), inner.len());
        for (i, (bezier, [b1, b2])) in beziers.iter().zip(inner).enumerate() {
            let end = raw[(i + 1) % raw.len()];
            assert_eq!(*bezier, CubicBezier3::new(vec3(raw[i]), vec3(*b1), vec3(*b2), vec3(end)));
            assert_eq!(s.segment(i), Some(*bezier));
        }
    }
    let rows: [(CatmullRom3<'_>, i64, Raw3, Raw3); 10] = [
        (
            o,
            H,
            [1_610_612_736, 5_100_273_663, -1_207_959_552],
            [4_294_967_298, 12_348_030_975, -2_952_790_017],
        ),
        (
            o,
            U + 1_431_655_765,
            [6_999_205_963, 5_090_331_611, -1_670_265_057],
            [9_305_762_475, -16_464_041_298, 2_505_397_599],
        ),
        (o, 2 * U, [12_884_901_889, -U, 7], [6_442_450_944, -2_147_483_649, 1_073_741_823]),
        (
            o,
            2 * U + H,
            [15_300_820_992, -1_342_177_280, 134_217_730],
            [3_758_096_382, 11_274_289_155, -268_435_473],
        ),
        (o, 4 * U - 1, [21_474_836_483, 12_884_901_888, 1], [2_147_483_649, 4_294_967_301, 3]),
        (o, 4 * U, [21_474_836_483, 12_884_901_888, 1], [2_147_483_649, 4_294_967_295, 3]),
        (c, 0, [0, 0, 0], [6_442_450_947, 12_884_901_888, 4_294_967_295]),
        (c, H, [12_884_901_891, 3_221_225_472, 2_952_790_015], [35_433_480_201, 0, 5_905_580_031]),
        // x is 1.4999999979 raw from the exact cubic.
        (c, U + 1, [25_769_803_781, -3, U], [6_442_450_926, -12_884_901_912, -2_147_483_655]),
        (
            c,
            2 * U + 2_863_311_531,
            [1_431_655_763, -8_589_934_589, -1_908_874_353],
            [-12_884_901_882, 34_359_738_366, 6_442_450_944],
        ),
    ];
    for (s, u, point, slope) in rows {
        assert_eq!(at(s, u), Some(point), "u={u}");
        assert_eq!(derivative(s, u), Some(slope), "u={u}");
    }
    assert_eq!(at(c, 3 * U), Some([0, 0, 0]));
    assert_eq!(derivative(c, 3 * U), derivative(c, 0), "the closed tangent agrees at the seam");
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_022_refusals_and_the_parameter_range() {
    let (p, q, r) = (vec3([0, 0, 0]), vec3([U, -3, 7]), vec3([2 * U, U, -5 * U]));
    for (points, closed, expected) in [
        (&[][..], false, Err(CatmullRomError::TooFewPoints)),
        (&[p][..], false, Err(CatmullRomError::TooFewPoints)),
        (&[p, q][..], false, Ok(1)),
        (&[p, q][..], true, Err(CatmullRomError::TooFewPoints)),
        (&[p, q, r][..], true, Ok(3)),
        (&[q, q][..], false, Err(CatmullRomError::AllPointsEqual)),
        (&[q, q, q][..], true, Err(CatmullRomError::AllPointsEqual)),
        (&[q, q, p][..], false, Ok(2)),
        (&[q, q, p][..], true, Ok(3)),
    ] {
        let made = CatmullRom3::new(points, closed);
        assert_eq!(made.map(CatmullRom3::segment_count), expected, "{points:?} closed={closed}");
        if let Ok(s) = made {
            assert_eq!((s.points(), s.is_closed()), (points, closed));
        }
    }
    let points = [p, q, r];
    for closed in [false, true] {
        let s = spline(&points, closed);
        let end = i64::try_from(s.segment_count()).expect("small") * U;
        for u in [MIN, -U, -1, end + 1, end + U, MAX] {
            assert_eq!((at(s, u), derivative(s, u)), (None, None), "u={u} closed={closed}");
        }
        assert!(at(s, 0).is_some() && at(s, end).is_some(), "closed={closed}");
        assert_eq!(s.segment(s.segment_count()), None);
        // u = end is owned by the last segment at t = 1.0.
        let last = s.segment(s.segment_count() - 1).expect("in range");
        assert_eq!(s.at(fx(end)), last.at(FixedQ32_32::ONE));
        assert_eq!(s.derivative_at(fx(end)), last.derivative_at(FixedQ32_32::ONE));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_023_overflow_on_and_past_each_threshold() {
    let out = |segment| Err(CatmullRomError::ControlPointOutOfRange { segment });
    // Segment 1's b1 = (6 (MAX - 1) + P2) / 6 = MAX - 1 + P2 / 6: MAX + 1/2 ties to the even
    // MAX + 1, out of range, so 8 fits and 9 does not.
    // Segment 0's b2 = (MIN + 1) - P2 / 6: MIN - 1/2 ties to the even MIN, so 9 still fits.
    for (middle, p2, expected) in [
        (MAX - 1, 8, Ok(2)),
        (MAX - 1, 9, out(1)),
        (MAX - 1, 10, out(1)),
        (MIN + 1, 8, Ok(2)),
        (MIN + 1, 9, Ok(2)),
        (MIN + 1, 10, out(0)),
    ] {
        let points = [[0, 0, 0], [middle, 0, 0], [p2, 0, 0]].map(vec3);
        let made = CatmullRom3::new(&points, false);
        assert_eq!(made.map(CatmullRom3::segment_count), expected, "middle={middle} p2={p2}");
    }
    // The tangent at point 1 is 3 (round((6 P1 + MAX - MIN) / 6) - P1): the exact sixth is
    // 3 074 457 345 618 258 602.5, which ties to 602 (3 times it is MAX - 1) only for P1 = 0;
    // for P1 = -1 and 1 the tie goes the other way, to 603, and 3 times that overflows.
    for (p1, expected) in [(-1, None), (0, Some([MAX - 1, 0, 0])), (1, None)] {
        let points = [[MIN, 0, 0], [p1, 0, 0], [MAX, 0, 0]].map(vec3);
        let s = spline(&points, false);
        assert_eq!(derivative(s, U), expected, "p1={p1}");
        let arriving = s.segment(0).and_then(|c| c.derivative_at(FixedQ32_32::ONE)).map(raw_of);
        assert_eq!(arriving, expected, "p1={p1}: the same tangent arriving");
        assert!(at(s, U).is_some() && at(s, U + H).is_some(), "evaluation is defined");
    }
}

/// Checks one spline against both references at its knots and at `samples` local parameters.
fn check_spline(raw: &[Raw3], closed: bool, samples: &[(usize, i64)], context: &str) -> [u32; 2] {
    let points: Vec<FixedVec3> = raw.iter().copied().map(vec3).collect();
    let s = spline(&points, closed);
    let count = port_count(raw, closed);
    let mut beyond = [0_u32; 2];
    for (i, bezier) in s.to_beziers().enumerate() {
        assert_eq!(Some(axes_of(bezier)), port_segment(raw, closed, i), "{context} segment={i}");
    }
    assert_eq!(s.to_beziers().count(), count, "{context}");
    for &(i, t) in samples {
        let u = i64::try_from(i % count).expect("small") * U + t;
        let (i, t) = port_locate(count, u);
        let axes = port_segment(raw, closed, i).expect("a valid spline");
        let q = port_neighbours(raw, closed, i);
        let context = format!("{context} u={u}");
        let point = at(s, u).expect("u in range");
        assert_eq!(point, axes.map(|p| port_at(p, t)), "{context}");
        let slope = derivative(s, u);
        let port_slope = axes.map(|p| port_derivative(p, t));
        assert_eq!(
            slope,
            port_slope[0].zip(port_slope[1]).zip(port_slope[2]).map(|((x, y), z)| [x, y, z]),
            "{context}"
        );
        for (k, &value) in point.iter().enumerate() {
            let (position, velocity) = exact(power_form(q, k), t);
            assert!(within(value, position, (15, 8)), "{context} axis={k}");
            beyond[0] += u32::from(!within(value, position, (3, 2)));
            if let Some(slope) = slope {
                assert!(within(slope[k], velocity, (9, 2)), "{context} axis={k}");
                beyond[1] += u32::from(!within(slope[k], velocity, (3, 1)));
            }
        }
    }
    for k in 0..=count {
        let u = i64::try_from(k).expect("small") * U;
        assert_eq!(at(s, u), Some(raw[k % raw.len()]), "{context} through point {k}");
        if (1..count).contains(&k) || closed {
            let arriving =
                s.segment((k + count - 1) % count).and_then(|c| c.derivative_at(FixedQ32_32::ONE));
            let leaving = s.segment(k % count).and_then(|c| c.derivative_at(fx(0)));
            if let (Some(arriving), Some(leaving)) = (arriving, leaving) {
                assert_eq!(arriving, leaving, "{context} tangent at point {k}");
            }
        }
        if let Some(slope) = derivative(s, u).filter(|_| k < count) {
            let q = port_neighbours(raw, closed, k);
            for (axis, &value) in slope.iter().enumerate() {
                let twice = i128::from(q[2][axis]) - i128::from(q[0][axis]);
                assert!(
                    (2 * i128::from(value) - twice).abs() <= 3,
                    "{context} knot {k} axis={axis}"
                );
            }
        }
    }
    if closed {
        let end = i64::try_from(count).expect("small") * U;
        assert_eq!(at(s, 0), at(s, end), "{context} closed seam");
    }
    beyond
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_024_seeded_splines_match_both_oracles() {
    println!("curve_tp_024 seed={SEED:#018x} lists=20000 samples=4 per list");
    let mut state = SEED;
    let (mut valid, mut refused, mut beyond) = (0_u32, 0_u32, [0_u32; 2]);
    for case in 0..20_000_u32 {
        let (n, closed) = (1 + pick(&mut state, 6) as usize, pick(&mut state, 2) == 1);
        let raw: Vec<Raw3> = (0..n)
            .map(|_| [coordinate(&mut state), coordinate(&mut state), coordinate(&mut state)])
            .collect();
        let samples: Vec<(usize, i64)> =
            (0..4).map(|_| (pick(&mut state, 8) as usize, local(&mut state))).collect();
        let context = format!("seed={SEED:#018x} case={case} points={raw:?} closed={closed}");
        let points: Vec<FixedVec3> = raw.iter().copied().map(vec3).collect();
        let made = CatmullRom3::new(&points, closed).map(|_| ());
        assert_eq!(made, port_new(&raw, closed), "{context}");
        if made.is_ok() {
            valid += 1;
            let [position, slope] = check_spline(&raw, closed, &samples, &context);
            beyond = [beyond[0] + position, beyond[1] + slope];
        } else {
            refused += 1;
        }
    }
    println!(
        "curve_tp_024 valid={valid} refused={refused} beyond_3/2={} beyond_3={}",
        beyond[0], beyond[1]
    );
    assert!(valid >= 5_000 && refused >= 2_000, "both classes are reached");
}
