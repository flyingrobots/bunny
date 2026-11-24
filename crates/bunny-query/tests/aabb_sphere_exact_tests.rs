//! Exactness of `aabb_intersects_sphere` (flyingrobots/bunny#501, `docs/topics/aabb-sphere-overlap/`).
//!
//! The oracle is written here and shares no helper with the kernel: integers are held as
//! eight base-`2^32` limbs and squared by schoolbook multiplication. `reference_before_501`
//! is a copy of the routine as it was before the fix (Q32.32 products rounded to nearest,
//! ties to even, and saturating sums), kept to pin which answers changed and which did not.
//!
//! Seeds and case counts are the constants below; each seeded test prints its seed and counts.

use core::cmp::Ordering;

use bunny_geom::{FixedAabb3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{aabb_intersects_sphere, closest_point_aabb};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;

/// The smallest radius magnitude (raw) whose rounded Q32.32 square saturates to `i64::MAX`
/// (about 46 340.95 units). From this radius up, the routine before the fix reported every
/// box as meeting the sphere.
const SATURATING_RADIUS_RAW: i64 = 199_032_864_766_431;

/// A squared distance and a squared radius (raw units squared) this close can be ordered
/// differently by the rounded products of the old routine than by the exact comparison.
const ROUNDING_BAND: u128 = 1 << 33;

const ORDINARY_SEED: u64 = 0x0501_A55E_0000_0001;
const ORDINARY_CASES: usize = 240_000;
const SMALL_SEED: u64 = 0x0501_A55E_0000_0002;
const SMALL_CASES: usize = 200_000;
const EXTREME_SEED: u64 = 0x0501_A55E_0000_0003;
const EXTREME_CASES: usize = 200_000;

// ---------------------------------------------------------------------------------------------
// Independent oracle: base-2^32 limbs.
// ---------------------------------------------------------------------------------------------

/// A non-negative integer as eight little-endian base-`2^32` limbs, each below `2^32`.
type Limbs = [u64; 8];

const LIMB_MASK: u64 = 0xFFFF_FFFF;

fn limbs_square(value: u64) -> Limbs {
    let digits = [value & LIMB_MASK, value >> 32];
    let mut out = [0_u64; 8];
    for (i, a) in digits.iter().enumerate() {
        for (j, b) in digits.iter().enumerate() {
            // Each partial product is below 2^64; split it into two limbs and carry.
            let product = a * b;
            out[i + j] += product & LIMB_MASK;
            out[i + j + 1] += product >> 32;
        }
    }
    normalize(out)
}

fn limbs_add(a: Limbs, b: Limbs) -> Limbs {
    let mut out = [0_u64; 8];
    for (slot, (x, y)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *slot = x + y;
    }
    normalize(out)
}

fn normalize(mut limbs: Limbs) -> Limbs {
    let mut carry = 0_u64;
    for limb in &mut limbs {
        let total = *limb + carry;
        *limb = total & LIMB_MASK;
        carry = total >> 32;
    }
    assert_eq!(carry, 0, "oracle value fits eight limbs");
    limbs
}

fn limbs_cmp(a: &Limbs, b: &Limbs) -> Ordering {
    for (x, y) in a.iter().rev().zip(b.iter().rev()) {
        match x.cmp(y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

/// `|a - b|` for two raw coordinates, below `2^64`.
fn raw_gap(a: i64, b: i64) -> u64 {
    let (a, b) = (i128::from(a), i128::from(b));
    let gap = if a >= b { a - b } else { b - a };
    u64::try_from(gap).expect("a gap between two i64 values is below 2^64")
}

/// The box read as given, per axis: below the minimum gives the minimum, else above the
/// maximum gives the maximum, else the coordinate itself.
fn oracle_clamp(value: i64, lo: i64, hi: i64) -> i64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

struct OracleAnswer {
    closest: [i64; 3],
    distance_sq: Limbs,
    radius_sq: Limbs,
}

impl OracleAnswer {
    fn new(aabb: &FixedAabb3, sphere: &FixedSphere3) -> Self {
        let c = raw3(sphere.center);
        let lo = raw3(aabb.min);
        let hi = raw3(aabb.max);
        let closest = [
            oracle_clamp(c[0], lo[0], hi[0]),
            oracle_clamp(c[1], lo[1], hi[1]),
            oracle_clamp(c[2], lo[2], hi[2]),
        ];
        let mut distance_sq = [0_u64; 8];
        for (p, q) in closest.iter().zip(c.iter()) {
            distance_sq = limbs_add(distance_sq, limbs_square(raw_gap(*p, *q)));
        }
        let radius_sq = limbs_square(sphere.radius.to_raw().unsigned_abs());
        Self { closest, distance_sq, radius_sq }
    }

    fn answer(&self) -> Option<[i64; 3]> {
        (limbs_cmp(&self.distance_sq, &self.radius_sq) != Ordering::Greater).then_some(self.closest)
    }

    /// `|distance^2 - radius^2| <= bound`, for a bound below `2^64`.
    fn within(&self, bound: u128) -> bool {
        let (big, small) = match limbs_cmp(&self.distance_sq, &self.radius_sq) {
            Ordering::Less => (&self.radius_sq, &self.distance_sq),
            _ => (&self.distance_sq, &self.radius_sq),
        };
        // Subtract limb by limb with borrow.
        let mut diff = [0_u64; 8];
        let mut borrow = 0_u64;
        for (slot, (x, y)) in diff.iter_mut().zip(big.iter().zip(small.iter())) {
            let need = y + borrow;
            if *x >= need {
                *slot = x - need;
                borrow = 0;
            } else {
                *slot = x + (1 << 32) - need;
                borrow = 1;
            }
        }
        if diff[3..].iter().any(|limb| *limb != 0) {
            return false;
        }
        let value = u128::from(diff[0]) | (u128::from(diff[1]) << 32) | (u128::from(diff[2]) << 64);
        value <= bound
    }
}

fn oracle(aabb: &FixedAabb3, sphere: &FixedSphere3) -> Option<[i64; 3]> {
    OracleAnswer::new(aabb, sphere).answer()
}

// ---------------------------------------------------------------------------------------------
// The routine before flyingrobots/bunny#501, and when it saturated.
// ---------------------------------------------------------------------------------------------

fn reference_before_501(aabb: &FixedAabb3, sphere: &FixedSphere3) -> Option<FixedVec3> {
    let closest = closest_point_aabb(aabb, sphere.center);
    let diff = closest - sphere.center;
    let dist_sq = diff.dot(diff);
    let r2 = sphere.radius * sphere.radius;
    if dist_sq <= r2 {
        Some(closest)
    } else {
        None
    }
}

/// `round_half_even(value / 2^32)`.
fn rhe_shift_32(value: u128) -> u128 {
    let q = value >> 32;
    let r = value & u128::from(LIMB_MASK);
    let half = 1_u128 << 31;
    if r > half || (r == half && q & 1 == 1) {
        q + 1
    } else {
        q
    }
}

/// True when some step of `reference_before_501` left the `i64` raw range and saturated: a
/// coordinate difference, a rounded product, a partial sum, or the rounded squared radius.
fn reference_saturates(aabb: &FixedAabb3, sphere: &FixedSphere3) -> bool {
    let limit = u128::try_from(MAX).expect("positive");
    let closest = raw3(closest_point_aabb(aabb, sphere.center));
    let centre = raw3(sphere.center);
    let mut sum = 0_u128;
    for (p, c) in closest.iter().zip(centre.iter()) {
        let diff = i128::from(*p) - i128::from(*c);
        if diff > i128::from(MAX) || diff < i128::from(MIN) {
            return true;
        }
        let term = rhe_shift_32(diff.unsigned_abs() * diff.unsigned_abs());
        if term > limit {
            return true;
        }
        sum += term;
        if sum > limit {
            return true;
        }
    }
    let radius = u128::from(sphere.radius.to_raw().unsigned_abs());
    rhe_shift_32(radius * radius) > limit
}

// ---------------------------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------------------------

fn fx(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn v3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fx(raw[0]), fx(raw[1]), fx(raw[2]))
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.to_raw(), v.y.to_raw(), v.z.to_raw()]
}

fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(v3(min), v3(max))
}

fn point_box(at: [i64; 3]) -> FixedAabb3 {
    aabb(at, at)
}

fn sphere(centre: [i64; 3], radius: i64) -> FixedSphere3 {
    FixedSphere3::new(v3(centre), fx(radius))
}

fn kernel(aabb: &FixedAabb3, sphere: &FixedSphere3) -> Option<[i64; 3]> {
    let answer = aabb_intersects_sphere(aabb, sphere).map(raw3);
    if let Some(point) = answer {
        assert_eq!(
            point,
            raw3(closest_point_aabb(aabb, sphere.center)),
            "returns the closest point"
        );
    }
    answer
}

fn assert_exact(aabb: &FixedAabb3, sphere: &FixedSphere3) {
    assert_eq!(kernel(aabb, sphere), oracle(aabb, sphere), "box {aabb:?}, sphere {sphere:?}");
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn wide(&mut self) -> u64 {
        (self.next() << 32) ^ self.next()
    }

    /// A value in `-bound..=bound`, for `bound < 2^62`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("small bound");
        i64::try_from(self.wide() % span).expect("below the span") - bound
    }

    fn any_i64(&mut self) -> i64 {
        i64::from_ne_bytes(self.wide().to_ne_bytes())
    }

    fn pick(&mut self, values: &[i64]) -> i64 {
        let len = u64::try_from(values.len()).expect("a short list");
        values[usize::try_from(self.next() % len).expect("below the length")]
    }

    /// A box with an independent minimum and maximum per axis, so about half the axes are
    /// inverted unless `ordered`.
    fn box_within(&mut self, bound: i64, ordered: bool) -> FixedAabb3 {
        let mut min = [0_i64; 3];
        let mut max = [0_i64; 3];
        for (lo, hi) in min.iter_mut().zip(max.iter_mut()) {
            let (a, b) = (self.signed(bound), self.signed(bound));
            (*lo, *hi) = if ordered { (a.min(b), a.max(b)) } else { (a, b) };
        }
        aabb(min, max)
    }
}

// ---------------------------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------------------------

/// ASO-TP-001: the example of flyingrobots/bunny#501.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_001_issue_example_is_not_an_intersection() {
    let far = point_box([MIN, MIN, MIN]);
    let ball = sphere([MAX, MAX, MAX], 65_536 * ONE_RAW);
    assert!(reference_saturates(&far, &ball));
    assert_eq!(reference_before_501(&far, &ball).map(raw3), Some([MIN, MIN, MIN]), "the defect");
    assert_eq!(kernel(&far, &ball), None);
    assert_eq!(oracle(&far, &ball), None);
}

/// ASO-TP-002: the boundary of the defect. Below `SATURATING_RADIUS_RAW` the rounded squared
/// radius fits and the old routine answered the far box correctly; from it up the square
/// saturated and every box was reported as met. The fix answers both exactly.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_002_defect_boundary_is_the_saturating_radius() {
    let limit = u128::try_from(MAX).expect("positive");
    let below = u128::try_from(SATURATING_RADIUS_RAW - 1).expect("positive");
    let at = u128::try_from(SATURATING_RADIUS_RAW).expect("positive");
    assert!(rhe_shift_32(below * below) < limit);
    assert!(rhe_shift_32(at * at) > limit);

    let far = point_box([MIN, MIN, MIN]);
    for sign in [1, -1] {
        let before = sphere([MAX, MAX, MAX], sign * (SATURATING_RADIUS_RAW - 1));
        let from = sphere([MAX, MAX, MAX], sign * SATURATING_RADIUS_RAW);
        assert_eq!(reference_before_501(&far, &before), None);
        assert!(reference_before_501(&far, &from).is_some(), "old routine: always met");
        assert_eq!(kernel(&far, &before), None);
        assert_eq!(kernel(&far, &from), None);
    }

    // At the saturating radius the old answer was `Some` for a box at any distance; the fix
    // reports a hit only within the radius, touching included.
    let r = SATURATING_RADIUS_RAW;
    let ball = sphere([0, 0, 0], r);
    assert!(reference_before_501(&point_box([r + 1, 0, 0]), &ball).is_some());
    assert_eq!(kernel(&point_box([r + 1, 0, 0]), &ball), None);
    assert_eq!(kernel(&point_box([r, 0, 0]), &ball), Some([r, 0, 0]));
    assert_eq!(kernel(&point_box([r - 1, 0, 0]), &ball), Some([r - 1, 0, 0]));
}

/// ASO-TP-003: touching at ordinary magnitudes, and one raw unit either side.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_003_touching_ordinary_magnitudes() {
    // (3, 4, 12) has length 13.
    for k in [1, 7, ONE_RAW / 3, ONE_RAW, 250 * ONE_RAW] {
        let target = point_box([0, 0, 0]);
        let ball = sphere([3 * k, -4 * k, 12 * k], 13 * k);
        assert_eq!(kernel(&target, &ball), Some([0, 0, 0]), "touching, k = {k}");
        assert_eq!(kernel(&target, &sphere([3 * k, -4 * k, 12 * k], 13 * k - 1)), None);
        assert_eq!(kernel(&target, &sphere([3 * k, -4 * k, 12 * k + 1], 13 * k)), None);
        assert_eq!(kernel(&target, &sphere([3 * k, -4 * k, 12 * k], -13 * k)), Some([0, 0, 0]));
        assert_exact(&target, &ball);
    }
    // A face contact on a real box: the centre is `r` above the top face.
    let unit = aabb([-ONE_RAW; 3], [ONE_RAW; 3]);
    let r = ONE_RAW / 2 + 3;
    let centre = [5, -7, ONE_RAW + r];
    assert_eq!(kernel(&unit, &sphere(centre, r)), Some([5, -7, ONE_RAW]));
    assert_eq!(kernel(&unit, &sphere(centre, r - 1)), None);
    // The centre inside the box: distance 0, met for every radius, zero included.
    assert_eq!(kernel(&unit, &sphere([1, 2, 3], 0)), Some([1, 2, 3]));
    // A zero radius meets only the box itself.
    assert_eq!(kernel(&unit, &sphere([ONE_RAW, 0, 0], 0)), Some([ONE_RAW, 0, 0]));
    assert_eq!(kernel(&unit, &sphere([ONE_RAW + 1, 0, 0], 0)), None);
}

/// ASO-TP-004: touching at the ends of the raw range, where the squared distance needs more
/// than 128 bits.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_004_touching_extreme_magnitudes() {
    // Radius `i64::MIN` has magnitude 2^63, the largest representable.
    let origin = point_box([0, 0, 0]);
    assert_eq!(kernel(&origin, &sphere([MIN, 0, 0], MIN)), Some([0, 0, 0]));
    assert_eq!(kernel(&point_box([1, 0, 0]), &sphere([MIN, 0, 0], MIN)), None);
    assert_eq!(kernel(&origin, &sphere([MIN, 0, 0], MAX)), None);
    assert_eq!(kernel(&origin, &sphere([MIN + 1, 0, 0], MAX)), Some([0, 0, 0]));

    // (3k, 4k, 12k) with 13k <= i64::MAX.
    let k = MAX / 13;
    let ball = sphere([-3 * k, -4 * k, -12 * k], 13 * k);
    assert_eq!(kernel(&origin, &ball), Some([0, 0, 0]));
    assert_eq!(kernel(&origin, &sphere([-3 * k, -4 * k, -12 * k], 13 * k - 1)), None);
    assert_eq!(kernel(&origin, &sphere([-3 * k, -4 * k, -12 * k - 1], 13 * k)), None);
    assert_exact(&origin, &ball);

    // Gaps of 2^64 - 1 on one, two and three axes: 2^128 and more, never met.
    let corner = point_box([MAX, MAX, MAX]);
    assert_eq!(kernel(&corner, &sphere([MIN, MAX, MAX], MIN)), None);
    assert_eq!(kernel(&corner, &sphere([MIN, MIN, MAX], MIN)), None);
    assert_eq!(kernel(&corner, &sphere([MIN, MIN, MIN], MIN)), None);
    // A gap of exactly 2^63 on each of two axes: 2^127 > 2^126.
    assert_eq!(kernel(&origin, &sphere([MIN, MIN, 0], MIN)), None);
}

/// ASO-TP-005: every corner of the raw range, against the oracle.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_005_raw_range_corners_match_the_oracle() {
    let ends = [MIN, MAX];
    let mut boxes = Vec::new();
    for x in ends {
        for y in ends {
            for z in ends {
                boxes.push(point_box([x, y, z]));
            }
        }
    }
    boxes.push(aabb([MIN; 3], [MAX; 3]));
    boxes.push(aabb([MAX; 3], [MIN; 3]));
    boxes.push(aabb([MIN, 0, MAX], [MAX, 0, MIN]));

    let coords = [MIN, MIN + 1, -1, 0, 1, MAX - 1, MAX];
    let radii = [0, 1, -1, SATURATING_RADIUS_RAW - 1, SATURATING_RADIUS_RAW, MAX, MIN, MIN + 1];
    let mut cases = 0_usize;
    for target in &boxes {
        for x in coords {
            for y in coords {
                for z in coords {
                    for r in radii {
                        assert_exact(target, &sphere([x, y, z], r));
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 11 * 343 * 8);
}

/// ASO-TP-006: seeded inputs across the whole raw range, against the oracle.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_006_seeded_full_range_matches_the_oracle() {
    let mut seeds = Seeds(EXTREME_SEED);
    let special = [MIN, MIN + 1, -1, 0, 1, MAX - 1, MAX, SATURATING_RADIUS_RAW];
    let (mut met, mut missed) = (0_usize, 0_usize);
    for _ in 0..EXTREME_CASES {
        let mut draw =
            || if seeds.next() % 8 == 0 { seeds.pick(&special) } else { seeds.any_i64() };
        let min = [draw(), draw(), draw()];
        let max = [draw(), draw(), draw()];
        let centre = [draw(), draw(), draw()];
        let radius = draw();
        let target = aabb(min, max);
        let ball = sphere(centre, radius);
        assert_exact(&target, &ball);
        if oracle(&target, &ball).is_some() {
            met += 1;
        } else {
            missed += 1;
        }
    }
    eprintln!("aso_tp_006 seed {EXTREME_SEED:#x}: {EXTREME_CASES} cases, {met} met, {missed} not");
    assert!(met > 0 && missed > 0);
}

/// ASO-TP-007: at ordinary magnitudes (coordinates and radii within 2^40 raw, 256 units),
/// ordered and inverted boxes, positive and negative radii, the answer is unchanged from the
/// routine before the fix wherever that routine did not saturate and the two squares are
/// more than `ROUNDING_BAND` apart; inside the band the answer is the oracle's.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_007_ordinary_magnitudes_unchanged() {
    let bound = 1_i64 << 40;
    let mut seeds = Seeds(ORDINARY_SEED);
    let (mut compared, mut inverted, mut negative, mut met, mut in_band) = (0, 0, 0, 0, 0);
    for case in 0..ORDINARY_CASES {
        let target = seeds.box_within(bound, case % 2 == 0);
        let centre = [seeds.signed(bound), seeds.signed(bound), seeds.signed(bound)];
        // Radii up to 2^40 raw, with a share near the distance so touching is exercised.
        let radius = seeds.signed(bound);
        let ball = sphere(centre, radius);
        let new = kernel(&target, &ball);
        assert_eq!(new, oracle(&target, &ball));
        assert!(!reference_saturates(&target, &ball), "ordinary inputs never saturated");
        if OracleAnswer::new(&target, &ball).within(ROUNDING_BAND) {
            in_band += 1;
            continue;
        }
        assert_eq!(new, reference_before_501(&target, &ball).map(raw3), "case {case}");
        compared += 1;
        inverted += usize::from(raw3(target.min).iter().zip(raw3(target.max)).any(|(a, b)| *a > b));
        negative += usize::from(radius < 0);
        met += usize::from(new.is_some());
    }
    eprintln!(
        "aso_tp_007 seed {ORDINARY_SEED:#x}: {ORDINARY_CASES} cases, {compared} compared \
         ({inverted} inverted, {negative} negative radius, {met} met), {in_band} in band"
    );
    assert!(compared >= 200_000);
    assert!(inverted > 50_000 && negative > 50_000 && met > 10_000);
}

/// ASO-TP-008: the rounding band. At small magnitudes the old routine's rounded products
/// could order the squares wrongly; the fix answers the exact comparison, and every changed
/// answer lies within `ROUNDING_BAND` of touching.
#[wasm_bindgen_test(unsupported = test)]
fn aso_tp_008_rounding_band_is_exact_now() {
    // A zero radius one raw unit outside a point box: 0 <= 0 before, 1 > 0 exactly.
    let origin = point_box([0, 0, 0]);
    let tiny = sphere([1, 0, 0], 0);
    assert_eq!(reference_before_501(&origin, &tiny).map(raw3), Some([0, 0, 0]));
    assert_eq!(kernel(&origin, &tiny), None);
    // Three products rounded up past a squared radius rounded down: 771 > 770 before,
    // 3 * 1_049_600^2 = 3_304_980_480_000 <= 1_817_961^2 = 3_304_982_197_521 exactly.
    let a = 1_049_600;
    let near = sphere([a, a, a], 1_817_961);
    assert_eq!(reference_before_501(&origin, &near), None);
    assert_eq!(kernel(&origin, &near), Some([0, 0, 0]));

    let bound = 1_i64 << 20;
    let mut seeds = Seeds(SMALL_SEED);
    let (mut changed, mut kept) = (0_usize, 0_usize);
    for case in 0..SMALL_CASES {
        let target = seeds.box_within(bound, case % 2 == 0);
        let centre = [seeds.signed(bound), seeds.signed(bound), seeds.signed(bound)];
        let ball = sphere(centre, seeds.signed(bound));
        let new = kernel(&target, &ball);
        let answer = OracleAnswer::new(&target, &ball);
        assert_eq!(new, answer.answer());
        if new == reference_before_501(&target, &ball).map(raw3) {
            kept += 1;
        } else {
            assert!(answer.within(ROUNDING_BAND), "changed outside the band: case {case}");
            changed += 1;
        }
    }
    eprintln!(
        "aso_tp_008 seed {SMALL_SEED:#x}: {SMALL_CASES} cases, {changed} changed, {kept} kept"
    );
    assert!(changed > 0 && kept > changed);
}
