//! Contract tests for the conservative bounds of `bunny-curve/1`
//! (`docs/topics/curve/test-plan.md`, CURVE-TP-060 to -067).
//!
//! The references share no code with `bunny-linalg` or `bunny-num`:
//!
//! - `port_box` is the per-axis minimum and maximum of raw points, and `port_split_box` the
//!   half's control box widened by 3 raw units per face and clipped to the curve's box,
//!   from its own de Casteljau port (`port_lerp`, `port_levels`);
//! - `port_segment` converts a Catmull-Rom segment with `i128` Euclidean division by 6 and
//!   ties to even, with its own neighbour indexing;
//! - containment is decided on raw `i64` coordinates, per axis, by `inside`, not by the
//!   kernel's `CurveBounds3::contains`, which is checked against it.
//!
//! Containment is checked against the kernel's own `at`: the bound is on what the shipped
//! evaluation returns, rounding included, not only on the exact curve.
//!
//! `U = 2^32` is `1.0` and `H = 2^31` is `0.5` in raw units.

// The ports name the de Casteljau points and the four neighbours as the chapter does.
#![allow(clippy::many_single_char_names)]

use bunny_linalg::{CatmullRom3, CubicBezier3, CurveBounds3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const SEED: u64 = 0xB0_0152_B0C5_0001;
/// The widening of a split half's box, in raw units (`docs/topics/curve/README.md`).
const PAD: i128 = 3;

type Raw3 = [i64; 3];
type Polygon = [Raw3; 4];
/// A box as `(min, max)` on raw coordinates.
type RawBox = (Raw3, Raw3);

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
    [c.p0, c.p1, c.p2, c.p3].map(raw_of)
}

fn raw_box(b: CurveBounds3) -> RawBox {
    (raw_of(b.min), raw_of(b.max))
}

fn port_box(points: &[Raw3]) -> RawBox {
    let mut low = [MAX; 3];
    let mut high = [MIN; 3];
    for point in points {
        for (k, &value) in point.iter().enumerate() {
            low[k] = low[k].min(value);
            high[k] = high[k].max(value);
        }
    }
    (low, high)
}

fn port_union(a: RawBox, b: RawBox) -> RawBox {
    ([0, 1, 2].map(|k| a.0[k].min(b.0[k])), [0, 1, 2].map(|k| a.1[k].max(b.1[k])))
}

fn inside(b: RawBox, point: Raw3) -> bool {
    (0..3).all(|k| b.0[k] <= point[k] && point[k] <= b.1[k])
}

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

/// The halves `(p0, a, d, m)` and `(m, e, c, p3)` of the port.
fn port_halves(p: &Polygon, t: i64) -> (Polygon, Polygon) {
    let levels = [0, 1, 2].map(|k| port_levels(p.map(|point| point[k]), t));
    let pick = |i: usize| [levels[0][i], levels[1][i], levels[2][i]];
    ([p[0], pick(0), pick(3), pick(5)], [pick(5), pick(4), pick(2), p[3]])
}

/// The half's control box widened by `PAD` raw units per face, clipped to the curve's box.
fn port_split_box(p: &Polygon, half: &Polygon) -> RawBox {
    let (outer, inner) = (port_box(p), port_box(half));
    let clip = |value: i128, low: i64, high: i64| {
        i64::try_from(value.clamp(i128::from(low), i128::from(high))).expect("clipped to i64")
    };
    (
        [0, 1, 2].map(|k| clip(i128::from(inner.0[k]) - PAD, outer.0[k], outer.1[k])),
        [0, 1, 2].map(|k| clip(i128::from(inner.1[k]) + PAD, outer.0[k], outer.1[k])),
    )
}

/// `round((6 at + toward - from) / 6)` to nearest, ties to even, or `None` outside `i64`.
fn port_round_sixth(at: i64, toward: i64, from: i64) -> Option<i64> {
    let numerator = 6 * i128::from(at) + i128::from(toward) - i128::from(from);
    let (q, r) = (numerator.div_euclid(6), numerator.rem_euclid(6));
    let up = r > 3 || (r == 3 && q.rem_euclid(2) == 1);
    i64::try_from(q + i128::from(up)).ok()
}

/// Segment `i`'s Bezier control points: wrapped neighbours when closed, clamped when open.
fn port_segment(points: &[Raw3], closed: bool, i: usize) -> Option<Polygon> {
    let n = points.len();
    let index = |j: isize| -> usize {
        let count = isize::try_from(n).expect("a short list");
        let j = isize::try_from(i).expect("a short list") + j;
        let wrapped = if closed { j.rem_euclid(count) } else { j.clamp(0, count - 1) };
        usize::try_from(wrapped).expect("non-negative")
    };
    let [a, b, c, d] = [-1, 0, 1, 2].map(|j| points[index(j)]);
    let first = [0, 1, 2].map(|k| port_round_sixth(b[k], c[k], a[k]));
    let second = [0, 1, 2].map(|k| port_round_sixth(c[k], b[k], d[k]));
    Some([b, [first[0]?, first[1]?, first[2]?], [second[0]?, second[1]?, second[2]?], c])
}

fn port_spline_box(points: &[Raw3], closed: bool) -> RawBox {
    let count = if closed { points.len() } else { points.len() - 1 };
    (0..count)
        .map(|i| port_box(&port_segment(points, closed, i).expect("a convertible segment")))
        .reduce(port_union)
        .expect("at least one segment")
}

/// Every `t` the brute-force checks evaluate: 257 multiples of `1/256`, and every raw `t`
/// within 16 raw units of 0, 1/2 and 1 inside `[0, 1]`; 324 values.
fn dense_parameters() -> Vec<i64> {
    let mut t: Vec<i64> = (0..=256).map(|j| j * (U / 256)).collect();
    t.extend(0..=16);
    t.extend(H - 16..=H + 16);
    t.extend(U - 16..=U);
    t
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
        0 => MIN,
        1 => MAX,
        2 => 0,
        3 => MIN + 1,
        4 => MAX - 1,
        5 => pick(state, 9).cast_signed() - 4,
        _ => next_seed(state).cast_signed() >> pick(state, 64),
    }
}

fn polygon(state: &mut u64) -> Polygon {
    let mut point = || [coordinate(state), coordinate(state), coordinate(state)];
    [point(), point(), point(), point()]
}

/// A parameter in `[0, 1]`: the ends and one raw unit inside them, multiples of 1/8, else
/// uniform.
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

/// Asserts every `at(t)` over `parameters` lies in `b`, exactly, per axis.
fn assert_contains_evaluations(c: CubicBezier3, b: RawBox, parameters: &[i64], context: &str) {
    for &t in parameters {
        let point = raw_of(c.at(fx(t)).expect("t in [0, 1]"));
        assert!(inside(b, point), "{context} t={t} at={point:?} box={b:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_060_hand_derived_boxes_and_contains() {
    // Equal control points: the box is that point.
    let point = [5 * U + 3, -7, MIN];
    let constant = curve(&[point; 4]);
    assert_eq!(raw_box(constant.control_bounds()), (point, point));
    assert_contains_evaluations(constant, (point, point), &dense_parameters(), "constant");

    // A flat curve: y constant and z zero give zero extent on those axes.
    let flat = [[0, 2 * U, 0], [U, 2 * U, 0], [3 * U, 2 * U, 0], [2 * U, 2 * U, 0]];
    assert_eq!(raw_box(curve(&flat).control_bounds()), ([0, 2 * U, 0], [3 * U, 2 * U, 0]));
    assert_contains_evaluations(curve(&flat), port_box(&flat), &dense_parameters(), "flat");

    // A face set by an inner control point is not reached by the curve, and is still the box.
    let bump = [[0, 0, 0], [0, 4 * U, 0], [U, 4 * U, 0], [U, 0, 0]];
    let bounds = curve(&bump).control_bounds();
    assert_eq!(raw_box(bounds), ([0, 0, 0], [U, 4 * U, 0]));
    // The exact curve peaks at y = 3 (at t = 1/2), inside the face at 4.
    assert_eq!(raw_of(curve(&bump).at(fx(H)).expect("t in [0, 1]"))[1], 3 * U);

    // The ends of the raw range.
    let wide = [[MIN, MAX, 0], [MAX, MIN, 0], [MIN, MAX, 0], [MAX, MIN, 0]];
    assert_eq!(raw_box(curve(&wide).control_bounds()), ([MIN, MIN, 0], [MAX, MAX, 0]));

    // contains: closed per axis, on each face and one raw unit outside it.
    let b = CurveBounds3 { min: vec3([-1, 0, 10]), max: vec3([1, 0, 20]) };
    for (k, (low, high)) in [(-1, 1), (0, 0), (10, 20)].into_iter().enumerate() {
        let mut on = [0, 0, 15];
        for (value, expected) in [(low, true), (high, true), (low - 1, false), (high + 1, false)] {
            on[k] = value;
            assert_eq!(b.contains(vec3(on)), expected, "axis={k} point={on:?}");
            assert_eq!(inside(raw_box(b), on), expected, "axis={k} point={on:?}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_061_seeded_control_boxes_contain_every_evaluation() {
    const CASES: usize = 1_000;
    println!("curve_tp_061 seed={SEED:#018x} cases={CASES} parameters=324");
    let parameters = dense_parameters();
    let mut state = SEED;
    let mut evaluations = 0_usize;
    for case in 0..CASES {
        let p = polygon(&mut state);
        let context = format!("seed={SEED:#018x} case={case} p={p:?}");
        let bounds = curve(&p).control_bounds();
        assert_eq!(raw_box(bounds), port_box(&p), "{context}");
        assert_contains_evaluations(curve(&p), port_box(&p), &parameters, &context);
        for &t in &parameters {
            assert!(bounds.contains(curve(&p).at(fx(t)).expect("t in [0, 1]")), "{context}");
        }
        evaluations += parameters.len();
    }
    assert_eq!(evaluations, CASES * 324);
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_062_ties_at_a_box_face_stay_inside() {
    // x = (0, 1, 1, 0) raw at t = 1/2: a = lerp(0, 1) ties to the even 0, c = lerp(1, 0) ties
    // to 1, d = lerp(0, 1) ties to 0, and m = lerp(0, 1) ties to 0: on the lower face, while
    // the exact value is 3/4 raw.
    let small = [[0, 0, 0], [1, 0, 0], [1, 0, 0], [0, 0, 0]];
    let bounds = curve(&small).control_bounds();
    assert_eq!(raw_box(bounds), ([0, 0, 0], [1, 0, 0]));
    assert_eq!(raw_of(curve(&small).at(fx(H)).expect("t in [0, 1]")), [0, 0, 0]);
    // A step of one raw unit ties only at t = 1/2: on it, one raw unit either side, and the ends.
    for t in [H - 1, H, H + 1, 0, U] {
        let point = raw_of(curve(&small).at(fx(t)).expect("t in [0, 1]"));
        assert!(inside(port_box(&small), point), "t={t} point={point:?}");
    }

    // The same at the ends of the raw range: x = (MIN, MAX, MIN, MAX) ties every first-level
    // step at t = 1/2 (a step of (2^64 - 1) / 2), and y the mirror image.
    let wide = [[MIN, MAX, 0], [MAX, MIN, 0], [MIN, MAX, 0], [MAX, MIN, 0]];
    let wide_box = port_box(&wide);
    for t in [0, 1, H - 1, H, H + 1, U - 1, U] {
        let point = raw_of(curve(&wide).at(fx(t)).expect("t in [0, 1]"));
        assert!(inside(wide_box, point), "t={t} point={point:?}");
    }
    // The faces next to the range ends: one raw unit inside them.
    let near = [[MAX - 1, MIN + 1, 0], [MAX, MIN, 0], [MAX, MIN, 0], [MAX - 1, MIN + 1, 0]];
    assert_eq!(raw_box(curve(&near).control_bounds()), ([MAX - 1, MIN, 0], [MAX, MIN + 1, 0]));
    assert_contains_evaluations(curve(&near), port_box(&near), &dense_parameters(), "near");
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_063_split_bounds_contain_the_original_on_each_range() {
    const CASES: usize = 2_000;
    // Constructed: x = (-2, -3, 1, -4) raw, split at t = 2 329 330 117 raw. The right half's
    // x control points are (-2, -2, -2, -4), but the original at s = 2 916 077 839 raw (in
    // [t, 1]) is -1: one raw unit outside the half's own box. The widened box holds it.
    let p = [[-2, 0, 0], [-3, 0, 0], [1, 0, 0], [-4, 0, 0]];
    let (t, s) = (2_329_330_117, 2_916_077_839);
    let (_, right) = curve(&p).split(fx(t)).expect("t in [0, 1]");
    assert_eq!(polygon_of(right).map(|point| point[0]), [-2, -2, -2, -4]);
    let original = raw_of(curve(&p).at(fx(s)).expect("s in [0, 1]"));
    assert_eq!(original, [-1, 0, 0]);
    assert!(!inside(raw_box(right.control_bounds()), original));
    let (left_box, right_box) = curve(&p).split_bounds(fx(t)).expect("t in [0, 1]");
    assert!(right_box.contains(vec3(original)));
    assert_eq!(raw_box(right_box), ([-4, 0, 0], [1, 0, 0]));
    assert_eq!(raw_box(left_box), port_split_box(&p, &port_halves(&p, t).0));

    // Refusals: outside [0, 1].
    for t in [MIN, -U, -1, U + 1, 2 * U, MAX] {
        assert_eq!(curve(&p).split_bounds(fx(t)), None, "t={t}");
    }

    println!("curve_tp_063 seed={SEED:#018x} cases={CASES}");
    let mut state = SEED ^ 0x0063;
    let mut escapes = 0_usize;
    for case in 0..CASES {
        let p = polygon(&mut state);
        let t = parameter(&mut state);
        let context = format!("seed={SEED:#018x} case={case} p={p:?} t={t}");
        let (left, right) = port_halves(&p, t);
        let (left_box, right_box) = curve(&p).split_bounds(fx(t)).expect("t in [0, 1]");
        assert_eq!(raw_box(left_box), port_split_box(&p, &left), "{context}");
        assert_eq!(raw_box(right_box), port_split_box(&p, &right), "{context}");
        // The original on [0, t] and on [t, 1]: 65 even steps and the raw ends of each range.
        let (t128, unit) = (i128::from(t), i128::from(U));
        let steps = |low: i128, high: i128| -> Vec<i64> {
            let mut s: Vec<i64> = (0..=64_i128)
                .map(|j| i64::try_from(low + (high - low) * j / 64).expect("in [0, 1]"))
                .collect();
            s.extend(
                [low, low + 1, high - 1, high]
                    .into_iter()
                    .filter(|&v| low <= v && v <= high)
                    .map(|v| i64::try_from(v).expect("in [0, 1]")),
            );
            s
        };
        let ranges: [(Vec<i64>, Polygon, CurveBounds3); 2] =
            [(steps(0, t128), left, left_box), (steps(t128, unit), right, right_box)];
        for (range, half, b) in ranges {
            for s in range {
                let point = raw_of(curve(&p).at(fx(s)).expect("s in [0, 1]"));
                assert!(b.contains(vec3(point)), "{context} s={s} at={point:?}");
                escapes += usize::from(!inside(port_box(&half), point));
            }
            // The half's own evaluations.
            assert_contains_evaluations(curve(&half), raw_box(b), &steps(0, unit), &context);
        }
    }
    println!("curve_tp_063 original points outside the unwidened half box: {escapes}");
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_064_catmull_rom_box_covers_the_overshoot() {
    // x = (0, 10, 10, 0): the middle segment overshoots 10, to about 11.25 at u = 1.5.
    let raw = [[0, 0, 0], [10 * U, U, 0], [10 * U, 2 * U, 0], [0, 3 * U, 0]];
    let points = raw.map(vec3);
    let points_box = port_box(&raw);

    let open = CatmullRom3::new(&points, false).expect("a valid point list");
    let open_box = open.control_bounds();
    assert_eq!(raw_box(open_box), port_spline_box(&raw, false));
    let peak = raw_of(open.at(fx(U + H)).expect("u in range"));
    assert!(peak[0] > 10 * U, "the curve leaves the points' box: {peak:?}");
    assert!(!inside(points_box, peak));
    assert!(open_box.contains(vec3(peak)));
    // The open box's x range: from the first point to the middle segment's inner points,
    // round((60 + 10) U / 6) = 35 U / 3, rounded up (35 U = 2 mod 3).
    assert_eq!(raw_box(open_box).0[0], 0);
    assert_eq!(raw_box(open_box).1[0], 35 * U / 3 + 1);

    // Closed: the segment from the last point back to the first dips below 0 at u = 3.5.
    let closed = CatmullRom3::new(&points, true).expect("a valid point list");
    let closed_box = closed.control_bounds();
    assert_eq!(raw_box(closed_box), port_spline_box(&raw, true));
    let dip = raw_of(closed.at(fx(3 * U + H)).expect("u in range"));
    assert!(dip[0] < 0, "the closing segment leaves the points' box: {dip:?}");
    assert!(!inside(raw_box(open_box), dip));
    assert!(closed_box.contains(vec3(dip)));
    assert_ne!(raw_box(open_box), raw_box(closed_box));

    for (spline, b) in [(open, open_box), (closed, closed_box)] {
        for u in spline_parameters(spline.segment_count()) {
            let point = raw_of(spline.at(fx(u)).expect("u in range"));
            assert!(inside(raw_box(b), point), "u={u} at={point:?}");
        }
    }
}

/// For each segment `k`: 33 multiples of `1/32` and 2 raw units either side of its ends.
fn spline_parameters(count: usize) -> Vec<i64> {
    let count = i64::try_from(count).expect("a short spline");
    let mut u = Vec::new();
    for k in 0..count {
        u.extend((0..=32).map(|j| k * U + j * (U / 32)));
        u.extend([k * U + 1, k * U + 2, (k + 1) * U - 2, (k + 1) * U - 1]);
    }
    u.push(count * U);
    u
}

/// A spline coordinate: seven in eight scaled down by 2^3, so that most lists convert
/// (`|x| <= 2^60` keeps every `(6 P1 + P2 - P0) / 6` inside the range).
fn spline_coordinate(state: &mut u64) -> i64 {
    let shift = if pick(state, 8) == 0 { 0 } else { 3 };
    coordinate(state) >> shift
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_065_seeded_catmull_rom_boxes_contain_every_evaluation() {
    const CASES: usize = 2_000;
    println!("curve_tp_065 seed={SEED:#018x} cases={CASES}");
    let mut state = SEED ^ 0x0065;
    let (mut built, mut overshoots) = (0_usize, 0_usize);
    for case in 0..CASES {
        let closed = pick(&mut state, 2) == 1;
        let n = usize::try_from(2 + pick(&mut state, 5)).expect("a short list");
        let raw: Vec<Raw3> =
            (0..n).map(|_| [0, 1, 2].map(|_| spline_coordinate(&mut state))).collect();
        let points: Vec<FixedVec3> = raw.iter().copied().map(vec3).collect();
        let Ok(spline) = CatmullRom3::new(&points, closed) else { continue };
        built += 1;
        let context = format!("seed={SEED:#018x} case={case} closed={closed} points={raw:?}");
        let bounds = spline.control_bounds();
        assert_eq!(raw_box(bounds), port_spline_box(&raw, closed), "{context}");
        for u in spline_parameters(spline.segment_count()) {
            let point = raw_of(spline.at(fx(u)).expect("u in range"));
            assert!(inside(raw_box(bounds), point), "{context} u={u} at={point:?}");
            overshoots += usize::from(!inside(port_box(&raw), point));
        }
    }
    println!("curve_tp_065 built={built} evaluations outside the points' box={overshoots}");
    assert!(built > CASES / 2, "too few valid splines: {built}");
    assert!(overshoots > 0, "no evaluation left the points' box");
}
