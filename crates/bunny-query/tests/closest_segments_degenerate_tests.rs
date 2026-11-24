//! `closest_points_segments` on degenerate and non-unique arrangements
//! (flyingrobots/bunny#506), against an exact oracle, and bit-identical to the previous
//! routine on every pair of non-degenerate, non-parallel segments.
//!
//! The oracle works in exact big-integer rationals and shares no code with the kernel. It
//! minimizes the squared distance `|r + d1 s - d2 t|^2` over `[0, 1]^2` by candidates: the
//! interior critical point when the segments are not parallel, and the clamped projection on
//! each of the four edges of the square. The smallest exact squared distance is the minimum.
//! The kernel instead follows one clamping sequence in rounded Q32.32 steps.
//!
//! `reference_closest_points_segments` is a frozen copy of the routine as it was before the
//! fix. The differential test (CLOSEST-TP-024) checks that every pair of non-degenerate,
//! non-parallel segments still gets exactly its answer, bit for bit. Degeneracy is decided by
//! the test's own exact checks, never by the kernel. The seeds are fixed and printed; every
//! failure message names the seed and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::closest_points_segments;
use core::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const SWEEP_SEED: u64 = 0xB050_6C10_5E57_0001;
const DIFFERENTIAL_SEED: u64 = 0xB050_6C10_5E57_0002;
const SWEEP_CASES_PER_SCALE: usize = 1_000;
const DIFFERENTIAL_CASES_PER_FAMILY: usize = 20_000;

type Raw3 = [i64; 3];
type Segments = [Raw3; 4];

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

fn kernel(s: Segments) -> (Raw3, Raw3) {
    let (c1, c2) = closest_points_segments(v(s[0]), v(s[1]), v(s[2]), v(s[3]));
    (raw(c1), raw(c2))
}

fn reference(s: Segments) -> (Raw3, Raw3) {
    let (c1, c2) = reference_closest_points_segments(v(s[0]), v(s[1]), v(s[2]), v(s[3]));
    (raw(c1), raw(c2))
}

// ---------------------------------------------------------------------------------------
// Exact arithmetic: a signed integer of 1 024 bits, enough for every product below.
// ---------------------------------------------------------------------------------------

const LIMBS: usize = 16;

/// A signed integer as a sign and a little-endian magnitude. Zero is never negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wide {
    negative: bool,
    magnitude: [u64; LIMBS],
}

fn magnitude_cmp(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> Ordering {
    a.iter().rev().cmp(b.iter().rev())
}

fn magnitude_add(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut out = [0; LIMBS];
    let mut carry = 0_u128;
    for ((o, x), y) in out.iter_mut().zip(a).zip(b) {
        let sum = u128::from(*x) + u128::from(*y) + carry;
        *o = u64::try_from(sum & u128::from(u64::MAX)).expect("masked");
        carry = sum >> 64;
    }
    assert_eq!(carry, 0, "Wide addition overflowed");
    out
}

/// `a - b` for `a >= b`.
fn magnitude_sub(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut out = [0; LIMBS];
    let mut borrow = false;
    for ((o, x), y) in out.iter_mut().zip(a).zip(b) {
        let (d1, b1) = x.overflowing_sub(*y);
        let (d2, b2) = d1.overflowing_sub(u64::from(borrow));
        *o = d2;
        borrow = b1 || b2;
    }
    assert!(!borrow, "magnitude_sub needs a >= b");
    out
}

impl Wide {
    const ZERO: Self = Self { negative: false, magnitude: [0; LIMBS] };

    fn from_i128(value: i128) -> Self {
        let m = value.unsigned_abs();
        let mut magnitude = [0; LIMBS];
        magnitude[0] = u64::try_from(m & u128::from(u64::MAX)).expect("masked");
        magnitude[1] = u64::try_from(m >> 64).expect("high half");
        Self::normalized(value < 0, magnitude)
    }

    fn normalized(negative: bool, magnitude: [u64; LIMBS]) -> Self {
        Self { negative: negative && magnitude != [0; LIMBS], magnitude }
    }

    fn is_zero(&self) -> bool {
        self.magnitude == [0; LIMBS]
    }

    fn abs(self) -> Self {
        Self { negative: false, magnitude: self.magnitude }
    }

    fn neg(self) -> Self {
        Self::normalized(!self.negative, self.magnitude)
    }

    fn add(self, rhs: Self) -> Self {
        if self.negative == rhs.negative {
            return Self::normalized(self.negative, magnitude_add(&self.magnitude, &rhs.magnitude));
        }
        match magnitude_cmp(&self.magnitude, &rhs.magnitude) {
            Ordering::Less => {
                Self::normalized(rhs.negative, magnitude_sub(&rhs.magnitude, &self.magnitude))
            }
            _ => Self::normalized(self.negative, magnitude_sub(&self.magnitude, &rhs.magnitude)),
        }
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    /// The number of limbs up to the highest nonzero one.
    fn len(&self) -> usize {
        self.magnitude.iter().rposition(|limb| *limb != 0).map_or(0, |i| i + 1)
    }

    fn mul(self, rhs: Self) -> Self {
        let (la, lb) = (self.len(), rhs.len());
        assert!(la + lb <= LIMBS, "Wide multiplication overflowed");
        let mut out = [0_u64; LIMBS];
        for (i, x) in self.magnitude[..la].iter().enumerate() {
            let mut carry = 0_u128;
            for (j, y) in rhs.magnitude[..lb].iter().enumerate() {
                let cell = u128::from(out[i + j]) + u128::from(*x) * u128::from(*y) + carry;
                out[i + j] = u64::try_from(cell & u128::from(u64::MAX)).expect("masked");
                carry = cell >> 64;
            }
            out[i + lb] = u64::try_from(carry).expect("carry fits a limb");
        }
        Self::normalized(self.negative != rhs.negative, out)
    }

    /// `self * 2^bits` for a non-negative value, `bits < 64`.
    fn shl(self, bits: u32) -> Self {
        if bits == 0 {
            return self;
        }
        assert!(self.len() < LIMBS, "Wide shift overflowed");
        let mut out = [0_u64; LIMBS];
        let mut carry = 0_u64;
        for (o, x) in out.iter_mut().zip(&self.magnitude) {
            *o = (x << bits) | carry;
            carry = x >> (64 - bits);
        }
        Self::normalized(self.negative, out)
    }

    /// `ceil(self / rhs)` for `self >= 0`, `rhs > 0` and a quotient below `2^64`.
    fn div_ceil_u64(self, rhs: Self) -> u64 {
        let mut rem = self;
        let mut q = 0_u64;
        for bit in (0..64).rev() {
            let step = rhs.shl(bit);
            if step <= rem {
                rem = rem.sub(step);
                q |= 1 << bit;
            }
        }
        assert!(rem < rhs, "quotient exceeds u64");
        q + u64::from(!rem.is_zero())
    }

    fn mul_i(self, rhs: i128) -> Self {
        self.mul(Self::from_i128(rhs))
    }
}

impl PartialOrd for Wide {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Wide {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => magnitude_cmp(&self.magnitude, &other.magnitude),
            (true, true) => magnitude_cmp(&other.magnitude, &self.magnitude),
        }
    }
}

fn w(value: i128) -> Wide {
    Wide::from_i128(value)
}

// ---------------------------------------------------------------------------------------
// Exact degeneracy checks over the whole raw range.
// ---------------------------------------------------------------------------------------

fn diff(a: Raw3, b: Raw3) -> [i128; 3] {
    [0, 1, 2].map(|k| i128::from(a[k]) - i128::from(b[k]))
}

/// Whether the exact cross product of `a` and `b` is zero. Each component is below `2^65`
/// in magnitude, so every product fits the wide integer.
fn cross_is_zero(a: [i128; 3], b: [i128; 3]) -> bool {
    [(1, 2), (2, 0), (0, 1)].iter().all(|&(i, j)| w(a[i]).mul_i(b[j]) == w(a[j]).mul_i(b[i]))
}

/// The arrangement of two segments, decided exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    SecondPoint,
    FirstPoint,
    BothPoints,
    Parallel,
    Collinear,
    General,
}

fn shape(s: Segments) -> Shape {
    match (s[0] == s[1], s[2] == s[3]) {
        (true, true) => Shape::BothPoints,
        (true, false) => Shape::FirstPoint,
        (false, true) => Shape::SecondPoint,
        (false, false) => {
            let (d1, d2) = (diff(s[1], s[0]), diff(s[3], s[2]));
            if !cross_is_zero(d1, d2) {
                Shape::General
            } else if cross_is_zero(d1, diff(s[2], s[0])) {
                Shape::Collinear
            } else {
                Shape::Parallel
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The exact oracle. Every coordinate is below `2^40` raw in magnitude, so a difference is
// below `2^41`, a dot product below `2^84`, a common denominator below `2^169`, and every
// comparison of squared distances below `2^770`.
// ---------------------------------------------------------------------------------------

const ORACLE_LIMIT: i64 = 1 << 40;

/// Parameters `s = sn / den` on the first segment and `t = tn / den` on the second.
#[derive(Clone, Copy, Debug)]
struct Pair {
    sn: Wide,
    tn: Wide,
    den: Wide,
}

struct Problem {
    p1: [i128; 3],
    p2: [i128; 3],
    d1: [i128; 3],
    d2: [i128; 3],
    r: [i128; 3],
    a: i128,
    b: i128,
    c: i128,
    e: i128,
    f: i128,
}

fn dot(x: [i128; 3], y: [i128; 3]) -> i128 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}

impl Problem {
    fn new(s: Segments) -> Self {
        assert!(
            s.iter().flatten().all(|c| (-ORACLE_LIMIT..ORACLE_LIMIT).contains(c)),
            "oracle input out of range: {s:?}"
        );
        let wide = |r: Raw3| r.map(i128::from);
        let (p1, q1, p2, q2) = (wide(s[0]), wide(s[1]), wide(s[2]), wide(s[3]));
        let d1 = [0, 1, 2].map(|k| q1[k] - p1[k]);
        let d2 = [0, 1, 2].map(|k| q2[k] - p2[k]);
        let r = [0, 1, 2].map(|k| p1[k] - p2[k]);
        Self {
            p1,
            p2,
            d1,
            d2,
            r,
            a: dot(d1, d1),
            b: dot(d1, d2),
            c: dot(d1, r),
            e: dot(d2, d2),
            f: dot(d2, r),
        }
    }

    /// `den * |r + d1 s - d2 t|`, squared, and `den^2`: the exact squared distance as a
    /// fraction.
    fn squared_distance(&self, pair: &Pair) -> (Wide, Wide) {
        let mut sum = Wide::ZERO;
        for k in 0..3 {
            let wk = pair
                .den
                .mul_i(self.r[k])
                .add(pair.sn.mul_i(self.d1[k]))
                .sub(pair.tn.mul_i(self.d2[k]));
            sum = sum.add(wk.mul(wk));
        }
        (sum, pair.den.mul(pair.den))
    }

    /// The points of `pair` on each segment, as numerators over `pair.den`.
    fn points(&self, pair: &Pair) -> ([Wide; 3], [Wide; 3]) {
        let x1 = [0, 1, 2].map(|k| pair.den.mul_i(self.p1[k]).add(pair.sn.mul_i(self.d1[k])));
        let x2 = [0, 1, 2].map(|k| pair.den.mul_i(self.p2[k]).add(pair.tn.mul_i(self.d2[k])));
        (x1, x2)
    }
}

/// `n / d` clamped to `[0, 1]`, as a numerator over `d > 0`.
fn clamp_unit(n: Wide, d: Wide) -> Wide {
    if n < Wide::ZERO {
        Wide::ZERO
    } else if n > d {
        d
    } else {
        n
    }
}

/// Every candidate the oracle considers. The exact minimizers are the candidates whose
/// squared distance is the least. `hints` are kernel outputs whose projections give extra
/// candidates, so that a non-unique minimum (parallel segments) is matched near the output.
fn candidates(problem: &Problem, hints: Option<(Raw3, Raw3)>) -> Vec<Pair> {
    let Problem { a, b, c, e, f, .. } = *problem;
    let one = w(1);
    match (a == 0, e == 0) {
        (true, true) => return vec![Pair { sn: Wide::ZERO, tn: Wide::ZERO, den: one }],
        (true, false) => {
            return vec![Pair { sn: Wide::ZERO, tn: clamp_unit(w(f), w(e)), den: w(e) }];
        }
        (false, true) => {
            return vec![Pair { sn: clamp_unit(w(-c), w(a)), tn: Wide::ZERO, den: w(a) }];
        }
        (false, false) => {}
    }
    let den = w(a).mul_i(e).sub(w(b).mul_i(b));
    if den > Wide::ZERO {
        let sn = w(b).mul_i(f).sub(w(c).mul_i(e));
        let tn = w(a).mul_i(f).sub(w(b).mul_i(c));
        if sn >= Wide::ZERO && sn <= den && tn >= Wide::ZERO && tn <= den {
            return vec![Pair { sn, tn, den }];
        }
    }
    let mut out = vec![
        // s = 0 and s = 1, with the best t; t = 0 and t = 1, with the best s.
        Pair { sn: Wide::ZERO, tn: clamp_unit(w(f), w(e)), den: w(e) },
        Pair { sn: w(e), tn: clamp_unit(w(f + b), w(e)), den: w(e) },
        Pair { sn: clamp_unit(w(-c), w(a)), tn: Wide::ZERO, den: w(a) },
        Pair { sn: clamp_unit(w(b - c), w(a)), tn: w(a), den: w(a) },
    ];
    if let Some((c1, c2)) = hints {
        let wide = |r: Raw3| r.map(i128::from);
        let ae = w(a).mul_i(e);
        // From the first output point: its parameter on the first segment, then the best t.
        let along1 = dot([0, 1, 2].map(|k| wide(c1)[k] - problem.p1[k]), problem.d1);
        let n = clamp_unit(w(along1), w(a));
        let tn = clamp_unit(w(f).mul_i(a).add(n.mul_i(b)), ae);
        out.push(Pair { sn: n.mul_i(e), tn, den: ae });
        // From the second output point: its parameter on the second segment, then the best s.
        let along2 = dot([0, 1, 2].map(|k| wide(c2)[k] - problem.p2[k]), problem.d2);
        let m = clamp_unit(w(along2), w(e));
        let sn = clamp_unit(m.mul_i(b).sub(w(c).mul_i(e)), ae);
        out.push(Pair { sn, tn: m.mul_i(a), den: ae });
    }
    out
}

/// The exact minimizers among the candidates.
fn minimizers(problem: &Problem, hints: Option<(Raw3, Raw3)>) -> Vec<Pair> {
    let all = candidates(problem, hints);
    let distances: Vec<(Wide, Wide)> = all.iter().map(|p| problem.squared_distance(p)).collect();
    let less = |x: &(Wide, Wide), y: &(Wide, Wide)| x.0.mul(y.1) < y.0.mul(x.1);
    let mut best = distances[0];
    for d in &distances {
        if less(d, &best) {
            best = *d;
        }
    }
    all.into_iter().zip(&distances).filter(|(_, d)| !less(&best, d)).map(|(p, _)| p).collect()
}

/// `ceil(|c * den - x| / den)` for one component, in raw units.
fn component_error(c: i64, x: Wide, den: Wide) -> u64 {
    den.mul_i(i128::from(c)).sub(x).abs().div_ceil_u64(den)
}

/// Whether `|c * den - x| <= bound * den`.
fn within(c: i64, x: Wide, den: Wide, bound: u64) -> bool {
    den.mul_i(i128::from(c)).sub(x).abs() <= den.mul_i(i128::from(bound))
}

/// The per-component bounds, in raw units, on each output point.
type Bounds = ([u64; 3], [u64; 3]);

/// The outcome of one case against the oracle.
struct Verdict {
    /// Within `bounds` of some exact closest pair.
    ok: bool,
    /// The least, over exact closest pairs, of the largest component error, in raw units.
    error: u64,
}

fn judge(s: Segments, got: (Raw3, Raw3), bounds: impl Fn(&Problem) -> Bounds) -> Verdict {
    let problem = Problem::new(s);
    let (b1, b2) = bounds(&problem);
    let mut verdict = Verdict { ok: false, error: u64::MAX };
    for pair in minimizers(&problem, Some(got)) {
        let (x1, x2) = problem.points(&pair);
        let ok = (0..3).all(|k| {
            within(got.0[k], x1[k], pair.den, b1[k]) && within(got.1[k], x2[k], pair.den, b2[k])
        });
        let error = (0..3)
            .flat_map(|k| {
                [
                    component_error(got.0[k], x1[k], pair.den),
                    component_error(got.1[k], x2[k], pair.den),
                ]
            })
            .max()
            .unwrap_or(0);
        verdict.ok |= ok;
        verdict.error = verdict.error.min(error);
    }
    verdict
}

/// The documented bound for a point on a segment `d` answered by the point-to-segment path:
/// `1 + min(|d_k|, ceil(|d_k| / 2^33) + ceil(2^35 |d_k| / |d|^2))` raw units per component.
fn segment_bound(d: [i128; 3]) -> [u64; 3] {
    let len_sq = dot(d, d);
    d.map(|dk| {
        let dk = dk.abs();
        if len_sq == 0 {
            return 1;
        }
        let ceil_div = |n: i128, m: i128| (n + m - 1) / m;
        let rounding = ceil_div(dk, 1 << 33) + ceil_div(dk << 35, len_sq);
        u64::try_from(1 + dk.min(rounding)).expect("bound fits u64")
    })
}

/// The documented bounds by arrangement: exact for a point, the segment bound for the
/// point-to-segment answer, and for parallel segments the larger of the two segment bounds on
/// both points. Non-parallel segments of nonzero length have no documented bound: `None`.
fn documented_bounds(problem: &Problem) -> Option<Bounds> {
    match (problem.a == 0, problem.e == 0) {
        (true, true) => Some(([0; 3], [0; 3])),
        (true, false) => Some(([0; 3], segment_bound(problem.d2))),
        (false, true) => Some((segment_bound(problem.d1), [0; 3])),
        (false, false) => cross_is_zero(problem.d1, problem.d2).then(|| {
            let (b1, b2) = (segment_bound(problem.d1), segment_bound(problem.d2));
            let both = [0, 1, 2].map(|k| b1[k].max(b2[k]));
            (both, both)
        }),
    }
}

/// The rounded `a e - b^2` the kernel tests for zero, for a pair of segments.
fn rounded_denominator(s: Segments) -> i64 {
    let (d1, d2) = (v(s[1]) - v(s[0]), v(s[3]) - v(s[2]));
    let (a, b, e) = (d1.dot(d1), d1.dot(d2), d2.dot(d2));
    (a * e - b * b).raw()
}

fn assert_documented(s: Segments, context: &str) -> (Raw3, Raw3) {
    let got = kernel(s);
    let verdict = judge(s, got, |p| documented_bounds(p).expect("a documented arrangement"));
    assert!(verdict.ok, "{context}: segments {s:?}: got {got:?}, error {} raw", verdict.error);
    got
}

// ---------------------------------------------------------------------------------------
// Seeded generators.
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

    fn index(&mut self, len: u64) -> usize {
        usize::try_from(self.next() % len).expect("small index")
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

/// A nonzero direction: large, raw-unit-sized, whole units, or along one or two axes.
fn direction(seeds: &mut Seeds, scale: usize) -> Raw3 {
    loop {
        let d = match scale {
            0 => seeds.triple(1 << 34),
            1 => seeds.triple(4),
            2 => seeds.triple(3).map(|c| c * ONE),
            _ => {
                let mut d = seeds.triple(1 << 34);
                d[seeds.index(3)] = 0;
                if seeds.below(2) == 0 {
                    d[seeds.index(3)] = 0;
                }
                d
            }
        };
        if d != [0; 3] {
            return d;
        }
    }
}

fn along(base: Raw3, dir: Raw3, s: i64) -> Raw3 {
    [0, 1, 2].map(|k| base[k] + dir[k] * s)
}

fn plus(a: Raw3, b: Raw3) -> Raw3 {
    [0, 1, 2].map(|k| a[k] + b[k])
}

/// An offset of the size of the direction scale.
fn offset(seeds: &mut Seeds, scale: usize) -> Raw3 {
    match scale {
        1 => seeds.triple(64),
        2 => seeds.triple(4).map(|c| c * ONE),
        _ => seeds.triple(1 << 36),
    }
}

/// The arrangements of the extent sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arrangement {
    SecondZero,
    FirstZero,
    BothZero,
    Parallel,
    CollinearOverlapping,
    CollinearDisjoint,
    Intersecting,
    SharedEndpoint,
    Skew,
}

const ARRANGEMENTS: [Arrangement; 9] = [
    Arrangement::SecondZero,
    Arrangement::FirstZero,
    Arrangement::BothZero,
    Arrangement::Parallel,
    Arrangement::CollinearOverlapping,
    Arrangement::CollinearDisjoint,
    Arrangement::Intersecting,
    Arrangement::SharedEndpoint,
    Arrangement::Skew,
];

impl Arrangement {
    /// The exact shape every case of the arrangement must have.
    const fn shape(self) -> Shape {
        match self {
            Self::SecondZero => Shape::SecondPoint,
            Self::FirstZero => Shape::FirstPoint,
            Self::BothZero => Shape::BothPoints,
            Self::Parallel => Shape::Parallel,
            Self::CollinearOverlapping | Self::CollinearDisjoint => Shape::Collinear,
            Self::Intersecting | Self::SharedEndpoint | Self::Skew => Shape::General,
        }
    }
}

/// One case of `arrangement`, or `None` when the draw has another exact shape (counted and
/// skipped). Every coordinate stays below `2^40` raw.
fn arrangement_case(seeds: &mut Seeds, arrangement: Arrangement, scale: usize) -> Option<Segments> {
    let base = seeds.triple(1 << 38);
    let u = direction(seeds, scale);
    let segment = |seeds: &mut Seeds| (base, along(base, u, 1 + seeds.below(6)));
    let near = |seeds: &mut Seeds| plus(along(base, u, seeds.signed(7) + 3), offset(seeds, scale));
    let case = match arrangement {
        Arrangement::SecondZero => {
            let (p, q) = segment(seeds);
            let point = near(seeds);
            [p, q, point, point]
        }
        Arrangement::FirstZero => {
            let (p, q) = segment(seeds);
            let point = near(seeds);
            [point, point, p, q]
        }
        Arrangement::BothZero => {
            let (a, b) = (near(seeds), near(seeds));
            [a, a, b, b]
        }
        Arrangement::Parallel => {
            let (p, q) = segment(seeds);
            let start = near(seeds);
            let k = (1 + seeds.below(4)) * if seeds.below(2) == 0 { 1 } else { -1 };
            [p, q, start, along(start, u, k)]
        }
        Arrangement::CollinearOverlapping | Arrangement::CollinearDisjoint => {
            let len = 2 + seeds.below(5);
            let start = if arrangement == Arrangement::CollinearOverlapping {
                seeds.below(u64::try_from(len + 1).expect("positive"))
            } else {
                len + 1 + seeds.below(3)
            };
            let k = (1 + seeds.below(4)) * if seeds.below(2) == 0 { 1 } else { -1 };
            let flip = seeds.below(2) == 0;
            let (p2, q2) = (along(base, u, start), along(base, u, start + k));
            let second = if flip { [q2, p2] } else { [p2, q2] };
            let (p1, q1) = (base, along(base, u, len));
            // A disjoint draw whose second segment reaches back into the first is skipped.
            [p1, q1, second[0], second[1]]
        }
        Arrangement::Intersecting | Arrangement::SharedEndpoint => {
            let w = direction(seeds, scale);
            let (a, b) = (1 + seeds.below(4), 1 + seeds.below(4));
            let (c, d) = (1 + seeds.below(4), 1 + seeds.below(4));
            if arrangement == Arrangement::Intersecting {
                [along(base, u, -a), along(base, u, b), along(base, w, -c), along(base, w, d)]
            } else {
                let (x, y) = (along(base, u, a), along(base, w, c));
                match seeds.below(4) {
                    0 => [base, x, base, y],
                    1 => [x, base, base, y],
                    2 => [base, x, y, base],
                    _ => [x, base, y, base],
                }
            }
        }
        Arrangement::Skew => {
            let w = direction(seeds, scale);
            let (p2, q2) = (near(seeds), near(seeds));
            let q2 = plus(q2, w);
            let (p, q) = segment(seeds);
            [p, q, p2, q2]
        }
    };
    let collinear_overlap_ok = arrangement != Arrangement::CollinearDisjoint || {
        // Disjoint: the second segment's parameters along `u` are both beyond the first's.
        let param = |x: Raw3| {
            let d = diff(x, base);
            dot(d, u.map(i128::from))
        };
        let end = param(case[1]);
        param(case[2]) > end && param(case[3]) > end
    };
    (shape(case) == arrangement.shape() && collinear_overlap_ok).then_some(case)
}

// ---------------------------------------------------------------------------------------
// Constructed cases.
// ---------------------------------------------------------------------------------------

/// CLOSEST-TP-020: the example of flyingrobots/bunny#506.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_020_the_issue_example_returns_the_closest_point_of_the_first_segment() {
    let s = [units(0, 0, 0), units(2, 0, 0), units(1, 1, 0), units(1, 1, 0)];
    assert_eq!(reference(s), ([0, 0, 0], units(1, 1, 0)), "the old routine returned p1");
    assert_eq!(kernel(s), (units(1, 0, 0), units(1, 1, 0)));
    // With the point first, the old routine was already right.
    let swapped = [units(1, 1, 0), units(1, 1, 0), units(0, 0, 0), units(2, 0, 0)];
    assert_eq!(reference(swapped), (units(1, 1, 0), units(1, 0, 0)));
    assert_eq!(kernel(swapped), (units(1, 1, 0), units(1, 0, 0)));
    assert_documented(s, "issue example");
}

/// Points whose closest point on `(0,0,0)-(2,0,0)` is a representable raw value, with it:
/// inside, on each end, beyond each end, and one raw unit beyond each end.
fn point_cases() -> [(Raw3, Raw3); 9] {
    let (a, b) = (units(0, 0, 0), units(2, 0, 0));
    [
        (units(1, 1, 0), units(1, 0, 0)),
        ([ONE / 2, -3 * ONE, 2 * ONE], [ONE / 2, 0, 0]),
        ([3 * ONE / 2, 0, -5 * ONE], [3 * ONE / 2, 0, 0]),
        (units(0, 5, 0), a),
        (units(2, -5, 1), b),
        (units(-1, 1, 0), a),
        (units(3, 1, 0), b),
        ([-1, ONE, 0], a),
        ([2 * ONE + 1, ONE, 0], b),
    ]
}

/// CLOSEST-TP-021: a zero-length second segment, for either direction of the first, gives
/// the closest point of the first segment and the point itself; one raw unit inside each
/// end stays on the segment within the bound. The answers equal the mirrored call with the
/// point first.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_021_a_zero_length_second_segment_gives_the_closest_point_of_the_first() {
    let (a, b) = (units(0, 0, 0), units(2, 0, 0));
    for (p, q) in [(a, b), (b, a)] {
        for (point, expected) in point_cases() {
            let s = [p, q, point, point];
            let context = format!("segment {p:?}-{q:?}, point {point:?}");
            assert_eq!(assert_documented(s, &context), (expected, point), "{context}");
            let (on_point, on_segment) = kernel([point, point, p, q]);
            assert_eq!((on_segment, on_point), (expected, point), "mirror, {context}");
        }
        for point in [[1, ONE, 0], [2 * ONE - 1, ONE, 0]] {
            let got = assert_documented([p, q, point, point], "one raw unit inside an end");
            assert!((0..=2 * ONE).contains(&got.0[0]) && got.0[1] == 0 && got.0[2] == 0);
        }
    }
}

/// CLOSEST-TP-022: a zero-length first segment, unchanged by the fix, gives the point and
/// its closest point on the second segment; two zero-length segments give both points.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_022_a_zero_length_first_segment_and_two_points() {
    let (a, b) = (units(0, 0, 0), units(2, 0, 0));
    for (p, q) in [(a, b), (b, a)] {
        for (point, expected) in point_cases() {
            let s = [point, point, p, q];
            assert_eq!(assert_documented(s, "first point"), (point, expected));
            assert_eq!(kernel(s), reference(s), "unchanged: {s:?}");
        }
    }
    for (x, y) in [
        (units(1, 1, 1), units(5, 0, 0)),
        ([i64::MAX; 3], [i64::MIN; 3]),
        ([0; 3], [0; 3]),
        ([1, -1, 1], [i64::MIN, 0, i64::MAX]),
    ] {
        assert_eq!(kernel([x, x, y, y]), (x, y), "two points {x:?}, {y:?}");
        assert_eq!(kernel([x, x, y, y]), reference([x, x, y, y]));
    }
}

/// CLOSEST-TP-023: segments one raw unit long against a zero-length segment, on each axis,
/// stay within the bound: here the bound is the segment's own extent plus one raw unit.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_023_one_raw_unit_segments_against_a_point() {
    let start = units(3, -2, 1);
    for axis in 0..3 {
        let mut end = start;
        end[axis] += 1;
        for point in [units(0, 0, 0), units(3, -2, 1), plus(start, [5, 5, 5]), [-7, 9, ONE]] {
            assert_documented([start, end, point, point], "second point, one raw unit");
            assert_documented([point, point, start, end], "first point, one raw unit");
        }
    }
}

/// CLOSEST-TP-027: a zero-length segment at the ends of the raw range. The segment
/// routine's intermediates saturate here (flyingrobots/bunny#116, decision D5), so the
/// answer is pinned, not endorsed; it is the mirror of the call with the point first.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_027_the_ends_of_the_raw_range_are_pinned() {
    let (lo, hi) = ([i64::MIN, 0, 0], [i64::MAX, 0, 0]);
    // The edge `MAX - MIN` and the offset `p - MIN` saturate to `MAX`, both dot products
    // saturate, the parameter is `1.0` and the answer is `MIN + MAX = -1` raw. The exact
    // closest point is the origin.
    let point = [0, ONE, 0];
    assert_eq!(kernel([lo, hi, point, point]), ([-1, 0, 0], point));
    assert_eq!(kernel([point, point, lo, hi]), (point, [-1, 0, 0]));
    // A short segment at the corner of the range: nothing saturates and the answer is exact.
    let corner = [i64::MAX - 4 * ONE, i64::MAX, i64::MAX];
    let end = [i64::MAX; 3];
    let p = [i64::MAX - 2 * ONE, i64::MAX - ONE, i64::MAX];
    assert_eq!(kernel([corner, end, p, p]), ([i64::MAX - 2 * ONE, i64::MAX, i64::MAX], p));
    assert_eq!(kernel([p, p, corner, end]), (p, [i64::MAX - 2 * ONE, i64::MAX, i64::MAX]));
}

// ---------------------------------------------------------------------------------------
// The extent sweep.
// ---------------------------------------------------------------------------------------

/// CLOSEST-TP-025: seeded cases of every arrangement against the exact oracle. The kernel is
/// within the documented bound on every arrangement that has one. An answer differs from the
/// previous routine only for a zero-length second segment, or for parallel segments whose
/// rounded `a e - b^2` is not zero. The counts are printed.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_025_seeded_arrangements_against_the_exact_oracle() {
    let mut seeds = Seeds(SWEEP_SEED);
    let mut worst_documented = 0;
    for arrangement in ARRANGEMENTS {
        let (mut skipped, mut kernel_worst, mut reference_worst) = (0, 0, 0);
        let (mut reference_wrong, mut changed, mut changed_wrong) = (0, 0, 0);
        for scale in 0..4 {
            let mut drawn = 0;
            while drawn < SWEEP_CASES_PER_SCALE {
                let Some(s) = arrangement_case(&mut seeds, arrangement, scale) else {
                    skipped += 1;
                    continue;
                };
                let context = format!("seed {SWEEP_SEED:#x}, {arrangement:?}, scale {scale}");
                let (got, previous) = (kernel(s), reference(s));
                let documented = documented_bounds(&Problem::new(s)).is_some();
                let loose = ([u64::MAX / 4; 3], [u64::MAX / 4; 3]);
                let k = judge(s, got, |p| documented_bounds(p).unwrap_or(loose));
                let r = judge(s, previous, |p| documented_bounds(p).unwrap_or(loose));
                assert!(
                    !documented || k.ok,
                    "{context}, case {drawn}: {s:?} gave {got:?}, error {} raw",
                    k.error
                );
                if got != previous {
                    let parallel_rerouted = matches!(shape(s), Shape::Parallel | Shape::Collinear)
                        && rounded_denominator(s) != 0;
                    assert!(
                        shape(s) == Shape::SecondPoint || parallel_rerouted,
                        "{context}, case {drawn}: {s:?} changed from {previous:?} to {got:?}"
                    );
                    changed += 1;
                    changed_wrong += usize::from(documented && !r.ok);
                }
                reference_wrong += usize::from(documented && !r.ok);
                kernel_worst = kernel_worst.max(k.error);
                reference_worst = reference_worst.max(r.error);
                if documented {
                    worst_documented = worst_documented.max(k.error);
                }
                drawn += 1;
            }
        }
        println!(
            "closest_tp_025: seed {SWEEP_SEED:#x}, {arrangement:?}: {} cases ({skipped} draws \
             of another shape skipped); previous routine outside the documented bound on \
             {reference_wrong}, worst error {reference_worst} raw; answer changed on {changed}, \
             of which {changed_wrong} were outside the bound; worst error now {kernel_worst} raw",
            4 * SWEEP_CASES_PER_SCALE
        );
        match arrangement {
            Arrangement::SecondZero => {
                assert!(reference_wrong > 1_000, "{arrangement:?}: {reference_wrong}");
            }
            Arrangement::Parallel | Arrangement::CollinearOverlapping => {
                assert!(reference_wrong > 10, "{arrangement:?}: {reference_wrong}");
            }
            Arrangement::FirstZero | Arrangement::BothZero => {
                assert_eq!((reference_wrong, changed), (0, 0), "{arrangement:?}");
            }
            Arrangement::Intersecting | Arrangement::SharedEndpoint | Arrangement::Skew => {
                assert_eq!(changed, 0, "{arrangement:?}");
            }
            Arrangement::CollinearDisjoint => {}
        }
    }
    println!("closest_tp_025: largest error on a documented arrangement {worst_documented} raw");
}

/// CLOSEST-TP-026: parallel and collinear segments. The choice among closest pairs is the
/// one the previous routine made when its rounded `a e - b^2` was zero: `p1` and its
/// projection when `p1` projects onto the second segment, otherwise the nearer end of the
/// second segment and its projection onto the first. A parallel pair whose rounded
/// `a e - b^2` is not zero used to take the non-parallel formulas and could return a pair
/// that is not closest; it now takes the same parallel rule.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_026_parallel_segments_take_the_parallel_rule() {
    let seg = |a: Raw3, b: Raw3, c: Raw3, d: Raw3| [a, b, c, d];
    let x = |n: i64| units(n, 0, 0);
    let cases = [
        // The golden of `closest_point_raw_q32_32_outputs`: unchanged.
        (seg(x(0), x(2), units(0, 2, 0), units(2, 2, 0)), (x(0), units(0, 2, 0))),
        // Collinear, the second inside the first, both directions.
        (seg(x(0), x(4), x(1), x(3)), (x(1), x(1))),
        (seg(x(0), x(4), x(3), x(1)), (x(1), x(1))),
        // Collinear, the second containing the first: p1 and its projection, which is `p1`
        // up to the rounding of `t = 1 / 6` (two raw units, within the bound).
        (seg(x(0), x(4), x(-1), x(5)), (x(0), [2, 0, 0])),
        // Collinear and disjoint, both directions of the second.
        (seg(x(0), x(2), x(5), x(7)), (x(2), x(5))),
        (seg(x(0), x(2), x(7), x(5)), (x(2), x(5))),
        // Parallel and offset, partly overlapping.
        (seg(x(0), x(4), units(2, 1, 0), units(6, 1, 0)), (x(2), units(2, 1, 0))),
    ];
    for (s, expected) in cases {
        assert_eq!(rounded_denominator(s), 0, "{s:?}");
        assert_eq!(kernel(s), expected, "{s:?}");
        assert_eq!(kernel(s), reference(s), "unchanged: {s:?}");
        assert_documented(s, "parallel, rounded denominator zero");
    }
    // An exactly parallel pair found by the sweep: the rounded `a e - b^2` is 10 raw, the old
    // answer is about 0.46 units from every closest pair, and the new one is within the bound.
    let s = [
        [-237_815_078_086, 116_379_279_042, -166_771_290_285],
        [-237_815_078_086, 93_027_424_914, -166_771_290_285],
        [-270_306_501_171, 122_193_399_589, -218_072_046_061],
        [-270_306_501_171, 91_057_594_085, -218_072_046_061],
    ];
    assert_eq!(shape(s), Shape::Parallel);
    assert_ne!(rounded_denominator(s), 0);
    let before = judge(s, reference(s), |p| documented_bounds(p).expect("parallel"));
    assert!(!before.ok && before.error > 1 << 30, "old error {} raw", before.error);
    assert_documented(s, "parallel, rounded denominator nonzero");
}

// ---------------------------------------------------------------------------------------
// The differential test.
// ---------------------------------------------------------------------------------------

/// A pair of segments from family `family`, of any shape.
fn general_case(seeds: &mut Seeds, family: usize) -> Segments {
    match family {
        // Every coordinate below 2^40 raw.
        0 => [0; 4].map(|_: i32| seeds.triple(1 << 40)),
        // The full raw range, where the routine saturates.
        1 => [0; 4].map(|_: i32| [seeds.full(), seeds.full(), seeds.full()]),
        // Whole units on a small grid: many exact crossings, shared ends and parallels.
        2 => [0; 4].map(|_: i32| seeds.triple(4).map(|c| c * ONE)),
        // Short segments: one end a few raw units from the other.
        3 => {
            let (p1, p2) = (seeds.triple(1 << 40), seeds.triple(1 << 40));
            let (q1, q2) = (plus(p1, seeds.triple(2)), plus(p2, seeds.triple(1 << 20)));
            if seeds.below(2) == 0 {
                [p1, q1, p2, q2]
            } else {
                [p2, q2, p1, q1]
            }
        }
        // One raw unit off an arrangement: near-parallel, near-collinear, near-crossing.
        _ => {
            let arrangement = ARRANGEMENTS[seeds.index(9)];
            let scale = seeds.index(4);
            loop {
                if let Some(mut s) = arrangement_case(seeds, arrangement, scale) {
                    let (end, axis) = (seeds.index(4), seeds.index(3));
                    s[end][axis] += if seeds.below(2) == 0 { 1 } else { -1 };
                    return s;
                }
            }
        }
    }
}

/// CLOSEST-TP-024: 100 000 seeded pairs of non-degenerate, non-parallel segments, five
/// families of 20 000, give exactly the previous routine's answer.
#[wasm_bindgen_test(unsupported = test)]
fn closest_tp_024_non_degenerate_non_parallel_pairs_are_bit_identical() {
    let mut seeds = Seeds(DIFFERENTIAL_SEED);
    let mut compared = 0;
    for family in 0..5 {
        let (mut family_compared, mut skipped) = (0, 0);
        while family_compared < DIFFERENTIAL_CASES_PER_FAMILY {
            let s = general_case(&mut seeds, family);
            if shape(s) != Shape::General {
                skipped += 1;
                continue;
            }
            assert_eq!(
                kernel(s),
                reference(s),
                "seed {DIFFERENTIAL_SEED:#x}, family {family}, case {family_compared}: {s:?}"
            );
            family_compared += 1;
        }
        println!(
            "closest_tp_024: seed {DIFFERENTIAL_SEED:#x}, family {family}: {family_compared} \
             compared, {skipped} degenerate or parallel skipped"
        );
        compared += family_compared;
    }
    assert_eq!(compared, 100_000);
}

// ---------------------------------------------------------------------------------------
// The previous routine, frozen: `closest_points_segments` as it was before
// flyingrobots/bunny#506, with its helpers inlined, used only as the bit-identity reference.
// ---------------------------------------------------------------------------------------

#[allow(clippy::many_single_char_names, clippy::similar_names)]
fn reference_closest_points_segments(
    p1: FixedVec3,
    q1: FixedVec3,
    p2: FixedVec3,
    q2: FixedVec3,
) -> (FixedVec3, FixedVec3) {
    let zero = FixedQ32_32::ZERO;
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let b = d1.dot(d2);
    let e = d2.dot(d2);
    let c = d1.dot(r);
    let f = d2.dot(r);
    let denom = a * e - b * b;

    let (mut s_num, mut s_denom, mut t_num, mut t_denom);
    if denom == zero {
        (s_num, s_denom, t_num, t_denom) = (zero, FixedQ32_32::ONE, f, e);
    } else {
        (s_num, s_denom, t_num, t_denom) = (b * f - e * c, denom, a * f - b * c, denom);
        if s_num < zero {
            (s_num, t_num, t_denom) = (zero, f, e);
        } else if s_num > s_denom {
            (s_num, t_num, t_denom) = (s_denom, f + b, e);
        }
    }
    if t_num < zero {
        t_num = zero;
        (s_num, s_denom) = (reference_clamp(-c, zero, a), a);
    }
    if t_num > t_denom {
        t_num = t_denom;
        (s_num, s_denom) = (reference_clamp(-c + b, zero, a), a);
    }
    let unit = |n, d| reference_clamp(reference_ratio(n, d), zero, FixedQ32_32::ONE);
    let s = unit(s_num, s_denom);
    let t = unit(t_num, t_denom);
    (p1 + d1 * s, p2 + d2 * t)
}

fn reference_ratio(numer: FixedQ32_32, denom: FixedQ32_32) -> FixedQ32_32 {
    if denom == FixedQ32_32::ZERO {
        FixedQ32_32::ZERO
    } else {
        numer.checked_div(denom).unwrap_or(FixedQ32_32::ZERO)
    }
}

fn reference_clamp(val: FixedQ32_32, min: FixedQ32_32, max: FixedQ32_32) -> FixedQ32_32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}
