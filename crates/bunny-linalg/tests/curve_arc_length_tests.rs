//! Contract tests for the `bunny-curve/1` arc-length table (`docs/topics/curve/test-plan.md`,
//! CURVE-TP-040 to CURVE-TP-046).
//!
//! The oracles share no code with `bunny-linalg` or `bunny-num`; the kernel's sampled points
//! come from `CubicBezier3::at` (part 1, proven by `curve_tests.rs`) and are an input here:
//!
//! - the rule port: `port_lengths` recomputes every entry from the sampled points with the
//!   test's own integer square root, exact `u128` sums and its own rounding, and
//!   `port_param_at` recomputes the lookup from the table's entries;
//! - the reference arc length: `reference_cumulative` is the polyline through 100 000 equal
//!   parameter steps of the exact cubic (exact rational points, rounded to `2^-16` raw), a
//!   reference and not an exact value; `PYTHON_ARC_RAW` pins the arc lengths of a Python
//!   `decimal` composite Simpson integration over 100 000 steps (posted in the pull request).
//!
//! `U = 2^32` is `1.0` and `H = 2^31` is `0.5` in raw units.

// Raw values cross between i64, i128 and u128 in both directions inside proven ranges.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]

use bunny_linalg::{
    ArcLengthError, ArcLengthTable, CubicBezier3, Curve3, FixedVec3, MAX_SEGMENTS_PER_SPAN,
};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = 1 << 32;
const H: i64 = 1 << 31;
const K: i64 = 2_372_044_938;
const FINE: i128 = 100_000;

type Polygon = [[i64; 3]; 4];

/// The quarter-circle-like arc, an S curve rising in z, and a planar loop whose xy path
/// crosses itself near `t = 0.11` and `t = 0.89`.
const CURVES: [(&str, Polygon); 3] = [
    ("arc", [[U, 0, 0], [U, K, 0], [K, U, 0], [0, U, 0]]),
    ("s", [[0, 0, 0], [U, 2 * U, 0], [2 * U, -2 * U, H], [3 * U, 0, H]]),
    ("loop", [[0, 0, 0], [4 * U, 4 * U, 0], [-2 * U, 4 * U, 0], [2 * U, 0, 0]]),
];
const PYTHON_ARC_RAW: [i64; 3] = [6_747_465_339, 17_070_500_551, 31_749_351_003];
const MEASURED_SEGMENTS: [u32; 3] = [8, 64, 512];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn point(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn bezier(p: Polygon) -> CubicBezier3 {
    CubicBezier3::new(point(p[0]), point(p[1]), point(p[2]), point(p[3]))
}

fn raws(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = 1_u128 << (128 - n.leading_zeros()).div_ceil(2);
    loop {
        let y = x.midpoint(n / x);
        if y >= x {
            return x;
        }
        x = y;
    }
}

fn round_half_even(numerator: u128, denominator: u128) -> u128 {
    let (q, twice_rest) = (numerator / denominator, 2 * (numerator % denominator));
    q + u128::from(twice_rest > denominator || (twice_rest == denominator && q % 2 == 1))
}

fn squared_distance(a: [i64; 3], b: [i64; 3]) -> u128 {
    (0..3).map(|c| (i128::from(b[c]) - i128::from(a[c])).unsigned_abs().pow(2)).sum()
}

fn port_param(k: u32, m: u32) -> i64 {
    round_half_even(u128::from(k) << 32, u128::from(m)) as i64
}

fn sampled(curve: &CubicBezier3, k: u32, m: u32) -> [i64; 3] {
    raws(curve.at(fixed(port_param(k, m))).expect("u_k is in [0, 1]"))
}

/// Entries by the stated rule: `floor(2^16 |chord|)` by the test's square root, an exact sum,
/// and `round_half_even(sum / 2^16)`.
fn port_lengths(curve: &CubicBezier3, m: u32) -> Vec<i64> {
    let (mut sum, mut lengths) = (0_u128, vec![0]);
    for k in 1..=m {
        sum += isqrt(squared_distance(sampled(curve, k - 1, m), sampled(curve, k, m)) << 32);
        lengths.push(round_half_even(sum, 1 << 16) as i64);
    }
    lengths
}

/// The lookup by the stated rule, from the kernel's entries.
fn port_param_at(lengths: &[i64], m: u32, s: i64) -> i64 {
    let total = *lengths.last().expect("non-empty");
    if s == total {
        return port_param(m, m);
    }
    let j = lengths.partition_point(|&l| l < s);
    if lengths[j] == s {
        return port_param(j as u32, m);
    }
    let (lower, upper) = (port_param(j as u32 - 1, m), port_param(j as u32, m));
    let numerator = (s - lengths[j - 1]) as u128 * (upper - lower) as u128;
    lower + round_half_even(numerator, (lengths[j] - lengths[j - 1]) as u128) as i64
}

fn exact_point_scaled(p: &Polygon, j: i128) -> [i128; 3] {
    let weights = [(FINE - j).pow(3), 3 * j * (FINE - j).pow(2), 3 * j * j * (FINE - j), j.pow(3)];
    let mut out = [0_i128; 3];
    for (c, slot) in out.iter_mut().enumerate() {
        let x: i128 = weights.iter().zip(p).map(|(w, q)| w * i128::from(q[c])).sum();
        *slot = (x << 16).div_euclid(FINE.pow(3));
    }
    out
}

/// The reference: cumulative lengths, in `2^-16` raw, of the polyline through the exact cubic
/// at `j / 100 000`. Each chord is floored to `2^-16` raw, so it is below the inscribed
/// polyline by less than 2 raw units in all, and that polyline is below the true arc length.
fn reference_cumulative(p: &Polygon) -> Vec<u128> {
    let mut cumulative = vec![0_u128];
    let mut previous = exact_point_scaled(p, 0);
    for j in 1..=FINE {
        let next = exact_point_scaled(p, j);
        let square: u128 = (0..3).map(|c| (next[c] - previous[c]).unsigned_abs().pow(2)).sum();
        cumulative.push(cumulative[cumulative.len() - 1] + isqrt(square));
        previous = next;
    }
    cumulative
}

/// The reference parameter (raw) at reference distance `s` raw, interpolated linearly
/// between the 100 000 reference steps.
fn reference_param(cumulative: &[u128], s: i64) -> i64 {
    let target = (s as u128) << 16;
    let j = cumulative.partition_point(|&l| l < target).clamp(1, cumulative.len() - 1);
    let (low, high) = (cumulative[j - 1], cumulative[j]);
    let numerator = ((j as u128 - 1) * (high - low) + (target.clamp(low, high) - low)) << 32;
    round_half_even(numerator, FINE as u128 * (high - low)) as i64
}

fn build(curve: &CubicBezier3, m: u32) -> (Vec<FixedQ32_32>, Result<(), ArcLengthError>) {
    let mut storage = vec![FixedQ32_32::ZERO; m as usize + 1];
    let result = ArcLengthTable::build(curve, m, &mut storage).map(|_| ());
    (storage, result)
}

fn table<'a>(curve: &CubicBezier3, m: u32, storage: &'a mut [FixedQ32_32]) -> ArcLengthTable<'a> {
    ArcLengthTable::build(curve, m, storage).expect("the test curve is accepted")
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_040_hand_derived_straight_curve() {
    // Collinear control points on the segment (0,0,0)-(3,4,0), symmetric about t = 1/2.
    let straight = bezier([[0, 0, 0], [3 * U / 4, U, 0], [9 * U / 4, 3 * U, 0], [3 * U, 4 * U, 0]]);
    assert_eq!(Curve3::span_count(&straight), 1);
    for t in [i64::MIN, -1, 0, 1, H, U - 1, U, U + 1] {
        assert_eq!(Curve3::at(&straight, fixed(t)), straight.at(fixed(t)), "t={t}");
    }
    for m in [1, 2, 3, 7, 8, 64, 512, 4095, MAX_SEGMENTS_PER_SPAN] {
        let mut storage = vec![FixedQ32_32::ZERO; m as usize + 1];
        let table = table(&straight, m, &mut storage);
        let total = table.total().raw();
        assert!((total - 5 * U).abs() <= 2, "m={m} total={total}");
        let middle = table.param_at_distance(fixed(5 * H)).expect("2.5 is inside").raw();
        assert!((middle - H).abs() <= 2, "m={m} param(2.5)={middle}");
        assert_eq!(table.param_at_distance(FixedQ32_32::ZERO), Some(FixedQ32_32::ZERO));
        assert_eq!(table.param_at_distance(table.total()), Some(FixedQ32_32::ONE));
        assert_eq!(table.end(), FixedQ32_32::ONE);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_041_entries_equal_the_rule_port() {
    for (name, p) in CURVES {
        for m in [1, 2, 3, 8, 64, 512, 4095, MAX_SEGMENTS_PER_SPAN] {
            let (storage, result) = build(&bezier(p), m);
            assert_eq!(result, Ok(()), "{name} m={m}");
            let kernel: Vec<i64> = storage.iter().map(|l| l.raw()).collect();
            assert_eq!(kernel, port_lengths(&bezier(p), m), "{name} m={m}");
            assert!(kernel.windows(2).all(|w| w[0] <= w[1]), "{name} m={m} non-decreasing");
        }
    }
    // Chords whose length is r + 1/2 to 2^-16 raw (found by search): the tie goes to even.
    for (chord, rounded) in [([29_999, 300, 0], 30_000), ([29_995, 623, 43], 30_002)] {
        let tie = bezier([[0; 3], [0; 3], chord, chord]);
        let (storage, result) = build(&tie, 1);
        assert_eq!((result, storage[1].raw()), (Ok(()), rounded), "{chord:?}");
        assert_eq!(port_lengths(&tie, 1), [0, rounded], "{chord:?}");
    }
}

/// Measured `python - total` in raw units, per curve, for 8, 64 and 512 segments.
const MEASURED_TOTAL_ERROR: [[i64; 3]; 3] = [
    [10_851_892, 169_586, 2_649],
    [194_220_052, 3_011_720, 47_057],
    [338_169_008, 5_298_611, 82_794],
];
/// The in-test reference is below the Python value by at most this, measured.
const REFERENCE_BELOW_PYTHON: i64 = 4;

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_042_total_against_the_reference_arc_length() {
    for ((name, p), (python, measured)) in
        CURVES.iter().zip(PYTHON_ARC_RAW.iter().zip(MEASURED_TOTAL_ERROR))
    {
        let reference = reference_cumulative(p);
        let reference_total = (reference[reference.len() - 1] >> 16) as i64;
        println!("curve_tp_042 {name} python={python} reference={reference_total}");
        assert!((0..=REFERENCE_BELOW_PYTHON).contains(&(python - reference_total)), "{name}");
        let mut errors = Vec::new();
        for (m, recorded) in MEASURED_SEGMENTS.iter().zip(measured) {
            let mut storage = vec![FixedQ32_32::ZERO; *m as usize + 1];
            let error = python - table(&bezier(*p), *m, &mut storage).total().raw();
            println!("curve_tp_042 {name} m={m} python-total={error} recorded={recorded}");
            // The table is below the arc length here; the proven bound allows it above.
            assert!((0..=recorded).contains(&error), "{name} m={m} error={error}");
            errors.push(error);
        }
        assert!(errors.windows(2).all(|w| w[0] > w[1]), "{name} improves: {errors:?}");
    }
}

/// A curve with stationary samples: equal points at the start, the middle and the end.
fn stationary_curves() -> [CubicBezier3; 3] {
    [
        bezier([[0, 0, 0], [0, 0, 0], [0, 0, 0], [U, 0, 0]]),
        bezier([[0, 0, 0], [U / 16, 0, 0], [0, 0, 0], [U / 16, 0, 0]]),
        bezier([[0, 0, 0], [U, 0, 0], [U, 0, 0], [U, 0, 0]]),
    ]
}

/// The distances probed: a grid of 4096 steps, every entry, and one raw unit either side.
fn probes(lengths: &[i64]) -> Vec<i64> {
    let total = lengths[lengths.len() - 1];
    let mut s: Vec<i64> = (0..=4096).map(|k| (i128::from(total) * k / 4096) as i64).collect();
    s.extend(lengths.iter().flat_map(|&l| [l - 1, l, l + 1]));
    s.retain(|&d| (0..=total).contains(&d));
    s.sort_unstable();
    s
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_043_lookup_rule_ends_runs_and_monotonicity() {
    let mut curves: Vec<CubicBezier3> = CURVES.iter().map(|(_, p)| bezier(*p)).collect();
    curves.extend(stationary_curves());
    let mut runs = [0_usize; 3];
    for (index, curve) in curves.iter().enumerate() {
        for m in [1, 3, 64, MAX_SEGMENTS_PER_SPAN] {
            let mut storage = vec![FixedQ32_32::ZERO; m as usize + 1];
            let table = table(curve, m, &mut storage);
            let lengths: Vec<i64> = table.lengths().iter().map(|l| l.raw()).collect();
            let total = lengths[lengths.len() - 1];
            for bad in [i64::MIN, -1, total + 1, i64::MAX] {
                assert_eq!(table.param_at_distance(fixed(bad)), None, "s={bad}");
            }
            let mut last = 0;
            for s in probes(&lengths) {
                let u = table.param_at_distance(fixed(s)).expect("inside").raw();
                assert_eq!(u, port_param_at(&lengths, m, s), "curve={index} m={m} s={s}");
                assert!(u >= last, "curve={index} m={m} s={s} decreases");
                last = u;
            }
            assert_eq!(table.param_at_distance(fixed(0)), Some(fixed(0)));
            assert_eq!(table.param_at_distance(fixed(total)), Some(FixedQ32_32::ONE));
            let repeats = lengths.windows(2).filter(|w| w[0] == w[1]);
            let (start, end) = (lengths[1] == 0, lengths[m as usize - 1] == total);
            let interior = repeats.filter(|w| w[0] != 0 && w[0] != total).count();
            runs = [runs[0] + usize::from(start), runs[1] + interior, runs[2] + usize::from(end)];
        }
    }
    println!("curve_tp_043 runs at start, interior, end: {runs:?}");
    assert!(runs.iter().all(|&count| count > 0), "every kind of run is reached: {runs:?}");
}

/// Measured spread (largest minus smallest of the 16 chords) in raw units, per curve and m.
const MEASURED_SPREAD: [[i64; 3]; 3] = [
    [7_843_847, 53_183, 7_728],
    [285_549_379, 17_658_873, 14_073_735],
    [579_021_637, 101_158_459, 97_916_333],
];
/// Measured largest `|u - u_reference|` in raw parameter units, per curve and m.
const MEASURED_PARAM_ERROR: [[i64; 3]; 3] =
    [[6_551_585, 102_354, 1_599], [44_872_345, 736_481, 10_943], [56_322_716, 898_567, 13_799]];

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_044_constant_speed_sampling() {
    for (curve_index, (name, p)) in CURVES.iter().enumerate() {
        let reference = reference_cumulative(p);
        for (m_index, &m) in MEASURED_SEGMENTS.iter().enumerate() {
            let mut storage = vec![FixedQ32_32::ZERO; m as usize + 1];
            let table = table(&bezier(*p), m, &mut storage);
            let total = i128::from(table.total().raw());
            let (mut points, mut worst_param) = (Vec::new(), 0);
            for k in 0..=16 {
                let s = (total * k / 16) as i64;
                let u = table.param_at_distance(fixed(s)).expect("inside");
                worst_param = worst_param.max((u.raw() - reference_param(&reference, s)).abs());
                points.push(raws(bezier(*p).at(u).expect("u is in [0, 1]")));
            }
            let chords: Vec<i64> =
                points.windows(2).map(|w| isqrt(squared_distance(w[0], w[1])) as i64).collect();
            let spread = chords.iter().max().expect("16") - chords.iter().min().expect("16");
            let (recorded_spread, recorded_param) =
                (MEASURED_SPREAD[curve_index][m_index], MEASURED_PARAM_ERROR[curve_index][m_index]);
            println!("curve_tp_044 {name} m={m} spread={spread} param_error={worst_param}");
            assert!(spread <= recorded_spread, "{name} m={m} spread={spread}");
            assert!(worst_param <= recorded_param, "{name} m={m} param error={worst_param}");
        }
    }
}

/// A curve that refuses one sample, or claims a span count.
struct Spans {
    count: u32,
    refuse: Option<i64>,
}

impl Curve3 for Spans {
    fn span_count(&self) -> u32 {
        self.count
    }

    fn at(&self, u: FixedQ32_32) -> Option<FixedVec3> {
        (self.refuse != Some(u.raw())).then(|| point([u.raw(), 0, 0]))
    }
}

fn axis_curve(from: i64, to: i64) -> CubicBezier3 {
    bezier([[from, 0, 0], [from, 0, 0], [to, 0, 0], [to, 0, 0]])
}

fn build_error(curve: &dyn Curve3, m: u32, storage_len: usize) -> Option<ArcLengthError> {
    let mut storage = vec![FixedQ32_32::ZERO; storage_len];
    ArcLengthTable::build(curve, m, &mut storage).err()
}

#[wasm_bindgen_test(unsupported = test)]
fn curve_tp_045_refusals_on_and_past_each_threshold() {
    use ArcLengthError::{
        ChordOverflow, CurveRefused, LengthOverflow, SegmentsOutOfRange, SpanCountOutOfRange,
        StorageTooSmall, TooManySamples, ZeroLength,
    };
    let line = Spans { count: 1, refuse: None };
    assert_eq!(build_error(&line, 0, 8), Some(SegmentsOutOfRange));
    assert_eq!(build_error(&line, 1, 2), None);
    assert_eq!(build_error(&line, 4096, 4097), None);
    assert_eq!(build_error(&line, 4097, 4098), Some(SegmentsOutOfRange));
    assert_eq!(build_error(&line, 8, 9), None);
    assert_eq!(build_error(&line, 8, 8), Some(StorageTooSmall { required: 9 }));
    assert_eq!(build_error(&Spans { count: 0, refuse: None }, 1, 8), Some(SpanCountOutOfRange));
    let too_many = Spans { count: 1 << 31, refuse: None };
    assert_eq!(build_error(&too_many, 1, 8), Some(SpanCountOutOfRange));
    let product = Spans { count: (1 << 20) + 1, refuse: None };
    assert_eq!(build_error(&product, 4096, 8), Some(TooManySamples));
    let mut two_span_storage = [FixedQ32_32::ZERO; 5];
    let two_spans =
        ArcLengthTable::build(&Spans { count: 2, refuse: None }, 2, &mut two_span_storage);
    let two_spans = two_spans.expect("two spans of a unit line");
    assert_eq!((two_spans.total().raw(), two_spans.end().raw()), (2 * U, 2 * U));
    assert_eq!(two_spans.param_at_distance(fixed(3 * H)), Some(fixed(3 * H)));
    assert_eq!(
        build_error(&Spans { count: 1, refuse: Some(H) }, 2, 3),
        Some(CurveRefused { sample: 1 })
    );
    let constant = bezier([[7, -3, U]; 4]);
    assert_eq!(build_error(&constant, 64, 65), Some(ZeroLength));
    // One chord of exactly i64::MAX raw is accepted and exact; one raw more leaves Q32.32.
    let mut storage = [FixedQ32_32::ZERO; 3];
    let longest = table(&axis_curve(-(1 << 62), (1 << 62) - 1), 1, &mut storage);
    assert_eq!(longest.total().raw(), i64::MAX);
    assert_eq!(
        build_error(&axis_curve(-(1 << 62) - 1, (1 << 62) - 1), 1, 2),
        Some(ChordOverflow { chord: 0 })
    );
    // Two chords summing to exactly i64::MAX are accepted; one raw more overflows the total.
    let halves = table(&axis_curve(-(1 << 62), (1 << 62) - 1), 2, &mut storage);
    assert_eq!(halves.total().raw(), i64::MAX);
    assert_eq!(
        build_error(&axis_curve(-(1 << 62) - 1, (1 << 62) - 1), 2, 3),
        Some(LengthOverflow { entry: 2 })
    );
}
