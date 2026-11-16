//! `TraceRay::hit_box` (contract `bunny-ray-box/1`) against an independent exact oracle, and
//! named cases for grazing rays, origins inside the box, boxes behind the origin, edge and
//! corner hits, rays parallel to a slab and refused inputs (TRACE-TP-509 to TRACE-TP-517).
//!
//! The oracle shares no arithmetic with the kernel. It keeps each crossing as a reduced
//! fraction of its own ([`Rat`]), takes the entry as the largest and the exit as the smallest
//! of the slab crossings, and finds the face by asking which faces the exact hit point lies
//! on (the kernel keeps the axis that produced the crossing). The generated rays use a fixed
//! seed; every failure message names it and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceBox, TraceBoxError, TraceRay, TRACE_COORD_LIMIT_RAW};
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const BOX_ORACLE_SEED: u64 = 0x5EED_B0C5_0359_0001;
const BOX_ORACLE_CASES: usize = 8_000;
const ONE: i64 = 1 << 32;
const DOMAIN: i64 = TRACE_COORD_LIMIT_RAW - 1;

type Raw3 = [i64; 3];

/// A fraction `num / den` with a positive denominator.
#[derive(Clone, Copy, Debug)]
struct Rat {
    num: i128,
    den: i128,
}

impl Rat {
    fn new(num: i128, den: i128) -> Self {
        if den < 0 {
            Self { num: -num, den: -den }
        } else {
            Self { num, den }
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }

    fn floor(&self) -> i128 {
        self.num.div_euclid(self.den)
    }
}

/// What the oracle expects: raw distance, front face and raw normal.
type Expected = Option<(i64, bool, Raw3)>;

/// One ray, one box and one distance range, all raw.
#[derive(Clone, Copy, Debug)]
struct Case {
    origin: Raw3,
    direction: Raw3,
    min: Raw3,
    max: Raw3,
    range: (i64, i64),
}

/// A case over distances `0..=i64::MAX` raw.
fn case(origin: Raw3, direction: Raw3, min: Raw3, max: Raw3) -> Case {
    Case { origin, direction, min, max, range: full() }
}

/// The unit box `[-1, 1]^3` as a case.
fn unit(origin: Raw3, direction: Raw3) -> Case {
    case(origin, direction, [-ONE; 3], [ONE; 3])
}

/// The exact oracle for `bunny-ray-box/1`.
///
/// The point at raw distance `t` is `origin + direction * t / 2^32`. Per axis with a nonzero
/// direction `d`, the crossings of the two faces are `(bound - origin) * 2^32 / d`; the ray is
/// inside the slab between them. An axis with `d == 0` rules the box out when the origin is
/// outside the closed slab and constrains nothing otherwise. The hit point's coordinates are
/// compared with the face planes as exact fractions to name the face: the lowest axis whose
/// face the point lies on and the ray crosses in the right sense.
fn oracle(case: Case) -> Expected {
    let Case { origin, direction, min, max, range } = case;
    let mut enter: Option<Rat> = None;
    let mut leave: Option<Rat> = None;
    for i in 0..3 {
        let (o, d) = (i128::from(origin[i]), i128::from(direction[i]));
        let (lo, hi) = (i128::from(min[i]), i128::from(max[i]));
        if d == 0 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let a = Rat::new((lo - o) << 32, d);
        let b = Rat::new((hi - o) << 32, d);
        let (first, last) = if a.cmp(&b) == Ordering::Greater { (b, a) } else { (a, b) };
        if enter.is_none_or(|e| first.cmp(&e) == Ordering::Greater) {
            enter = Some(first);
        }
        if leave.is_none_or(|l| last.cmp(&l) == Ordering::Less) {
            leave = Some(last);
        }
    }
    let (enter, leave) = (enter.expect("a direction is nonzero"), leave.expect("likewise"));
    if enter.cmp(&leave) == Ordering::Greater {
        return None;
    }
    let within = |k: i128| (i128::from(range.0)..=i128::from(range.1)).contains(&k);
    let (at, front) = if within(enter.floor()) {
        (enter, true)
    } else if within(leave.floor()) {
        (leave, false)
    } else {
        return None;
    };
    // Is the exact hit point on the plane `bound` of axis `i`? The point is
    // `(origin * 2^32 * at.den + direction * at.num) / (2^32 * at.den)`.
    let on = |i: usize, bound: i64| {
        let scaled = (i128::from(origin[i]) << 32) * at.den + i128::from(direction[i]) * at.num;
        scaled == (i128::from(bound) << 32) * at.den
    };
    let face = (0..3).find_map(|i| {
        let d = direction[i].signum();
        // Entering: through the minimum face going up or the maximum face going down.
        // Leaving: the reverse.
        let (up, down) = if front { (min[i], max[i]) } else { (max[i], min[i]) };
        let through = (d > 0 && on(i, up)) || (d < 0 && on(i, down));
        through.then_some((i, if front { -d } else { d }))
    });
    let (axis, sign) = face.expect("the hit point is on a face the ray crosses");
    let mut normal = [0; 3];
    normal[axis] = sign * ONE;
    let distance = i64::try_from(at.floor()).expect("within an i64 range");
    Some((distance, front, normal))
}

fn vec(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn raw(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn kernel(case: Case) -> Expected {
    let Case { origin, direction, min, max, range } = case;
    let ray = TraceRay::new(vec(origin), vec(direction)).expect("ray in the trace domain");
    let aabb = TraceBox::new(vec(min), vec(max)).expect("box");
    let (t_min, t_max) = (FixedQ32_32::from_raw(range.0), FixedQ32_32::from_raw(range.1));
    ray.hit_box(&aabb, t_min, t_max)
        .map(|hit| (hit.distance.raw(), hit.front_face, raw(hit.normal())))
}

/// Checks the kernel against the oracle and returns what both said.
fn agree(case: Case) -> Expected {
    let expected = oracle(case);
    assert_eq!(kernel(case), expected, "{case:?}");
    expected
}

fn full() -> (i64, i64) {
    (0, i64::MAX)
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(bound).expect("positive") * 2 + 1;
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    /// A value in `low..=high`.
    fn between(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low).expect("ordered") + 1;
        low + i64::try_from(self.next() % span).expect("below the span")
    }

    /// A magnitude of a random number of bits up to `bits`, at least one.
    fn scale(&mut self, bits: u32) -> i64 {
        let width = 1 + u32::try_from(self.next() % u64::from(bits)).expect("small");
        let value = self.signed((1_i64 << width) - 1).unsigned_abs().max(1);
        i64::try_from(value).expect("fits")
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [0; 3].map(|_| self.signed(bound))
    }

    fn one_in(&mut self, n: u64) -> bool {
        self.next().is_multiple_of(n)
    }
}

/// `aim` shifted right until every component is below `2^32`, then down a random number
/// of further bits, so directions of every length appear.
fn fit_direction(aim: Raw3, seeds: &mut Seeds) -> Raw3 {
    let largest = aim.iter().map(|c| c.unsigned_abs()).max().unwrap_or(0);
    let shift = (64 - largest.leading_zeros()).saturating_sub(32);
    let extra = u32::try_from(seeds.next() % 24).expect("small");
    aim.map(|c| c >> (shift + extra).min(62))
}

/// A box with random corners inside the domain; one axis in ten is flat.
fn random_box(seeds: &mut Seeds) -> (Raw3, Raw3) {
    let mut min = [0; 3];
    let mut max = [0; 3];
    for i in 0..3 {
        let width = if seeds.one_in(10) { 0 } else { seeds.scale(40).min(2 * DOMAIN) };
        min[i] = seeds.between(-DOMAIN, DOMAIN - width);
        max[i] = min[i] + width;
    }
    (min, max)
}

/// A coordinate on the box: a bound, or a random value between them.
fn feature(seeds: &mut Seeds, min: i64, max: i64) -> i64 {
    match seeds.next() % 3 {
        0 => min,
        1 => max,
        _ => seeds.between(min, max),
    }
}

/// The kinds of generated case, in the order of the counts in [`generate`].
const KINDS: [&str; 6] = ["aimed", "aimed", "inside", "parallel", "feature", "random"];

/// A generated case of one of six kinds, with a random distance range one time in four.
fn generate(seeds: &mut Seeds) -> (Case, usize) {
    let (min, max) = random_box(seeds);
    let kind = usize::try_from(seeds.next() % 6).expect("small");
    let inside = |seeds: &mut Seeds| [0, 1, 2].map(|i| seeds.between(min[i], max[i]));
    let aimed = |seeds: &mut Seeds| {
        let origin = seeds.triple(DOMAIN);
        let target = inside(seeds);
        let aim = [0, 1, 2].map(|i| target[i] - origin[i]);
        (origin, fit_direction(aim, seeds))
    };
    let (mut origin, mut direction) = match kind {
        0 | 1 => aimed(seeds),
        2 => (inside(seeds), fit_direction(seeds.triple(DOMAIN), seeds)),
        3 => {
            let (mut origin, mut direction) = aimed(seeds);
            // One or two zero components; the origin then sits in the slab, on a bound
            // or one raw unit outside it.
            let zeroed = 1 + seeds.next() % 2;
            for i in 0..3 {
                if seeds.next() % 3 >= zeroed {
                    continue;
                }
                direction[i] = 0;
                origin[i] = match seeds.next() % 4 {
                    0 => min[i],
                    1 => max[i],
                    2 => (min[i] - 1).max(-DOMAIN),
                    _ => (max[i] + 1).min(DOMAIN),
                };
                if seeds.one_in(2) {
                    origin[i] = seeds.between(min[i], max[i]);
                }
            }
            (origin, direction)
        }
        4 => {
            // The ray passes exactly through a point on a face, edge or corner of the box
            // after a whole number of direction lengths.
            let target = [0, 1, 2].map(|i| feature(seeds, min[i], max[i]));
            let direction = [0; 3].map(|_| if seeds.one_in(4) { 0 } else { seeds.signed(ONE) });
            let steps = seeds.between(-100, 100);
            let origin = [0, 1, 2].map(|i| target[i] - direction[i] * steps);
            if origin.iter().all(|c| c.abs() <= DOMAIN) {
                (origin, direction)
            } else {
                aimed(seeds)
            }
        }
        _ => (seeds.triple(DOMAIN), seeds.triple(ONE)),
    };
    if direction == [0; 3] {
        direction = [ONE, 0, 0];
    }
    for c in &mut origin {
        *c = (*c).clamp(-DOMAIN, DOMAIN);
    }
    let range = if seeds.one_in(4) {
        let low = seeds.signed(i64::MAX / 2);
        (low, low.saturating_add(seeds.signed(i64::MAX / 2).abs()))
    } else {
        full()
    };
    (Case { origin, direction, min, max, range }, kind)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_509_hits_match_an_exact_rational_oracle_on_generated_rays() {
    let mut seeds = Seeds(BOX_ORACLE_SEED);
    // Misses, front hits, back hits, hits with a zero direction component.
    let mut reached = [0_usize; 4];
    let mut by_kind = [0_usize; 6];
    for index in 0..BOX_ORACLE_CASES {
        let (case, kind) = generate(&mut seeds);
        let context = format!("seed={BOX_ORACLE_SEED:#018x} case={index} {}", KINDS[kind]);
        let expected = oracle(case);
        assert_eq!(kernel(case), expected, "{context} {case:?}");
        reached[match expected {
            None => 0,
            Some((_, true, _)) => 1,
            Some((_, false, _)) => 2,
        }] += 1;
        if expected.is_some() && case.direction.contains(&0) {
            reached[3] += 1;
            by_kind[kind] += 1;
        }
    }
    let context = format!("seed={BOX_ORACLE_SEED:#018x} reached={reached:?} parallel={by_kind:?}");
    println!("{context}");
    let [misses, front, back, parallel] = reached;
    assert!(misses >= MISSES_FLOOR && front >= FRONT_FLOOR && back >= BACK_FLOOR, "{context}");
    assert!(parallel >= PARALLEL_FLOOR, "{context}");
}

// Each floor is about 75 % of what the fixed seed reaches (counts in the test plan).
const MISSES_FLOOR: usize = 3638;
const FRONT_FLOOR: usize = 1811;
const BACK_FLOOR: usize = 550;
const PARALLEL_FLOOR: usize = 593;

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_510_a_grazing_ray_hits_and_one_raw_unit_outside_misses() {
    // Along the face plane y = 1, parallel to x: the plane belongs to the box.
    let along = agree(unit([-5 * ONE, ONE, 0], [ONE, 0, 0]));
    assert_eq!(along, Some((4 * ONE, true, [-ONE, 0, 0])));
    assert_eq!(agree(unit([-5 * ONE, ONE + 1, 0], [ONE, 0, 0])), None);
    assert_eq!(agree(unit([-5 * ONE, ONE - 1, 0], [ONE, 0, 0])).map(|h| h.0), Some(4 * ONE));
    // Diagonally through the edge x = -1, y = 1 and out again: the line touches the box at
    // (-1, 1, 0) only, at distance 2.0, through the x face (x and y tie, x wins).
    let edge = agree(unit([-3 * ONE, -ONE, 0], [ONE, ONE, 0]));
    assert_eq!(edge, Some((2 * ONE, true, [-ONE, 0, 0])));
    // One raw unit higher it passes outside the edge and misses.
    assert_eq!(agree(unit([-3 * ONE, -ONE + 1, 0], [ONE, ONE, 0])), None);
    // One raw unit lower it clips the box, entering through the x face.
    let inner = agree(unit([-3 * ONE, -ONE - 1, 0], [ONE, ONE, 0]));
    assert_eq!(inner.map(|h| (h.1, h.2)), Some((true, [-ONE, 0, 0])));
    // The same graze with the box on the other side: the ray from above passes the edge
    // x = -1, y = 1 going down and right, so one raw unit higher it enters through y.
    let above = agree(unit([-3 * ONE, 3 * ONE + 1, 0], [ONE, -ONE, 0]));
    assert_eq!(above.map(|h| (h.1, h.2)), Some((true, [0, ONE, 0])));
    // A ray running exactly along an edge line (x = 1, y = 1) enters at the end face.
    let along_edge = agree(unit([ONE, ONE, -5 * ONE], [0, 0, ONE]));
    assert_eq!(along_edge, Some((4 * ONE, true, [0, 0, -ONE])));
    assert_eq!(agree(unit([ONE + 1, ONE, -5 * ONE], [0, 0, ONE])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_511_an_origin_inside_the_box_hits_the_exit_from_inside() {
    let from_center = agree(unit([0; 3], [ONE, 0, 0]));
    assert_eq!(from_center, Some((ONE, false, [ONE, 0, 0])));
    let backwards = agree(unit([0; 3], [-ONE / 4, 0, 0]));
    assert_eq!(backwards, Some((4 * ONE, false, [-ONE, 0, 0])));
    // Off-centre, leaving through the nearer of two faces.
    let off = agree(unit([ONE / 2, -ONE / 2, 0], [ONE / 4, -ONE / 2, 0]));
    assert_eq!(off, Some((ONE, false, [0, -ONE, 0])));
    // With t_min past the exit there is nothing left to hit.
    let past = Case { range: (ONE + 1, i64::MAX), ..unit([0; 3], [ONE, 0, 0]) };
    assert_eq!(agree(past), None);
    // On a face: entering gives distance zero, front face; leaving gives zero, back face.
    assert_eq!(agree(unit([-ONE, 0, 0], [ONE, 0, 0])), Some((0, true, [-ONE, 0, 0])));
    assert_eq!(agree(unit([ONE, 0, 0], [ONE, 0, 0])), Some((0, false, [ONE, 0, 0])));
    // Inside a flat box (a slab one point thick): the ray is in the plane, leaving at once.
    let flat = case([0; 3], [ONE, 0, 0], [-ONE, 0, -ONE], [ONE, 0, ONE]);
    assert_eq!(agree(flat), Some((ONE, false, [ONE, 0, 0])));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_512_a_box_behind_the_origin_is_not_hit() {
    // Faces at distances -6 and -4: both behind.
    assert_eq!(agree(unit([5 * ONE, 0, 0], [ONE, 0, 0])), None);
    // With a negative t_min the same box is hit at its entry, from outside.
    let backwards = agree(Case { range: (-20 * ONE, 0), ..unit([5 * ONE, 0, 0], [ONE, 0, 0]) });
    assert_eq!(backwards, Some((-6 * ONE, true, [-ONE, 0, 0])));
    // Pointing away from a box ahead of the origin.
    assert_eq!(agree(unit([-5 * ONE, 0, 0], [-ONE, 0, 0])), None);
    // An entry clipped by t_min reports the exit, from inside (faces at 4 and 6).
    let clipped = agree(Case { range: (5 * ONE, i64::MAX), ..unit([-5 * ONE, 0, 0], [ONE, 0, 0]) });
    assert_eq!(clipped, Some((6 * ONE, false, [ONE, 0, 0])));
    // An entry beyond t_max is a miss.
    assert_eq!(agree(Case { range: (0, 3 * ONE), ..unit([-5 * ONE, 0, 0], [ONE, 0, 0]) }), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_513_edge_and_corner_hits_report_the_lowest_axis() {
    // Edge x = -1, y = -1, entered at (-1, -1, 0) after 2.0: x and y tie, x wins.
    let edge = agree(unit([-3 * ONE, -3 * ONE, 0], [ONE, ONE, 0]));
    assert_eq!(edge, Some((2 * ONE, true, [-ONE, 0, 0])));
    // Edge y = -1, z = -1: y wins.
    let edge = agree(unit([0, -3 * ONE, -3 * ONE], [0, ONE, ONE]));
    assert_eq!(edge, Some((2 * ONE, true, [0, -ONE, 0])));
    // Edge x = -1, z = -1 with y strictly inside: x wins.
    let edge = agree(unit([-3 * ONE, 0, -3 * ONE], [ONE, 0, ONE]));
    assert_eq!(edge, Some((2 * ONE, true, [-ONE, 0, 0])));
    // Corner (-1, -1, -1): x wins. Corner (1, 1, 1) from the other side: x wins, normal +x.
    let corner = agree(unit([-3 * ONE; 3], [ONE; 3]));
    assert_eq!(corner, Some((2 * ONE, true, [-ONE, 0, 0])));
    let corner = agree(unit([3 * ONE; 3], [-ONE; 3]));
    assert_eq!(corner, Some((2 * ONE, true, [ONE, 0, 0])));
    // Leaving through an edge or a corner: the lowest axis again, outward.
    assert_eq!(agree(unit([0; 3], [ONE; 3])), Some((ONE, false, [ONE, 0, 0])));
    assert_eq!(agree(unit([0; 3], [0, ONE, ONE])), Some((ONE, false, [0, ONE, 0])));
    assert_eq!(agree(unit([0; 3], [-ONE, 0, -ONE])), Some((ONE, false, [-ONE, 0, 0])));
    // A tie is a tie of the true crossings, not of their rounded distances: the y face is
    // crossed 1.0000000002 raw units after the x face, the floors are both 1, and y wins.
    let near = agree(unit([-ONE - 1, -ONE - 1, 0], [ONE, ONE - 1, 0]));
    assert_eq!(near, Some((1, true, [0, -ONE, 0])));
    // The mirror image, with x the later face, reports x.
    let near = agree(unit([-ONE - 1, -ONE - 1, 0], [ONE - 1, ONE, 0]));
    assert_eq!(near, Some((1, true, [-ONE, 0, 0])));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_514_a_ray_parallel_to_a_slab_is_decided_by_the_origin_in_it() {
    // Parallel to the x slab (direction x is zero): the origin's x decides.
    assert_eq!(agree(unit([0, -5 * ONE, 0], [0, ONE, 0])).map(|h| h.0), Some(4 * ONE));
    assert_eq!(agree(unit([ONE + 1, -5 * ONE, 0], [0, ONE, 0])), None);
    assert_eq!(agree(unit([-ONE - 1, -5 * ONE, 0], [0, ONE, 0])), None);
    // Exactly in a face plane: the plane belongs to the box, at either bound.
    let on_max = agree(unit([ONE, -5 * ONE, 0], [0, ONE, 0]));
    assert_eq!(on_max, Some((4 * ONE, true, [0, -ONE, 0])));
    let on_min = agree(unit([-ONE, -5 * ONE, 0], [0, ONE, 0]));
    assert_eq!(on_min, Some((4 * ONE, true, [0, -ONE, 0])));
    // A parallel axis never supplies the face, even when the origin is on its plane.
    let inside = agree(unit([ONE, 0, 0], [0, 0, ONE]));
    assert_eq!(inside, Some((ONE, false, [0, 0, ONE])));
    // Two parallel axes, one in and one out.
    assert_eq!(agree(unit([0, 2 * ONE, -5 * ONE], [0, 0, ONE])), None);
    // A zero-thickness box in x is hit by a ray in its plane and by a ray crossing it.
    let sheet = |origin, direction| case(origin, direction, [0, -ONE, -ONE], [0, ONE, ONE]);
    assert_eq!(agree(sheet([-ONE, 0, 0], [ONE, 0, 0])), Some((ONE, true, [-ONE, 0, 0])));
    assert_eq!(agree(sheet([0, -5 * ONE, 0], [0, ONE, 0])).map(|h| h.0), Some(4 * ONE));
    assert_eq!(agree(sheet([ONE, -5 * ONE, 0], [0, ONE, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_515_boxes_outside_the_trace_domain_are_refused_with_a_typed_error() {
    let make = |min: Raw3, max: Raw3| TraceBox::new(vec(min), vec(max));
    let limit = TRACE_COORD_LIMIT_RAW;
    let corner = Err(TraceBoxError::CornerOutsideTraceDomain);
    assert_eq!(make([0; 3], [limit, 0, 0]), corner);
    assert_eq!(make([0, -limit, 0], [ONE; 3]), corner);
    assert_eq!(make([i64::MIN; 3], [0; 3]), corner);
    assert_eq!(make([0; 3], [i64::MAX; 3]), corner);
    // A corner outside the domain is reported even when the corners are also reversed.
    assert_eq!(make([limit, 0, 0], [0; 3]), corner);
    let inverted = Err(TraceBoxError::MinimumAboveMaximum);
    assert_eq!(make([1, 0, 0], [0; 3]), inverted);
    assert_eq!(make([0, 0, DOMAIN], [0, 0, -DOMAIN]), inverted);
    let widest = make([-DOMAIN; 3], [DOMAIN; 3]).expect("the largest box is accepted");
    assert_eq!(raw(widest.min()), [-DOMAIN; 3]);
    assert_eq!(raw(widest.max()), [DOMAIN; 3]);
    assert_eq!(raw(widest.bounds().min), [-DOMAIN; 3]);
    assert_eq!(raw(widest.bounds().max), [DOMAIN; 3]);
    assert!(make([ONE; 3], [ONE; 3]).is_ok(), "a point is a flat box");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_516_the_widest_inputs_match_the_oracle_without_overflow() {
    // Every corner origin against the largest box and a box at the opposite corner, with the
    // longest and the shortest directions; the widest intermediates of the kernel occur here.
    let signs = [-1, 1];
    for (sx, sy, sz) in
        signs.iter().flat_map(|&x| signs.iter().flat_map(move |&y| signs.map(|z| (x, y, z))))
    {
        let origin = [sx * DOMAIN, sy * DOMAIN, sz * DOMAIN];
        for direction in [
            [-sx * ONE, -sy * ONE, -sz * ONE],
            [-sx, 0, 0],
            [-sx * ONE, -sy, 0],
            [0, 0, -sz],
            [sx * ONE, sy * ONE, sz * ONE],
        ] {
            for (min, max) in [
                ([-DOMAIN; 3], [DOMAIN; 3]),
                (origin.map(|c| -c.abs()), origin.map(|c| -c.abs() + 1)),
                ([0; 3], [1; 3]),
            ] {
                let widest = case(origin, direction, min, max);
                let _ = agree(widest);
                let _ = agree(Case { range: (i64::MIN, i64::MAX), ..widest });
            }
        }
    }
    let ray = TraceRay::from_ray(&bunny_geom::FixedRay3::new(
        vec([-DOMAIN, 0, 0]),
        bunny_linalg::FixedUnitVec3::try_from_unit(vec([ONE + 1, 0, 0])).expect("unit"),
    ))
    .expect("unit-direction ray");
    let aabb = TraceBox::new(vec([DOMAIN - 1, -1, -1]), vec([DOMAIN, 1, 1])).expect("box");
    let hit = ray.hit_box(&aabb, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX));
    let expected =
        oracle(case([-DOMAIN, 0, 0], [ONE + 1, 0, 0], [DOMAIN - 1, -1, -1], [DOMAIN, 1, 1]));
    assert_eq!(hit.map(|h| (h.distance.raw(), h.front_face, raw(h.normal()))), expected);
    assert!(expected.is_some(), "the long ray reaches the far box");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_517_the_oracle_names_the_face_it_expects() {
    // The oracle is itself checked against hand-computed hits, so a kernel and an oracle
    // that share a mistake cannot agree on it.
    assert_eq!(oracle(unit([-3 * ONE; 3], [ONE; 3])), Some((2 * ONE, true, [-ONE, 0, 0])));
    assert_eq!(oracle(unit([0, 3 * ONE, 0], [0, -ONE, 0])), Some((2 * ONE, true, [0, ONE, 0])));
    assert_eq!(oracle(unit([0; 3], [0, 0, -ONE])), Some((ONE, false, [0, 0, -ONE])));
    assert_eq!(oracle(unit([0, 2 * ONE, 0], [ONE, 0, 0])), None);
}
