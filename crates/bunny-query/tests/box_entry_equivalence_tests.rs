//! Equivalence of `TraceRay::box_entry` with the retained pre-#258 implementation.
//!
//! The box test was rewritten for speed (flyingrobots/bunny#258) as an exact replacement: for
//! every ray, box and distance limit it must return the same `Option<FixedQ32_32>` as the
//! reference in `support/box_entry_reference.rs`. These cases compare the two on the golden and
//! oracle corpora of `trace_tests.rs`, on seeded inputs across the trace domain, and on box
//! coordinates at the ends of the `i64` range. The benchmark scene's real box tests are
//! compared in `crates/bunny-perf/src/profile.rs`.

#[path = "support/box_entry_reference.rs"]
mod box_entry_reference;

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

/// The seed of `trace_tests.rs::trace_tp_007`, so its corpus is replayed draw for draw.
const ORACLE_SEED: u64 = 0xB002_7ACE_0000_0001 ^ 7;
const SEEDED_SEED: u64 = 0xB002_0258_0000_0001;
const EXTREME_SEED: u64 = 0xB002_0258_0000_0002;
const SEEDED_CASES: usize = 300_000;
const EXTREME_CASES: usize = 200_000;
/// Largest origin coordinate magnitude the trace domain accepts.
const ORIGIN_MAX: i64 = TRACE_COORD_LIMIT_RAW - 1;
const DIRECTION_MAX: i64 = TRACE_DIRECTION_LIMIT_RAW;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A full-width 64-bit value (two draws).
    fn wide(&mut self) -> u64 {
        (self.next() << 32) ^ self.next()
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * i128::from(bound) + 1).unwrap_or(u64::MAX);
        let offset = i128::from(self.wide() % span);
        i64::try_from(offset - i128::from(bound)).expect("inside the bound")
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }

    fn pick<T: Copy>(&mut self, values: &[T]) -> T {
        values[usize::try_from(self.next()).expect("fits usize") % values.len()]
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(vec3(min), vec3(max))
}

/// Compares the two implementations and counts what the reference returned at `i64::MAX`,
/// so each corpus can assert that it exercised both outcomes.
struct Checker {
    /// Draws the seeded distance limits; independent of the corpus generators.
    limits: Seeds,
    some: usize,
    none: usize,
}

impl Checker {
    fn new(seed: u64) -> Self {
        Self { limits: Seeds(seed), some: 0, none: 0 }
    }

    /// Compares one ray and box over every class of distance limit: negative, zero, small,
    /// `i64::MAX`, seeded values, and the reference's own entry bound and its two
    /// neighbours, where the limit decides the answer.
    fn assert_same(&mut self, ray: &TraceRay, aabb: &FixedAabb3, case: usize) {
        let far = box_entry_reference::box_entry(ray, aabb, FixedQ32_32::from_raw(i64::MAX));
        let entry = far.map_or(0, FixedQ32_32::raw);
        let random = i64::try_from(self.limits.wide() >> 1).expect("63 bits");
        let limits = [
            i64::MIN,
            -1,
            0,
            1,
            i64::MAX,
            random,
            random >> 32,
            entry.saturating_sub(1),
            entry,
            entry.saturating_add(1),
        ];
        for limit in limits.map(FixedQ32_32::from_raw) {
            let expected = box_entry_reference::box_entry(ray, aabb, limit);
            let actual = ray.box_entry(aabb, limit);
            assert_eq!(actual, expected, "case={case} ray={ray:?} box={aabb:?} t_max={}", limit.raw());
        }
        if far.is_some() {
            self.some += 1;
        } else {
            self.none += 1;
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_301_box_entry_matches_the_reference_on_the_golden_and_oracle_corpora() {
    let one = ONE_RAW;
    let unit = ([-one, -one, -one], [one, one, one]);
    let n = 1 << 20;
    let grazed = ([n, -3 * n, -1], [2 * n, 0, 1]);
    // The inputs of `trace_tp_010`, whose raw outputs are locked there.
    let goldens = [
        ([0, 0, 5 * one], [0, 0, -one], unit),
        ([0, 0, 2 * one], [0, 0, -3], unit),
        ([-3 * one, 0, 0], [one, one / 2, 0], unit),
        ([-3 * one, 0, 0], [one, one, 0], unit),
        ([0, 2 * one, 0], [one, 0, 0], unit),
        ([0, one, 0], [one, 0, 0], unit),
        ([0, 0, 0], [1, -3, 0], grazed),
        ([0, 0, 0], [1, 0, 0], unit),
    ];
    let mut checker = Checker::new(SEEDED_SEED ^ 0xA);
    for (case, (origin, direction, (min, max))) in goldens.into_iter().enumerate() {
        let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("golden ray is in the domain");
        checker.assert_same(&ray, &aabb(min, max), case);
    }
    // The corpus of `trace_tp_007`, drawn with the same seed in the same order.
    let mut seeds = Seeds(ORACLE_SEED);
    for case in 0..40_000 {
        let origin = seeds.triple_small(4 * one);
        let mut direction = seeds.triple_small(DIRECTION_MAX);
        for component in &mut direction {
            match seeds.next() % 12 {
                0 => *component = 0,
                1 => *component = seeds.signed_small(3),
                _ => {}
            }
        }
        let (a, b) = (seeds.triple_small(4 * one), seeds.triple_small(4 * one));
        let min = [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])];
        let max = [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])];
        let Some(ray) = TraceRay::new(vec3(origin), vec3(direction)) else { continue };
        checker.assert_same(&ray, &aabb(min, max), case);
    }
    assert!(checker.some > 2000 && checker.none > 2000, "both outcomes: {} {}", checker.some, checker.none);
}

impl Seeds {
    /// `trace_tests.rs`'s `signed`: one draw, for replaying its corpus exactly.
    fn signed_small(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn triple_small(&mut self, bound: i64) -> [i64; 3] {
        [self.signed_small(bound), self.signed_small(bound), self.signed_small(bound)]
    }
}

/// A direction component from one of the classes the box test treats differently.
fn component(seeds: &mut Seeds) -> i64 {
    match seeds.next() % 8 {
        0 => 0,
        1 => seeds.signed(3),
        2 => seeds.pick(&[1, -1, DIRECTION_MAX, -DIRECTION_MAX, DIRECTION_MAX - 1, 3, -3]),
        3 => seeds.signed(1 << 12),
        _ => seeds.signed(DIRECTION_MAX),
    }
}

/// A direction from one of: general, near-axis (one large component, the others tiny or
/// zero) and axis-parallel (exactly one nonzero component).
fn direction(seeds: &mut Seeds) -> [i64; 3] {
    let mut direction = [component(seeds), component(seeds), component(seeds)];
    let axis = usize::try_from(seeds.next() % 3).expect("below 3");
    match seeds.next() % 4 {
        0 => {
            direction = direction.map(|_| seeds.signed(2));
            direction[axis] = seeds.pick(&[DIRECTION_MAX, -DIRECTION_MAX, ONE_RAW / 2, -ONE_RAW / 3]);
        }
        1 => {
            direction = [0; 3];
            direction[axis] = seeds.signed(DIRECTION_MAX);
        }
        _ => {}
    }
    direction
}

fn origin(seeds: &mut Seeds) -> [i64; 3] {
    match seeds.next() % 3 {
        0 => seeds.triple(ORIGIN_MAX),
        1 => seeds.triple(4 * ONE_RAW),
        _ => seeds.triple(1 << 20),
    }
}

/// A box placed around, ahead of, behind or far from the origin, sometimes touching it with
/// a face and sometimes inverted on one axis.
fn placed_box(seeds: &mut Seeds, origin: [i64; 3], direction: [i64; 3]) -> ([i64; 3], [i64; 3]) {
    let scale = seeds.pick(&[1 << 8, 1 << 24, ONE_RAW, 8 * ONE_RAW, 1 << 40, 1 << 52, 1 << 61]);
    let step = i64::try_from(seeds.next() % 64).expect("small") - 16;
    let mut min = [0; 3];
    let mut max = [0; 3];
    for axis in 0..3 {
        // Ahead of the origin along the direction for positive steps, behind for negative.
        let along = i128::from(direction[axis]) * i128::from(step) * i128::from(scale >> 32).max(1);
        let centre = i128::from(origin[axis]) + along + i128::from(seeds.signed(scale));
        let half = i128::from(seeds.signed(scale)).abs();
        let clamp = |v: i128| i64::try_from(v.clamp(i128::from(i64::MIN), i128::from(i64::MAX)));
        min[axis] = clamp(centre - half).expect("clamped");
        max[axis] = clamp(centre + half).expect("clamped");
        match seeds.next() % 16 {
            0 => min[axis] = origin[axis],
            1 => max[axis] = origin[axis],
            2 => std::mem::swap(&mut min[axis], &mut max[axis]),
            3 => (min[axis], max[axis]) = (i64::MIN, i64::MAX),
            _ => {}
        }
    }
    (min, max)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_302_box_entry_matches_the_reference_on_seeded_trace_domain_cases() {
    let mut seeds = Seeds(SEEDED_SEED);
    let mut checker = Checker::new(SEEDED_SEED ^ 0xB);
        for case in 0..SEEDED_CASES {
        let origin = origin(&mut seeds);
        let direction = direction(&mut seeds);
        let Some(ray) = TraceRay::new(vec3(origin), vec3(direction)) else { continue };
        let (min, max) = placed_box(&mut seeds, origin, direction);
        checker.assert_same(&ray, &aabb(min, max), case);
    }
    let compared = checker.some + checker.none;
    assert!(compared > SEEDED_CASES * 9 / 10, "most cases form a valid ray: {compared}");
    assert!(checker.some > SEEDED_CASES / 10, "reached boxes: {}", checker.some);
    assert!(checker.none > SEEDED_CASES / 10, "rejected boxes: {}", checker.none);
}

/// Box coordinates at and near the ends of `i64` and of the trace domain.
const EXTREMES: [i64; 15] = [
    i64::MIN,
    i64::MIN + 1,
    i64::MIN + ORIGIN_MAX,
    -(1 << 62),
    -ORIGIN_MAX - 1,
    -ORIGIN_MAX,
    -1,
    0,
    1,
    ORIGIN_MAX,
    ORIGIN_MAX + 1,
    1 << 62,
    i64::MAX - ORIGIN_MAX,
    i64::MAX - 1,
    i64::MAX,
];

const EXTREME_ORIGINS: [i64; 7] = [-ORIGIN_MAX, -ORIGIN_MAX + 1, -1, 0, 1, ORIGIN_MAX - 1, ORIGIN_MAX];
const EXTREME_DIRECTIONS: [i64; 9] =
    [-DIRECTION_MAX, -DIRECTION_MAX + 1, -3, -1, 0, 1, 3, DIRECTION_MAX - 1, DIRECTION_MAX];

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_303_box_entry_matches_the_reference_at_the_ends_of_the_i64_range() {
    let mut seeds = Seeds(EXTREME_SEED);
    let mut checker = Checker::new(EXTREME_SEED ^ 0xB);
        // Every (min, max) pair of extremes on one axis, inverted pairs included, for every
    // extreme origin coordinate and direction component on that axis; the other two axes
    // hold the origin with a unit direction so that this axis decides.
    let mut case = 0;
    for axis in 0..3 {
        for (&lo, &hi) in EXTREMES.iter().flat_map(|lo| EXTREMES.iter().map(move |hi| (lo, hi))) {
            for &o in &EXTREME_ORIGINS {
                for &d in &EXTREME_DIRECTIONS {
                    let (mut origin, mut direction) = ([0; 3], [ONE_RAW; 3]);
                    let (mut min, mut max) = ([i64::MIN; 3], [i64::MAX; 3]);
                    (origin[axis], direction[axis], min[axis], max[axis]) = (o, d, lo, hi);
                    let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("in the domain");
                    checker.assert_same(&ray, &aabb(min, max), case);
                    case += 1;
                }
            }
        }
    }
    // Seeded boxes whose every coordinate is an extreme or within a few units of one.
    for case in 0..EXTREME_CASES {
        let coordinate = |seeds: &mut Seeds| {
            let base = seeds.pick(&EXTREMES);
            let nudge = seeds.signed(3) * seeds.pick(&[1, ONE_RAW]);
            base.saturating_add(nudge)
        };
        let (min, max): ([i64; 3], [i64; 3]) = (
            [coordinate(&mut seeds), coordinate(&mut seeds), coordinate(&mut seeds)],
            [coordinate(&mut seeds), coordinate(&mut seeds), coordinate(&mut seeds)],
        );
        let origin = [0; 3].map(|_| seeds.pick(&EXTREME_ORIGINS));
        let direction = [0; 3].map(|_| seeds.pick(&EXTREME_DIRECTIONS));
        let Some(ray) = TraceRay::new(vec3(origin), vec3(direction)) else { continue };
        checker.assert_same(&ray, &aabb(min, max), case);
    }
    assert!(checker.some > 10_000 && checker.none > 10_000, "both outcomes: {} {}", checker.some, checker.none);
}
