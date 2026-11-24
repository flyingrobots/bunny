//! Transform-aware bounds (flyingrobots/bunny#119): the smallest raw box containing the exact
//! image of a box, points, a triangle or a sphere under a `FixedAffine3`. Hand-derived cases
//! for identity, translation, rotation, scale and mixed transforms, refusals one raw unit
//! either side of each threshold, seeded oracles and pinned goldens.
//!
//! The oracle is exact big-integer arithmetic (`transform_bounds_oracle::Big`), sharing nothing
//! with the kernel. Every exact image coordinate is carried as `N = 2^32 t + sum m p`, its value
//! times `2^32` in raw units, and every box is formed from all eight corners (not the per-axis
//! extremes the kernel uses). A face `F` is right exactly when it contains (`2^32 F <= N_min`)
//! and is tight (`2^32 (F + 1) > N_min`). For a sphere, the low face contains when `D = N_c -
//! 2^32 F` is at least zero and `D^2 >= R^2 |row|^2`, decided without a square root. The goldens
//! come from a separate Python model posted on the pull request.

mod transform_bounds_oracle;

use bunny_geom::{BoundsError, FixedAabb3, FixedSphere3};
use bunny_linalg::{FixedAffine3, FixedMat3, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};
use transform_bounds_oracle::Big;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const SEED: u64 = 0xB002_0119_0000_0001;
const CASES: usize = 10_000;
/// `round(2^32 / sqrt(2))`: the rounded cosine and sine of an eighth turn.
const C45: i64 = 3_037_000_500;

type Rows = [[i64; 3]; 3];
type Faces = ([i64; 3], [i64; 3]);

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

fn rows_of(a: FixedAffine3) -> (Rows, [i64; 3]) {
    let l = a.linear;
    ([r3(l.row0()), r3(l.row1()), r3(l.row2())], r3(a.translation))
}

fn faces(b: FixedAabb3) -> Faces {
    (r3(b.min), r3(b.max))
}

fn boxed(m: Rows, t: [i64; 3], lo: [i64; 3], hi: [i64; 3]) -> Result<Faces, BoundsError> {
    FixedAabb3::new(v3(lo), v3(hi)).transformed_bounds(&affine(m, t)).map(faces)
}

fn pointed(m: Rows, t: [i64; 3], points: &[[i64; 3]]) -> Result<Faces, BoundsError> {
    let points: Vec<FixedVec3> = points.iter().copied().map(v3).collect();
    FixedAabb3::from_transformed_points(&points, &affine(m, t)).map(faces)
}

fn sphered(m: Rows, t: [i64; 3], c: [i64; 3], r: i64) -> Result<Faces, BoundsError> {
    let sphere = FixedSphere3::new(v3(c), FixedQ32_32::from_raw(r));
    FixedAabb3::from_transformed_sphere(&sphere, &affine(m, t)).map(faces)
}

// ---- The oracle --------------------------------------------------------------------------

fn big(v: i128) -> Big {
    Big::from_i128(v)
}

/// `2^32 face`, exactly.
fn scaled(face: i128) -> Big {
    big(face).mul(&big(1 << 32))
}

/// `N = 2^32 t + sum_j m_j p_j`, the exact image coordinate times `2^32`.
fn exact(row: [i64; 3], t: i64, p: [i64; 3]) -> Big {
    let mut n = scaled(i128::from(t));
    for (m, x) in row.into_iter().zip(p) {
        n = n.add(&big(i128::from(m)).mul(&big(i128::from(x))));
    }
    n
}

/// The exact least and greatest `N` of the images of `points`, per axis.
fn extremes(m: Rows, t: [i64; 3], points: &[[i64; 3]]) -> [(Big, Big); 3] {
    [0, 1, 2].map(|axis| {
        let ns: Vec<Big> = points.iter().map(|p| exact(m[axis], t[axis], *p)).collect();
        let lo = ns.iter().min().expect("points").clone();
        (lo, ns.into_iter().max().expect("points"))
    })
}

fn corners(lo: [i64; 3], hi: [i64; 3]) -> Vec<[i64; 3]> {
    let pick = |k: usize, axis: usize| if k >> axis & 1 == 0 { lo[axis] } else { hi[axis] };
    (0..8).map(|k| [pick(k, 0), pick(k, 1), pick(k, 2)]).collect()
}

/// Checks a box result against exact extremes: containing and tight faces, or `OutOfRange`
/// exactly when the floor of a least or the ceiling of a greatest leaves `i64`. Returns
/// whether the result was in range.
fn check_box(got: Result<Faces, BoundsError>, want: &[(Big, Big); 3], label: &str) -> bool {
    let low_fits = |n: &Big| scaled(i128::from(i64::MIN)) <= *n;
    let high_fits = |n: &Big| *n <= scaled(i128::from(i64::MAX));
    let fits = want.iter().all(|(lo, hi)| low_fits(lo) && high_fits(hi));
    let Ok((lo, hi)) = got else {
        assert_eq!(got, Err(BoundsError::OutOfRange), "{label}");
        assert!(!fits, "{label}: refused, but the exact image fits");
        return false;
    };
    assert!(fits, "{label}: accepted, but the exact image leaves the range");
    for (axis, (n_lo, n_hi)) in want.iter().enumerate() {
        let (f, g) = (i128::from(lo[axis]), i128::from(hi[axis]));
        assert!(scaled(f) <= *n_lo && scaled(f + 1) > *n_lo, "{label}: low face {axis}");
        assert!(scaled(g) >= *n_hi && scaled(g - 1) < *n_hi, "{label}: high face {axis}");
    }
    true
}

/// Whether the low face `f` contains the sphere's image on this axis: `D = N_c - 2^32 f >= 0`
/// and `D^2 >= X`, with `X = R^2 |row|^2`.
fn sphere_low_contains(f: i128, n_c: &Big, x: &Big) -> bool {
    let d = n_c.sub(&scaled(f));
    !d.is_negative() && d.mul(&d) >= *x
}

fn sphere_high_contains(g: i128, n_c: &Big, x: &Big) -> bool {
    let d = scaled(g).sub(n_c);
    !d.is_negative() && d.mul(&d) >= *x
}

/// Checks a sphere result as [`check_box`] does, the sphere given as `(centre, radius)`.
fn check_sphere(
    got: Result<Faces, BoundsError>,
    (linear, shift): (Rows, [i64; 3]),
    (centre, radius): ([i64; 3], i64),
    label: &str,
) -> bool {
    let wide = |v: i64| big(i128::from(v));
    let axes = [0, 1, 2].map(|axis| {
        let length = linear[axis].iter().fold(big(0), |sum, e| sum.add(&wide(*e).mul(&wide(*e))));
        let reach_squared = wide(radius).mul(&wide(radius)).mul(&length);
        (exact(linear[axis], shift[axis], centre), reach_squared)
    });
    let fits = axes.iter().all(|(n, x)| {
        sphere_low_contains(i128::from(i64::MIN), n, x)
            && sphere_high_contains(i128::from(i64::MAX), n, x)
    });
    let Ok((lo, hi)) = got else {
        assert_eq!(got, Err(BoundsError::OutOfRange), "{label}");
        assert!(!fits, "{label}: refused, but the exact image fits");
        return false;
    };
    assert!(fits, "{label}: accepted, but the exact image leaves the range");
    for (axis, (n, x)) in axes.iter().enumerate() {
        let (f, g) = (i128::from(lo[axis]), i128::from(hi[axis]));
        assert!(sphere_low_contains(f, n, x), "{label}: low face {axis} does not contain");
        assert!(!sphere_low_contains(f + 1, n, x), "{label}: low face {axis} is not tight");
        assert!(sphere_high_contains(g, n, x), "{label}: high face {axis} does not contain");
        assert!(!sphere_high_contains(g - 1, n, x), "{label}: high face {axis} is not tight");
    }
    true
}

// ---- Boxes -------------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_050_box_bounds_under_identity_translation_rotation_scale_and_mixed() {
    // Identity and translation are exact: the box itself, and the box moved.
    let (lo, hi) = ([-5, 7, -ONE - 3], [ONE + 1, 9, 0]);
    assert_eq!(boxed(diagonal([ONE; 3]), [0; 3], lo, hi), Ok((lo, hi)));
    let moved = boxed(diagonal([ONE; 3]), [ONE, -2, 3], lo, hi);
    assert_eq!(moved, Ok(([ONE - 5, 5, -ONE], [2 * ONE + 1, 7, 3])));
    // An eighth turn about +Z maps the unit cube to x in [-c, c], y in [0, 2c]: every product
    // c * ONE is exact, so the faces are the exact hull's.
    let turn = [[C45, -C45, 0], [C45, C45, 0], [0, 0, ONE]];
    let cube = boxed(turn, [0; 3], [0; 3], [ONE; 3]);
    assert_eq!(cube, Ok(([-C45, 0, 0], [C45, 2 * C45, ONE])));
    // A non-uniform, negative scale swaps which face is the minimum.
    let flip = boxed(
        diagonal([-2 * ONE, 3 * ONE, ONE / 4]),
        [ONE, 0, 0],
        [ONE, -ONE, 0],
        [2 * ONE, ONE, ONE],
    );
    assert_eq!(flip, Ok(([-3 * ONE, -3 * ONE, 0], [-ONE, 3 * ONE, ONE / 4])));
    // m00 = 1 raw: x' = x / 2^32 raw. 1 + 2^-32 to 3 - 2^-32 rounds out to [1, 3].
    let tiny = diagonal([1, ONE, ONE]);
    assert_eq!(
        boxed(tiny, [0; 3], [ONE + 1, 0, 0], [3 * ONE - 1, 0, 0]),
        Ok(([1, 0, 0], [3, 0, 0]))
    );
    // The exact image 1/2 raw: the rounded point transform gives 0 (a tie to even), outside the
    // exact image's face; the bounds are [0, 1] and contain both.
    assert_eq!(boxed(tiny, [0; 3], [1 << 31, 0, 0], [1 << 31, 0, 0]), Ok(([0, 0, 0], [1, 0, 0])));
    // No intermediate is range-checked: 2 * 2^62 - 2 * 2^62 overflows the rounded point
    // transform, but the exact image is 0, and the eight-corner hull refuses what this bounds.
    let cancel = [[2 * ONE, -2 * ONE, 0], [0, ONE, 0], [0, 0, ONE]];
    let p = [1 << 62, 1 << 62, 0];
    assert_eq!(boxed(cancel, [0; 3], p, p), Ok(([0, 1 << 62, 0], [0, 1 << 62, 0])));
    let rounded = FixedAabb3::new(v3(p), v3(p)).checked_transform(&affine(cancel, [0; 3]));
    assert!(rounded.is_err());
    // Mixed: rotation, non-uniform scale and translation composed by from_trs, against the
    // big-integer oracle.
    let rotation = FixedMat3::rotation_z(FixedAngle::from_raw(1 << 60));
    let trs =
        FixedAffine3::from_trs(v3([ONE, -ONE / 2, 7]), rotation, v3([2 * ONE, ONE / 3, -5 * ONE]))
            .expect("in range");
    let (m, t) = rows_of(trs);
    let (lo, hi) = ([-ONE, 3, -ONE / 7], [ONE / 3, 2 * ONE, ONE]);
    assert!(check_box(boxed(m, t, lo, hi), &extremes(m, t, &corners(lo, hi)), "mixed"));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_051_box_refusals_one_raw_unit_either_side() {
    let identity = diagonal([ONE; 3]);
    for axis in [0, 1, 2] {
        let mut hi = [ONE; 3];
        hi[axis] = -1;
        assert_eq!(boxed(identity, [0; 3], [0; 3], hi), Err(BoundsError::InvertedBox));
        hi[axis] = 0;
        assert!(boxed(identity, [0; 3], [0; 3], hi).is_ok());
    }
    // Inverted is refused before any arithmetic, even when the image would leave the range.
    let huge = diagonal([i64::MAX; 3]);
    assert_eq!(boxed(huge, [i64::MAX; 3], [ONE; 3], [0; 3]), Err(BoundsError::InvertedBox));
    // Translation i64::MAX - 5 raw: max x = 5 reaches i64::MAX, 6 is past it.
    let up = |x: i64| boxed(identity, [i64::MAX - 5, 0, 0], [0; 3], [x, 0, 0]);
    assert_eq!(up(5).map(|f| f.1), Ok([i64::MAX, 0, 0]));
    assert_eq!(up(6), Err(BoundsError::OutOfRange));
    let down = |x: i64| boxed(identity, [i64::MIN + 5, 0, 0], [x, 0, 0], [0; 3]);
    assert_eq!(down(-5).map(|f| f.0), Ok([i64::MIN, 0, 0]));
    assert_eq!(down(-6), Err(BoundsError::OutOfRange));
    // A fractional face: m00 = 1 + 2^-32. x = 0x7FFF_FFFF_7FFF_FFFF raw has an exact image whose
    // ceiling is i64::MAX; one raw unit more and the ceiling is past it, though the rounded
    // image of 0x7FFF_FFFF_8000_0000 is i64::MAX - 1 / 2 rounded, inside the range.
    let stretch = diagonal([ONE + 1, ONE, ONE]);
    let top = |x: i64| boxed(stretch, [0; 3], [0; 3], [x, 0, 0]);
    assert_eq!(top(0x7FFF_FFFF_7FFF_FFFF).map(|f| f.1), Ok([i64::MAX, 0, 0]));
    assert_eq!(top(0x7FFF_FFFF_8000_0000), Err(BoundsError::OutOfRange));
    let bottom = |x: i64| boxed(stretch, [0; 3], [x, 0, 0], [0; 3]);
    assert_eq!(bottom(-0x7FFF_FFFF_8000_0000).map(|f| f.0), Ok([i64::MIN, 0, 0]));
    assert_eq!(bottom(-0x7FFF_FFFF_8000_0001), Err(BoundsError::OutOfRange));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_052_seeded_boxes_match_the_big_integer_oracle() {
    let (mut state, mut in_range) = (SEED, 0);
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 0), vec(&mut state, 0));
        let (a, b) = (vec(&mut state, 0), vec(&mut state, 0));
        let (lo, hi) = (pick(a, b, i64::min), pick(a, b, i64::max));
        let label = format!("seed {SEED:#x}, case {case}");
        in_range +=
            usize::from(check_box(boxed(m, t, lo, hi), &extremes(m, t, &corners(lo, hi)), &label));
    }
    assert_eq!(in_range, EXPECTED_BOXES_IN_RANGE, "seed {SEED:#x}");
    // A moderate domain where every result is in range.
    let mut state = SEED ^ 1;
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 24), vec(&mut state, 2));
        let (a, b) = (vec(&mut state, 24), vec(&mut state, 24));
        let (lo, hi) = (pick(a, b, i64::min), pick(a, b, i64::max));
        let label = format!("seed {:#x}, case {case}", SEED ^ 1);
        assert!(check_box(boxed(m, t, lo, hi), &extremes(m, t, &corners(lo, hi)), &label));
    }
}

/// How many seeded full-range boxes are in range.
const EXPECTED_BOXES_IN_RANGE: usize = 2_408;

// ---- Points, triangles and meshes --------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_053_points_and_triangles() {
    assert_eq!(pointed(diagonal([ONE; 3]), [0; 3], &[]), Err(BoundsError::NoPoints));
    // One point: the floor and ceiling of its exact image. 1/2 raw and -3/2 raw.
    let half = diagonal([1 << 31, ONE, -(3 << 31)]);
    assert_eq!(pointed(half, [0; 3], &[[1, 5, 1]]), Ok(([0, 5, -2], [1, 5, -1])));
    // Identity: from_points exactly. Translation: moved exactly.
    let pts = [[3, -9, ONE], [-ONE, 4, 0], [7, 7, 7]];
    let plain: Vec<FixedVec3> = pts.iter().copied().map(v3).collect();
    let want = FixedAabb3::from_points(&plain).map(faces);
    assert_eq!(pointed(diagonal([ONE; 3]), [0; 3], &pts), want);
    assert_eq!(
        pointed(diagonal([ONE; 3]), [1, 2, 3], &pts),
        Ok(([-ONE + 1, -7, 3], [8, 9, ONE + 3]))
    );
    // The triangle is the points of its vertices, degenerate ones included.
    let general = [[ONE + 3, -7, 11], [5, ONE - 1, 1 << 31], [-1, 9, 2 * ONE]];
    let t = [3, -5, 7];
    let tri = FixedAabb3::from_transformed_triangle(
        [v3(pts[0]), v3(pts[1]), v3(pts[2])],
        &affine(general, t),
    );
    assert_eq!(tri.map(faces), pointed(general, t, &pts));
    let flat = FixedAabb3::from_transformed_triangle([v3(pts[0]); 3], &affine(general, t));
    assert_eq!(flat.map(faces), pointed(general, t, &pts[..1]));
    // Rotation: the eighth turn maps (ONE, 0, 0) to (c, c, 0) exactly, (0, ONE, 0) to (-c, c, 0).
    let turn = [[C45, -C45, 0], [C45, C45, 0], [0, 0, ONE]];
    let rotated = pointed(turn, [0; 3], &[[ONE, 0, 0], [0, ONE, 0]]);
    assert_eq!(rotated, Ok(([-C45, C45, 0], [C45, C45, 0])));
    // The top of the range, one raw unit either side.
    let up = |x: i64| pointed(diagonal([ONE + 1, ONE, ONE]), [0; 3], &[[0; 3], [x, 0, 0]]);
    assert_eq!(up(0x7FFF_FFFF_7FFF_FFFF).map(|f| f.1), Ok([i64::MAX, 0, 0]));
    assert_eq!(up(0x7FFF_FFFF_8000_0000), Err(BoundsError::OutOfRange));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_054_seeded_points_match_the_big_integer_oracle() {
    let (mut state, mut in_range) = (SEED ^ 2, 0);
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 0), vec(&mut state, 0));
        let count = 1 + usize::try_from(next(&mut state) % 6).unwrap_or(0);
        let points: Vec<[i64; 3]> = (0..count).map(|_| vec(&mut state, 0)).collect();
        let label = format!("seed {:#x}, case {case}", SEED ^ 2);
        in_range +=
            usize::from(check_box(pointed(m, t, &points), &extremes(m, t, &points), &label));
    }
    assert_eq!(in_range, EXPECTED_POINT_SETS_IN_RANGE, "seed {:#x}", SEED ^ 2);
}

/// How many seeded full-range point sets are in range.
const EXPECTED_POINT_SETS_IN_RANGE: usize = 1_753;

// ---- Spheres -----------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_055_sphere_bounds_under_identity_translation_rotation_scale_and_mixed() {
    let identity = diagonal([ONE; 3]);
    let c = [ONE, -2 * ONE, 5];
    // Identity and translation: centre +- r, exactly.
    assert_eq!(
        sphered(identity, [0; 3], c, ONE),
        Ok(([0, -3 * ONE, 5 - ONE], [2 * ONE, -ONE, 5 + ONE]))
    );
    assert_eq!(
        sphered(identity, [1, 0, 0], c, 3),
        Ok(([ONE - 2, -2 * ONE - 3, 2], [ONE + 4, -2 * ONE + 3, 8]))
    );
    // A radius of zero: the bounds of the exact image of the centre (here 1/2 raw).
    assert_eq!(
        sphered(diagonal([1 << 31, ONE, ONE]), [0; 3], [1, 0, 0], 0),
        Ok(([0, 0, 0], [1, 0, 0]))
    );
    // Non-uniform scale (2, 3, 1/4): extents 2, 3 and 1/4 of the radius, exactly.
    let scale = diagonal([2 * ONE, 3 * ONE, ONE / 4]);
    assert_eq!(
        sphered(scale, [0; 3], [0; 3], ONE),
        Ok(([-2 * ONE, -3 * ONE, -ONE / 4], [2 * ONE, 3 * ONE, ONE / 4]))
    );
    // The eighth turn: the row (c, -c, 0) has length c sqrt 2 = 2^32 + 0.39.. raw, so the unit
    // sphere reaches 2^32 + 1 raw on x and y, and exactly 2^32 on z.
    let turn = [[C45, -C45, 0], [C45, C45, 0], [0, 0, ONE]];
    assert_eq!(
        sphered(turn, [0; 3], [0; 3], ONE),
        Ok(([-ONE - 1, -ONE - 1, -ONE], [ONE + 1, ONE + 1, ONE]))
    );
    // A singular scale flattens the image.
    assert_eq!(
        sphered(diagonal([ONE, 0, ONE]), [0; 3], c, ONE).map(|f| (f.0[1], f.1[1])),
        Ok((0, 0))
    );
    // Mixed, against the oracle.
    let rotation = FixedMat3::rotation_z(FixedAngle::from_raw(1 << 60));
    let trs =
        FixedAffine3::from_trs(v3([ONE, -ONE / 2, 7]), rotation, v3([2 * ONE, ONE / 3, -5 * ONE]))
            .expect("in range");
    let (m, t) = rows_of(trs);
    let (c, r) = ([-ONE, 3, ONE / 7], 3 * ONE + 5);
    assert!(check_sphere(sphered(m, t, c, r), (m, t), (c, r), "mixed"));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_056_sphere_refusals_one_raw_unit_either_side() {
    let identity = diagonal([ONE; 3]);
    assert_eq!(sphered(identity, [0; 3], [0; 3], -1), Err(BoundsError::NegativeRadius));
    assert_eq!(sphered(identity, [0; 3], [0; 3], i64::MIN), Err(BoundsError::NegativeRadius));
    // Negative radius is checked first, even when the image would leave the range.
    assert_eq!(
        sphered(diagonal([i64::MAX; 3]), [i64::MAX; 3], [ONE; 3], -1),
        Err(BoundsError::NegativeRadius)
    );
    assert!(sphered(identity, [0; 3], [0; 3], 0).is_ok());
    // Radius 7 at x = i64::MAX - 7 reaches i64::MAX exactly; one raw unit further is refused.
    let at = |x: i64| sphered(identity, [0; 3], [x, 0, 0], 7);
    assert_eq!(at(i64::MAX - 7).map(|f| f.1[0]), Ok(i64::MAX));
    assert_eq!(at(i64::MAX - 6), Err(BoundsError::OutOfRange));
    assert_eq!(at(i64::MIN + 7).map(|f| f.0[0]), Ok(i64::MIN));
    assert_eq!(at(i64::MIN + 6), Err(BoundsError::OutOfRange));
    // The largest radius under the identity at the origin: i64::MAX reaches both ends but the
    // low one is i64::MIN + 1.
    assert_eq!(
        sphered(identity, [0; 3], [0; 3], i64::MAX).map(|f| (f.0[0], f.1[0])),
        Ok((-i64::MAX, i64::MAX))
    );
    // The full-range row of i64::MAX entries: the reach is far outside the range.
    assert_eq!(sphered([[i64::MAX; 3]; 3], [0; 3], [0; 3], 1 << 40), Err(BoundsError::OutOfRange));
}

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_057_seeded_spheres_match_the_big_integer_oracle() {
    let (mut state, mut in_range) = (SEED ^ 3, 0);
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 0), vec(&mut state, 0));
        let (c, r) = (vec(&mut state, 0), shifted(&mut state, 0).abs());
        let label = format!("seed {:#x}, case {case}", SEED ^ 3);
        in_range += usize::from(check_sphere(sphered(m, t, c, r), (m, t), (c, r), &label));
    }
    assert_eq!(in_range, EXPECTED_SPHERES_IN_RANGE, "seed {:#x}", SEED ^ 3);
    // A moderate domain: also inside the bounds of the sphere's box, whose image contains the
    // sphere's.
    let mut state = SEED ^ 4;
    for case in 0..CASES {
        let (m, t) = (rows(&mut state, 24), vec(&mut state, 2));
        let (c, r) = (vec(&mut state, 24), shifted(&mut state, 24).abs());
        let got = sphered(m, t, c, r);
        let label = format!("seed {:#x}, case {case}", SEED ^ 4);
        assert!(check_sphere(got, (m, t), (c, r), &label));
        let sphere_box =
            FixedAabb3::from_sphere(&FixedSphere3::new(v3(c), FixedQ32_32::from_raw(r)))
                .expect("in range");
        let (lo, hi) = got.expect("in range");
        let (blo, bhi) = boxed(m, t, r3(sphere_box.min), r3(sphere_box.max)).expect("in range");
        for axis in [0, 1, 2] {
            assert!(
                blo[axis] <= lo[axis] && hi[axis] <= bhi[axis],
                "seed {:#x}, case {case}",
                SEED ^ 4
            );
        }
    }
}

/// How many seeded full-range spheres are in range.
const EXPECTED_SPHERES_IN_RANGE: usize = 3_137;

// ---- Goldens -----------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn trs_tp_058_pinned_goldens() {
    // The sheared, non-uniformly scaled transform of TRS-TP-029.
    let linear = [
        [0x1_8000_0001, -0x4000_0003, 0x7],
        [0x2000_0005, 0x2_0000_0000, -0xC000_0001],
        [-0x11, 0x8000_0000, 0x5555_5555],
    ];
    let shift = [0x3_0000_0001, -0x1_8000_0000, 0x7];
    // The box of TRS-TP-029: the rounded eight-corner hull there has y min -0xA000_0004 and
    // z max 0xA000_0018, each inside the exact image; these faces contain it.
    let hull = boxed(
        linear,
        shift,
        [-0x1_0000_0003, 0x8000_0001, -0x2_0000_0000],
        [0x2_0000_0005, 0x1_4000_0000, 0x1],
    );
    assert_eq!(
        hull,
        Ok((
            [0x1_2FFF_FFE9, -0xA000_0005, -0x6AAA_AAC5],
            [0x5_E000_0009, 0x2_C000_000D, 0xA000_0019]
        ))
    );
    let point = [0x1_2345_6789, -0x9876_5432, 0x2_0000_0001];
    let tri = pointed(
        linear,
        shift,
        &[point, [-0x3_0000_0001, 0x5, 0x7FFF_FFFF], [0x0, 0x4_0000_0003, -0x1_0000_0000]],
    );
    assert_eq!(
        tri,
        Ok((
            [-0x1_8000_0002, -0x4_0C83_FB70, 0x2AAA_AAE6],
            [0x4_DB05_B06C, 0x7_4000_0007, 0x1_AAAA_AAB4]
        ))
    );
    let sphere = sphered(linear, shift, point, 0x2_8000_0003);
    assert_eq!(
        sphere,
        Ok((
            [0x1_0DC7_BC5C, -0x9_65E5_1D3D, -0x1_2228_15B7],
            [0x8_A843_A47C, 0x1_4CDD_265D, 0x1_DF07_16C1]
        ))
    );
}

// ---- Seeded inputs -----------------------------------------------------------------------

fn pick(a: [i64; 3], b: [i64; 3], f: fn(i64, i64) -> i64) -> [i64; 3] {
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
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
