//! `bunny-bounds/1`: box helpers on `FixedAabb3` (flyingrobots/bunny#151).
//!
//! The oracle below is written separately from the kernel: it works on raw `i64` values in
//! `i128` and `u128`, decides ranges with `i64::try_from`, rounds the centre by choosing the
//! even neighbour of an exact half, and decides radii by comparing squares, never by a root.

use bunny_geom::{BoundsError, FixedAabb3, FixedSphere3, BOUNDS_CONTRACT_ID};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB015_1000_0000_0001;
const CASES: usize = 10_000;
const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;

type Raw3 = [i64; 3];

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

fn aabb(min: Raw3, max: Raw3) -> FixedAabb3 {
    FixedAabb3::new(v(min), v(max))
}

fn corners(b: &FixedAabb3) -> (Raw3, Raw3) {
    (raw(b.min), raw(b.max))
}

fn sphere(center: Raw3, radius: i64) -> FixedSphere3 {
    FixedSphere3::new(v(center), FixedQ32_32::from_raw(radius))
}

fn s(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

// ---- oracle -------------------------------------------------------------------------------

fn oracle_points(points: &[Raw3]) -> Option<(Raw3, Raw3)> {
    if points.is_empty() {
        return None;
    }
    let lo = [0, 1, 2].map(|axis| points.iter().map(|p| p[axis]).min().expect("non-empty"));
    let hi = [0, 1, 2].map(|axis| points.iter().map(|p| p[axis]).max().expect("non-empty"));
    Some((lo, hi))
}

fn oracle_ordered(min: Raw3, max: Raw3) -> bool {
    (0..3).all(|axis| min[axis] <= max[axis])
}

fn oracle_merge(a: (Raw3, Raw3), b: (Raw3, Raw3)) -> Result<(Raw3, Raw3), BoundsError> {
    if !oracle_ordered(a.0, a.1) || !oracle_ordered(b.0, b.1) {
        return Err(BoundsError::InvertedBox);
    }
    oracle_points(&[a.0, a.1, b.0, b.1]).ok_or(BoundsError::NoPoints)
}

fn oracle_expand(b: (Raw3, Raw3), m: Raw3) -> Result<(Raw3, Raw3), BoundsError> {
    if !oracle_ordered(b.0, b.1) {
        return Err(BoundsError::InvertedBox);
    }
    let lo = [0, 1, 2].map(|a| i128::from(b.0[a]) - i128::from(m[a]));
    let hi = [0, 1, 2].map(|a| i128::from(b.1[a]) + i128::from(m[a]));
    if (0..3).any(|a| lo[a] > hi[a]) {
        return Err(BoundsError::ShrinkInvertsBox);
    }
    let fit = |w: [i128; 3]| -> Option<Raw3> {
        Some([i64::try_from(w[0]).ok()?, i64::try_from(w[1]).ok()?, i64::try_from(w[2]).ok()?])
    };
    match (fit(lo), fit(hi)) {
        (Some(lo), Some(hi)) => Ok((lo, hi)),
        _ => Err(BoundsError::OutOfRange),
    }
}

/// The midpoint of `lo` and `hi`, rounded to nearest with an exact half going to the even
/// neighbour.
fn oracle_centre(lo: i64, hi: i64) -> i64 {
    let sum = i128::from(lo) + i128::from(hi);
    let mid = if sum % 2 == 0 {
        sum / 2
    } else {
        let (below, above) = ((sum - 1) / 2, (sum + 1) / 2);
        if below % 2 == 0 {
            below
        } else {
            above
        }
    };
    i64::try_from(mid).expect("between lo and hi")
}

fn dist_sq(centre: Raw3, point: Raw3) -> u128 {
    (0..3)
        .map(|a| {
            let d = (i128::from(point[a]) - i128::from(centre[a])).unsigned_abs();
            d * d
        })
        .sum()
}

fn box_corners(lo: Raw3, hi: Raw3) -> [Raw3; 8] {
    let mut out = [[0; 3]; 8];
    for (index, corner) in out.iter_mut().enumerate() {
        for (axis, value) in corner.iter_mut().enumerate() {
            *value = if index >> axis & 1 == 0 { lo[axis] } else { hi[axis] };
        }
    }
    out
}

/// The smallest `r` with `r * r >= n`, by bisection on squares (no square root). `n` is below
/// `2^128`, so `r` is at most `2^64` and every `mid * mid` below is checked.
fn ceil_root(n: u128) -> u128 {
    let (mut lo, mut hi) = (0_u128, 1_u128 << 64);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match mid.checked_mul(mid) {
            Some(sq) if sq < n => lo = mid + 1,
            _ => hi = mid,
        }
    }
    lo
}

/// Checks a bounding sphere of the box against the oracle: its centre, that `r^2` reaches
/// every corner and `(r - 1)^2` misses the farthest one, or the refusal when the oracle's
/// radius does not fit `i64`.
fn check_bounding_sphere(lo: Raw3, hi: Raw3, got: Result<FixedSphere3, BoundsError>, ctx: &str) {
    let centre = [0, 1, 2].map(|a| oracle_centre(lo[a], hi[a]));
    let far = box_corners(lo, hi).iter().map(|c| dist_sq(centre, *c)).max().expect("8 corners");
    let r = ceil_root(far);
    let Ok(radius) = i64::try_from(r) else {
        assert_eq!(got, Err(BoundsError::OutOfRange), "{ctx}");
        return;
    };
    let sphere = got.unwrap_or_else(|e| panic!("{ctx}: refused with {e:?}"));
    assert_eq!(raw(sphere.center), centre, "centre; {ctx}");
    assert_eq!(sphere.radius.raw(), radius, "radius; {ctx}");
    let r = u128::try_from(radius).expect("non-negative");
    for corner in box_corners(lo, hi) {
        assert!(dist_sq(centre, corner) <= r * r, "corner {corner:?} outside; {ctx}");
    }
    assert!(r == 0 || (r - 1) * (r - 1) < far, "radius not the smallest; {ctx}");
}

// ---- seeded generator -----------------------------------------------------------------------

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(0x1405_7B7E_F767_814F);
    *state >> 11 ^ *state << 53 ^ *state
}

/// Any raw value, every magnitude: a uniform pattern shifted right by 0 to 63 bits.
fn raw_any(state: &mut u64) -> i64 {
    let shift = next(state) % 64;
    i64::from_ne_bytes(next(state).to_ne_bytes()) >> shift
}

fn triple(state: &mut u64) -> Raw3 {
    [0; 3].map(|_| raw_any(state))
}

/// An ordered box, or with probability 1/8 an inverted one.
fn random_box(state: &mut u64) -> (Raw3, Raw3) {
    let (a, b) = (triple(state), triple(state));
    let lo = [0, 1, 2].map(|i| a[i].min(b[i]));
    let hi = [0, 1, 2].map(|i| a[i].max(b[i]));
    if next(state) % 8 == 0 && lo != hi {
        (hi, lo)
    } else {
        (lo, hi)
    }
}

// ---- hand-derived -------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_001_points_and_triangles_hand_derived() {
    assert_eq!(BOUNDS_CONTRACT_ID, "bunny-bounds/1");
    assert_eq!(FixedAabb3::from_points(&[]), Err(BoundsError::NoPoints));
    let one = FixedAabb3::from_points(&[v([3, -4, 5])]).expect("one point");
    assert_eq!(corners(&one), ([3, -4, 5], [3, -4, 5]), "a single point is a degenerate box");
    let pts = [v([1, 9, -2]), v([-3, 4, 7]), v([5, -6, 0]), v([1, 9, -2])];
    let got = FixedAabb3::from_points(&pts).expect("four points");
    assert_eq!(corners(&got), ([-3, -6, -2], [5, 9, 7]));
    let extremes = [v([MIN, MAX, 0]), v([MAX, MIN, 0])];
    let got = FixedAabb3::from_points(&extremes).expect("extremes");
    assert_eq!(corners(&got), ([MIN, MIN, 0], [MAX, MAX, 0]), "the whole raw range, exact");

    let tri = FixedAabb3::from_triangle([v([0, 0, 0]), v([4, -1, 2]), v([-2, 3, 1])]);
    assert_eq!(corners(&tri), ([-2, -1, 0], [4, 3, 2]));
    let point = FixedAabb3::from_triangle([v([7, 7, 7]); 3]);
    assert_eq!(corners(&point), ([7, 7, 7], [7, 7, 7]), "a degenerate triangle");
    let wide = FixedAabb3::from_triangle([v([MIN; 3]), v([MAX; 3]), v([0, MIN, MAX])]);
    assert_eq!(corners(&wide), ([MIN; 3], [MAX; 3]));
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_002_merge_hand_derived_and_inverted_refused() {
    let a = aabb([0, 0, 0], [2, 2, 2]);
    let b = aabb([1, -3, 5], [4, 1, 6]);
    assert_eq!(a.merge(&b).map(|m| corners(&m)), Ok(([0, -3, 0], [4, 2, 6])));
    assert_eq!(a.merge(&b), b.merge(&a), "merge is symmetric");
    assert_eq!(a.merge(&a), Ok(a), "merge with itself is the identity");
    let inner = aabb([1, 1, 1], [1, 1, 1]);
    assert_eq!(a.merge(&inner), Ok(a), "a contained box adds nothing");
    let full = aabb([MIN; 3], [MAX; 3]);
    assert_eq!(full.merge(&a), Ok(full), "exact at the raw range");
    // Inverted on one axis only (x: min 1 above max 0), on either side.
    let inverted = aabb([1, 0, 0], [0, 2, 2]);
    assert_eq!(a.merge(&inverted), Err(BoundsError::InvertedBox));
    assert_eq!(inverted.merge(&a), Err(BoundsError::InvertedBox));
    // Two inverted boxes whose plain min/max would look like a real box.
    let p = aabb([2, 0, 0], [0, 1, 1]);
    let q = aabb([-5, 0, 0], [-7, 1, 1]);
    assert_eq!(p.merge(&q), Err(BoundsError::InvertedBox));
    // One raw unit either side of ordered: min == max is valid, min == max + 1 is not.
    let flat = aabb([3, 0, 0], [3, 1, 1]);
    assert_eq!(flat.merge(&a).map(|m| corners(&m)), Ok(([0, 0, 0], [3, 2, 2])));
    let barely = aabb([4, 0, 0], [3, 1, 1]);
    assert_eq!(barely.merge(&a), Err(BoundsError::InvertedBox));
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_003_expand_margins_hand_derived() {
    let b = aabb([0, 0, 0], [4, 6, 8]);
    assert_eq!(b.expand(s(0)), Ok(b), "a zero margin is the identity");
    assert_eq!(b.expand(s(1)).map(|e| corners(&e)), Ok(([-1, -1, -1], [5, 7, 9])));
    assert_eq!(b.expand(s(-1)).map(|e| corners(&e)), Ok(([1, 1, 1], [3, 5, 7])));
    // Shrinking x to a point is allowed; one raw unit further inverts it and is refused.
    assert_eq!(b.expand(s(-2)).map(|e| corners(&e)), Ok(([2, 2, 2], [2, 4, 6])));
    assert_eq!(b.expand(s(-3)), Err(BoundsError::ShrinkInvertsBox));
    // An odd extent cannot shrink to a point: [0, 1] by -1 would be [1, 0].
    let odd = aabb([0, 0, 0], [1, 2, 2]);
    assert_eq!(odd.expand(s(-1)), Err(BoundsError::ShrinkInvertsBox));
    // Per axis: grow x, keep y, shrink z.
    let got = b.expand_axes(v([2, 0, -4]));
    assert_eq!(got.map(|e| corners(&e)), Ok(([-2, 0, 4], [6, 6, 4])));
    assert_eq!(b.expand_axes(v([0, 0, -5])), Err(BoundsError::ShrinkInvertsBox));
    // An inverted box is refused, even by a margin that would make it look ordered.
    let inverted = aabb([1, 0, 0], [0, 1, 1]);
    assert_eq!(inverted.expand(s(1)), Err(BoundsError::InvertedBox));
    assert_eq!(inverted.expand(s(0)), Err(BoundsError::InvertedBox));
    assert_eq!(inverted.expand_axes(v([5, 5, 5])), Err(BoundsError::InvertedBox));
    // The most negative margin: the whole range shrinks by 2^63 per side and inverts.
    let full = aabb([MIN; 3], [MAX; 3]);
    assert_eq!(full.expand(s(MIN)), Err(BoundsError::ShrinkInvertsBox));
    assert_eq!(full.expand(s(MIN + 1)).map(|e| corners(&e)), Ok(([-1; 3], [0; 3])));
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_004_expand_overflow_one_raw_unit_either_side() {
    let top = aabb([0; 3], [MAX - 1, 0, 0]);
    assert_eq!(top.expand_axes(v([0, 0, 0])), Ok(top));
    assert_eq!(top.expand_axes(v([1, 0, 0])).map(|e| corners(&e)), Ok(([-1, 0, 0], [MAX, 0, 0])));
    assert_eq!(top.expand_axes(v([2, 0, 0])), Err(BoundsError::OutOfRange));
    let bottom = aabb([0, MIN + 1, 0], [0; 3]);
    assert_eq!(bottom.expand_axes(v([0, 1, 0])).map(|e| corners(&e)), Ok(([0, MIN, 0], [0, 1, 0])));
    assert_eq!(bottom.expand_axes(v([0, 2, 0])), Err(BoundsError::OutOfRange));
    // A point grown by the largest margin reaches MIN + 1 and MAX; at the range edge by one.
    let origin = aabb([0; 3], [0; 3]);
    assert_eq!(origin.expand(s(MAX)).map(|e| corners(&e)), Ok(([MIN + 1; 3], [MAX; 3])));
    let edge = aabb([1; 3], [1; 3]);
    assert_eq!(edge.expand(s(MAX)), Err(BoundsError::OutOfRange));
    let edge = aabb([-1; 3], [-1; 3]);
    assert_eq!(edge.expand(s(MAX)).map(|e| corners(&e)), Ok(([MIN; 3], [MAX - 1; 3])));
    // Shrinking never leaves the range: [MAX, MAX] by -1 inverts and says so.
    let corner = aabb([MAX; 3], [MAX; 3]);
    assert_eq!(corner.expand(s(-1)), Err(BoundsError::ShrinkInvertsBox));
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_005_sphere_to_box_hand_derived_and_boundaries() {
    let got = FixedAabb3::from_sphere(&sphere([1, 2, 3], 5));
    assert_eq!(got.map(|b| corners(&b)), Ok(([-4, -3, -2], [6, 7, 8])));
    let point = FixedAabb3::from_sphere(&sphere([9, 9, 9], 0));
    assert_eq!(point.map(|b| corners(&b)), Ok(([9; 3], [9; 3])), "radius zero is a point");
    assert_eq!(FixedAabb3::from_sphere(&sphere([0; 3], -1)), Err(BoundsError::NegativeRadius));
    assert_eq!(FixedAabb3::from_sphere(&sphere([0; 3], MIN)), Err(BoundsError::NegativeRadius));
    let r = 1_000;
    let high = FixedAabb3::from_sphere(&sphere([MAX - r, 0, 0], r));
    assert_eq!(high.map(|b| corners(&b).1[0]), Ok(MAX));
    let high = FixedAabb3::from_sphere(&sphere([MAX - r + 1, 0, 0], r));
    assert_eq!(high, Err(BoundsError::OutOfRange));
    let low = FixedAabb3::from_sphere(&sphere([0, 0, MIN + r], r));
    assert_eq!(low.map(|b| corners(&b).0[2]), Ok(MIN));
    let low = FixedAabb3::from_sphere(&sphere([0, 0, MIN + r - 1], r));
    assert_eq!(low, Err(BoundsError::OutOfRange));
    let widest = FixedAabb3::from_sphere(&sphere([0; 3], MAX));
    assert_eq!(widest.map(|b| corners(&b)), Ok(([MIN + 1; 3], [MAX; 3])));
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_006_box_to_sphere_rounds_the_radius_outward() {
    // A unit square in raw units: the corner (1, 1, 0) is sqrt(2) from the origin. A radius
    // rounded to nearest (1) would leave it outside; the outward radius is 2.
    let square = aabb([0, 0, 0], [2, 2, 0]);
    let got = square.bounding_sphere().expect("in range");
    assert_eq!((raw(got.center), got.radius.raw()), ([1, 1, 0], 2));
    let unit = aabb([0, 0, 0], [1, 1, 0]);
    let got = unit.bounding_sphere().expect("in range");
    // Midpoints 1/2 round to the even neighbour 0; the far corner (1, 1, 0) is at squared
    // distance 2, so the radius is 2.
    assert_eq!((raw(got.center), got.radius.raw()), ([0, 0, 0], 2));
    let cube = aabb([-3, -3, -3], [3, 3, 3]);
    let got = cube.bounding_sphere().expect("in range");
    // Squared half-diagonal 27; 5^2 = 25 is short, 6^2 = 36 reaches it.
    assert_eq!((raw(got.center), got.radius.raw()), ([0; 3], 6));
    let exact = aabb([0, 0, 0], [6, 8, 0]);
    let got = exact.bounding_sphere().expect("in range");
    assert_eq!((raw(got.center), got.radius.raw()), ([3, 4, 0], 5), "3-4-5 is exact");
    let point = aabb([5, -5, 5], [5, -5, 5]);
    let got = point.bounding_sphere().expect("in range");
    assert_eq!((raw(got.center), got.radius.raw()), ([5, -5, 5], 0));
    // Ties to even on both signs: 3/2 -> 2, 5/2 -> 2, -3/2 -> -2, -1/2 -> 0.
    let ties = aabb([1, 2, -2], [2, 3, -1]);
    assert_eq!(ties.bounding_sphere().map(|g| raw(g.center)), Ok([2, 2, -2]));
    let neg = aabb([-1, 0, 0], [0, 0, 0]);
    assert_eq!(neg.bounding_sphere().map(|g| raw(g.center)), Ok([0, 0, 0]));
    let inverted = aabb([1, 0, 0], [0, 0, 0]);
    assert_eq!(inverted.bounding_sphere(), Err(BoundsError::InvertedBox));
    for b in [square, unit, cube, exact, point, ties, neg] {
        let (lo, hi) = corners(&b);
        check_bounding_sphere(lo, hi, b.bounding_sphere(), &format!("{lo:?} {hi:?}"));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_007_box_to_sphere_radius_range_one_raw_unit_either_side() {
    // One axis: the radius is the half extent from the rounded centre.
    let fits = aabb([MIN + 1, 0, 0], [MAX, 0, 0]);
    assert_eq!(fits.bounding_sphere().map(|g| (raw(g.center), g.radius.raw())), Ok(([0; 3], MAX)));
    let fits = aabb([MIN + 2, 0, 0], [MAX, 0, 0]);
    assert_eq!(fits.bounding_sphere().map(|g| (raw(g.center), g.radius.raw())), Ok(([0; 3], MAX)));
    let over = aabb([MIN, 0, 0], [MAX, 0, 0]);
    assert_eq!(over.bounding_sphere(), Err(BoundsError::OutOfRange), "radius 2^63");
    // Two axes: squared distance MAX^2 is a radius of MAX; MAX^2 + 1 needs MAX + 1.
    let fits = aabb([MIN + 1, 0, 0], [MAX, 0, 0]);
    assert_eq!(fits.bounding_sphere().map(|g| g.radius.raw()), Ok(MAX));
    let over = aabb([MIN + 1, 0, 0], [MAX, 1, 0]);
    assert_eq!(over.bounding_sphere(), Err(BoundsError::OutOfRange), "MAX^2 + 1");
    let full = aabb([MIN; 3], [MAX; 3]);
    assert_eq!(full.bounding_sphere(), Err(BoundsError::OutOfRange), "the widest box");
    for b in [fits, over, full] {
        let (lo, hi) = corners(&b);
        check_bounding_sphere(lo, hi, b.bounding_sphere(), &format!("{lo:?} {hi:?}"));
    }
}

// ---- seeded -------------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_008_seeded_points_merge_and_expand_match_the_oracle() {
    let mut state = SEED;
    let mut counts = [0_usize; 4]; // expand: ok, inverted input, shrink inverts, out of range
    for case in 0..CASES {
        let n = usize::try_from(next(&mut state) % 9).expect("small");
        let pts: Vec<Raw3> = (0..n).map(|_| triple(&mut state)).collect();
        let ctx = format!("seed={SEED:#x} case={case}");
        let got = FixedAabb3::from_points(&pts.iter().map(|p| v(*p)).collect::<Vec<_>>());
        assert_eq!(got.map(|b| corners(&b)), oracle_points(&pts).ok_or(BoundsError::NoPoints), "{ctx}");
        let tri = [triple(&mut state), triple(&mut state), triple(&mut state)];
        let got = corners(&FixedAabb3::from_triangle(tri.map(v)));
        assert_eq!(Some(got), oracle_points(&tri), "triangle; {ctx}");
        let (a, b) = (random_box(&mut state), random_box(&mut state));
        let got = aabb(a.0, a.1).merge(&aabb(b.0, b.1)).map(|m| corners(&m));
        assert_eq!(got, oracle_merge(a, b), "merge {a:?} {b:?}; {ctx}");
        if let Ok((lo, hi)) = got {
            for (min, max) in [a, b] {
                assert!((0..3).all(|i| lo[i] <= min[i] && max[i] <= hi[i]), "contains; {ctx}");
            }
        }
        let m = if next(&mut state) % 2 == 0 { triple(&mut state) } else { [raw_any(&mut state); 3] };
        let got = aabb(a.0, a.1).expand_axes(v(m)).map(|e| corners(&e));
        assert_eq!(got, oracle_expand(a, m), "expand {a:?} by {m:?}; {ctx}");
        if m[0] == m[1] && m[1] == m[2] {
            assert_eq!(aabb(a.0, a.1).expand(s(m[0])).map(|e| corners(&e)), got, "{ctx}");
        }
        if let Ok((lo, hi)) = got {
            let by = |i: usize| i128::from(a.0[i]) - i128::from(lo[i]);
            assert!((0..3).all(|i| by(i) == i128::from(m[i])), "exactly the margin; {ctx}");
            assert!((0..3).all(|i| i128::from(hi[i]) - i128::from(a.1[i]) == by(i)), "{ctx}");
        }
        counts[match got {
            Ok(_) => 0,
            Err(BoundsError::InvertedBox) => 1,
            Err(BoundsError::ShrinkInvertsBox) => 2,
            Err(_) => 3,
        }] += 1;
    }
    println!("seed={SEED:#x}: {CASES} cases; expand ok/inverted/shrink/range = {counts:?}");
    assert_eq!(counts, EXPECTED_EXPAND_COUNTS, "seed={SEED:#x}");
}

/// Outcomes of the 10 000 seeded `expand_axes` calls: in range, inverted input, shrink
/// inverts, out of range.
const EXPECTED_EXPAND_COUNTS: [usize; 4] = [0, 0, 0, 0];

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_009_seeded_spheres_contain_every_corner() {
    let mut state = SEED ^ 9;
    let (mut fit, mut over) = (0_usize, 0_usize);
    for case in 0..CASES {
        let (lo, hi) = random_box(&mut state);
        let ctx = format!("seed={SEED:#x}^9 case={case} box={lo:?} {hi:?}");
        let b = aabb(lo, hi);
        if !oracle_ordered(lo, hi) {
            assert_eq!(b.bounding_sphere(), Err(BoundsError::InvertedBox), "{ctx}");
            continue;
        }
        let got = b.bounding_sphere();
        check_bounding_sphere(lo, hi, got, &ctx);
        if let Ok(sph) = got {
            fit += 1;
            // Back to a box: it contains the original box, or does not fit the range.
            if let Ok(back) = FixedAabb3::from_sphere(&sph) {
                let (blo, bhi) = corners(&back);
                assert!((0..3).all(|i| blo[i] <= lo[i] && hi[i] <= bhi[i]), "{ctx}");
            }
        } else {
            over += 1;
        }
        let (centre, radius) = (triple(&mut state), raw_any(&mut state));
        let got = FixedAabb3::from_sphere(&sphere(centre, radius)).map(|e| corners(&e));
        let want = if radius < 0 {
            Err(BoundsError::NegativeRadius)
        } else {
            oracle_expand((centre, centre), [radius; 3])
        };
        assert_eq!(got, want, "sphere {centre:?} r={radius}; {ctx}");
    }
    println!("seed={SEED:#x}^9: {CASES} boxes; spheres in range {fit}, radius too large {over}");
    assert_eq!((fit, over), EXPECTED_SPHERE_COUNTS, "seed={SEED:#x}^9");
}

/// Of the ordered seeded boxes: bounding spheres in range, and radii too large for `i64`.
const EXPECTED_SPHERE_COUNTS: (usize, usize) = (0, 0);
