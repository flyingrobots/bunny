//! `TraceRay::hit_disc` (contract `bunny-ray-disc/1`) against an independent exact oracle,
//! and named cases for rim hits, grazing rays, rays in the plane, origins on the disc,
//! discs behind the origin and refused inputs (TRACE-TP-560 to TRACE-TP-566).
//!
//! The oracle shares no arithmetic with the kernel. It keeps the meeting point of the ray
//! and the plane as an exact fraction `(tn / td, td > 0)`, takes each coordinate of the hit
//! as the fraction `(o td + d tn) / td`, compares the exact squared lengths in its own
//! arbitrary-precision unsigned integers ([`Big`]) and rounds the distance with truncating
//! division and a remainder correction. The generated rays use a fixed seed; every failure
//! message names it and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceDisc, TraceDiscError, TraceRay, RAY_DISC_ID, TRACE_COORD_LIMIT_RAW};
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const DISC_ORACLE_SEED: u64 = 0x5EED_D15C_0363_0001;
const DISC_ORACLE_CASES: usize = 6_000;
const ONE: i64 = 1 << 32;
const DOMAIN: i64 = TRACE_COORD_LIMIT_RAW - 1;
const MISS_FLOOR: usize = 2_750;
const FRONT_FLOOR: usize = 880;
const BACK_FLOOR: usize = 820;
const BEHIND_FLOOR: usize = 165;
const DISCRIMINATING_FLOOR: usize = 129;

type Raw3 = [i64; 3];

/// An unsigned integer of any size: little-endian 32-bit limbs, no trailing zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u32>);

impl Big {
    fn from(value: u128) -> Self {
        let mut rest = value;
        let mut limbs = Vec::new();
        while rest != 0 {
            limbs.push(u32::try_from(rest & 0xFFFF_FFFF).expect("masked"));
            rest >>= 32;
        }
        Self(limbs)
    }

    fn mul(&self, rhs: &Self) -> Self {
        let mut out = vec![0_u64; self.0.len() + rhs.0.len() + 1];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in rhs.0.iter().enumerate() {
                let cell = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = cell & 0xFFFF_FFFF;
                carry = cell >> 32;
            }
            out[i + rhs.0.len()] += carry;
        }
        Self::trimmed(out.into_iter().map(|c| u32::try_from(c).expect("limb")).collect())
    }

    fn add(&self, rhs: &Self) -> Self {
        let (mut out, mut carry) = (Vec::new(), 0_u64);
        for i in 0..self.0.len().max(rhs.0.len()) {
            let cell =
                u64::from(*self.0.get(i).unwrap_or(&0)) + u64::from(*rhs.0.get(i).unwrap_or(&0));
            let cell = cell + carry;
            out.push(u32::try_from(cell & 0xFFFF_FFFF).expect("masked"));
            carry = cell >> 32;
        }
        out.push(u32::try_from(carry).expect("one bit"));
        Self::trimmed(out)
    }

    fn compare(&self, rhs: &Self) -> Ordering {
        self.0.len().cmp(&rhs.0.len()).then_with(|| self.0.iter().rev().cmp(rhs.0.iter().rev()))
    }

    fn trimmed(mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self(limbs)
    }
}

/// `value^2` as a [`Big`], for any `i128` below `2^127`.
fn square(value: i128) -> Big {
    let magnitude = Big::from(value.unsigned_abs());
    magnitude.mul(&magnitude)
}

/// What the oracle expects: raw distance and front face.
type Expected = Option<(i64, bool)>;

/// One ray, one disc and one distance range, all raw.
#[derive(Clone, Copy, Debug)]
struct Case {
    origin: Raw3,
    direction: Raw3,
    height: i64,
    radius: i64,
    range: (i64, i64),
}

/// A case over distances `0..=i64::MAX` raw.
fn case(origin: Raw3, direction: Raw3, height: i64, radius: i64) -> Case {
    Case { origin, direction, height, radius, range: full() }
}

/// The exact oracle for `bunny-ray-disc/1`.
///
/// The ray meets the plane `z = h` where `o.z + d.z t = h`, so `t = tn / td` with
/// `td = |d.z|` and `tn = (h - o.z) sign(d.z)`. The hit coordinate on an axis `a` is
/// `(o.a td + d.a tn) / td`; it is inside the disc when the sum of the two squared
/// numerators is at most `r^2 td^2`. The raw distance is `floor(tn 2^32 / td)`.
fn oracle(case: Case) -> Expected {
    let (distance, inside) = meeting(case)?;
    let range = case.range;
    if distance < i128::from(range.0) || distance > i128::from(range.1) || !inside {
        return None;
    }
    let front = case.direction[2] < 0;
    Some((i64::try_from(distance).expect("within an i64 range"), front))
}

/// The raw distance to the plane and whether the exact meeting point is within the radius;
/// `None` for a ray parallel to the plane.
fn meeting(case: Case) -> Option<(i128, bool)> {
    let Case { origin, direction, height, radius, .. } = case;
    let [ox, oy, oz] = origin.map(i128::from);
    let [dx, dy, dz] = direction.map(i128::from);
    if dz == 0 {
        return None;
    }
    let td = dz.abs();
    let tn = (i128::from(height) - oz) * dz.signum();
    let scaled = tn * (1_i128 << 32);
    let mut distance = scaled / td;
    if scaled % td < 0 {
        distance -= 1;
    }
    let lengths = square(ox * td + dx * tn).add(&square(oy * td + dy * tn));
    let limit = square(i128::from(radius) * td);
    Some((distance, lengths.compare(&limit) != Ordering::Greater))
}

/// The radius test on the point at the rounded distance, floor-rounded per coordinate: the
/// decision a kernel that tested the rounded hit point would make. Used only to count how
/// many generated cases tell it from the exact test.
fn rounded_inside(case: Case, distance: i128) -> bool {
    let [ox, oy, _] = case.origin.map(i128::from);
    let [dx, dy, _] = case.direction.map(i128::from);
    let (x, y) = (ox + ((dx * distance) >> 32), oy + ((dy * distance) >> 32));
    let lengths = square(x).add(&square(y));
    lengths.compare(&square(i128::from(case.radius))) != Ordering::Greater
}

fn vec(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn kernel(case: Case) -> Expected {
    let Case { origin, direction, height, radius, range } = case;
    let ray = TraceRay::new(vec(origin), vec(direction)).expect("ray in the trace domain");
    let disc =
        TraceDisc::new(FixedQ32_32::from_raw(height), FixedQ32_32::from_raw(radius)).expect("disc");
    let (t_min, t_max) = (FixedQ32_32::from_raw(range.0), FixedQ32_32::from_raw(range.1));
    ray.hit_disc(&disc, t_min, t_max).map(|hit| {
        assert_eq!(hit.normal().into_inner(), vec([0, 0, ONE]));
        (hit.distance.raw(), hit.front_face)
    })
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

    /// A magnitude of a random number of bits up to `bits`, at least one.
    fn scale(&mut self, bits: u32) -> i64 {
        let width = 1 + u32::try_from(self.next() % u64::from(bits)).expect("small");
        let value = self.signed((1_i64 << width) - 1).unsigned_abs().max(1);
        i64::try_from(value).expect("fits")
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [0; 3].map(|_| self.signed(bound))
    }
}

/// A generated case, of three kinds (one half, one quarter, one quarter):
///
/// - random: height anywhere, the direction aimed at a point near the disc (so many rays
///   pass close to the rim), from an origin within three radii of the axis, or random, with
///   a small `|d.z|` or a zero `d.z` now and then;
/// - rim, whole distance: built to meet the plane exactly on a Pythagorean point `(3k, 4k)`
///   of the circle of radius `5k` after a whole number of direction lengths, then moved by
///   -1, 0 or +1 raw unit along `x`: exactly on the rim, just inside and just outside;
/// - rim, inexact distance: `d.x = d.z u`, `d.y = d.z v`, so the exact meeting point is
///   `origin + (u, v) s` with whole `s`, placed on the rim as above, while the distance
///   `s / d.z` is not a whole raw unit (any `d.z` that is not a power of two, down to
///   `|d.z| = 1`), either sign. The hit point of the rounded distance is then off the exact
///   one, so the exact and the rounded radius test can disagree.
///
/// The distance range is `0..=i64::MAX` (most), all of `i64` or a random one; the rim kinds
/// use all of `i64` half the time, so hits behind the origin are reached.
fn generate(seeds: &mut Seeds) -> Case {
    let kind = seeds.next() % 4;
    loop {
        let candidate = match kind {
            0 | 1 => Some(random_case(seeds)),
            2 => rim_case(seeds),
            _ => inexact_rim_case(seeds),
        };
        if let Some(candidate) = candidate {
            return candidate;
        }
    }
}

fn random_case(seeds: &mut Seeds) -> Case {
    let radius = seeds.scale(39);
    let height = seeds.signed(DOMAIN);
    let reach = (3 * radius).min(DOMAIN);
    let origin = [seeds.signed(reach), seeds.signed(reach), seeds.signed(DOMAIN)];
    let jitter = seeds.triple(radius + radius / 4);
    let aim: Raw3 = [jitter[0] - origin[0], jitter[1] - origin[1], height - origin[2]];
    let mut direction = if seeds.next().is_multiple_of(4) {
        seeds.triple(ONE)
    } else {
        let largest = aim.iter().map(|c| c.unsigned_abs()).max().unwrap_or(0);
        let shift = (64 - largest.leading_zeros()).saturating_sub(32);
        let extra = u32::try_from(seeds.next() % 24).expect("small");
        aim.map(|c| c >> (shift + extra).min(62))
    };
    match seeds.next() % 16 {
        0 => direction[2] = 0,
        1 => direction[2] = [-1, 1][usize::try_from(seeds.next() % 2).expect("bit")],
        _ => {}
    }
    if direction == [0; 3] {
        direction = [0, 0, ONE];
    }
    let range = match seeds.next() % 8 {
        0 | 1 => (i64::MIN, i64::MAX),
        2 => {
            let low = seeds.signed(i64::MAX / 2);
            (low, low.saturating_add(seeds.signed(i64::MAX / 2).abs()))
        }
        _ => full(),
    };
    Case { origin, direction, height, radius, range }
}

fn rim_case(seeds: &mut Seeds) -> Option<Case> {
    let k = seeds.scale(34);
    let (a, b) = if seeds.next().is_multiple_of(2) { (3 * k, 4 * k) } else { (4 * k, 3 * k) };
    let (a, b) = (a * [-1, 1][usize::try_from(seeds.next() % 2).expect("bit")], b);
    let dz = seeds.signed(ONE);
    let dz = if dz == 0 { ONE } else { dz };
    let (dx, dy) = (seeds.signed(ONE), seeds.signed(ONE));
    // The ray covers `q` direction lengths on its way to the plane (a whole number, so the
    // meeting point has integer coordinates); it is not negative, so the hit is ahead.
    let q = seeds.signed(1 << 6).abs();
    let nudge = seeds.signed(1);
    let origin_z = seeds.signed(DOMAIN / 2);
    let height = origin_z + dz * q;
    let origin = [a + nudge - dx * q, b - dy * q, origin_z];
    let inside = origin.iter().chain([&height]).all(|&c| c.abs() <= DOMAIN);
    let range = rim_range(seeds);
    inside.then_some(Case { origin, direction: [dx, dy, dz], height, radius: 5 * k, range })
}

fn rim_range(seeds: &mut Seeds) -> (i64, i64) {
    if seeds.next().is_multiple_of(2) {
        (i64::MIN, i64::MAX)
    } else {
        full()
    }
}

fn inexact_rim_case(seeds: &mut Seeds) -> Option<Case> {
    let scale = seeds.scale(34);
    let (rim_x, rim_y) = if seeds.next().is_multiple_of(2) {
        (3 * scale, 4 * scale)
    } else {
        (4 * scale, 3 * scale)
    };
    let (rim_x, rim_y) = (rim_x * [-1, 1][usize::try_from(seeds.next() % 2).expect("bit")], rim_y);
    let dz = seeds.scale(32) * [-1, 1][usize::try_from(seeds.next() % 2).expect("bit")];
    let reach = (ONE / dz.abs()).min(1 << 10);
    let (slope_x, slope_y) = (seeds.signed(reach), seeds.signed(reach));
    // The whole number of units between the origin and the plane, either sign.
    let steps = seeds.scale(26) * [-1, 1][usize::try_from(seeds.next() % 2).expect("bit")];
    let nudge = seeds.signed(1);
    let origin_z = seeds.signed(DOMAIN / 2);
    let height = origin_z + steps;
    let origin = [rim_x + nudge - slope_x * steps, rim_y - slope_y * steps, origin_z];
    let inside = origin.iter().chain([&height]).all(|&c| c.abs() <= DOMAIN);
    let range = rim_range(seeds);
    inside.then_some(Case {
        origin,
        direction: [dz * slope_x, dz * slope_y, dz],
        height,
        radius: 5 * scale,
        range,
    })
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_560_hits_match_an_exact_rational_oracle_on_generated_rays() {
    let mut seeds = Seeds(DISC_ORACLE_SEED);
    let mut reached = [0_usize; 3];
    let (mut behind, mut discriminating) = (0_usize, 0_usize);
    for index in 0..DISC_ORACLE_CASES {
        let case = generate(&mut seeds);
        let context = format!("seed={DISC_ORACLE_SEED:#018x} case={index} {case:?}");
        let expected = oracle(case);
        assert_eq!(kernel(case), expected, "{context}");
        if let Some((distance, _)) = expected {
            behind += usize::from(distance < 0);
        }
        if let Some((distance, inside)) = meeting(case) {
            let in_range =
                (i128::from(case.range.0)..=i128::from(case.range.1)).contains(&distance);
            discriminating += usize::from(in_range && inside != rounded_inside(case, distance));
        }
        reached[match expected {
            None => 0,
            Some((_, true)) => 1,
            Some((_, false)) => 2,
        }] += 1;
    }
    // Each floor is about 75 % of what the fixed seed reaches (counts in the test plan).
    let [misses, front, back] = reached;
    let context = format!(
        "seed={DISC_ORACLE_SEED:#018x} reached={reached:?} behind={behind} \
         discriminating={discriminating}"
    );
    assert!(misses >= MISS_FLOOR && front >= FRONT_FLOOR && back >= BACK_FLOOR, "{context}");
    assert!(behind >= BEHIND_FLOOR && discriminating >= DISCRIMINATING_FLOOR, "{context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_561_the_rim_is_inside_and_one_raw_unit_beyond_it_is_outside() {
    // Disc of radius 5 at z = 0; the ray falls straight down from z = 2 onto (3, 4, 0).
    let down = |x: i64, y: i64| case([x, y, 2 * ONE], [0, 0, -ONE], 0, 5 * ONE);
    assert_eq!(kernel(down(3 * ONE, 4 * ONE)), Some((2 * ONE, true)));
    assert_eq!(kernel(down(3 * ONE + 1, 4 * ONE)), None);
    assert_eq!(kernel(down(3 * ONE - 1, 4 * ONE)), Some((2 * ONE, true)));
    assert_eq!(kernel(down(-5 * ONE, 0)), Some((2 * ONE, true)));
    assert_eq!(kernel(down(0, 5 * ONE + 1)), None);
    // A slanted ray whose exact meeting point is the rim point (3, 4, 0): it travels
    // (-3, -4, -2) in four units of its direction (-0.75, -1, -0.5) per unit.
    let slanted = case([6 * ONE, 8 * ONE, 2 * ONE], [-3 * ONE / 4, -ONE, -ONE / 2], 0, 5 * ONE);
    assert_eq!(kernel(slanted), Some((4 * ONE, true)));
    let past = Case { origin: [6 * ONE + 2, 8 * ONE, 2 * ONE], ..slanted };
    assert_eq!(kernel(past), None);
    assert_eq!(oracle(past), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_562_grazing_rays_decide_on_the_exact_hit_not_the_rounded_one() {
    // `d.z = -1` raw, origin 3 raw above the plane: the ray meets it after exactly 3
    // direction lengths, at x = 3 * 2^32 raw. A disc of that radius takes the rim hit, one
    // raw unit less misses.
    let on_rim = case([0, 0, 3], [ONE, 0, -1], 0, 3 * ONE);
    assert_eq!(kernel(on_rim), Some((3 * ONE, true)));
    assert_eq!(kernel(Case { radius: 3 * ONE - 1, ..on_rim }), None);
    // Origin 2 raw above the plane with `d.z = -3` raw: the ray meets it after 2/3 of a
    // direction length, at x = 2^33 / 3 = 2863311530.67 raw. The distance rounds down to
    // 2863311530 and the rounded hit point is x = 2863311530, which is on the rim of a disc
    // of that radius. The exact hit is beyond it, so the kernel must refuse; one raw unit
    // wider it hits.
    let third = case([0, 0, 2], [ONE, 0, -3], 0, 2_863_311_530);
    assert_eq!(kernel(third), None);
    assert_eq!(kernel(Case { radius: 2_863_311_531, ..third }), Some((2_863_311_530, true)));
    // The shortest and the barely tilted directions agree with the oracle.
    for direction in [[0, 0, -1], [0, 0, 1], [1, 0, 1], [ONE, ONE, -1], [-ONE, ONE, 1]] {
        let grazing = case([5, -7, ONE], direction, -ONE, 2 * ONE);
        assert_eq!(kernel(grazing), oracle(grazing), "{direction:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_563_a_ray_parallel_to_the_plane_misses_even_in_the_plane() {
    // In the plane, through the centre and the rim: parallel, so a miss.
    let in_plane = case([-2 * ONE, 0, 7 * ONE], [ONE, 0, 0], 7 * ONE, 5 * ONE);
    assert_eq!(kernel(in_plane), None);
    assert_eq!(kernel(Case { range: (-i64::MAX, i64::MAX), ..in_plane }), None);
    // In the plane, starting at the centre, any direction in the plane.
    assert_eq!(kernel(case([0, 0, 7 * ONE], [ONE, ONE, 0], 7 * ONE, 5 * ONE)), None);
    // One raw unit above or below the plane, parallel, over the middle of the disc.
    assert_eq!(kernel(case([-2 * ONE, 0, 7 * ONE + 1], [ONE, 0, 0], 7 * ONE, 5 * ONE)), None);
    assert_eq!(kernel(case([-2 * ONE, 0, 7 * ONE - 1], [ONE, 0, 0], 7 * ONE, 5 * ONE)), None);
    // One raw unit of tilt makes the same ray a hit, one direction length away.
    let tilted = case([0, 0, 7 * ONE + 1], [ONE, 0, -1], 7 * ONE, 5 * ONE);
    assert_eq!(kernel(tilted), Some((ONE, true)));
    assert_eq!(kernel(tilted), oracle(tilted));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_564_an_origin_on_or_over_the_disc_hits_it_from_the_right_side() {
    // The origin is on the disc: distance 0, and either direction gives a hit, front face
    // on the direction that points toward -z.
    assert_eq!(kernel(case([ONE, 0, 0], [0, 0, -ONE], 0, 5 * ONE)), Some((0, true)));
    assert_eq!(kernel(case([ONE, 0, 0], [0, 0, ONE], 0, 5 * ONE)), Some((0, false)));
    // On the rim, and one raw unit past it.
    assert_eq!(kernel(case([5 * ONE, 0, 0], [0, 0, ONE], 0, 5 * ONE)), Some((0, false)));
    assert_eq!(kernel(case([5 * ONE + 1, 0, 0], [0, 0, ONE], 0, 5 * ONE)), None);
    // Above the centre: front face looking down, back face from below looking up.
    let above = case([0, 0, 3 * ONE], [0, 0, -ONE], 0, ONE);
    assert_eq!(kernel(above), Some((3 * ONE, true)));
    let below = case([0, 0, -3 * ONE], [0, 0, ONE / 2], 0, ONE);
    assert_eq!(kernel(below), Some((6 * ONE, false)));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_565_a_disc_behind_the_origin_is_not_hit() {
    // The disc is at z = -4 and the ray runs toward +z: it met the plane at distance -4.
    let behind = case([0, 0, 0], [0, 0, ONE], -4 * ONE, ONE);
    assert_eq!(kernel(behind), None);
    // A negative t_min lets the caller see it, from the front (the ray travels toward +z,
    // so it hits the -z side: back face) at distance -4.0.
    let seen = Case { range: (-8 * ONE, 0), ..behind };
    assert_eq!(kernel(seen), Some((-4 * ONE, false)));
    // The rounded-down distance is compared with the limits: just below t_min misses.
    let rounded = case([0, 0, 1], [0, 0, ONE], 0, ONE);
    assert_eq!(kernel(rounded), None);
    assert_eq!(kernel(Case { range: (-1, 0), ..rounded }), Some((-1, false)));
    // A distance that is not a whole raw unit rounds down, away from zero when negative:
    // -1/3 of a direction length is -1431655765.33 raw, which is -1431655766.
    let third = Case { range: (-ONE, 0), ..case([0, 0, 1], [0, 0, 3], 0, ONE) };
    assert_eq!(kernel(third), Some((-1_431_655_766, false)));
    // A ray pointing away from a disc ahead of the origin.
    assert_eq!(kernel(case([0, 0, 0], [0, 0, -ONE], 5 * ONE, ONE)), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_566_discs_outside_the_trace_domain_are_refused_with_a_typed_error() {
    let disc = |height: i64, radius: i64| {
        TraceDisc::new(FixedQ32_32::from_raw(height), FixedQ32_32::from_raw(radius))
    };
    let limit = TRACE_COORD_LIMIT_RAW;
    assert_eq!(disc(limit, ONE), Err(TraceDiscError::HeightOutsideTraceDomain));
    assert_eq!(disc(-limit, ONE), Err(TraceDiscError::HeightOutsideTraceDomain));
    assert_eq!(disc(i64::MIN, ONE), Err(TraceDiscError::HeightOutsideTraceDomain));
    assert_eq!(disc(i64::MAX, ONE), Err(TraceDiscError::HeightOutsideTraceDomain));
    assert_eq!(disc(0, 0), Err(TraceDiscError::RadiusNotPositive));
    assert_eq!(disc(0, i64::MIN), Err(TraceDiscError::RadiusNotPositive));
    assert_eq!(disc(0, -1), Err(TraceDiscError::RadiusNotPositive));
    assert_eq!(disc(0, limit), Err(TraceDiscError::RadiusOutsideTraceDomain));
    assert_eq!(disc(0, i64::MAX), Err(TraceDiscError::RadiusOutsideTraceDomain));
    let edge = disc(-DOMAIN, DOMAIN).expect("the largest disc is accepted");
    assert_eq!(edge.height(), FixedQ32_32::from_raw(-DOMAIN));
    assert_eq!(edge.radius(), FixedQ32_32::from_raw(DOMAIN));
    let bounds = edge.bounds();
    assert_eq!(
        (bounds.min, bounds.max),
        (vec([-DOMAIN, -DOMAIN, -DOMAIN]), vec([DOMAIN, DOMAIN, -DOMAIN]))
    );
    assert_eq!(disc(0, 1).map(|d| d.radius()), Ok(FixedQ32_32::from_raw(1)));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_567_the_widest_inputs_match_the_oracle_without_overflow() {
    // Every corner origin against the largest discs at both extreme heights, with the
    // longest and the shortest directions: the widest intermediates of the kernel occur here.
    let signs = [-1, 1];
    for (sx, sy, sz) in
        signs.iter().flat_map(|&x| signs.iter().flat_map(move |&y| signs.map(|z| (x, y, z))))
    {
        let origin = [sx * DOMAIN, sy * DOMAIN, sz * DOMAIN];
        for direction in [[-sx * ONE, -sy * ONE, -sz * ONE], [-sx, -sy, -sz], [ONE, -ONE, -sz]] {
            for height in [-DOMAIN, 0, DOMAIN] {
                for radius in [1, ONE, DOMAIN] {
                    let context = format!("{origin:?} {direction:?} h={height} r={radius}");
                    let widest = case(origin, direction, height, radius);
                    assert_eq!(kernel(widest), oracle(widest), "{context}");
                }
            }
        }
    }
    let ray = TraceRay::from_ray(&bunny_geom::FixedRay3::new(
        vec([-DOMAIN, DOMAIN, DOMAIN]),
        bunny_linalg::FixedUnitVec3::try_from_unit(vec([0, 0, -ONE - 1])).expect("unit"),
    ))
    .expect("unit-direction ray");
    let disc = TraceDisc::new(FixedQ32_32::from_raw(-DOMAIN), FixedQ32_32::from_raw(DOMAIN))
        .expect("disc");
    let hit = ray.hit_disc(&disc, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX));
    let expected = oracle(case([-DOMAIN, DOMAIN, DOMAIN], [0, 0, -ONE - 1], -DOMAIN, DOMAIN));
    assert_eq!(hit.map(|h| (h.distance.raw(), h.front_face)), expected);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_569_the_distance_limits_are_inclusive_at_both_ends() {
    // A ray falling 2.0 onto the disc: the hit distance is exactly 2 * 2^32 raw.
    let hit = case([0, 0, 2 * ONE], [0, 0, -ONE], 0, ONE);
    let at = |low: i64, high: i64| kernel(Case { range: (low, high), ..hit });
    let expected = Some((2 * ONE, true));
    assert_eq!(at(2 * ONE, 3 * ONE), expected, "distance exactly on t_min");
    assert_eq!(at(ONE, 2 * ONE), expected, "distance exactly on t_max");
    assert_eq!(at(2 * ONE, 2 * ONE), expected, "t_min == t_max == distance");
    assert_eq!(at(2 * ONE + 1, 3 * ONE), None, "one raw unit below t_min");
    assert_eq!(at(ONE, 2 * ONE - 1), None, "one raw unit past t_max");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_570_the_contract_identity_is_public() {
    assert_eq!(RAY_DISC_ID, "bunny-ray-disc/1");
}
