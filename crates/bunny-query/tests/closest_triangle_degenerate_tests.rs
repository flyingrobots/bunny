//! `closest_point_triangle` on degenerate triangles (flyingrobots/bunny#500), against an
//! exact oracle, and bit-identical to the previous routine on every other triangle.
//!
//! A degenerate triangle (collinear or repeated vertices) is the segment between its two
//! farthest vertices, or a point. Its closest point to `p` is the closest point of that
//! segment. The oracle here finds the farthest pair by exact squared distance and projects
//! `p` in `i128` rationals; it shares no code with the kernel, which picks the hull ends by
//! the extremes along one axis and calls `closest_points_segments`.
//!
//! `reference_closest_point_triangle` is a frozen copy of the routine as it was before the
//! fix. The differential test (CLOSEST-TP-007) checks that every non-degenerate triangle still
//! gets exactly its answer, bit for bit. The seeds are fixed and printed; every failure
//! message names the seed and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{closest_point_triangle, closest_points_segments};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const SWEEP_SEED: u64 = 0xB050_0C10_5E57_0001;
const DIFFERENTIAL_SEED: u64 = 0xB050_0C10_5E57_0002;
const SWEEP_CASES_PER_FAMILY: usize = 6_000;
const DIFFERENTIAL_CASES_PER_FAMILY: usize = 25_000;

type Raw3 = [i64; 3];

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn units(x: i64, y: i64, z: i64) -> Raw3 {
    [x * ONE, y * ONE, z * ONE]
}

fn kernel(t: [Raw3; 3], p: Raw3) -> Raw3 {
    raw(closest_point_triangle(v(t[0]), v(t[1]), v(t[2]), v(p)))
}

fn reference(t: [Raw3; 3], p: Raw3) -> Raw3 {
    raw(reference_closest_point_triangle(v(t[0]), v(t[1]), v(t[2]), v(p)))
}

// ---------------------------------------------------------------------------------------
// The exact oracle. Every coordinate is assumed below `2^40` raw in magnitude, so a
// difference is below `2^41`, a squared length or a dot product below `2^84`, and every
// product formed below is below `2^126`.
// ---------------------------------------------------------------------------------------

/// The exact closest point as rational numerators over a positive denominator, and the
/// raw edge `b - a` of the hull segment (zero for a point).
struct Exact {
    numerators: [i128; 3],
    denominator: i128,
    edge: [i128; 3],
    edge_len_sq: i128,
}

fn wide(r: Raw3) -> [i128; 3] {
    r.map(i128::from)
}

fn squared_distance(a: [i128; 3], b: [i128; 3]) -> i128 {
    (a[0] - b[0]).pow(2) + (a[1] - b[1]).pow(2) + (a[2] - b[2]).pow(2)
}

fn oracle(t: [Raw3; 3], p: Raw3) -> Exact {
    let vertices = t.map(wide);
    let p = wide(p);
    let mut ends = (vertices[0], vertices[1]);
    let mut longest = squared_distance(vertices[0], vertices[1]);
    for (i, j) in [(0, 2), (1, 2)] {
        let len = squared_distance(vertices[i], vertices[j]);
        if len > longest {
            longest = len;
            ends = (vertices[i], vertices[j]);
        }
    }
    let (a, b) = ends;
    let edge = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    if longest == 0 {
        return Exact { numerators: a, denominator: 1, edge, edge_len_sq: 0 };
    }
    let along = edge[0] * (p[0] - a[0]) + edge[1] * (p[1] - a[1]) + edge[2] * (p[2] - a[2]);
    let (num, den) = if along <= 0 {
        (0, 1)
    } else if along >= longest {
        (1, 1)
    } else {
        (along, longest)
    };
    let numerators = [0, 1, 2].map(|k| a[k] * den + edge[k] * num);
    Exact { numerators, denominator: den, edge, edge_len_sq: longest }
}

/// The documented bound for component `k`, in raw units:
/// `1 + min(|d_k|, ceil(|d_k| / 2^33) + ceil(2^35 |d_k| / |d|^2))` for the hull edge `d`.
fn bound(exact: &Exact, k: usize) -> i128 {
    let d = exact.edge[k].abs();
    if exact.edge_len_sq == 0 {
        return 1;
    }
    let ceil_div = |n: i128, m: i128| (n + m - 1) / m;
    let rounding = ceil_div(d, 1_i128 << 33) + ceil_div(d << 35, exact.edge_len_sq);
    1 + d.min(rounding)
}

/// `|result_k - exact_k|` in raw units, as a numerator over the oracle's denominator.
fn error_numerator(result: Raw3, exact: &Exact, k: usize) -> i128 {
    (i128::from(result[k]) * exact.denominator - exact.numerators[k]).abs()
}

fn within_bound(result: Raw3, exact: &Exact) -> bool {
    (0..3).all(|k| error_numerator(result, exact, k) <= bound(exact, k) * exact.denominator)
}

/// The largest error, rounded up to whole raw units.
fn error_ceil(result: Raw3, exact: &Exact) -> i128 {
    (0..3)
        .map(|k| {
            let n = error_numerator(result, exact, k);
            (n + exact.denominator - 1) / exact.denominator
        })
        .max()
        .unwrap_or(0)
}

/// Whether the raw vertices are collinear (or repeated), by exact cross product. Each
/// difference has magnitude below `2^64`, so each product's magnitude fits `u128`.
fn exactly_degenerate(t: [Raw3; 3]) -> bool {
    let e1 = [0, 1, 2].map(|k| i128::from(t[1][k]) - i128::from(t[0][k]));
    let e2 = [0, 1, 2].map(|k| i128::from(t[2][k]) - i128::from(t[0][k]));
    let signed = |a: i128, b: i128| {
        let m = a.unsigned_abs() * b.unsigned_abs();
        (m != 0 && (a < 0) != (b < 0), m)
    };
    [(1, 2), (2, 0), (0, 1)].iter().all(|&(i, j)| signed(e1[i], e2[j]) == signed(e1[j], e2[i]))
}

/// The answer the documented rule gives: `closest_points_segments(p, p, a, b).1` for the
/// farthest pair of vertices `(a, b)` in vertex order, the first such pair on a tie.
fn delegated(t: [Raw3; 3], p: Raw3) -> Raw3 {
    let w = t.map(wide);
    let mut ends = (0, 1);
    for (i, j) in [(0, 2), (1, 2)] {
        if squared_distance(w[i], w[j]) > squared_distance(w[ends.0], w[ends.1]) {
            ends = (i, j);
        }
    }
    raw(closest_points_segments(v(p), v(p), v(t[ends.0]), v(t[ends.1])).1)
}

fn assert_contract(t: [Raw3; 3], p: Raw3, context: &str) -> Raw3 {
    let exact = oracle(t, p);
    let got = kernel(t, p);
    assert!(
        within_bound(got, &exact),
        "{context}: triangle {t:?}, point {p:?}: got {got:?}, error {} raw",
        error_ceil(got, &exact)
    );
    got
}

// ---------------------------------------------------------------------------------------
// Constructed cases.
// ---------------------------------------------------------------------------------------

/// CLOSEST-TP-001: the example of flyingrobots/bunny#500.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_001_the_issue_example_returns_the_closest_point_of_the_segment() {
    let t = [units(0, 0, 0), units(0, 0, 0), units(2, 0, 0)];
    let p = units(1, 1, 0);
    assert_eq!(reference(t, p), [0, 0, 0], "the old routine returned the repeated vertex");
    assert_eq!(kernel(t, p), units(1, 0, 0));
    assert_contract(t, p, "issue example");
}

/// The three arrangements of a repeated vertex `a` with a distinct `b`, in vertex order.
fn repeated_arrangements(a: Raw3, b: Raw3) -> [(&'static str, [Raw3; 3]); 3] {
    [("v0 = v1", [a, a, b]), ("v1 = v2", [b, a, a]), ("v0 = v2", [a, b, a])]
}

/// CLOSEST-TP-002: every repeated-vertex arrangement, with either end repeated, for points
/// whose projection is inside the segment, on each end, beyond each end and one raw unit
/// either side of each end.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_002_repeated_vertices_give_the_closest_point_of_the_segment() {
    let (a, b) = (units(0, 0, 0), units(2, 0, 0));
    // Points whose exact answer is a representable raw value, with that answer.
    let exact_cases = [
        (units(1, 1, 0), units(1, 0, 0)),
        ([ONE / 2, -3 * ONE, 2 * ONE], [ONE / 2, 0, 0]),
        ([3 * ONE / 2, 0, -5 * ONE], [3 * ONE / 2, 0, 0]),
        (units(0, 5, 0), a),
        (units(2, -5, 1), b),
        (units(-1, 1, 0), a),
        (units(3, 1, 0), b),
        ([-1, ONE, 0], a),
        ([2 * ONE + 1, ONE, 0], b),
    ];
    // One raw unit inside each end: the exact answer is one raw unit from the end, which the
    // segment routine reaches within its bound.
    let inside_ends = [[1, ONE, 0], [2 * ONE - 1, ONE, 0]];
    for (ends, flip) in [((a, b), false), ((b, a), true)] {
        for (name, t) in repeated_arrangements(ends.0, ends.1) {
            for (p, expected) in exact_cases {
                let context = format!("{name}, repeated end flipped: {flip}");
                assert_eq!(assert_contract(t, p, &context), expected, "{context}, point {p:?}");
            }
            for p in inside_ends {
                let got = assert_contract(t, p, name);
                assert!((0..=2 * ONE).contains(&got[0]) && got[1] == 0 && got[2] == 0, "{name}");
            }
        }
    }
}

/// CLOSEST-TP-003: three equal vertices are a point, returned exactly for every query point,
/// including the ends of the raw range.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_003_three_equal_vertices_are_the_point() {
    let corners =
        [[0, 0, 0], units(3, -7, 11), [i64::MAX; 3], [i64::MIN; 3], [i64::MIN, 0, i64::MAX]];
    let points = [[0, 0, 0], units(1, 1, 1), [i64::MAX; 3], [i64::MIN; 3], [1, -1, 1]];
    for c in corners {
        for p in points {
            assert_eq!(kernel([c, c, c], p), c, "vertex {c:?}, point {p:?}");
        }
    }
}

/// CLOSEST-TP-004: collinear distinct vertices with the middle vertex at each index, on lines
/// along x, along y (x constant), along z and on a diagonal: points closest to the interior
/// of the longest edge, to the middle vertex, to each end and beyond each end.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_004_collinear_vertices_give_the_closest_point_of_the_longest_edge() {
    let lines: [(Raw3, Raw3); 4] = [
        (units(0, 0, 0), units(1, 0, 0)),
        (units(5, -2, 0), units(0, 1, 0)),
        (units(0, 3, 3), units(0, 0, 1)),
        (units(-1, 2, 0), units(1, 2, -1)),
    ];
    for (origin, dir) in lines {
        let at = |s: i64| [0, 1, 2].map(|k| origin[k] + dir[k] * s);
        // An offset perpendicular to `dir`: `dir x (1, 1, 1)` scaled, or another axis.
        let side = {
            let c = [dir[1] - dir[2], dir[2] - dir[0], dir[0] - dir[1]].map(|x| x.signum() * ONE);
            if c == [0; 3] {
                units(0, 0, 1)
            } else {
                c
            }
        };
        let (lo, mid, hi) = (at(0), at(1), at(4));
        for (name, t) in [
            ("middle v0", [mid, lo, hi]),
            ("middle v1", [lo, mid, hi]),
            ("middle v2", [lo, hi, mid]),
        ] {
            assert!(exactly_degenerate(t));
            for (s, expected) in
                [(3, at(3)), (1, mid), (2, at(2)), (0, lo), (4, hi), (-3, lo), (9, hi)]
            {
                let p = [0, 1, 2].map(|k| at(s)[k] + side[k] * 2);
                let context = format!("{name}, line {dir:?}, s = {s}");
                assert_eq!(assert_contract(t, p, &context), expected, "{context}");
            }
        }
    }
}

/// CLOSEST-TP-006: the non-degenerate neighbours of the cases above, one raw unit off the
/// line, keep the previous routine's answer bit for bit, while the degenerate case itself
/// changes where the previous answer was wrong.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_006_neighbours_one_raw_unit_away_are_unchanged() {
    let (a, b, m) = (units(0, 0, 0), units(2, 0, 0), units(1, 0, 0));
    let bases = [[a, a, b], [b, a, a], [a, b, a], [a, m, b], [m, a, b], [a, b, m]];
    let points =
        [units(1, 1, 0), units(-1, 1, 0), units(3, 1, 0), [1, ONE, 0], [ONE / 2, -ONE, ONE]];
    let (mut checked, mut skipped) = (0, 0);
    for base in bases {
        for vertex in 0..3 {
            for (axis, delta) in [(1, 1), (1, -1), (2, 1), (2, -1)] {
                let mut t = base;
                t[vertex][axis] += delta;
                if exactly_degenerate(t) {
                    // Moving the vertex that is not repeated leaves a repeated vertex.
                    skipped += 1;
                    continue;
                }
                for p in points {
                    assert_eq!(kernel(t, p), reference(t, p), "neighbour {t:?}, point {p:?}");
                    checked += 1;
                }
            }
        }
    }
    assert_eq!((checked, skipped), ((6 * 3 * 4 - 3 * 4) * 5, 3 * 4));
}

/// CLOSEST-TP-008: degenerate triangles at the ends of the raw range. Every intermediate of
/// the segment routine saturates here (flyingrobots/bunny#116, decision D5), so the answer is
/// pinned, not endorsed.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_008_ends_of_the_raw_range_are_pinned() {
    let (lo, hi) = ([i64::MIN, 0, 0], [i64::MAX, 0, 0]);
    // The edge `MAX - MIN` saturates to `MAX`; `p - a` saturates to `MAX`; the parameter is
    // `MAX / MAX = 1.0`, so the answer is `MIN + MAX = -1` raw. The exact answer is `0`.
    assert_eq!(kernel([lo, lo, hi], [0, ONE, 0]), [-1, 0, 0]);
    // `p - a = 2^62` is exact, but the dot product saturates to `MAX`, so the parameter is
    // still `1.0`: `-1` raw again, while the exact answer is `MIN / 2`. A saturated wrong
    // answer, the same kind as for non-degenerate triangles at these magnitudes.
    assert_eq!(kernel([lo, lo, hi], [i64::MIN / 2, ONE, 0]), [-1, 0, 0]);
    // A short segment at the corner of the range: nothing saturates and the answer is exact.
    let corner = [i64::MAX - 4 * ONE, i64::MAX, i64::MAX];
    let end = [i64::MAX, i64::MAX, i64::MAX];
    let p = [i64::MAX - 2 * ONE, i64::MAX - ONE, i64::MAX];
    assert_eq!(kernel([corner, end, end], p), [i64::MAX - 2 * ONE, i64::MAX, i64::MAX]);
}

// ---------------------------------------------------------------------------------------
// Seeded cases.
// ---------------------------------------------------------------------------------------

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> i64 {
        i64::try_from(self.next() % bound).expect("small bound")
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }

    /// Any `i64`.
    fn full(&mut self) -> i64 {
        let (high, low) = (self.next(), self.next());
        i64::from_ne_bytes(((high << 11) ^ low).to_ne_bytes())
    }
}

/// The arrangements a degenerate triangle can take.
const CLASSES: [&str; 7] =
    ["v0 = v1", "v1 = v2", "v0 = v2", "all equal", "middle v0", "middle v1", "middle v2"];

/// A line direction: large, raw-unit-sized, whole units, or along one or two axes.
fn direction(seeds: &mut Seeds, scale: usize) -> Raw3 {
    loop {
        let d = match scale {
            0 => seeds.triple(1 << 34),
            1 => seeds.triple(4),
            2 => seeds.triple(3).map(|c| c * ONE),
            _ => {
                let mut d = seeds.triple(1 << 34);
                d[usize::try_from(seeds.below(3)).expect("small")] = 0;
                if seeds.below(2) == 0 {
                    d[usize::try_from(seeds.below(3)).expect("small")] = 0;
                }
                d
            }
        };
        if d != [0; 3] {
            return d;
        }
    }
}

/// A degenerate triangle of class `class` on the line `base + s dir`, with its ends at
/// `s = 0` and `s` in `4..=6` and its middle vertex in `1..=3`, and a query point near the
/// line, at `s` anywhere in `-4..=10`. All coordinates stay below `2^40` raw.
fn degenerate_case(seeds: &mut Seeds, class: usize, scale: usize) -> ([Raw3; 3], Raw3) {
    let base = seeds.triple(1 << 38);
    let dir = direction(seeds, scale);
    let at = |s: i64| [0, 1, 2].map(|k| base[k] + dir[k] * s);
    let (a, m, b) = (at(0), at(1 + seeds.below(3)), at(4 + seeds.below(3)));
    let t = match class {
        0 => [a, a, b],
        1 => [a, b, b],
        2 => [a, b, a],
        3 => [a, a, a],
        4 => [m, a, b],
        5 => [a, m, b],
        _ => [a, b, m],
    };
    let near = at(seeds.signed(7) + 3);
    let off = if scale == 1 { seeds.triple(64) } else { seeds.triple(1 << 36) };
    (t, [0, 1, 2].map(|k| near[k] + off[k]))
}

/// CLOSEST-TP-005: seeded degenerate triangles of every class and four direction scales
/// against the exact oracle; the previous routine exceeds the bound on many of them.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_005_seeded_degenerate_triangles_match_the_oracle() {
    let mut seeds = Seeds(SWEEP_SEED);
    let mut worst = 0;
    for (class, name) in CLASSES.iter().enumerate() {
        let (mut reference_wrong, mut changed) = (0, 0);
        for scale in 0..4 {
            for index in 0..SWEEP_CASES_PER_FAMILY {
                let (t, p) = degenerate_case(&mut seeds, class, scale);
                assert!(exactly_degenerate(t));
                let exact = oracle(t, p);
                let got = kernel(t, p);
                assert!(
                    within_bound(got, &exact),
                    "seed {SWEEP_SEED:#x}, {name}, scale {scale}, case {index}: {t:?} {p:?} gave \
                     {got:?}, error {} raw",
                    error_ceil(got, &exact)
                );
                assert_eq!(got, delegated(t, p), "seed {SWEEP_SEED:#x}, {name}, case {index}");
                worst = worst.max(error_ceil(got, &exact));
                let previous = reference(t, p);
                if !within_bound(previous, &exact) {
                    reference_wrong += 1;
                }
                if previous != got {
                    changed += 1;
                }
            }
        }
        println!(
            "closest_tp_005: seed {SWEEP_SEED:#x}, {name}: {} cases, previous routine outside \
             the bound on {reference_wrong}, answer changed on {changed}",
            4 * SWEEP_CASES_PER_FAMILY
        );
        // The previous routine was exact for three equal vertices and wrong for a repeated
        // first edge and for collinear distinct vertices.
        match class {
            3 => assert_eq!((reference_wrong, changed), (0, 0), "{name}"),
            0 => assert!(reference_wrong > 1_000, "{name}: {reference_wrong}"),
            4..=6 => assert!(reference_wrong > 100, "{name}: {reference_wrong}"),
            _ => {}
        }
    }
    println!("closest_tp_005: largest error {worst} raw");
}

/// A non-degenerate triangle and query point from family `family`.
fn general_case(seeds: &mut Seeds, family: usize) -> ([Raw3; 3], Raw3) {
    match family {
        // Every coordinate below 2^40 raw.
        0 => (
            [seeds.triple(1 << 40), seeds.triple(1 << 40), seeds.triple(1 << 40)],
            seeds.triple(1 << 40),
        ),
        // The full raw range, where the routine saturates.
        1 => {
            let mut r = || [seeds.full(), seeds.full(), seeds.full()];
            ([r(), r(), r()], r())
        }
        // Whole units, query points on a half-unit grid.
        2 => (
            [
                seeds.triple(8).map(|c| c * ONE),
                seeds.triple(8).map(|c| c * ONE),
                seeds.triple(8).map(|c| c * ONE),
            ],
            seeds.triple(24).map(|c| c * ONE / 2),
        ),
        // One raw unit off a degenerate triangle of any class.
        _ => {
            let class = usize::try_from(seeds.below(7)).expect("small");
            let scale = usize::try_from(seeds.below(4)).expect("small");
            let (mut t, p) = degenerate_case(seeds, class, scale);
            let vertex = usize::try_from(seeds.below(3)).expect("small");
            let axis = usize::try_from(seeds.below(3)).expect("small");
            t[vertex][axis] += if seeds.below(2) == 0 { 1 } else { -1 };
            (t, p)
        }
    }
}

/// CLOSEST-TP-007: 100 000 seeded non-degenerate triangles in four families give exactly the
/// previous routine's answer.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_007_non_degenerate_triangles_are_bit_identical_to_the_previous_routine() {
    let mut seeds = Seeds(DIFFERENTIAL_SEED);
    let mut compared = 0;
    for family in 0..4 {
        let mut family_compared = 0;
        let mut skipped = 0;
        while family_compared < DIFFERENTIAL_CASES_PER_FAMILY {
            let (t, p) = general_case(&mut seeds, family);
            if exactly_degenerate(t) {
                skipped += 1;
                continue;
            }
            assert_eq!(
                kernel(t, p),
                reference(t, p),
                "seed {DIFFERENTIAL_SEED:#x}, family {family}, case {family_compared}: {t:?} {p:?}"
            );
            family_compared += 1;
        }
        println!(
            "closest_tp_007: seed {DIFFERENTIAL_SEED:#x}, family {family}: {family_compared} \
             compared, {skipped} degenerate skipped"
        );
        compared += family_compared;
    }
    assert_eq!(compared, 100_000);
}

// ---------------------------------------------------------------------------------------
// The previous routine, frozen: the region tests of `closest_point_triangle`, inlined from
// its helpers as they were before flyingrobots/bunny#500, used only as the bit-identity reference.
// ---------------------------------------------------------------------------------------

fn reference_closest_point_triangle(
    v0: FixedVec3,
    v1: FixedVec3,
    v2: FixedVec3,
    p: FixedVec3,
) -> FixedVec3 {
    let ab = v1 - v0;
    let ac = v2 - v0;
    let ap = p - v0;
    let bp = p - v1;
    let cp = p - v2;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    let zero = FixedQ32_32::ZERO;

    if d1 <= zero && d2 <= zero {
        return v0;
    }
    if d3 >= zero && d4 <= d3 {
        return v1;
    }
    if d6 >= zero && d5 <= d6 {
        return v2;
    }
    let va = d3 * d6 - d5 * d4;
    let vb = d5 * d2 - d1 * d6;
    let vc = d1 * d4 - d3 * d2;
    if vc <= zero && d1 >= zero && d3 <= zero {
        return v0 + ab * reference_ratio(d1, d1 - d3);
    }
    if vb <= zero && d2 >= zero && d6 <= zero {
        return v0 + ac * reference_ratio(d2, d2 - d6);
    }
    if va <= zero && (d4 - d3) >= zero && (d5 - d6) >= zero {
        let w = reference_ratio(d4 - d3, (d4 - d3) + (d5 - d6));
        return v1 + (v2 - v1) * w;
    }
    let denom = va + vb + vc;
    v0 + ab * reference_ratio(vb, denom) + ac * reference_ratio(vc, denom)
}

fn reference_ratio(numer: FixedQ32_32, denom: FixedQ32_32) -> FixedQ32_32 {
    if denom == FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else {
        numer.checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
    }
}
