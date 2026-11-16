//! `TraceRay::hit_sphere` (contract `bunny-ray-sphere/1`) against an independent exact
//! oracle, and named cases for tangent rays, origins inside the sphere, spheres behind the
//! origin and refused inputs (TRACE-TP-501 to TRACE-TP-506).
//!
//! The oracle shares no arithmetic with the kernel. It works in its own arbitrary-precision
//! integers ([`Big`]), takes the discriminant as `b^2 - a c` (the kernel uses Lagrange's
//! identity), takes no square root, and finds each rounded root by bisection on the exact
//! predicate "raw distance `k` is at most the root". The generated rays use a fixed seed;
//! every failure message names it and the case index.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceSphere, TraceSphereError, TRACE_COORD_LIMIT_RAW};
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const SPHERE_ORACLE_SEED: u64 = 0x5EED_5F4E_0358_0001;
const SPHERE_ORACLE_CASES: usize = 6_000;
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

fn big_dot(a: Raw3, b: Raw3) -> Big {
    (0..3).fold(Big::from(0), |sum, i| {
        sum.add(&Big::from(i128::from(a[i])).mul(&Big::from(i128::from(b[i]))))
    })
}

/// What the oracle expects: raw distance, front face and raw normal.
type Expected = Option<(i64, bool, Raw3)>;

/// One ray, one sphere and one distance range, all raw.
#[derive(Clone, Copy, Debug)]
struct Case {
    origin: Raw3,
    direction: Raw3,
    center: Raw3,
    radius: i64,
    range: (i64, i64),
}

/// A case over distances `0..=i64::MAX` raw.
fn case(origin: Raw3, direction: Raw3, center: Raw3, radius: i64) -> Case {
    Case { origin, direction, center, radius, range: full() }
}

/// The exact oracle for `bunny-ray-sphere/1`.
///
/// With raw `f = origin - center`, the point at raw distance `k` is `(f * 2^32 + d k) / 2^32`
/// raw. The roots `k0 <= k1` of `|f * 2^32 + d k|^2 = r^2 * 2^64` are
/// `(-2^32 b -+ sqrt(2^64 (b^2 - a c))) / a`, with `a = d.d`, `b = f.d`, `c = f.f - r^2`.
/// `k <= k0` holds exactly when `a k + 2^32 b <= 0` and `(a k + 2^32 b)^2 >= 2^64 disc`;
/// `k <= k1` when `a k + 2^32 b <= 0` or `(a k + 2^32 b)^2 <= 2^64 disc`. Both predicates
/// are monotone in `k`, so bisection finds the floors.
fn oracle(case: Case) -> Expected {
    let Case { origin, direction, center, radius, range } = case;
    let offset: Raw3 = [0, 1, 2].map(|i| origin[i] - center[i]);
    let (quad, half) = (big_dot(direction, direction), big_dot(offset, direction));
    let radius_squared = Big::from(i128::from(radius) * i128::from(radius));
    let constant = big_dot(offset, offset).sub(&radius_squared);
    let disc = half.mul(&half).sub(&quad.mul(&constant));
    if disc.signum() < 0 {
        return None;
    }
    let scaled_disc = disc.mul(&Big::from(1 << 64));
    let scaled_half = half.mul(&Big::from(1 << 32));
    // `(sign of a k + 2^32 b, sign of (a k + 2^32 b)^2 - 2^64 disc)`.
    let signs = |k: i128| {
        let lead = quad.mul(&Big::from(k)).add(&scaled_half);
        (lead.signum(), lead.mul(&lead).sub(&scaled_disc).signum())
    };
    let near = floor_root(|k| matches!(signs(k), (lead, gap) if lead <= 0 && gap >= 0));
    let far = floor_root(|k| matches!(signs(k), (lead, gap) if lead <= 0 || gap <= 0));
    let within = |k: i128| (i128::from(range.0)..=i128::from(range.1)).contains(&k);
    let (root, front) = if within(near) {
        (near, true)
    } else if within(far) {
        (far, false)
    } else {
        return None;
    };
    let root = i64::try_from(root).expect("within an i64 range");
    let normal = [0, 1, 2].map(|i| {
        let along = i128::from(direction[i]) * i128::from(root);
        let numerator = (i128::from(offset[i]) << 32) + along;
        i64::try_from(numerator.div_euclid(i128::from(radius))).expect("normal fits")
    });
    Some((root, front, normal))
}

/// The largest `k` in `-2^110..2^110` for which `at_most(k)` holds; `at_most` is true below
/// some point and false above it. Every root in the trace domain is inside this range.
fn floor_root(at_most: impl Fn(i128) -> bool) -> i128 {
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

fn vec(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn raw(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn kernel(case: Case) -> Expected {
    let Case { origin, direction, center, radius, range } = case;
    let ray = TraceRay::new(vec(origin), vec(direction)).expect("ray in the trace domain");
    let sphere = TraceSphere::new(vec(center), FixedQ32_32::from_raw(radius)).expect("sphere");
    let (t_min, t_max) = (FixedQ32_32::from_raw(range.0), FixedQ32_32::from_raw(range.1));
    ray.hit_sphere(&sphere, t_min, t_max)
        .map(|hit| (hit.distance.raw(), hit.front_face, raw(hit.normal())))
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

/// A generated case: often aimed at the sphere, with the aim off by up to about the radius
/// (so many rays pass close to tangent), the origin inside one time in five, and a random
/// distance range one time in four.
fn generate(seeds: &mut Seeds) -> Case {
    let radius = seeds.scale(39);
    let center = seeds.triple(DOMAIN - radius.min(DOMAIN));
    let inside = seeds.next().is_multiple_of(5);
    let reach = if inside { radius / 2 } else { DOMAIN };
    let origin = [0, 1, 2].map(|i| (center[i] + seeds.signed(reach)).clamp(-DOMAIN, DOMAIN));
    let jitter = seeds.triple(radius + radius / 4);
    let aim: Raw3 = [0, 1, 2].map(|i| center[i] + jitter[i] - origin[i]);
    let direction =
        if seeds.next().is_multiple_of(4) { seeds.triple(ONE) } else { fit_direction(aim, seeds) };
    let range = if seeds.next().is_multiple_of(4) {
        let low = seeds.signed(i64::MAX / 2);
        (low, low.saturating_add(seeds.signed(i64::MAX / 2).abs()))
    } else {
        full()
    };
    let direction = if direction == [0; 3] { [ONE, 0, 0] } else { direction };
    Case { origin, direction, center, radius, range }
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
fn trace_tp_501_hits_match_an_exact_rational_oracle_on_generated_rays() {
    let mut seeds = Seeds(SPHERE_ORACLE_SEED);
    let mut reached = [0_usize; 3];
    for index in 0..SPHERE_ORACLE_CASES {
        let case = generate(&mut seeds);
        let context = format!("seed={SPHERE_ORACLE_SEED:#018x} case={index} {case:?}");
        let expected = oracle(case);
        assert_eq!(kernel(case), expected, "{context}");
        reached[match expected {
            None => 0,
            Some((_, true, _)) => 1,
            Some((_, false, _)) => 2,
        }] += 1;
    }
    // Each floor is about 75 % of what the fixed seed reaches (counts in the test plan).
    let [misses, front, back] = reached;
    let context = format!("seed={SPHERE_ORACLE_SEED:#018x} reached={reached:?}");
    assert!(misses >= 3_070 && front >= 765 && back >= 655, "{context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_502_a_tangent_ray_hits_once_and_a_ray_one_raw_unit_outside_misses() {
    // Unit sphere at the origin; the ray runs along y = 1, touching it at (0, 1, 0).
    let tangent = kernel(case([-4 * ONE, ONE, 0], [ONE, 0, 0], [0; 3], ONE));
    assert_eq!(tangent, Some((4 * ONE, true, [0, ONE, 0])));
    let outside = kernel(case([-4 * ONE, ONE + 1, 0], [ONE, 0, 0], [0; 3], ONE));
    assert_eq!(outside, None);
    // One raw unit inside: a chord of half-length sqrt(2^33 - 1) raw, about 92682 raw units.
    let inside = kernel(case([-4 * ONE, ONE - 1, 0], [ONE, 0, 0], [0; 3], ONE));
    assert_eq!(inside, oracle(case([-4 * ONE, ONE - 1, 0], [ONE, 0, 0], [0; 3], ONE)));
    let (distance, front, _) = inside.expect("the ray crosses the sphere");
    assert!(front && distance < 4 * ONE && distance > 4 * ONE - 92_683, "{distance}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_503_an_origin_inside_the_sphere_hits_the_far_side_from_inside() {
    let from_center = kernel(case([0; 3], [ONE, 0, 0], [0; 3], 2 * ONE));
    assert_eq!(from_center, Some((2 * ONE, false, [ONE, 0, 0])));
    // Off-centre, with a half-length direction: the exit at x = sqrt(4 - 1) = 1.732.. is at
    // distance 2 * 1.732.. = 3.464.. in direction multiples.
    let off = kernel(case([0, ONE, 0], [ONE / 2, 0, 0], [0; 3], 2 * ONE));
    assert_eq!(off, oracle(case([0, ONE, 0], [ONE / 2, 0, 0], [0; 3], 2 * ONE)));
    let (distance, front, _) = off.expect("an inside origin always hits");
    assert!(!front && distance / (ONE / 1000) == 3464, "{distance}");
    // With t_min past the exit there is nothing left to hit.
    let past = (3 * ONE + ONE / 2, i64::MAX);
    assert_eq!(
        kernel(Case { range: past, ..case([0, ONE, 0], [ONE / 2, 0, 0], [0; 3], 2 * ONE) }),
        None
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_504_a_sphere_behind_the_origin_is_not_hit() {
    // Roots at -11 and -9: both behind.
    assert_eq!(kernel(case([0; 3], [ONE, 0, 0], [-10 * ONE, 0, 0], ONE)), None);
    // With a negative t_min the same sphere is hit at its near root, from outside.
    let backwards =
        kernel(Case { range: (-20 * ONE, 0), ..case([0; 3], [ONE, 0, 0], [-10 * ONE, 0, 0], ONE) });
    assert_eq!(backwards, Some((-11 * ONE, true, [-ONE, 0, 0])));
    // Pointing away from a sphere ahead of the origin.
    assert_eq!(kernel(case([0; 3], [-ONE, 0, 0], [5 * ONE, 0, 0], ONE)), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_505_spheres_outside_the_trace_domain_are_refused_with_a_typed_error() {
    let sphere =
        |center: Raw3, radius: i64| TraceSphere::new(vec(center), FixedQ32_32::from_raw(radius));
    let limit = TRACE_COORD_LIMIT_RAW;
    assert_eq!(sphere([limit, 0, 0], ONE), Err(TraceSphereError::CenterOutsideTraceDomain));
    assert_eq!(sphere([0, 0, -limit], ONE), Err(TraceSphereError::CenterOutsideTraceDomain));
    assert_eq!(sphere([i64::MIN, 0, 0], ONE), Err(TraceSphereError::CenterOutsideTraceDomain));
    assert_eq!(sphere([0; 3], 0), Err(TraceSphereError::RadiusNotPositive));
    assert_eq!(sphere([0; 3], i64::MIN), Err(TraceSphereError::RadiusNotPositive));
    assert_eq!(sphere([0; 3], limit), Err(TraceSphereError::RadiusOutsideTraceDomain));
    assert_eq!(sphere([0; 3], i64::MAX), Err(TraceSphereError::RadiusOutsideTraceDomain));
    let edge = sphere([DOMAIN, -DOMAIN, DOMAIN], DOMAIN).expect("the largest sphere is accepted");
    assert_eq!(edge.center(), vec([DOMAIN, -DOMAIN, DOMAIN]));
    assert_eq!(edge.radius(), FixedQ32_32::from_raw(DOMAIN));
    assert_eq!(raw(edge.bounds().min), [0, -2 * DOMAIN, 0]);
    assert_eq!(raw(edge.bounds().max), [2 * DOMAIN, 0, 2 * DOMAIN]);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_506_the_widest_inputs_match_the_oracle_without_overflow() {
    // Every corner origin against the largest spheres at opposite corners, with the longest
    // and the shortest directions; the widest intermediates of the kernel occur here.
    let signs = [-1, 1];
    for (sx, sy, sz) in
        signs.iter().flat_map(|&x| signs.iter().flat_map(move |&y| signs.map(|z| (x, y, z))))
    {
        let origin = [sx * DOMAIN, sy * DOMAIN, sz * DOMAIN];
        for direction in [[-sx * ONE, -sy * ONE, -sz * ONE], [-sx, 0, 0], [-sx * ONE, -sy, 0]] {
            for (center, radius) in [([0; 3], DOMAIN), (origin.map(|c| -c), DOMAIN), ([0; 3], 1)] {
                let context =
                    format!("origin={origin:?} direction={direction:?} center={center:?}");
                let widest = case(origin, direction, center, radius);
                assert_eq!(kernel(widest), oracle(widest), "{context}");
            }
        }
    }
    let ray = TraceRay::from_ray(&bunny_geom::FixedRay3::new(
        vec([-DOMAIN, 0, 0]),
        bunny_linalg::FixedUnitVec3::try_from_unit(vec([ONE + 1, 0, 0])).expect("unit"),
    ))
    .expect("unit-direction ray");
    let sphere =
        TraceSphere::new(vec([DOMAIN, 0, 0]), FixedQ32_32::from_raw(DOMAIN)).expect("sphere");
    let hit = ray.hit_sphere(&sphere, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX));
    let expected = oracle(case([-DOMAIN, 0, 0], [ONE + 1, 0, 0], [DOMAIN, 0, 0], DOMAIN));
    assert_eq!(hit.map(|h| (h.distance.raw(), h.front_face, raw(h.normal()))), expected);
}
