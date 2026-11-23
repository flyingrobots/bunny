//! `TraceRay::hit_sphere` (contract `bunny-ray-sphere/1`) against an independent exact
//! oracle, and named cases for tangent rays, origins inside the sphere, spheres behind the
//! origin and refused inputs (TRACE-TP-501 to TRACE-TP-506), the reviewed normal reproducers,
//! the range limits, the normal bound across radii and the contract identity (TRACE-TP-701 to
//! TRACE-TP-706).
//!
//! The oracle shares no arithmetic with the kernel. It works in its own arbitrary-precision
//! integers ([`Big`]), takes the discriminant as `b^2 - a c` (the kernel uses Lagrange's
//! identity), takes no square root, and finds each rounded root by bisection on the exact
//! predicate "raw distance `k` is at most the root". The normal is checked by properties,
//! never by the kernel's formula: a component bound against the exact rational offset at the
//! exact root, a positive dot product with that offset, and unit length by
//! `FixedUnitVec3::try_from_unit` ([`normal_holds`]). The generated rays use a fixed seed;
//! every failure message names it and the case index.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceSphere, TraceSphereError, RAY_SPHERE_ID, TRACE_COORD_LIMIT_RAW};
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const SPHERE_ORACLE_SEED: u64 = 0x5EED_5F4E_0358_0001;
const SPHERE_ORACLE_CASES: usize = 6_000;
/// The second stream that moves ranges onto the roots in [`limits`].
const SPHERE_LIMIT_SEED: u64 = 0x5EED_5F4E_0422_0001;
// Each floor is about 75 % of what the fixed seeds reach: 4 244 misses, 925 front hits,
// 831 back hits, 232 hits at a radius of at most 2^8 raw, 128 distances on t_min, 235 on
// t_max and 235 misses with a root one raw unit past t_max.
const MISS_FLOOR: usize = 3_183;
const FRONT_FLOOR: usize = 693;
const BACK_FLOOR: usize = 623;
const SMALL_RADIUS_FLOOR: usize = 174;
const ON_T_MIN_FLOOR: usize = 96;
const ON_T_MAX_FLOOR: usize = 176;
const PAST_T_MAX_FLOOR: usize = 176;
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

/// What the distance oracle expects: raw distance and front face.
type Expected = Option<(i64, bool)>;

/// What the kernel returns: raw distance, front face and the raw normal.
type Reported = Option<(i64, bool, Raw3)>;

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

/// The exact quadratic of a case, in [`Big`] integers: with raw `f = origin - center`, the
/// point at distance `t` (in direction multiples) is on the sphere when
/// `a t^2 + 2 b t + c = 0`, `a = d.d`, `b = f.d`, `c = f.f - r^2`, `disc = b^2 - a c`.
struct Exact {
    offset: Raw3,
    a: Big,
    b: Big,
    disc: Big,
}

impl Exact {
    fn new(case: Case) -> Self {
        let Case { origin, direction, center, radius, .. } = case;
        let offset: Raw3 = [0, 1, 2].map(|i| origin[i] - center[i]);
        let (a, b) = (big_dot(direction, direction), big_dot(offset, direction));
        let radius_squared = Big::from(i128::from(radius) * i128::from(radius));
        let c = big_dot(offset, offset).sub(&radius_squared);
        let disc = b.mul(&b).sub(&a.mul(&c));
        Self { offset, a, b, disc }
    }

    /// The floors of both roots in raw units, near first, or `None` when the line misses.
    ///
    /// The roots `k0 <= k1` in raw units are `(-2^32 b -+ sqrt(2^64 disc)) / a`.
    /// `k <= k0` holds exactly when `a k + 2^32 b <= 0` and `(a k + 2^32 b)^2 >= 2^64 disc`;
    /// `k <= k1` when `a k + 2^32 b <= 0` or `(a k + 2^32 b)^2 <= 2^64 disc`. Both predicates
    /// are monotone in `k`, so bisection finds the floors. No square root is taken.
    fn roots(&self) -> Option<(i128, i128)> {
        if self.disc.signum() < 0 {
            return None;
        }
        let scaled_disc = self.disc.mul(&Big::from(1 << 64));
        let scaled_b = self.b.mul(&Big::from(1 << 32));
        let signs = |k: i128| {
            let lead = self.a.mul(&Big::from(k)).add(&scaled_b);
            (lead.signum(), lead.mul(&lead).sub(&scaled_disc).signum())
        };
        let near = floor_root(|k| matches!(signs(k), (lead, gap) if lead <= 0 && gap >= 0));
        let far = floor_root(|k| matches!(signs(k), (lead, gap) if lead <= 0 || gap <= 0));
        Some((near, far))
    }
}

/// The exact distance oracle for `bunny-ray-sphere/1`: the near root's floor when it is in
/// the range, else the far root's, else no hit.
fn oracle(case: Case) -> Expected {
    let (near, far) = Exact::new(case).roots()?;
    let within = |k: i128| (i128::from(case.range.0)..=i128::from(case.range.1)).contains(&k);
    let (root, front) = if within(near) {
        (near, true)
    } else if within(far) {
        (far, false)
    } else {
        return None;
    };
    Some((i64::try_from(root).expect("within an i64 range"), front))
}

/// The sign of `p + q sqrt(disc)` for `disc >= 0`, decided exactly by squaring.
fn sign_with_root(p: &Big, q: &Big, disc: &Big) -> i32 {
    let (sp, sq) = (p.signum(), if disc.signum() == 0 { 0 } else { q.signum() });
    if sq == 0 || sp == sq {
        return if sp == 0 { sq } else { sp };
    }
    if sp == 0 {
        return sq;
    }
    match p.mul(p).sub(&q.mul(q).mul(disc)).signum() {
        0 => 0,
        1 => sp,
        _ => sq,
    }
}

/// The per-component bound on the normal, in raw units: `1/2 + 2^-24`, as
/// `NORMAL_BOUND_NUMERATOR / 2^25`. It is derived in `docs/topics/ray-tracing/README.md`
/// (Ray-Sphere, "The normal") before it is measured here.
const NORMAL_BOUND_NUMERATOR: i128 = (1 << 24) + 2;

/// Checks the kernel's normal `n` (raw) without the kernel's formula, against the exact hit
/// point at the exact root `t = (-b -+ sqrt(disc)) / a`: the exact unit normal is
/// `(f + d t) / r`, so `2^32 r a` times it is `2^32 (a f - b d) -+ 2^32 d sqrt(disc)`.
///
/// - each component is within `1/2 + 2^-24` raw units of `2^32` times the exact unit normal;
/// - `n . (f + d t) > 0`: it points outward from the exact offset;
/// - `FixedUnitVec3::try_from_unit` accepts `n`: unit length by the unit-vector rule.
fn normal_holds(case: Case, front: bool, n: Raw3) -> Result<(), String> {
    let exact = Exact::new(case);
    let dir = case.direction.map(|c| Big::from(i128::from(c)));
    let off = exact.offset.map(|c| Big::from(i128::from(c)));
    let (one, scale) = (Big::from(1 << 32), Big::from(1 << 25));
    let sign = Big::from(if front { -1 } else { 1 });
    let ra = exact.a.mul(&Big::from(i128::from(case.radius)));
    let bound = ra.mul(&Big::from(NORMAL_BOUND_NUMERATOR));
    let mut dot_p = Big::from(0);
    let mut dot_q = Big::from(0);
    for i in 0..3 {
        let point = exact.a.mul(&off[i]).sub(&exact.b.mul(&dir[i]));
        let ni = Big::from(i128::from(n[i]));
        // err * 2^25 * r a = 2^25 (n r a - 2^32 point) - (sign) 2^25 2^32 d sqrt(disc).
        let rational = scale.mul(&ni.mul(&ra).sub(&one.mul(&point)));
        let irrational = scale.mul(&one).mul(&dir[i]).mul(&sign).mul(&Big::from(-1));
        let above = sign_with_root(&rational.sub(&bound), &irrational, &exact.disc);
        let below = sign_with_root(&rational.add(&bound), &irrational, &exact.disc);
        if above > 0 || below < 0 {
            return Err(format!("component {i} of {n:?} is outside the bound"));
        }
        dot_p = dot_p.add(&ni.mul(&point));
        dot_q = dot_q.add(&ni.mul(&dir[i]).mul(&sign));
    }
    if sign_with_root(&dot_p, &dot_q, &exact.disc) <= 0 {
        return Err(format!("{n:?} does not point outward"));
    }
    FixedUnitVec3::try_from_unit(vec(n)).map(|_| ()).ok_or(format!("{n:?} is not unit"))
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

fn kernel(case: Case) -> Reported {
    let Case { origin, direction, center, radius, range } = case;
    let ray = TraceRay::new(vec(origin), vec(direction)).expect("ray in the trace domain");
    let sphere = TraceSphere::new(vec(center), FixedQ32_32::from_raw(radius)).expect("sphere");
    let (t_min, t_max) = (FixedQ32_32::from_raw(range.0), FixedQ32_32::from_raw(range.1));
    ray.hit_sphere(&sphere, t_min, t_max)
        .map(|hit| (hit.distance.raw(), hit.front_face, raw(hit.normal().into_inner())))
}

/// The kernel's answer, after checking its distance and face against [`oracle`] and its
/// normal with [`normal_holds`].
fn checked(case: Case) -> Reported {
    let got = kernel(case);
    assert_eq!(got.map(|(t, front, _)| (t, front)), oracle(case), "{case:?}");
    if let Some((_, front, n)) = got {
        normal_holds(case, front, n).unwrap_or_else(|why| panic!("{why}; {case:?}"));
    }
    got
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

/// Hits counted by the generated test, per hard region.
#[derive(Debug, Default)]
struct Reached {
    misses: usize,
    front: usize,
    back: usize,
    /// Hits by radius: at most `2^8` raw, at most `2^24`, at most `2^32` (1.0), above.
    by_radius: [usize; 4],
    /// Reported distances exactly on `t_min` and on `t_max`.
    on_limit: [usize; 2],
    /// Misses with a root exactly one raw unit past `t_max`.
    past_limit: usize,
}

/// One time in three, the range is moved onto a limit of the case's own exact roots (from
/// the oracle, a second seed stream, so the base corpus stays the one of #375): `t_min` on
/// the near root or one past it, `t_max` on the near root or one before it, or both limits
/// around the far root.
fn limits(case: Case, seeds: &mut Seeds) -> Case {
    let Some((near, far)) = Exact::new(case).roots() else { return case };
    let (Ok(near), Ok(far)) = (i64::try_from(near), i64::try_from(far)) else { return case };
    if !seeds.next().is_multiple_of(3) || near == i64::MIN || far == i64::MAX {
        return case;
    }
    let range = match seeds.next() % 6 {
        0 => (near, i64::MAX),
        1 => (near + 1, i64::MAX),
        2 => (i64::MIN, near),
        3 => (i64::MIN, near - 1),
        4 => (near + 1, far),
        _ => (near + 1, far - 1),
    };
    Case { range, ..case }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_501_hits_match_an_exact_rational_oracle_on_generated_rays() {
    let mut seeds = Seeds(SPHERE_ORACLE_SEED);
    let mut limit_seeds = Seeds(SPHERE_LIMIT_SEED);
    let mut r = Reached::default();
    for index in 0..SPHERE_ORACLE_CASES {
        let case = limits(generate(&mut seeds), &mut limit_seeds);
        let context = format!("seed={SPHERE_ORACLE_SEED:#018x} case={index} {case:?}");
        let got = kernel(case);
        assert_eq!(got.map(|(t, front, _)| (t, front)), oracle(case), "{context}");
        let Some((t, front, n)) = got else {
            r.misses += 1;
            let roots = Exact::new(case).roots();
            let past = |k: i128| k == i128::from(case.range.1) + 1;
            r.past_limit += usize::from(roots.is_some_and(|(near, far)| past(near) || past(far)));
            continue;
        };
        normal_holds(case, front, n).unwrap_or_else(|why| panic!("{why}; {context}"));
        if front {
            r.front += 1;
        } else {
            r.back += 1;
        }
        let class = [1 << 8, 1 << 24, ONE].iter().take_while(|&&c| case.radius > c).count();
        r.by_radius[class] += 1;
        r.on_limit[0] += usize::from(t == case.range.0);
        r.on_limit[1] += usize::from(t == case.range.1);
    }
    let context = format!("seed={SPHERE_ORACLE_SEED:#018x} reached={r:?}");
    let floors = [
        (r.misses, MISS_FLOOR),
        (r.front, FRONT_FLOOR),
        (r.back, BACK_FLOOR),
        (r.by_radius[0], SMALL_RADIUS_FLOOR),
        (r.on_limit[0], ON_T_MIN_FLOOR),
        (r.on_limit[1], ON_T_MAX_FLOOR),
        (r.past_limit, PAST_T_MAX_FLOOR),
    ];
    assert!(floors.iter().all(|&(count, floor)| count >= floor), "{context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_502_a_tangent_ray_hits_once_and_a_ray_one_raw_unit_outside_misses() {
    // Unit sphere at the origin; the ray runs along y = 1, touching it at (0, 1, 0).
    let tangent = checked(case([-4 * ONE, ONE, 0], [ONE, 0, 0], [0; 3], ONE));
    assert_eq!(tangent, Some((4 * ONE, true, [0, ONE, 0])));
    let outside = checked(case([-4 * ONE, ONE + 1, 0], [ONE, 0, 0], [0; 3], ONE));
    assert_eq!(outside, None);
    // One raw unit inside: a chord of half-length sqrt(2^33 - 1) raw, about 92682 raw units.
    let inside = checked(case([-4 * ONE, ONE - 1, 0], [ONE, 0, 0], [0; 3], ONE));
    let (distance, front, _) = inside.expect("the ray crosses the sphere");
    assert!(front && distance < 4 * ONE && distance > 4 * ONE - 92_683, "{distance}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_503_an_origin_inside_the_sphere_hits_the_far_side_from_inside() {
    let from_center = checked(case([0; 3], [ONE, 0, 0], [0; 3], 2 * ONE));
    assert_eq!(from_center, Some((2 * ONE, false, [ONE, 0, 0])));
    // Off-centre, with a half-length direction: the exit at x = sqrt(4 - 1) = 1.732.. is at
    // distance 2 * 1.732.. = 3.464.. in direction multiples.
    let off = checked(case([0, ONE, 0], [ONE / 2, 0, 0], [0; 3], 2 * ONE));
    let (distance, front, _) = off.expect("an inside origin always hits");
    assert!(!front && distance / (ONE / 1000) == 3464, "{distance}");
    // With t_min past the exit there is nothing left to hit.
    let past = (3 * ONE + ONE / 2, i64::MAX);
    assert_eq!(
        checked(Case { range: past, ..case([0, ONE, 0], [ONE / 2, 0, 0], [0; 3], 2 * ONE) }),
        None
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_504_a_sphere_behind_the_origin_is_not_hit() {
    // Roots at -11 and -9: both behind.
    assert_eq!(checked(case([0; 3], [ONE, 0, 0], [-10 * ONE, 0, 0], ONE)), None);
    // With a negative t_min the same sphere is hit at its near root, from outside.
    let backwards = checked(Case {
        range: (-20 * ONE, 0),
        ..case([0; 3], [ONE, 0, 0], [-10 * ONE, 0, 0], ONE)
    });
    assert_eq!(backwards, Some((-11 * ONE, true, [-ONE, 0, 0])));
    // Pointing away from a sphere ahead of the origin.
    assert_eq!(checked(case([0; 3], [-ONE, 0, 0], [5 * ONE, 0, 0], ONE)), None);
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
                checked(case(origin, direction, center, radius));
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
    let unit_case = case([-DOMAIN, 0, 0], [ONE + 1, 0, 0], [DOMAIN, 0, 0], DOMAIN);
    let hit = hit.expect("the ray crosses the sphere");
    assert_eq!(Some((hit.distance.raw(), hit.front_face)), oracle(unit_case));
    let n = raw(hit.normal().into_inner());
    normal_holds(unit_case, hit.front_face, n).expect("normal within the bound");
}

/// The seed's cases whose old floored normal was the zero vector (radius 1 raw each), from
/// the independent review of flyingrobots/bunny#375.
const SEED_ZERO_NORMAL_CASES: [usize; 7] = [22, 129, 757, 988, 1275, 1628, 2635];

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_701_a_one_raw_unit_sphere_hit_from_its_centre_has_a_unit_normal() {
    // Radius 1 raw, origin at the centre, direction (1, 1, 1): the far root floors to 0 and
    // the old normal was (0, 0, 0). The exact normal is (1, 1, 1) / sqrt 3, 2479700524.87.. raw.
    let zero = checked(case([0; 3], [ONE, ONE, ONE], [0; 3], 1));
    let (distance, front, n) = zero.expect("an origin inside always hits");
    assert_eq!((distance, front), (0, false));
    assert!(n.iter().all(|&c| (2_479_700_524..=2_479_700_525).contains(&c)), "{n:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_702_a_sphere_of_radius_one_hundredth_has_a_unit_normal() {
    // Radius 0.01, from (-1, -1, -1) along (1, 1, 1): the old normal was 2^32 + 56.2 raw
    // units long and each component 32.4 raw units from the exact one.
    let small = case([-ONE, -ONE, -ONE], [ONE, ONE, ONE], [12_345, -777, 4_242], 42_949_673);
    let (distance, front, _) = checked(small).expect("the ray crosses the sphere");
    assert_eq!((distance, front), (4_270_175_561, true));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_703_the_seeds_seven_old_zero_normals_are_unit_normals() {
    // The base corpus of TRACE-TP-501, unmoved by the range limits; every one of the seven
    // is checked before the test fails, so a failure names all that break.
    let mut seeds = Seeds(SPHERE_ORACLE_SEED);
    let corpus: Vec<Case> = (0..=SEED_ZERO_NORMAL_CASES[6]).map(|_| generate(&mut seeds)).collect();
    let mut broken = Vec::new();
    for index in SEED_ZERO_NORMAL_CASES {
        let case = corpus[index];
        assert_eq!(case.radius, 1, "case {index}");
        let got = kernel(case);
        assert_eq!(got.map(|(t, front, _)| (t, front)), oracle(case), "case {index}");
        let (_, front, n) = got.expect("each of the seven hits");
        if let Err(why) = normal_holds(case, front, n) {
            broken.push(format!("case {index}: {why}"));
        }
    }
    assert!(broken.is_empty(), "{broken:#?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_704_the_distance_limits_are_inclusive_at_both_ends() {
    // The unit sphere from (-4, 0, 0) along +x: exact roots at 3.0 (near) and 5.0 (far).
    let hit = case([-4 * ONE, 0, 0], [ONE, 0, 0], [0; 3], ONE);
    let at = |low: i64, high: i64| checked(Case { range: (low, high), ..hit });
    let near = Some((3 * ONE, true, [-ONE, 0, 0]));
    let far = Some((5 * ONE, false, [ONE, 0, 0]));
    assert_eq!(at(3 * ONE, 4 * ONE), near, "near root exactly on t_min");
    assert_eq!(at(3 * ONE + 1, 6 * ONE), far, "near root one raw unit below t_min");
    assert_eq!(at(0, 3 * ONE), near, "near root exactly on t_max");
    assert_eq!(at(0, 3 * ONE - 1), None, "near root one raw unit past t_max");
    assert_eq!(at(3 * ONE, 3 * ONE), near, "t_min == t_max == near root");
    assert_eq!(at(3 * ONE + 1, 5 * ONE), far, "far root exactly on t_max");
    assert_eq!(at(3 * ONE + 1, 5 * ONE - 1), None, "far root one raw unit past t_max");
    assert_eq!(at(5 * ONE, 6 * ONE), far, "far root exactly on t_min");
    assert_eq!(at(5 * ONE + 1, 6 * ONE), None, "far root one raw unit below t_min");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_705_the_normal_bound_holds_from_the_smallest_radius_to_the_largest() {
    // A skew direction through an off-axis centre at radius 1 raw, 0.01, 1.0 and the largest,
    // from outside (near root) and from the centre (far root). Below the largest radius the
    // outside origin is three direction lengths back from the centre; the largest sphere is
    // reached from the domain corner, along the offset to its centre shifted down 9 bits.
    let skew = [ONE, -(ONE / 3), ONE / 7];
    for radius in [1, 42_949_673, ONE, DOMAIN] {
        let center = [radius / 5, radius / 9, -(radius / 11)];
        let (origin, direction) = if radius < DOMAIN {
            ([0, 1, 2].map(|i| center[i] - 3 * skew[i]), skew)
        } else {
            ([-DOMAIN; 3], [0, 1, 2].map(|i| (center[i] + DOMAIN) >> 9))
        };
        let outside = checked(case(origin, direction, center, radius));
        assert!(outside.is_some_and(|(_, front, _)| front), "radius {radius} from outside");
        let inside = checked(case(center, direction, center, radius));
        assert!(inside.is_some_and(|(_, front, _)| !front), "radius {radius} from the centre");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_706_the_contract_identity_is_public() {
    assert_eq!(RAY_SPHERE_ID, "bunny-ray-sphere/1");
}
