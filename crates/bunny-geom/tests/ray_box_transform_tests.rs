//! Ray and box transforms under a `FixedAffine3` (flyingrobots/bunny#108): the unnormalised ray
//! image and its parameter bound, the eight-corner box hull, refusals one raw unit either side
//! of each threshold, seeded oracles and pinned goldens.
//!
//! Every oracle here is exact `i128` arithmetic written in this file: products are rounded by
//! `div_euclid`, `rem_euclid` and a comparison of twice the remainder, range checks use
//! `i64::try_from`, and exact values are carried as `q * 2^32 + r` so a sum of three full-range
//! products never leaves `i128`. The goldens come from a separate Python model of the
//! documented rule (see `docs/topics/transform/test-plan.md`).

use bunny_geom::{FixedAabb3, FixedRay3, GeomError, TransformError, TransformedRay3};
use bunny_linalg::{FixedAffine3, FixedMat3, FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const SEED: u64 = 0xB002_0108_0000_0001;
const CASES: usize = 10_000;
/// `round(2^32 / sqrt(2))`: the rounded cosine and sine of an eighth turn.
const C45: i64 = 3_037_000_500;

type Rows = [[i64; 3]; 3];

fn v3(v: [i64; 3]) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(v[0]), raw(v[1]), raw(v[2]))
}

fn r3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn affine(m: Rows, t: [i64; 3]) -> FixedAffine3 {
    FixedAffine3::from_parts(FixedMat3::from_rows(v3(m[0]), v3(m[1]), v3(m[2])), v3(t))
}

fn diagonal(s: [i64; 3]) -> Rows {
    [[s[0], 0, 0], [0, s[1], 0], [0, 0, s[2]]]
}

fn unit(v: [i64; 3]) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(v3(v)).expect("the test direction is a unit vector")
}

fn ray(origin: [i64; 3], direction: [i64; 3]) -> FixedRay3 {
    FixedRay3::new(v3(origin), unit(direction))
}

fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(v3(min), v3(max))
}

fn bounds(b: FixedAabb3) -> ([i64; 3], [i64; 3]) {
    (r3(b.min), r3(b.max))
}

fn image(r: TransformedRay3) -> ([i64; 3], [i64; 3]) {
    (r3(r.origin), r3(r.direction))
}

/// `numerator / 2^32` rounded to nearest, ties to even.
fn round32(numerator: i128) -> i128 {
    let unit = 1_i128 << 32;
    let (floor, remainder) = (numerator.div_euclid(unit), numerator.rem_euclid(unit));
    floor + i128::from(2 * remainder > unit || (2 * remainder == unit && floor % 2 != 0))
}

fn in_range(value: i128) -> Option<i64> {
    i64::try_from(value).ok()
}

/// The documented rule: each product rounded once, every product and partial sum in range,
/// then the translation added and checked.
fn oracle_apply(m: Rows, t: [i64; 3], p: [i64; 3]) -> Option<[i64; 3]> {
    let mut out = [0_i64; 3];
    for ((slot, row), shift) in out.iter_mut().zip(m).zip(t) {
        let mut sum = 0_i64;
        for (entry, coordinate) in row.into_iter().zip(p) {
            let product = in_range(round32(i128::from(entry) * i128::from(coordinate)))?;
            sum = in_range(i128::from(sum) + i128::from(product))?;
        }
        *slot = in_range(i128::from(sum) + i128::from(shift))?;
    }
    Some(out)
}

/// The exact `row . p + t` in units of `2^-64`, as `(q, r)` with value `q * 2^32 + r`.
fn exact(row: [i64; 3], p: [i64; 3], t: i64) -> (i128, i128) {
    let unit = 1_i128 << 32;
    let (mut q, mut r) = (i128::from(t), 0_i128);
    for (entry, coordinate) in row.into_iter().zip(p) {
        let product = i128::from(entry) * i128::from(coordinate);
        q += product.div_euclid(unit);
        r += product.rem_euclid(unit);
    }
    (q + r.div_euclid(unit), r.rem_euclid(unit))
}

/// Whether `raw` is within 3/2 raw units of the exact `(q, r)`.
fn within_three_halves(raw: i64, (q, r): (i128, i128)) -> bool {
    let difference = (i128::from(raw) - q).checked_mul(1 << 32).map(|d| d - r);
    difference.is_some_and(|d| d.abs() <= 3 << 31)
}

fn corner(min: [i64; 3], max: [i64; 3], index: usize) -> [i64; 3] {
    let pick = |axis: usize| if index >> axis & 1 == 0 { min[axis] } else { max[axis] };
    [pick(0), pick(1), pick(2)]
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_020_ray_image_keeps_its_parameter() {
    // Scale (2, 1, 1), then translate by (1, 0, 0): the ray from (1, 0, 0) along +X.
    let transform = affine(diagonal([2 * ONE, ONE, ONE]), [ONE, 0, 0]);
    let moved = ray([ONE, 0, 0], [ONE, 0, 0]).checked_transform(&transform).expect("in range");
    // Origin 2 * 1 + 1 = 3; direction (2, 0, 0), not renormalised.
    assert_eq!(image(moved), ([3 * ONE, 0, 0], [2 * ONE, 0, 0]));
    // The original point at t = 3 is (4, 0, 0); its image is (9, 0, 0) = origin + 3 * direction.
    assert_eq!(
        oracle_apply(diagonal([2 * ONE, ONE, ONE]), [ONE, 0, 0], [4 * ONE, 0, 0]),
        Some([9 * ONE, 0, 0])
    );
    // Renormalising gives a unit direction, so t = 3 now reaches (6, 0, 0), not (9, 0, 0).
    let renormalised = moved.try_into_ray().expect("nonzero direction");
    assert_eq!(renormalised.origin, v3([3 * ONE, 0, 0]));
    assert_eq!(renormalised.direction, FixedUnitVec3::UNIT_X);
    // Identity maps the ray onto itself; translation moves the origin only.
    let tilted = ray([5, -7, ONE], [2_576_980_378, 3_435_973_837, 0]);
    let same = tilted.checked_transform(&FixedAffine3::IDENTITY).expect("identity");
    assert_eq!(image(same), (r3(tilted.origin), r3(tilted.direction.into_inner())));
    let shifted = tilted.checked_transform(&affine(diagonal([ONE; 3]), [ONE, 2, -3]));
    assert_eq!(image(shifted.expect("in range")).1, r3(tilted.direction.into_inner()));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_021_ray_refusals_one_raw_unit_either_side() {
    let along_x = |x: i64| ray([0; 3], [x, 0, 0]);
    // m00 = i64::MAX raw times the unit direction 2^32 is exactly i64::MAX; 2^32 + 1 is past it.
    let big = affine(diagonal([i64::MAX, ONE, ONE]), [0; 3]);
    assert_eq!(image(along_x(ONE).checked_transform(&big).expect("on")).1, [i64::MAX, 0, 0]);
    assert_eq!(along_x(ONE + 1).checked_transform(&big), Err(TransformError::OutOfRange));
    assert!(along_x(ONE - 1).checked_transform(&big).is_ok());
    let low = affine(diagonal([i64::MIN, ONE, ONE]), [0; 3]);
    assert_eq!(image(along_x(ONE).checked_transform(&low).expect("on")).1, [i64::MIN, 0, 0]);
    assert_eq!(along_x(ONE + 1).checked_transform(&low), Err(TransformError::OutOfRange));
    // Origin: translation i64::MAX - 5 raw; origin x = 5 reaches i64::MAX, 6 is past it.
    let shift = affine(diagonal([ONE; 3]), [i64::MAX - 5, 0, 0]);
    let from = |x: i64| ray([x, 0, 0], [0, ONE, 0]).checked_transform(&shift);
    assert_eq!(image(from(5).expect("on")).0, [i64::MAX, 0, 0]);
    assert_eq!(from(6), Err(TransformError::OutOfRange));
    assert!(from(4).is_ok());
    // A singular scale collapses +X to zero.
    let flat = affine(diagonal([0, ONE, ONE]), [0; 3]);
    assert_eq!(along_x(ONE).checked_transform(&flat), Err(TransformError::DegenerateDirection));
    // m00 = 1 raw: the image x is round(d_x / 2^32); 2^31 is a tie to even (0), 2^31 + 1 is 1.
    let tiny = affine(diagonal([1, 0, 0]), [0; 3]);
    let half = |x: i64, y: i64| ray([0; 3], [x, y, 0]).checked_transform(&tiny);
    assert_eq!(half((1 << 31) - 1, 3_719_550_787), Err(TransformError::DegenerateDirection));
    assert_eq!(half(1 << 31, 3_719_550_786), Err(TransformError::DegenerateDirection));
    assert_eq!(image(half((1 << 31) + 1, 3_719_550_786).expect("nonzero")).1, [1, 0, 0]);
    // The origin is checked first: an origin out of range and a zero direction is OutOfRange.
    let skew = affine([[0, 2 * ONE, 0], [0; 3], [0; 3]], [0; 3]);
    let both = ray([0, 1 << 62, 0], [ONE, 0, 0]).checked_transform(&skew);
    assert_eq!(both, Err(TransformError::OutOfRange));
    let only_direction = ray([0, (1 << 62) - 1, 0], [ONE, 0, 0]).checked_transform(&skew);
    assert_eq!(only_direction, Err(TransformError::DegenerateDirection));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_022_seeded_rays_match_the_oracle_and_the_parameter_bound() {
    let mut state = SEED;
    let mut counts = [0_usize; 3];
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 0), vec(&mut state, 0));
        let origin = vec(&mut state, 0);
        let Some(direction) = random_unit(&mut state) else { continue };
        let got = FixedRay3::new(v3(origin), direction).checked_transform(&affine(m, t));
        let want = oracle_ray(m, t, origin, r3(direction.into_inner()));
        assert_eq!(got.map(image), want, "seed {SEED:#x}, case {case}");
        counts[match want {
            Ok(_) => 0,
            Err(TransformError::OutOfRange) => 1,
            Err(_) => 2,
        }] += 1;
    }
    assert_eq!(counts, EXPECTED_RAY_COUNTS, "seed {SEED:#x}");
    let mut state = SEED ^ 2;
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 25), vec(&mut state, 17));
        let origin = vec(&mut state, 17);
        let Some(direction) = random_unit(&mut state) else { continue };
        // The parameter t, raw, below 2^46 (16 384 units).
        let along = i128::from(next64(&mut state) >> 17).abs();
        let d = r3(direction.into_inner());
        let moved = FixedRay3::new(v3(origin), direction).checked_transform(&affine(m, t));
        let (o2, d2) = image(moved.expect("the moderate domain is in range"));
        // In units of 2^-96: the transformed ray at t against the exact image of the original
        // ray at t, within 3/2 (1 + |t|) raw.
        for axis in [0, 1, 2] {
            let got = (i128::from(o2[axis]) << 64) + along * i128::from(d2[axis]) * (1 << 32);
            let mut want = i128::from(t[axis]) << 64;
            for ((entry, oj), dj) in m[axis].into_iter().zip(origin).zip(d) {
                want += i128::from(entry) * ((i128::from(oj) << 32) + along * i128::from(dj));
            }
            let bound = (3_i128 << 63) + (3_i128 << 31) * along;
            assert!((got - want).abs() <= bound, "seed {:#x}, case {case}, axis {axis}", SEED ^ 2);
        }
    }
}

/// How many seeded full-range rays are in range, out of range and degenerate.
const EXPECTED_RAY_COUNTS: [usize; 3] = [4_115, 4_790, 0];

fn oracle_ray(
    m: Rows,
    t: [i64; 3],
    origin: [i64; 3],
    direction: [i64; 3],
) -> Result<([i64; 3], [i64; 3]), TransformError> {
    let o2 = oracle_apply(m, t, origin).ok_or(TransformError::OutOfRange)?;
    let d2 = oracle_apply(m, [0; 3], direction).ok_or(TransformError::OutOfRange)?;
    if d2 == [0; 3] {
        return Err(TransformError::DegenerateDirection);
    }
    Ok((o2, d2))
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_023_box_hull_of_a_rotated_cube_and_a_point() {
    // An eighth turn about +Z: rows (c, -c, 0), (c, c, 0), (0, 0, 1). The unit cube's corners
    // map to x in {0, c, -c, 0} and y in {0, c, c, 2c}; every product is exact.
    let turn = affine([[C45, -C45, 0], [C45, C45, 0], [0, 0, ONE]], [0; 3]);
    let cube = aabb([0; 3], [ONE; 3]);
    let hull = cube.checked_transform(&turn).expect("in range");
    assert_eq!(bounds(hull), ([-C45, 0, 0], [C45, 2 * C45, ONE]));
    // The hull is 2c by 2c by 1, about 2.0, while the rotated cube has volume 1: the axis-aligned
    // box of a rotated box is larger than the image.
    let area = i128::from(2 * C45) * i128::from(2 * C45);
    assert!(area > (2_i128 << 64) - (1 << 34) && area < (2_i128 << 64) + (1 << 34));
    // A negative scale swaps which corner gives the minimum; translation shifts both faces.
    let flip = affine(diagonal([-2 * ONE, ONE, 3 * ONE]), [ONE, 0, -ONE]);
    let moved = aabb([ONE, -ONE, 0], [2 * ONE, ONE, ONE]).checked_transform(&flip);
    assert_eq!(bounds(moved.expect("in range")), ([-3 * ONE, -ONE, -ONE], [-ONE, ONE, 2 * ONE]));
    // The identity gives the box back; a translation moves both corners by it.
    let odd = aabb([-5, 7, -ONE - 3], [ONE + 1, 9, 0]);
    assert_eq!(odd.checked_transform(&FixedAffine3::IDENTITY), Ok(odd));
    let shift = affine(diagonal([ONE; 3]), [ONE, -2, 3]);
    let shifted = odd.checked_transform(&shift).map(bounds);
    assert_eq!(shifted, Ok(([ONE - 5, 5, -ONE], [2 * ONE + 1, 7, 3])));
    // A point box gives the point transform exactly, as a point box.
    let general = affine([[ONE + 3, -7, 11], [5, ONE - 1, 1 << 31], [-1, 9, 2 * ONE]], [3, -5, 7]);
    let p = [123_456_789, -987_654_321, 5];
    let want = general.checked_transform_point(v3(p)).map(r3).expect("in range");
    let point_box = aabb(p, p).checked_transform(&general).expect("in range");
    assert_eq!(bounds(point_box), (want, want));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_024_box_refusals_one_raw_unit_either_side() {
    let identity = FixedAffine3::IDENTITY;
    for axis in [0, 1, 2] {
        let mut max = [ONE; 3];
        max[axis] = -1;
        let inverted = aabb([0; 3], max);
        assert_eq!(inverted.checked_transform(&identity), Err(TransformError::InvertedBox));
        assert_eq!(
            FixedAabb3::try_new(inverted.min, inverted.max),
            Err(GeomError::InvalidAabbBounds)
        );
        max[axis] = 0;
        assert!(aabb([0; 3], max).checked_transform(&identity).is_ok());
    }
    // An inverted box is refused before any arithmetic, even under an overflowing transform.
    let huge = affine(diagonal([i64::MAX; 3]), [i64::MAX; 3]);
    assert_eq!(aabb([ONE; 3], [0; 3]).checked_transform(&huge), Err(TransformError::InvertedBox));
    // Translation i64::MAX - 5 raw: max x = 5 reaches i64::MAX, 6 is past it.
    let up = affine(diagonal([ONE; 3]), [i64::MAX - 5, 0, 0]);
    let reach = |x: i64| aabb([0; 3], [x, 0, 0]).checked_transform(&up);
    assert_eq!(bounds(reach(5).expect("on")).1, [i64::MAX, 0, 0]);
    assert_eq!(reach(6), Err(TransformError::OutOfRange));
    assert!(reach(4).is_ok());
    // Translation i64::MIN + 5 raw: min x = -5 reaches i64::MIN, -6 is past it.
    let down = affine(diagonal([ONE; 3]), [i64::MIN + 5, 0, 0]);
    let reach = |x: i64| aabb([x, 0, 0], [0; 3]).checked_transform(&down);
    assert_eq!(bounds(reach(-5).expect("on")).0, [i64::MIN, 0, 0]);
    assert_eq!(reach(-6), Err(TransformError::OutOfRange));
    assert!(reach(-4).is_ok());
    // A product: 2 * (2^62 - 1) raw fits, 2 * 2^62 does not, whichever corner it is.
    let double = affine(diagonal([2 * ONE, ONE, ONE]), [0; 3]);
    let reach = |x: i64| aabb([-1, 0, 0], [x, 0, 0]).checked_transform(&double);
    assert_eq!(bounds(reach((1 << 62) - 1).expect("on")).1, [i64::MAX - 1, 0, 0]);
    assert_eq!(reach(1 << 62), Err(TransformError::OutOfRange));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_025_seeded_boxes_contain_every_rounded_corner() {
    let mut state = SEED ^ 1;
    let mut in_range_count = 0;
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 0), vec(&mut state, 0));
        let (a, b) = (vec(&mut state, 0), vec(&mut state, 0));
        let (min, max) = (minmax(a, b, i64::min), minmax(a, b, i64::max));
        let got = aabb(min, max).checked_transform(&affine(m, t)).map(bounds);
        let corners: Option<Vec<[i64; 3]>> =
            (0..8).map(|i| oracle_apply(m, t, corner(min, max, i))).collect();
        let Some(corners) = corners else {
            assert_eq!(got, Err(TransformError::OutOfRange), "seed {:#x}, case {case}", SEED ^ 1);
            continue;
        };
        in_range_count += 1;
        let (lo, hi) = got.expect("every corner is in range");
        for axis in [0, 1, 2] {
            let values = corners.iter().map(|c| c[axis]);
            assert_eq!(lo[axis], values.clone().min().unwrap_or_default(), "case {case}");
            assert_eq!(hi[axis], values.max().unwrap_or_default(), "case {case}");
            let exacts = (0..8).map(|i| exact(m[axis], corner(min, max, i), t[axis]));
            assert!(within_three_halves(lo[axis], exacts.clone().min().unwrap_or_default()));
            assert!(within_three_halves(hi[axis], exacts.max().unwrap_or_default()));
        }
    }
    assert_eq!(in_range_count, EXPECTED_BOXES_IN_RANGE, "seed {:#x}", SEED ^ 1);
}

/// How many of the seeded full-range boxes are in range.
const EXPECTED_BOXES_IN_RANGE: usize = 2_370;

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_029_pinned_goldens() {
    let m = [
        [0x1_8000_0001, -0x4000_0003, 0x7],
        [0x2000_0005, 0x2_0000_0000, -0xC000_0001],
        [-0x11, 0x8000_0000, 0x5555_5555],
    ];
    let transform = affine(m, [0x3_0000_0001, -0x1_8000_0000, 0x7]);
    let r = ray([0x1_2345_6789, -0x9876_5432, 0x2_0000_0001], [2_576_980_378, 3_435_973_837, 0]);
    let moved = r.checked_transform(&transform).expect("in range");
    assert_eq!(
        image(moved),
        ([0x4_DB05_B06C, -0x4_0C83_FB70, 0x5E6F_8085], [0xB333_3332, 0x1_ACCC_CCD0, 0x6666_665C])
    );
    let b =
        aabb([-0x1_0000_0003, 0x8000_0001, -0x2_0000_0000], [0x2_0000_0005, 0x1_4000_0000, 0x1]);
    assert_eq!(
        bounds(b.checked_transform(&transform).expect("in range")),
        ([0x1_2FFF_FFE9, -0xA000_0004, -0x6AAA_AAC5], [0x5_E000_0009, 0x2_C000_000D, 0xA000_0018])
    );
}

fn minmax(a: [i64; 3], b: [i64; 3], pick: fn(i64, i64) -> i64) -> [i64; 3] {
    [pick(a[0], b[0]), pick(a[1], b[1]), pick(a[2], b[2])]
}

fn random_unit(state: &mut u64) -> Option<FixedUnitVec3> {
    let v = [shifted(state, 1), shifted(state, 1), shifted(state, 1)];
    FixedUnitVec3::new(v3(v))
}

fn rows(state: &mut u64, min_shift: u32) -> Rows {
    [vec(state, min_shift), vec(state, min_shift), vec(state, min_shift)]
}

fn vec(state: &mut u64, min_shift: u32) -> [i64; 3] {
    [shifted(state, min_shift), shifted(state, min_shift), shifted(state, min_shift)]
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// A uniform 64-bit pattern from three 31-bit draws.
fn next64(state: &mut u64) -> i64 {
    let (a, b, c) = (next(state), next(state), next(state));
    i64::from_ne_bytes(((a << 33) ^ (b << 2) ^ (c & 3)).to_ne_bytes())
}

/// A uniform 64-bit pattern shifted right by `min_shift` to 63 bits.
fn shifted(state: &mut u64, min_shift: u32) -> i64 {
    let span = u64::from(64 - min_shift);
    let shift = min_shift + u32::try_from(next(state) % span).unwrap_or(0);
    next64(state) >> shift
}
