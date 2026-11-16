//! `TraceRay::hit_cylinder` (contract `bunny-ray-cylinder/1`) against an independent exact
//! oracle, and named cases for grazing rays, origins inside, cylinders behind the origin,
//! rim hits, rays parallel to the axis, rays in a cap's plane and refused inputs
//! (TRACE-TP-540 to TRACE-TP-548).
//!
//! The oracle shares no arithmetic with the kernel. The kernel intersects two intervals
//! (the slab between the caps and the inside of the infinite cylinder); the oracle instead
//! lists every point where the line meets a surface (a side root whose height is between
//! the caps, a cap crossing inside the cap's disc), each checked exactly, and takes the
//! earliest and latest. It works in its own arbitrary-precision integers ([`Big`]), takes
//! the side's discriminant as `b^2 - a c` (the kernel uses Lagrange's identity), takes no
//! square root, and finds each rounded root by bisection on an exact predicate. The
//! generated rays use a fixed seed; every failure message names it and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{
    TraceCylinder, TraceCylinderError, TraceRay, RAY_CYLINDER_ID, TRACE_COORD_LIMIT_RAW,
};
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const CYLINDER_ORACLE_SEED: u64 = 0x5EED_C71D_0360_0001;
const CYLINDER_ORACLE_CASES: usize = 6_000;
const ONE: i64 = 1 << 32;
const DOMAIN: i64 = TRACE_COORD_LIMIT_RAW - 1;

type Raw3 = [i64; 3];

/// A signed integer of any size: a sign and little-endian 32-bit limbs, no trailing zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big {
    negative: bool,
    limbs: Vec<u32>,
}

impl Big {
    fn from(value: i128) -> Self {
        let mut magnitude = value.unsigned_abs();
        let mut limbs = Vec::new();
        while magnitude != 0 {
            limbs.push(u32::try_from(magnitude & 0xFFFF_FFFF).expect("masked"));
            magnitude >>= 32;
        }
        Self { negative: value < 0, limbs }
    }

    fn mul(&self, rhs: &Self) -> Self {
        let mut out = vec![0_u64; self.limbs.len() + rhs.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in rhs.limbs.iter().enumerate() {
                let cell = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = cell & 0xFFFF_FFFF;
                carry = cell >> 32;
            }
            out[i + rhs.limbs.len()] += carry;
        }
        let limbs = out.into_iter().map(|c| u32::try_from(c).expect("normalised")).collect();
        Self::normalised(self.negative != rhs.negative, limbs)
    }

    fn add(&self, rhs: &Self) -> Self {
        if self.negative == rhs.negative {
            return Self::normalised(self.negative, add_magnitudes(&self.limbs, &rhs.limbs));
        }
        match compare_magnitudes(&self.limbs, &rhs.limbs) {
            Ordering::Less => {
                Self::normalised(rhs.negative, sub_magnitudes(&rhs.limbs, &self.limbs))
            }
            _ => Self::normalised(self.negative, sub_magnitudes(&self.limbs, &rhs.limbs)),
        }
    }

    fn sub(&self, rhs: &Self) -> Self {
        self.add(&Self { negative: !rhs.negative, limbs: rhs.limbs.clone() })
    }

    fn signum(&self) -> i32 {
        match (self.limbs.is_empty(), self.negative) {
            (true, _) => 0,
            (false, true) => -1,
            (false, false) => 1,
        }
    }

    fn normalised(negative: bool, mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self { negative: negative && !limbs.is_empty(), limbs }
    }
}

fn compare_magnitudes(a: &[u32], b: &[u32]) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut out, mut carry) = (Vec::new(), 0_u64);
    for i in 0..a.len().max(b.len()) {
        let cell = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(u32::try_from(cell & 0xFFFF_FFFF).expect("masked"));
        carry = cell >> 32;
    }
    out.push(u32::try_from(carry).expect("one bit"));
    out
}

/// `a - b` for `|a| >= |b|`.
fn sub_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut out, mut borrow) = (Vec::new(), 0_i64);
    for (i, &limb) in a.iter().enumerate() {
        let mut cell = i64::from(limb) - i64::from(*b.get(i).unwrap_or(&0)) - borrow;
        borrow = i64::from(cell < 0);
        cell += borrow << 32;
        out.push(u32::try_from(cell).expect("in limb range"));
    }
    out
}

fn big(value: i128) -> Big {
    Big::from(value)
}

/// The sign of `l + m sqrt(disc)` for `disc >= 0`, exactly.
fn sign_with_root(l: &Big, m: &Big, disc: &Big) -> i32 {
    let (ls, ms) = (l.signum(), if disc.signum() == 0 { 0 } else { m.signum() });
    if ls == ms || ms == 0 {
        return ls;
    }
    if ls == 0 {
        return ms;
    }
    // Opposite signs: the larger magnitude wins.
    match l.mul(l).sub(&m.mul(m).mul(disc)).signum() {
        0 => 0,
        s if s > 0 => ls,
        _ => ms,
    }
}

/// A surface the line meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Bottom,
    Top,
    Side,
}

/// An exact crossing time in direction multiples: `num / den` (`den > 0`), or a side root
/// `(-b + s sqrt(disc)) / a`.
#[derive(Clone, Copy, Debug)]
enum Time {
    Ratio(i128, i128),
    Root(i32),
}

/// What the oracle expects: raw distance, front face and raw normal.
type Expected = Option<(i64, bool, Raw3)>;

/// One ray, one cylinder and one distance range, all raw.
#[derive(Clone, Copy, Debug)]
struct Case {
    origin: Raw3,
    direction: Raw3,
    radius: i64,
    caps: (i64, i64),
    range: (i64, i64),
}

/// A case over distances `0..=i64::MAX` raw.
fn case(origin: Raw3, direction: Raw3, radius: i64, caps: (i64, i64)) -> Case {
    Case { origin, direction, radius, caps, range: (0, i64::MAX) }
}

/// The side's quadratic in direction multiples: `a t^2 + 2 b t + c = 0` over `x` and `z`.
struct Quadratic {
    a: Big,
    b: Big,
    disc: Big,
}

impl Quadratic {
    fn new(case: &Case) -> Self {
        let ([ox, _, oz], [dx, _, dz]) = (case.origin, case.direction);
        let [ox, oz, dx, dz, radius] = [ox, oz, dx, dz, case.radius].map(|v| big(i128::from(v)));
        let a = dx.mul(&dx).add(&dz.mul(&dz));
        let b = ox.mul(&dx).add(&oz.mul(&dz));
        let c = ox.mul(&ox).add(&oz.mul(&oz)).sub(&radius.mul(&radius));
        let disc = b.mul(&b).sub(&a.mul(&c));
        Self { a, b, disc }
    }

    /// The order of two exact times.
    fn compare(&self, lhs: Time, rhs: Time) -> Ordering {
        match (lhs, rhs) {
            (Time::Ratio(n1, d1), Time::Ratio(n2, d2)) => (n1 * d2).cmp(&(n2 * d1)),
            (Time::Root(_), Time::Root(_)) if self.disc.signum() == 0 => Ordering::Equal,
            (Time::Root(s1), Time::Root(s2)) => s1.cmp(&s2),
            (Time::Ratio(num, den), Time::Root(sign)) => {
                // num / den - (-b + s sqrt(disc)) / a has the sign of
                // a num + den b - s den sqrt(disc).
                let rational = self.a.mul(&big(num)).add(&big(den).mul(&self.b));
                let irrational = big(-i128::from(sign) * den);
                sign_with_root(&rational, &irrational, &self.disc).cmp(&0)
            }
            (Time::Root(_), Time::Ratio(_, _)) => self.compare(rhs, lhs).reverse(),
        }
    }

    /// The largest raw `k` with `k <= 2^32 * (-b + s sqrt(disc)) / a`.
    fn floor_root(&self, s: i32) -> i128 {
        let scaled_b = self.b.mul(&big(1 << 32));
        floor_search(|k| {
            let l = big(0).sub(&scaled_b).sub(&self.a.mul(&big(k)));
            sign_with_root(&l, &big(i128::from(s) << 32), &self.disc) >= 0
        })
    }
}

/// The largest `k` in `-2^110..2^110` for which `at_most(k)` holds.
fn floor_search(at_most: impl Fn(i128) -> bool) -> i128 {
    let (mut low, mut high) = (-(1_i128 << 110), 1_i128 << 110);
    assert!(at_most(low) && !at_most(high), "the root is outside the search range");
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if at_most(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// A cap time as a ratio with a positive denominator: `(height - o_y) / d_y`.
fn cap_time(case: &Case, height: i64) -> Time {
    let (n, d) = (i128::from(height - case.origin[1]), i128::from(case.direction[1]));
    Time::Ratio(n * d.signum(), d.abs())
}

/// Every point where the line meets a surface of the closed cylinder.
fn crossings(case: &Case, quad: &Quadratic) -> Vec<(Time, Surface)> {
    let ([ox, oy, oz], [dx, dy, dz]) = (case.origin, case.direction);
    let (bottom, top) = case.caps;
    let mut out = Vec::new();
    if dy != 0 {
        for (height, surface) in [(bottom, Surface::Bottom), (top, Surface::Top)] {
            // At t = n / dy the point is o + d n / dy; inside the disc when
            // (o_x dy + d_x n)^2 + (o_z dy + d_z n)^2 <= r^2 dy^2.
            let n = i128::from(height - oy);
            let (dy, r) = (i128::from(dy), i128::from(case.radius));
            let x = big(i128::from(ox) * dy + i128::from(dx) * n);
            let z = big(i128::from(oz) * dy + i128::from(dz) * n);
            let gap = x.mul(&x).add(&z.mul(&z)).sub(&big(r * r).mul(&big(dy * dy)));
            if gap.signum() <= 0 {
                out.push((cap_time(case, height), surface));
            }
        }
    }
    if quad.a.signum() != 0 && quad.disc.signum() >= 0 {
        for s in [-1, 1] {
            let root = Time::Root(s);
            let between = if dy == 0 {
                (bottom..=top).contains(&oy)
            } else {
                let (low, high) = if dy > 0 { (bottom, top) } else { (top, bottom) };
                quad.compare(cap_time(case, low), root) != Ordering::Greater
                    && quad.compare(root, cap_time(case, high)) != Ordering::Greater
            };
            if between {
                out.push((root, Surface::Side));
            }
        }
    }
    out
}

/// What the oracle decides: raw distance, front face, and the surface and exact time hit.
type Decided = Option<(i64, bool, Surface, Time)>;

/// The exact oracle for `bunny-ray-cylinder/1`: the earliest and latest crossings, a cap
/// winning a tie with the side, then the range rule on their floors.
fn oracle(case: Case) -> Decided {
    let quad = Quadratic::new(&case);
    let all = crossings(&case, &quad);
    let pick = |want: Ordering| {
        all.iter().copied().reduce(|best, next| match quad.compare(next.0, best.0) {
            o if o == want => next,
            Ordering::Equal if next.1 != Surface::Side => next,
            _ => best,
        })
    };
    let (entry, exit) = (pick(Ordering::Less)?, pick(Ordering::Greater)?);
    let floor = |(time, _): (Time, Surface)| match time {
        Time::Ratio(n, d) => (n << 32).div_euclid(d),
        Time::Root(s) => quad.floor_root(s),
    };
    let within = |k: i128| (i128::from(case.range.0)..=i128::from(case.range.1)).contains(&k);
    let (chosen, front) = if within(floor(entry)) {
        (entry, true)
    } else if within(floor(exit)) {
        (exit, false)
    } else {
        return None;
    };
    let t = i64::try_from(floor(chosen)).expect("within an i64 range");
    Some((t, front, chosen.1, chosen.0))
}

/// The bound, in raw units, on each component of a side normal against the exact unit
/// normal, stated before measuring: the kernel's floored square root moves the vector by at
/// most `|d|` against a length of at least `2^64` times `|d|` (below `2^-32` raw once
/// normalized), the shift to 62 bits and the floored length each add below `2^-28` raw,
/// and the rounded division of `FixedVec3::normalize` adds at most half a raw unit. The sum
/// is below one raw unit.
const SIDE_NORMAL_BOUND_RAW: i128 = 1;

/// The kernel's result for `case`, after checking it against the oracle: the distance and
/// face equal; a cap's normal is the exact axis; a side's normal has `y = 0`, each other
/// component within [`SIDE_NORMAL_BOUND_RAW`] of the exact unit normal, and a positive dot
/// product with the exact offset of the hit from the axis. The exact normal is built from
/// the oracle's own root, `a (o + d t) = a o - b d + s d sqrt(disc)`, not the kernel's form.
fn checked(case: Case, got: Expected, context: &str) -> Expected {
    let decided = oracle(case);
    assert_eq!(got.map(|g| (g.0, g.1)), decided.map(|d| (d.0, d.1)), "{context}");
    let (Some((_, _, normal)), Some((_, _, surface, time))) = (got, decided) else {
        return got;
    };
    match (surface, time) {
        (Surface::Bottom, _) => assert_eq!(normal, [0, -ONE, 0], "{context}"),
        (Surface::Top, _) => assert_eq!(normal, [0, ONE, 0], "{context}"),
        (Surface::Side, Time::Root(s)) => check_side_normal(&case, s, normal, context),
        (Surface::Side, Time::Ratio(..)) => panic!("a side crossing has a root time"),
    }
    got
}

fn check_side_normal(case: &Case, sign: i32, normal: Raw3, context: &str) {
    assert_eq!(normal[1], 0, "{context}");
    let quad = Quadratic::new(case);
    let wide = |v: i64| big(i128::from(v));
    let size = quad.a.mul(&wide(case.radius));
    let scale = big(1 << 32);
    let mut dot = (big(0), big(0));
    for axis in [0, 2] {
        let origin = wide(case.origin[axis]);
        let (dir, n) = (wide(case.direction[axis]), wide(normal[axis]));
        // a P = rational + s d sqrt(disc); the exact unit normal is a P / (a r).
        let rational = quad.a.mul(&origin).sub(&quad.b.mul(&dir));
        let lead = n.mul(&size).sub(&scale.mul(&rational));
        let irrational = big(-i128::from(sign)).mul(&scale).mul(&dir);
        let slack = size.mul(&big(SIDE_NORMAL_BOUND_RAW));
        let high = sign_with_root(&lead.sub(&slack), &irrational, &quad.disc);
        let low = sign_with_root(&lead.add(&slack), &irrational, &quad.disc);
        assert!(high <= 0 && low >= 0, "axis {axis} beyond the bound: {context}");
        dot = (dot.0.add(&n.mul(&rational)), dot.1.add(&big(i128::from(sign)).mul(&n).mul(&dir)));
    }
    assert!(sign_with_root(&dot.0, &dot.1, &quad.disc) > 0, "not outward: {context}");
}

fn vec(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn raw(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn cylinder(radius: i64, (bottom, top): (i64, i64)) -> Result<TraceCylinder, TraceCylinderError> {
    let f = FixedQ32_32::from_raw;
    TraceCylinder::new(f(radius), f(bottom), f(top))
}

fn kernel(case: Case) -> Expected {
    let Case { origin, direction, radius, caps, range } = case;
    let ray = TraceRay::new(vec(origin), vec(direction)).expect("ray in the trace domain");
    let cylinder = cylinder(radius, caps).expect("cylinder in the trace domain");
    let (t_min, t_max) = (FixedQ32_32::from_raw(range.0), FixedQ32_32::from_raw(range.1));
    ray.hit_cylinder(&cylinder, t_min, t_max)
        .map(|hit| (hit.distance.raw(), hit.front_face, raw(hit.normal().into_inner())))
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

    /// A magnitude of a random number of bits up to `bits`, at least one.
    fn scale(&mut self, bits: u32) -> i64 {
        let width = 1 + u32::try_from(self.next() % u64::from(bits)).expect("small");
        self.signed((1_i64 << width) - 1).abs().max(1)
    }

    fn one_in(&mut self, n: u64) -> bool {
        self.next().is_multiple_of(n)
    }
}

/// A generated case: a cylinder of random size and height, a ray usually aimed at a point
/// near its surface (so many rays graze the side or a rim), the origin inside one time in
/// five, the direction parallel to the axis or to the caps one time in ten each (with the
/// origin on a cap's plane half of those times), a ray through a rim point one time in ten
/// ([`through_rim`]), and a random distance range one time in four.
fn generate(seeds: &mut Seeds) -> Case {
    let radius = seeds.scale(38);
    let bottom = seeds.signed(DOMAIN / 2);
    let top = bottom + seeds.scale(38);
    let height = top - bottom;
    let inside = seeds.one_in(5);
    let origin = if inside {
        [
            seeds.signed(radius / 2),
            bottom + height / 2 + seeds.signed(height / 4),
            seeds.signed(radius / 2),
        ]
    } else {
        [seeds.signed(DOMAIN), seeds.signed(DOMAIN), seeds.signed(DOMAIN)]
    };
    let reach = radius + radius / 4;
    let aim_y = bottom - height / 4 + seeds.signed(height * 3 / 4).abs();
    let aim = [seeds.signed(reach) - origin[0], aim_y - origin[1], seeds.signed(reach) - origin[2]];
    let mut direction =
        if seeds.one_in(4) { [0; 3].map(|_| seeds.signed(ONE)) } else { fit_direction(aim, seeds) };
    let mut origin = origin;
    match seeds.next() % 10 {
        0 => direction = [0, direction[1], 0],
        1 => {
            direction[1] = 0;
            if seeds.one_in(2) {
                origin[1] = if seeds.one_in(2) { bottom } else { top };
            }
        }
        2 => (origin, direction) = through_rim(seeds, radius, (bottom, top)),
        _ => {}
    }
    if direction == [0; 3] {
        direction = [0, ONE, 0];
    }
    let range = if seeds.one_in(4) {
        let low = seeds.signed(i64::MAX / 2);
        (low, low.saturating_add(seeds.signed(i64::MAX / 2).abs()))
    } else {
        (0, i64::MAX)
    };
    Case { origin, direction, radius, caps: (bottom, top), range }
}

/// A ray that passes exactly through a point of a rim with integer coordinates, at a whole
/// number of direction multiples from its origin, so the side and a cap tie there.
fn through_rim(seeds: &mut Seeds, radius: i64, (bottom, top): (i64, i64)) -> (Raw3, Raw3) {
    let height = if seeds.one_in(2) { bottom } else { top };
    let sign = if seeds.one_in(2) { 1 } else { -1 };
    let rim = if seeds.one_in(2) { [sign * radius, height, 0] } else { [0, height, sign * radius] };
    let direction = [0; 3].map(|_| seeds.signed(ONE));
    let steps = 1 + i64::try_from(seeds.next() % 8).expect("small");
    ([0, 1, 2].map(|i| rim[i] - steps * direction[i]), direction)
}

/// `aim` shifted right until every component is below `2^32`, then down a random number
/// of further bits, so directions of every length appear.
fn fit_direction(aim: Raw3, seeds: &mut Seeds) -> Raw3 {
    let largest = aim.iter().map(|c| c.unsigned_abs()).max().unwrap_or(0);
    let shift = (64 - largest.leading_zeros()).saturating_sub(32);
    let extra = u32::try_from(seeds.next() % 24).expect("small");
    aim.map(|c| c >> (shift + extra).min(62))
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_540_hits_match_an_exact_rational_oracle_on_generated_rays() {
    let mut seeds = Seeds(CYLINDER_ORACLE_SEED);
    // Misses, front cap, front side, back cap, back side.
    let mut reached = [0_usize; 5];
    for index in 0..CYLINDER_ORACLE_CASES {
        let case = generate(&mut seeds);
        let context = format!("seed={CYLINDER_ORACLE_SEED:#018x} case={index} {case:?}");
        let expected = checked(case, kernel(case), &context);
        reached[match expected {
            None => 0,
            Some((_, true, [_, 0, _])) => 1,
            Some((_, true, _)) => 2,
            Some((_, false, [_, 0, _])) => 3,
            Some((_, false, _)) => 4,
        }] += 1;
    }
    let context = format!("seed={CYLINDER_ORACLE_SEED:#018x} reached={reached:?}");
    // Each floor is about 75 % of what the fixed seed reaches (counts in the test plan).
    let floors = [2_931, 305, 663, 330, 270];
    assert!(reached.iter().zip(floors).all(|(&got, floor)| got >= floor), "{context}");
}

/// The unit cylinder `x^2 + z^2 <= 1`, `-1 <= y <= 1`, used by the named cases.
const UNIT: (i64, (i64, i64)) = (ONE, (-ONE, ONE));

fn unit(origin: Raw3, direction: Raw3) -> Case {
    case(origin, direction, UNIT.0, UNIT.1)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_541_a_grazing_ray_touches_the_side_once_and_one_raw_unit_out_misses() {
    // The line z = 1, y = 0 touches the side at (0, 0, 1), distance 4.
    let tangent = kernel(unit([-4 * ONE, 0, ONE], [ONE, 0, 0]));
    assert_eq!(tangent, Some((4 * ONE, true, [0, 0, ONE])));
    assert_eq!(kernel(unit([-4 * ONE, 0, ONE + 1], [ONE, 0, 0])), None);
    // One raw unit inside: a chord of half-length sqrt(2^33 - 1) raw, about 92682 raw units.
    let inside = unit([-4 * ONE, 0, ONE - 1], [ONE, 0, 0]);
    checked(inside, kernel(inside), "inside");
    let (distance, front, _) = kernel(inside).expect("the ray crosses the side");
    assert!(front && distance < 4 * ONE && distance > 4 * ONE - 92_683, "{distance}");
    // A ray just above the top cap, aimed across it, misses; at the cap's height it grazes
    // the cap's plane and is reported at the side (TRACE-TP-546).
    assert_eq!(kernel(unit([-4 * ONE, ONE + 1, 0], [ONE, 0, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_542_an_origin_inside_hits_the_exit_from_inside() {
    // From the centre: out through the side, out through the top cap, out through the
    // bottom cap.
    assert_eq!(kernel(unit([0; 3], [ONE, 0, 0])), Some((ONE, false, [ONE, 0, 0])));
    assert_eq!(kernel(unit([0; 3], [0, ONE, 0])), Some((ONE, false, [0, ONE, 0])));
    assert_eq!(kernel(unit([0; 3], [0, -ONE / 2, 0])), Some((2 * ONE, false, [0, -ONE, 0])));
    // Off-centre, half-length direction: the side exit at x = sqrt(1 - 0.25) = 0.866.. is at
    // distance 1.732.. in direction multiples.
    let off = unit([0, 0, ONE / 2], [ONE / 2, 0, 0]);
    checked(off, kernel(off), "off");
    let (distance, front, _) = kernel(off).expect("an inside origin always hits");
    assert!(!front && distance / (ONE / 1000) == 1732, "{distance}");
    // With t_min past the exit nothing is left.
    assert_eq!(kernel(Case { range: (2 * ONE, i64::MAX), ..off }), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_543_a_cylinder_behind_the_origin_is_not_hit() {
    // Side crossings at -11 and -9: both behind.
    assert_eq!(kernel(unit([10 * ONE, 0, 0], [ONE, 0, 0])), None);
    // With a negative t_min the near side, at x = -1, is hit from outside.
    let backwards = Case { range: (-20 * ONE, 0), ..unit([10 * ONE, 0, 0], [ONE, 0, 0]) };
    assert_eq!(kernel(backwards), Some((-11 * ONE, true, [-ONE, 0, 0])));
    // Below the bottom cap, pointing down.
    assert_eq!(kernel(unit([0, -5 * ONE, 0], [0, -ONE, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_544_at_the_rim_the_cap_wins_the_tie() {
    // From (-2, 2, 0) along (1, -1, 0): the line meets the side x = -1 and the top cap
    // y = 1 at the same point, the rim, at distance 1; it leaves through the rim at
    // (1, -1, 0), on the side x = 1 and the bottom cap y = -1, at distance 3.
    let rim = unit([-2 * ONE, 2 * ONE, 0], [ONE, -ONE, 0]);
    assert_eq!(kernel(rim), Some((ONE, true, [0, ONE, 0])));
    assert_eq!(
        kernel(Case { range: (2 * ONE, i64::MAX), ..rim }),
        Some((3 * ONE, false, [0, -ONE, 0]))
    );
    // From (-2, 0, 0) along (1, 1, 0) the line touches the cylinder only at the rim point
    // (-1, 1, 0): the side's entry and the top cap's exit are the same point, and the cap
    // wins there too.
    assert_eq!(kernel(unit([-2 * ONE, 0, 0], [ONE, ONE, 0])), Some((ONE, true, [0, ONE, 0])));
    // One raw unit higher the line enters through the top cap (inside its disc), and one
    // raw unit lower through the side.
    let above = kernel(unit([-2 * ONE, 2 * ONE + 1, 0], [ONE, -ONE, 0])).expect("hit");
    assert_eq!((above.1, above.2), (true, [0, ONE, 0]));
    let below = kernel(unit([-2 * ONE, 2 * ONE - 1, 0], [ONE, -ONE, 0])).expect("hit");
    assert_eq!((below.1, below.2[1]), (true, 0));
    checked(unit([-2 * ONE, 2 * ONE - 1, 0], [ONE, -ONE, 0]), Some(below), "below");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_545_a_ray_parallel_to_the_axis_hits_only_a_cap() {
    // The side's quadratic is degenerate (a = 0): inside the radius the caps decide.
    let up = unit([ONE / 2, -5 * ONE, 0], [0, ONE, 0]);
    assert_eq!(kernel(up), Some((4 * ONE, true, [0, -ONE, 0])));
    assert_eq!(
        kernel(Case { range: (5 * ONE, i64::MAX), ..up }),
        Some((6 * ONE, false, [0, ONE, 0]))
    );
    // On the side itself: the bottom rim, where the cap wins.
    assert_eq!(kernel(unit([ONE, -5 * ONE, 0], [0, ONE, 0])), Some((4 * ONE, true, [0, -ONE, 0])));
    // One raw unit outside the radius: no hit at all.
    assert_eq!(kernel(unit([ONE + 1, -5 * ONE, 0], [0, ONE, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_546_a_ray_in_a_caps_plane_hits_the_side() {
    // y = 1 exactly, parallel to the caps: the slab does not bound the line, so the side
    // decides, with the side's normal.
    let plane = unit([-4 * ONE, ONE, 0], [ONE, 0, 0]);
    assert_eq!(kernel(plane), Some((3 * ONE, true, [-ONE, 0, 0])));
    assert_eq!(
        kernel(Case { range: (4 * ONE, i64::MAX), ..plane }),
        Some((5 * ONE, false, [ONE, 0, 0]))
    );
    let bottom = unit([-4 * ONE, -ONE, 0], [ONE, 0, 0]);
    assert_eq!(kernel(bottom), Some((3 * ONE, true, [-ONE, 0, 0])));
    // One raw unit outside either plane misses.
    assert_eq!(kernel(unit([-4 * ONE, -ONE - 1, 0], [ONE, 0, 0])), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_547_cylinders_outside_the_trace_domain_are_refused_with_a_typed_error() {
    let limit = TRACE_COORD_LIMIT_RAW;
    assert_eq!(cylinder(0, (0, ONE)), Err(TraceCylinderError::RadiusNotPositive));
    assert_eq!(cylinder(i64::MIN, (0, ONE)), Err(TraceCylinderError::RadiusNotPositive));
    assert_eq!(cylinder(limit, (0, ONE)), Err(TraceCylinderError::RadiusOutsideTraceDomain));
    assert_eq!(cylinder(i64::MAX, (0, ONE)), Err(TraceCylinderError::RadiusOutsideTraceDomain));
    assert_eq!(cylinder(ONE, (-limit, 0)), Err(TraceCylinderError::CapOutsideTraceDomain));
    assert_eq!(cylinder(ONE, (0, limit)), Err(TraceCylinderError::CapOutsideTraceDomain));
    assert_eq!(cylinder(ONE, (i64::MIN, i64::MAX)), Err(TraceCylinderError::CapOutsideTraceDomain));
    assert_eq!(cylinder(ONE, (ONE, ONE)), Err(TraceCylinderError::CapsNotOrdered));
    assert_eq!(cylinder(ONE, (ONE, -ONE)), Err(TraceCylinderError::CapsNotOrdered));
    // A bad radius is reported before a bad cap.
    assert_eq!(cylinder(0, (limit, 0)), Err(TraceCylinderError::RadiusNotPositive));
    let edge = cylinder(DOMAIN, (-DOMAIN, DOMAIN)).expect("the largest cylinder is accepted");
    assert_eq!(
        (edge.radius().raw(), edge.bottom().raw(), edge.top().raw()),
        (DOMAIN, -DOMAIN, DOMAIN)
    );
    assert_eq!(raw(edge.bounds().min), [-DOMAIN; 3]);
    assert_eq!(raw(edge.bounds().max), [DOMAIN; 3]);
    let thin = cylinder(1, (0, 1)).expect("the smallest cylinder is accepted");
    assert_eq!((raw(thin.bounds().min), raw(thin.bounds().max)), ([-1, 0, -1], [1, 1, 1]));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_548_the_widest_inputs_match_the_oracle_without_overflow() {
    // Every corner origin against the largest and the thinnest cylinders, with the longest,
    // the shortest, an axis-parallel and a cap-parallel direction; the widest
    // intermediates of the kernel occur here.
    let signs = [-1, 1];
    for (sx, sy, sz) in
        signs.iter().flat_map(|&x| signs.iter().flat_map(move |&y| signs.map(|z| (x, y, z))))
    {
        let origin = [sx * DOMAIN, sy * DOMAIN, sz * DOMAIN];
        let directions =
            [[-sx * ONE, -sy * ONE, -sz * ONE], [-sx, 0, 0], [0, -sy, 0], [-sx * ONE, 0, -sz]];
        for direction in directions {
            for (radius, caps) in
                [(DOMAIN, (-DOMAIN, DOMAIN)), (1, (0, 1)), (DOMAIN, (DOMAIN - 1, DOMAIN))]
            {
                let context = format!("origin={origin:?} direction={direction:?} radius={radius}");
                let widest = case(origin, direction, radius, caps);
                checked(widest, kernel(widest), &context);
            }
        }
    }
    let ray = TraceRay::from_ray(&bunny_geom::FixedRay3::new(
        vec([-DOMAIN, 0, 0]),
        bunny_linalg::FixedUnitVec3::try_from_unit(vec([ONE + 1, 0, 0])).expect("unit"),
    ))
    .expect("unit-direction ray");
    let wide = cylinder(DOMAIN, (-DOMAIN, DOMAIN)).expect("cylinder");
    let hit = ray.hit_cylinder(&wide, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX));
    let got = hit.map(|h| (h.distance.raw(), h.front_face, raw(h.normal().into_inner())));
    let unit_case = case([-DOMAIN, 0, 0], [ONE + 1, 0, 0], DOMAIN, (-DOMAIN, DOMAIN));
    assert_eq!(checked(unit_case, got, "unit direction"), Some((0, true, [-ONE, 0, 0])));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_550_the_distance_limits_are_inclusive_at_both_ends() {
    // From (-4, 0, 0) along +x the unit cylinder's side is entered at exactly 3.0 and left
    // at exactly 5.0.
    let ray = unit([-4 * ONE, 0, 0], [ONE, 0, 0]);
    let at = |low: i64, high: i64| kernel(Case { range: (low, high), ..ray });
    let entry = Some((3 * ONE, true, [-ONE, 0, 0]));
    let exit = Some((5 * ONE, false, [ONE, 0, 0]));
    assert_eq!(at(3 * ONE, 3 * ONE), entry, "entry exactly on t_min and t_max");
    assert_eq!(at(0, 3 * ONE), entry, "entry exactly on t_max");
    assert_eq!(at(0, 3 * ONE - 1), None, "entry one raw unit past t_max");
    assert_eq!(at(3 * ONE + 1, 5 * ONE), exit, "exit exactly on t_max");
    assert_eq!(at(5 * ONE, i64::MAX), exit, "exit exactly on t_min");
    assert_eq!(at(3 * ONE + 1, 5 * ONE - 1), None, "exit one raw unit past t_max");
    assert_eq!(at(5 * ONE + 1, i64::MAX), None, "exit one raw unit before t_min");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_551_the_contract_identity_is_published() {
    assert_eq!(RAY_CYLINDER_ID, "bunny-ray-cylinder/1");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_552_side_normals_of_the_smallest_cylinder_are_unit_and_outward() {
    // Radius one raw unit: the offset at the rounded distance is meaningless at this size,
    // so the normal must come from the exact root. Directions at every octant.
    for (dx, dz) in [(ONE, ONE), (ONE, -ONE / 3), (-ONE / 7, ONE), (-ONE, -1), (3, -5)] {
        let thin = case([-dx * 8, 0, -dz * 8], [dx, 0, dz], 1, (-ONE, ONE));
        let context = format!("direction=({dx}, {dz})");
        let hit = checked(thin, kernel(thin), &context).expect("aimed at the axis");
        assert!(hit.1 && hit.2[1] == 0, "{context}");
    }
}
