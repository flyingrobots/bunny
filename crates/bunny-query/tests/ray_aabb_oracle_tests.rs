//! Oracles for the flyingrobots/bunny#211 contract change of `ray_intersects_aabb`.
//!
//! Two independent oracles check the function over seeded corpora:
//!
//! 1. **Differential.** `reference` is a verbatim copy of `ray_intersects_aabb` as it was before
//!    the change (flyingrobots/bunny `origin/main` at e4289c4). Wherever the reference returns
//!    `Some`, the current function must return the same bits. This is the part of the contract
//!    the change may not touch.
//! 2. **Exact.** `exact_slab` decides hit or miss with `i128` rational arithmetic and
//!    cross-multiplication, without any rounding. Wherever the reference returns `None`, the
//!    current function must agree with it. Wherever the current function returns `Some`, each
//!    bound must equal the exact bound rounded to nearest (ties to even) and saturated to the
//!    Q32.32 range, and `enter <= exit`.
//!
//! The reference may report a hit that the exact test rejects only at a rounding tie: the rounded
//! bounds are then equal, or the exit rounds to zero. Such hits are unchanged and are counted.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::ray_intersects_aabb;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const ORDINARY_SEED: u64 = 0xB002_0211_0000_0001;
const OVERFLOW_SEED: u64 = 0xB002_0211_0000_0002;
const ORDINARY_CASES: usize = 200_000;
const OVERFLOW_CASES: usize = 200_000;

/// The function before the flyingrobots/bunny#211 change, copied verbatim (doc comment removed).
mod reference {
    use std::cmp::{max, min};

    use bunny_geom::{FixedAabb3, FixedRay3};
    use bunny_num::FixedQ32_32;

    pub(crate) fn ray_intersects_aabb(
        ray: &FixedRay3,
        aabb: &FixedAabb3,
    ) -> Option<(FixedQ32_32, FixedQ32_32)> {
        let dir = ray.direction.into_inner();
        let mut interval = RayInterval::unbounded();

        interval.update(AxisBounds::new(ray.origin.x, dir.x, aabb.min.x, aabb.max.x))?;
        interval.update(AxisBounds::new(ray.origin.y, dir.y, aabb.min.y, aabb.max.y))?;
        interval.update(AxisBounds::new(ray.origin.z, dir.z, aabb.min.z, aabb.max.z))?;

        if interval.is_miss() {
            None
        } else {
            Some((interval.enter, interval.exit))
        }
    }

    #[derive(Clone, Copy)]
    struct AxisBounds {
        origin: FixedQ32_32,
        direction: FixedQ32_32,
        min_bound: FixedQ32_32,
        max_bound: FixedQ32_32,
    }

    impl AxisBounds {
        const fn new(
            origin: FixedQ32_32,
            direction: FixedQ32_32,
            min_bound: FixedQ32_32,
            max_bound: FixedQ32_32,
        ) -> Self {
            Self { origin, direction, min_bound, max_bound }
        }
    }

    struct RayInterval {
        enter: FixedQ32_32,
        exit: FixedQ32_32,
    }

    impl RayInterval {
        const fn unbounded() -> Self {
            Self { enter: FixedQ32_32::from_raw(i64::MIN), exit: FixedQ32_32::from_raw(i64::MAX) }
        }

        fn update(&mut self, axis: AxisBounds) -> Option<()> {
            if axis.direction == FixedQ32_32::ZERO {
                return axis_contains_origin(axis.origin, axis.min_bound, axis.max_bound);
            }

            let t0 = axis.min_bound.checked_sub(axis.origin)?.checked_div(axis.direction)?;
            let t1 = axis.max_bound.checked_sub(axis.origin)?.checked_div(axis.direction)?;
            self.enter = max(self.enter, min(t0, t1));
            self.exit = min(self.exit, max(t0, t1));
            Some(())
        }

        fn is_miss(&self) -> bool {
            self.enter > self.exit || self.exit < FixedQ32_32::ZERO
        }
    }

    fn axis_contains_origin(
        origin: FixedQ32_32,
        min_bound: FixedQ32_32,
        max_bound: FixedQ32_32,
    ) -> Option<()> {
        (origin >= min_bound && origin <= max_bound).then_some(())
    }

    /// Whether the reference stops on a checked subtraction or division (the defect's route),
    /// as opposed to a genuine zero-component miss or a completed interval. Axes are visited in
    /// the reference's order, so a zero-component miss before the overflowing axis wins.
    pub(crate) fn overflows(ray: &FixedRay3, aabb: &FixedAabb3) -> bool {
        let dir = ray.direction.into_inner();
        let axes = [
            (ray.origin.x, dir.x, aabb.min.x, aabb.max.x),
            (ray.origin.y, dir.y, aabb.min.y, aabb.max.y),
            (ray.origin.z, dir.z, aabb.min.z, aabb.max.z),
        ];
        for (origin, direction, lo, hi) in axes {
            if direction == FixedQ32_32::ZERO {
                if axis_contains_origin(origin, lo, hi).is_none() {
                    return false;
                }
                continue;
            }
            let fits = |bound: FixedQ32_32| {
                bound.checked_sub(origin).and_then(|d| d.checked_div(direction))
            };
            if fits(lo).is_none() || fits(hi).is_none() {
                return true;
            }
        }
        false
    }
}

/// An exact rational `num / den` with `den > 0`, in Q32.32 units of distance.
#[derive(Clone, Copy, Debug)]
struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    fn slab(bound: FixedQ32_32, origin: FixedQ32_32, direction: FixedQ32_32) -> Self {
        let diff = i128::from(bound.to_raw()) - i128::from(origin.to_raw());
        let dir = i128::from(direction.to_raw());
        // Cross products stay below (2^64) * (2^63) = 2^127.
        Self { num: diff * dir.signum(), den: dir.abs() }
    }

    fn lt(self, other: Self) -> bool {
        self.num * other.den < other.num * self.den
    }

    /// Round to the nearest raw Q32.32 value (ties to even) and clamp to `i64`, computed from
    /// the floor quotient, independently of the library's magnitude-based rounding.
    fn rounded_saturated_raw(self) -> i64 {
        let scaled = self.num << 32;
        let floor = scaled.div_euclid(self.den);
        let twice_rem = 2 * scaled.rem_euclid(self.den);
        let nearest = match twice_rem.cmp(&self.den) {
            std::cmp::Ordering::Less => floor,
            std::cmp::Ordering::Greater => floor + 1,
            std::cmp::Ordering::Equal => floor + (floor & 1),
        };
        i64::try_from(nearest.clamp(i128::from(i64::MIN), i128::from(i64::MAX))).expect("clamped")
    }
}

/// Exact slab result: hit or miss, and the exact interval (`None` = not bounded on that side).
struct Exact {
    hit: bool,
    enter: Option<Rational>,
    exit: Option<Rational>,
}

fn exact_slab(ray: &FixedRay3, aabb: &FixedAabb3) -> Exact {
    let dir = ray.direction.into_inner();
    let axes = [
        (ray.origin.x, dir.x, aabb.min.x, aabb.max.x),
        (ray.origin.y, dir.y, aabb.min.y, aabb.max.y),
        (ray.origin.z, dir.z, aabb.min.z, aabb.max.z),
    ];
    let mut enter: Option<Rational> = None;
    let mut exit: Option<Rational> = None;
    let mut inside_flat_slabs = true;
    for (origin, direction, lo, hi) in axes {
        if direction == FixedQ32_32::ZERO {
            inside_flat_slabs &= lo <= origin && origin <= hi;
            continue;
        }
        let a = Rational::slab(lo, origin, direction);
        let b = Rational::slab(hi, origin, direction);
        let (near, far) = if b.lt(a) { (b, a) } else { (a, b) };
        enter = Some(match enter {
            Some(current) if !current.lt(near) => current,
            _ => near,
        });
        exit = Some(match exit {
            Some(current) if !far.lt(current) => current,
            _ => far,
        });
    }
    let ordered = match (enter, exit) {
        (Some(e), Some(x)) => !x.lt(e),
        _ => true,
    };
    let ahead = exit.is_none_or(|x| x.num >= 0);
    Exact { hit: inside_flat_slabs && ordered && ahead, enter, exit }
}

/// `SplitMix64`: a fixed, portable generator, so every case replays from its seed and index.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn any_i64(&mut self) -> i64 {
        i64::from_le_bytes(self.next().to_le_bytes())
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// Uniform in `-magnitude..=magnitude`.
    fn signed(&mut self, magnitude: i64) -> i64 {
        let span = u64::try_from(magnitude).expect("non-negative") * 2 + 1;
        i64::try_from(self.below(span)).expect("fits") - magnitude
    }

    fn sign(&mut self) -> i64 {
        if self.next() & 1 == 0 {
            1
        } else {
            -1
        }
    }
}

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

/// Ordinary coordinates: within +-1000 units, often whole or half units so faces line up.
fn ordinary_coord(rng: &mut Rng) -> i64 {
    match rng.below(4) {
        0 => rng.signed(1_000) * ONE_RAW,
        1 => rng.signed(2_000) * (ONE_RAW / 2),
        _ => rng.signed(1_000 * ONE_RAW),
    }
}

/// Large coordinates: anywhere in `i64`, near 2^62..2^63 in magnitude, or at the extremes.
fn large_coord(rng: &mut Rng) -> i64 {
    match rng.below(5) {
        0 => rng.any_i64(),
        1 => rng.sign() * ((1_i64 << 62) + i64::try_from(rng.below(1 << 62)).expect("fits")),
        2 => i64::MIN + i64::try_from(rng.below(1 << 40)).expect("fits"),
        3 => i64::MAX - i64::try_from(rng.below(1 << 40)).expect("fits"),
        _ => ordinary_coord(rng),
    }
}

/// A unit direction whose minor components are 0 or a few raw units, accepted by `try_from_unit`.
fn tiny_direction(rng: &mut Rng, max_tiny: u64) -> Option<FixedUnitVec3> {
    let mut components = [0_i64; 3];
    let major = usize::try_from(rng.below(3)).expect("fits");
    for (axis, component) in components.iter_mut().enumerate() {
        *component = if axis == major {
            rng.sign() * (ONE_RAW + rng.signed(1))
        } else if rng.below(4) == 0 {
            0
        } else {
            rng.sign() * (1 + i64::try_from(rng.below(max_tiny)).expect("fits"))
        };
    }
    let [x, y, z] = components;
    FixedUnitVec3::try_from_unit(FixedVec3::new(q(x), q(y), q(z)))
}

/// A normalized direction from a random vector; one component may be forced tiny or zero.
fn general_direction(rng: &mut Rng) -> Option<FixedUnitVec3> {
    let mut components =
        [rng.signed(1_000 * ONE_RAW), rng.signed(1_000 * ONE_RAW), rng.signed(1_000 * ONE_RAW)];
    if rng.below(3) == 0 {
        let axis = usize::try_from(rng.below(3)).expect("fits");
        components[axis] = rng.signed(4_000);
    }
    let [x, y, z] = components;
    FixedUnitVec3::new(FixedVec3::new(q(x), q(y), q(z)))
}

fn axis_direction(rng: &mut Rng) -> FixedUnitVec3 {
    let axes = [FixedUnitVec3::UNIT_X, FixedUnitVec3::UNIT_Y, FixedUnitVec3::UNIT_Z];
    let unit = axes[usize::try_from(rng.below(3)).expect("fits")].into_inner();
    let sign = q(rng.sign() * ONE_RAW);
    FixedUnitVec3::try_from_unit(FixedVec3::new(unit.x * sign, unit.y * sign, unit.z * sign))
        .expect("signed axis is a unit vector")
}

fn direction(rng: &mut Rng, overflow_heavy: bool) -> FixedUnitVec3 {
    loop {
        let candidate = match rng.below(4) {
            0 => Some(axis_direction(rng)),
            1 if !overflow_heavy => general_direction(rng),
            _ => tiny_direction(rng, if overflow_heavy { 4_000 } else { 64 }),
        };
        if let Some(unit) = candidate {
            return unit;
        }
    }
}

fn extent(rng: &mut Rng, overflow_heavy: bool) -> i64 {
    match rng.below(if overflow_heavy { 6 } else { 5 }) {
        0 => 0,
        1 => i64::try_from(rng.below(4_000)).expect("fits"),
        2 => i64::try_from(rng.below(10)).expect("fits") * ONE_RAW,
        3 => i64::try_from(rng.below(500 << 32)).expect("fits"),
        4 => i64::try_from(rng.below(1 << 20)).expect("fits"),
        _ => i64::try_from(rng.below(1 << 63)).expect("fits"),
    }
}

/// A box built around a centre point, grown by independent extents per side (saturating).
fn box_around(rng: &mut Rng, centre: FixedVec3, overflow_heavy: bool) -> FixedAabb3 {
    let mut lo = [0_i64; 3];
    let mut hi = [0_i64; 3];
    for (axis, c) in [centre.x, centre.y, centre.z].into_iter().enumerate() {
        lo[axis] = c.to_raw().saturating_sub(extent(rng, overflow_heavy));
        hi[axis] = c.to_raw().saturating_add(extent(rng, overflow_heavy));
        if rng.below(64) == 0 {
            std::mem::swap(&mut lo[axis], &mut hi[axis]);
        }
    }
    FixedAabb3::new(
        FixedVec3::new(q(lo[0]), q(lo[1]), q(lo[2])),
        FixedVec3::new(q(hi[0]), q(hi[1]), q(hi[2])),
    )
}

/// One seeded case: a ray and a box that it often reaches, grazes, contains, or just misses.
fn case(rng: &mut Rng, overflow_heavy: bool) -> (FixedRay3, FixedAabb3) {
    let coord = |rng: &mut Rng| if overflow_heavy { large_coord(rng) } else { ordinary_coord(rng) };
    let origin = FixedVec3::new(q(coord(rng)), q(coord(rng)), q(coord(rng)));
    let unit = direction(rng, overflow_heavy);
    let ray = FixedRay3::new(origin, unit);
    let dir = unit.into_inner();
    let centre = match rng.below(4) {
        // A point on the ray's line, ahead or behind (saturating arithmetic).
        0 | 1 => {
            let t = q(if overflow_heavy && rng.below(2) == 0 {
                large_coord(rng)
            } else {
                ordinary_coord(rng)
            });
            FixedVec3::new(origin.x + dir.x * t, origin.y + dir.y * t, origin.z + dir.z * t)
        }
        // Origin inside or on the box.
        2 => origin,
        // Anywhere.
        _ => FixedVec3::new(q(coord(rng)), q(coord(rng)), q(coord(rng))),
    };
    (ray, box_around(rng, centre, overflow_heavy))
}

#[derive(Default, Debug)]
struct Tally {
    cases: usize,
    unchanged_hits: usize,
    rounded_ties: usize,
    genuine_misses: usize,
    overflow_cases: usize,
    overflow_now_hit: usize,
    overflow_still_miss: usize,
    saturated_bounds: usize,
}

fn check(ray: &FixedRay3, aabb: &FixedAabb3, tally: &mut Tally, label: &str) {
    let old = reference::ray_intersects_aabb(ray, aabb);
    let new = ray_intersects_aabb(ray, aabb);
    let exact = exact_slab(ray, aabb);
    tally.cases += 1;

    if let Some((enter, exit)) = old {
        assert_eq!(new, old, "{label}: a previous hit changed bits for {ray:?} {aabb:?}");
        tally.unchanged_hits += 1;
        if !exact.hit {
            assert!(
                enter == exit || exit == FixedQ32_32::ZERO,
                "{label}: non-tie false hit {ray:?} {aabb:?}"
            );
            tally.rounded_ties += 1;
        }
    } else if reference::overflows(ray, aabb) {
        assert_eq!(
            new.is_some(),
            exact.hit,
            "{label}: overflow case decided wrongly {ray:?} {aabb:?}"
        );
        tally.overflow_cases += 1;
        if new.is_some() {
            tally.overflow_now_hit += 1;
        } else {
            tally.overflow_still_miss += 1;
        }
    } else {
        assert!(!exact.hit, "{label}: rounded miss contradicts the exact oracle {ray:?} {aabb:?}");
        assert_eq!(new, None, "{label}: a previous genuine miss changed {ray:?} {aabb:?}");
        tally.genuine_misses += 1;
    }

    if let Some((enter, exit)) = new {
        assert!(enter <= exit, "{label}: enter > exit {ray:?} {aabb:?}");
        let expected_enter = exact.enter.map_or(i64::MIN, Rational::rounded_saturated_raw);
        let expected_exit = exact.exit.map_or(i64::MAX, Rational::rounded_saturated_raw);
        assert_eq!(
            (enter.to_raw(), exit.to_raw()),
            (expected_enter, expected_exit),
            "{label}: bounds {ray:?} {aabb:?}"
        );
        let extreme = |raw: i64| raw == i64::MIN || raw == i64::MAX;
        if extreme(enter.to_raw()) || extreme(exit.to_raw()) {
            tally.saturated_bounds += 1;
        }
    }
}

fn run_corpus(seed: u64, cases: usize, overflow_heavy: bool) -> Tally {
    let mut rng = Rng(seed);
    let mut tally = Tally::default();
    for index in 0..cases {
        let (ray, aabb) = case(&mut rng, overflow_heavy);
        check(&ray, &aabb, &mut tally, &format!("seed {seed:#x} case {index}"));
    }
    tally
}

#[wasm_bindgen_test(unsupported = test)]
fn every_previous_hit_keeps_its_bits_on_the_ordinary_corpus() {
    // Ordinary rays, axis-parallel rays, tiny components (up to 64 raw units), touching and
    // grazing boxes built on the ray's line, origins inside boxes, and some inverted boxes.
    let tally = run_corpus(ORDINARY_SEED, ORDINARY_CASES, false);
    assert_eq!(tally.cases, ORDINARY_CASES);
    assert!(tally.unchanged_hits > ORDINARY_CASES / 4, "{tally:?}");
    assert!(tally.genuine_misses > ORDINARY_CASES / 10, "{tally:?}");
    assert!(tally.overflow_now_hit > 1_000, "{tally:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn overflow_cases_agree_with_the_exact_rational_oracle() {
    // Coordinates near and beyond 2^62 in magnitude (differences near and past 2^63), extremes of
    // i64, components of 1 to 4 000 raw units, and boxes on and off the ray's line.
    let tally = run_corpus(OVERFLOW_SEED, OVERFLOW_CASES, true);
    assert_eq!(tally.cases, OVERFLOW_CASES);
    assert!(tally.overflow_cases > OVERFLOW_CASES / 4, "{tally:?}");
    assert!(tally.overflow_now_hit > 10_000, "{tally:?}");
    assert!(tally.overflow_still_miss > 10_000, "{tally:?}");
    assert!(tally.saturated_bounds > 500, "{tally:?}");
    assert!(tally.unchanged_hits > 1_000, "{tally:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn rounding_oracle_matches_checked_div_on_representable_quotients() {
    // The exact oracle's rounding is written independently of the library; pin it to
    // `checked_div` wherever the quotient fits, including exact ties.
    let mut rng = Rng(ORDINARY_SEED ^ 0x5EED);
    for _ in 0..50_000 {
        let numerator = q(rng.any_i64() >> rng.below(64));
        let denominator = q((rng.any_i64() >> rng.below(64)) | 1);
        let ratio = Rational::slab(numerator, FixedQ32_32::ZERO, denominator);
        if let Some(quotient) = numerator.checked_div(denominator) {
            assert_eq!(
                ratio.rounded_saturated_raw(),
                quotient.to_raw(),
                "{numerator:?} / {denominator:?}"
            );
        }
    }
    // Exact ties: 1 / 2^33 raw is 0.5 raw and rounds to 0; 3 / 2^33 raw is 1.5 raw and rounds to 2.
    for (num, den) in [(1_i64, 1_i64 << 33), (3, 1 << 33), (-1, 1 << 33), (-3, 1 << 33)] {
        let ratio = Rational::slab(q(num), FixedQ32_32::ZERO, q(den));
        assert_eq!(
            Some(ratio.rounded_saturated_raw()),
            q(num).checked_div(q(den)).map(FixedQ32_32::to_raw)
        );
    }
}
