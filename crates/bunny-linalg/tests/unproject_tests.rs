//! Unprojection contract tests (`bunny-unproject/1`): hand-derived golden rays, every refusal
//! on and one raw unit either side of its threshold, a seeded comparison with an independent
//! exact oracle (the general inverse of `clip_from_world` by Cramer's rule), and the round
//! trip from a world point through the viewport and back onto the ray.

mod unproject_oracle;

use std::cmp::Ordering;

use bunny_linalg::{
    FixedMat4, FixedVec2, FixedVec3, UnprojectError, Viewport, ViewportRay, UNPROJECT_CONTRACT_ID,
};
use bunny_num::FixedQ32_32;
use unproject_oracle::{rounded, Case, Int, Ratio, Raw4};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF: i64 = ONE / 2;
const QUARTER: i64 = ONE / 4;
const SEED: u64 = 0x0150_0E7A_0000_0001;

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn matrix(rows: Raw4) -> FixedMat4 {
    let row = |r: [i64; 4]| (fixed(r[0]), fixed(r[1]), fixed(r[2]), fixed(r[3]));
    FixedMat4::from_rows(row(rows[0]), row(rows[1]), row(rows[2]), row(rows[3]))
}

fn raw4(m: FixedMat4) -> Raw4 {
    [m.row0(), m.row1(), m.row2(), m.row3()].map(|r| [r.0, r.1, r.2, r.3].map(FixedQ32_32::raw))
}

fn viewport(width: u32, height: u32) -> Viewport {
    Viewport::try_new(width, height).expect("non-empty viewport")
}

fn unproject(
    vp: Viewport,
    (x, y): (i64, i64),
    projection: FixedMat4,
    view: FixedMat4,
) -> Result<ViewportRay, UnprojectError> {
    vp.try_unproject(FixedVec2::new(fixed(x), fixed(y)), projection, view)
}

fn ray(raw: ([i64; 3], [i64; 3])) -> ViewportRay {
    ViewportRay { origin: vec3(raw.0), direction: vec3(raw.1) }
}

/// The camera at `(1, 2, 3)` looking down `+X` with up `+Y`: right is `+Z`, back is `-X`.
fn turned_view() -> FixedMat4 {
    let view = FixedMat4::try_look_at(
        vec3([ONE, 2 * ONE, 3 * ONE]),
        vec3([2 * ONE, 2 * ONE, 3 * ONE]),
        vec3([0, ONE, 0]),
    )
    .expect("an axis-aligned view");
    let rows = [[0, 0, ONE, -3 * ONE], [0, ONE, 0, -2 * ONE], [-ONE, 0, 0, ONE], [0, 0, 0, ONE]];
    assert_eq!(raw4(view), rows, "the look-at matrix is exact");
    view
}

fn identity() -> FixedMat4 {
    matrix([[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]])
}

/// `try_perspective(1, 2, 1, 2)`: `m00 = 1/2`, `m11 = 1`, `m22 = -2`, `m23 = -2`.
fn golden_perspective() -> FixedMat4 {
    let m = FixedMat4::try_perspective(fixed(ONE), fixed(2 * ONE), fixed(ONE), fixed(2 * ONE))
        .expect("a projection");
    let rows = [[HALF, 0, 0, 0], [0, ONE, 0, 0], [0, 0, -2 * ONE, -2 * ONE], [0, 0, -ONE, 0]];
    assert_eq!(raw4(m), rows, "the perspective matrix is exact");
    m
}

/// `try_orthographic(2, 1, 1, 3)`: `m00 = 1/2`, `m11 = 1`, `m22 = -1/2`, `m23 = -1/2`.
fn golden_orthographic() -> FixedMat4 {
    let m = FixedMat4::try_orthographic(fixed(2 * ONE), fixed(ONE), fixed(ONE), fixed(3 * ONE))
        .expect("a projection");
    let rows = [[HALF, 0, 0, 0], [0, ONE, 0, 0], [0, 0, -HALF, -HALF], [0, 0, 0, ONE]];
    assert_eq!(raw4(m), rows, "the orthographic matrix is exact");
    m
}

/// Viewport coordinates of a 4 x 2 viewport, in quarter pixels: the centre, the four corners,
/// the centres of the top-left and bottom-right pixels and two edge midpoints.
const SAMPLES: [(i64, i64); 9] =
    [(8, 4), (0, 0), (16, 0), (0, 8), (16, 8), (2, 2), (14, 6), (8, 0), (0, 4)];

/// NDC of a sample in quarter units: `x_ndc = x / 2 - 1` and `y_ndc = 1 - y` on a 4 x 2 viewport.
const fn ndc_quarters((x, y): (i64, i64)) -> (i64, i64) {
    (x / 2 - 4, 4 - y)
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_013_perspective_golden_rays_at_centre_corners_and_boundary_pixels() {
    assert_eq!(UNPROJECT_CONTRACT_ID, "bunny-unproject/1");
    let (vp, projection) = (viewport(4, 2), golden_perspective());
    for sample in SAMPLES {
        let (xn, yn) = ndc_quarters(sample);
        let at = (sample.0 * QUARTER, sample.1 * QUARTER);
        // View space: the depth-0 point is (2 xn, yn, -1) and the depth-1 point (4 xn, 2 yn, -2).
        let expected =
            ([2 * xn * QUARTER, yn * QUARTER, -ONE], [2 * xn * QUARTER, yn * QUARTER, -ONE]);
        let got = unproject(vp, at, projection, identity());
        assert_eq!(got, Ok(ray(expected)), "identity view, sample {sample:?}");
        // World = eye + x_v right + y_v up + z_v back, right +Z, up +Y, back -X.
        let origin = [2 * ONE, 2 * ONE + yn * QUARTER, 3 * ONE + 2 * xn * QUARTER];
        let direction = [ONE, yn * QUARTER, 2 * xn * QUARTER];
        let got = unproject(vp, at, projection, turned_view());
        assert_eq!(got, Ok(ray((origin, direction))), "turned view, sample {sample:?}");
    }
    // Spot values written out: the top-left corner and the bottom-right pixel centre.
    let corner = unproject(vp, (0, 0), projection, turned_view());
    assert_eq!(corner, Ok(ray(([2 * ONE, 3 * ONE, ONE], [ONE, ONE, -2 * ONE]))));
    let pixel = unproject(vp, (3 * ONE + HALF, ONE + HALF), projection, turned_view());
    let expected = ([2 * ONE, ONE + HALF, 4 * ONE + HALF], [ONE, -HALF, ONE + HALF]);
    assert_eq!(pixel, Ok(ray(expected)));
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_014_orthographic_golden_rays_at_centre_corners_and_boundary_pixels() {
    let (vp, projection) = (viewport(4, 2), golden_orthographic());
    for sample in SAMPLES {
        let (xn, yn) = ndc_quarters(sample);
        let at = (sample.0 * QUARTER, sample.1 * QUARTER);
        // View space: the depth-0 point is (2 xn, yn, -1) and the depth-1 point (2 xn, yn, -3).
        let expected = ([2 * xn * QUARTER, yn * QUARTER, -ONE], [0, 0, -2 * ONE]);
        let got = unproject(vp, at, projection, identity());
        assert_eq!(got, Ok(ray(expected)), "identity view, sample {sample:?}");
        let origin = [2 * ONE, 2 * ONE + yn * QUARTER, 3 * ONE + 2 * xn * QUARTER];
        let got = unproject(vp, at, projection, turned_view());
        assert_eq!(got, Ok(ray((origin, [2 * ONE, 0, 0]))), "turned view, sample {sample:?}");
    }
    let corner = unproject(vp, (4 * ONE, 2 * ONE), projection, turned_view());
    assert_eq!(corner, Ok(ray(([2 * ONE, ONE, 5 * ONE], [2 * ONE, 0, 0]))));
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_015_coordinates_outside_the_closed_viewport_are_refused() {
    let (vp, projection) = (viewport(4, 2), golden_perspective());
    let at = |x, y| unproject(vp, (x, y), projection, identity());
    let outside = Err(UnprojectError::OutsideViewport);
    for (x, y) in [(-1, ONE), (4 * ONE + 1, ONE), (ONE, -1), (ONE, 2 * ONE + 1)] {
        assert_eq!(at(x, y), outside, "({x}, {y})");
    }
    for (x, y) in [(0, ONE), (4 * ONE, ONE), (ONE, 0), (ONE, 2 * ONE), (1, 1)] {
        assert!(at(x, y).is_ok(), "({x}, {y}) is inside, edges included");
    }
    assert_eq!(at(4 * ONE - 1, 2 * ONE - 1).map(|r| r.direction.z.raw()), Ok(-ONE));
}

/// A perspective-shaped matrix with raw `m00`, `m11`, `m22`, `m23`.
fn perspective_raw(p: i64, q: i64, r: i64, s: i64) -> FixedMat4 {
    matrix([[p, 0, 0, 0], [0, q, 0, 0], [0, 0, r, s], [0, 0, -ONE, 0]])
}

/// An orthographic-shaped matrix with raw `m00`, `m11`, `m22`, `m23`.
fn orthographic_raw(p: i64, q: i64, r: i64, s: i64) -> FixedMat4 {
    matrix([[p, 0, 0, 0], [0, q, 0, 0], [0, 0, r, s], [0, 0, 0, ONE]])
}

/// An affine view with diagonal linear part `diag` and translation `t`.
fn diagonal_view(diag: [i64; 3], t: [i64; 3]) -> FixedMat4 {
    let [a, b, c] = diag;
    matrix([[a, 0, 0, t[0]], [0, b, 0, t[1]], [0, 0, c, t[2]], [0, 0, 0, ONE]])
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_016_projection_shape_singularity_and_homogeneous_w_thresholds() {
    let vp = viewport(2, 2);
    let centre = (ONE, ONE);
    let at = |projection: FixedMat4| unproject(vp, centre, projection, identity());
    let base = raw4(golden_perspective());
    // Every entry outside the two shapes, one raw unit off zero (or off the last row).
    let mut off =
        vec![(0, 1), (0, 2), (0, 3), (1, 0), (1, 2), (1, 3), (2, 0), (2, 1), (3, 0), (3, 1)];
    off.extend([(3, 2), (3, 3)]);
    for (i, j) in off {
        for delta in [-1, 1] {
            let mut rows = base;
            rows[i][j] += delta;
            let expected = Err(UnprojectError::UnsupportedProjection);
            assert_eq!(at(matrix(rows)), expected, "m{i}{j} {delta:+}");
        }
    }
    for delta in [-1, 1] {
        let mut rows = raw4(golden_orthographic());
        rows[3][3] += delta;
        assert_eq!(at(matrix(rows)), Err(UnprojectError::UnsupportedProjection));
    }
    let singular = Err(UnprojectError::SingularProjection);
    for raw in [-1, 0, 1] {
        let expect = |ok: bool| if ok { Ok(()) } else { singular };
        let wide = [ONE, ONE, -2 * ONE, -2 * ONE];
        assert_eq!(at(perspective_raw(raw, ONE, wide[2], wide[3])).map(drop), expect(raw != 0));
        assert_eq!(at(perspective_raw(ONE, raw, wide[2], wide[3])).map(drop), expect(raw != 0));
        // `m23 = raw` with `m22 = -2`: the sign of `m23` must match, so only `-1` is accepted.
        // The view stretches depth by 2^32 so the tiny accepted ray does not round to zero.
        let stretched = diagonal_view([ONE, ONE, 1], [0; 3]);
        let projection = perspective_raw(ONE, ONE, -2 * ONE, raw);
        let s = unproject(vp, centre, projection, stretched).map(drop);
        let behind = Err(UnprojectError::PointBehindCamera);
        assert_eq!(s, [Ok(()), singular, behind][usize::try_from(raw + 1).expect("small")]);
        // An orthographic `m22` of one raw unit is accepted; the view keeps its direction small.
        let view = diagonal_view([ONE, ONE, 1 << 62], [0; 3]);
        let o = unproject(vp, centre, orthographic_raw(ONE, ONE, raw, 0), view).map(drop);
        assert_eq!(o, expect(raw != 0), "orthographic m22 {raw}");
    }
    // Depth 1 at infinity: `m22 = -1.0` with `m23 < 0`. Beyond it the depth-1 point is behind.
    // The view scales depth by 2^-30 so the far-away accepted points stay representable.
    let squeezed = diagonal_view([ONE, ONE, 1 << 62], [0; 3]);
    let at = |projection| unproject(vp, centre, projection, squeezed);
    let far = |r| at(perspective_raw(ONE, ONE, r, -ONE)).map(drop);
    assert_eq!(
        [far(-ONE - 1), far(-ONE), far(-ONE + 1)],
        [Ok(()), Err(UnprojectError::PointAtInfinity), Err(UnprojectError::PointBehindCamera)]
    );
    // Depth 0 at infinity: `m22 = 0` with `m23 > 0` (a reversed depth range).
    let near = |r| at(perspective_raw(ONE, ONE, r, ONE)).map(drop);
    assert_eq!(
        [near(1), near(0), near(-1)],
        [Ok(()), Err(UnprojectError::PointAtInfinity), Err(UnprojectError::PointBehindCamera)]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_017_view_overflow_and_degenerate_thresholds() {
    let vp = viewport(2, 2);
    let projection = orthographic_raw(ONE, ONE, -ONE, 0);
    let at = |x, view| unproject(vp, (x, ONE), projection, view);
    for (j, delta) in [(0, 1), (1, -1), (2, 1), (3, -1), (3, 1)] {
        let mut rows = raw4(identity());
        rows[3][j] += delta;
        assert_eq!(at(ONE, matrix(rows)), Err(UnprojectError::UnsupportedView), "m3{j}");
    }
    let rank_two =
        |a11| matrix([[ONE, ONE, 0, 0], [ONE, a11, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]);
    assert_eq!(at(ONE, rank_two(ONE)), Err(UnprojectError::SingularView));
    assert!(at(ONE, rank_two(ONE - 1)).is_ok() && at(ONE, rank_two(ONE + 1)).is_ok());
    // Origin x is `x - ONE - t_x` raw: the top of the range, then one and two raw above it.
    let origin_x = |x, t| at(x, diagonal_view([ONE; 3], [t, 0, 0])).map(|r| r.origin.x.raw());
    assert_eq!(origin_x(ONE + 1, i64::MIN + 2), Ok(i64::MAX));
    assert_eq!(origin_x(ONE + 1, i64::MIN + 1), Err(UnprojectError::Overflow));
    assert_eq!(origin_x(ONE + 1, i64::MIN), Err(UnprojectError::Overflow));
    assert_eq!(origin_x(ONE - 1, i64::MAX), Ok(i64::MIN));
    assert_eq!(origin_x(ONE - 2, i64::MAX), Err(UnprojectError::Overflow));
    // Direction z is `2^64 / m22` raw: `i64::MIN` at -2, one past `i64::MAX` at 2.
    let direction_z = |r| {
        unproject(vp, (ONE, ONE), orthographic_raw(ONE, ONE, r, 0), identity())
            .map(|r| r.direction.z.raw())
    };
    assert_eq!([direction_z(-2), direction_z(3)], [Ok(i64::MIN), Ok(6_148_914_691_236_517_205)]);
    assert_eq!(
        [direction_z(-1), direction_z(1), direction_z(2)],
        [Err(UnprojectError::Overflow); 3]
    );
    // Direction z is `2^96 / (m22 a)` raw for a view scale `a`: exactly -1/2 at `a = 2^35`,
    // which ties to the even 0, so the ray is degenerate; one raw unit either side.
    let projection = orthographic_raw(ONE, ONE, -(1 << 62), 0);
    let scaled = |a| {
        unproject(vp, (ONE, ONE), projection, diagonal_view([ONE, ONE, a], [0; 3]))
            .map(|r| r.direction.z.raw())
    };
    assert_eq!(scaled((1 << 35) - 1), Ok(-1));
    assert_eq!(scaled(1 << 35), Err(UnprojectError::DegenerateRay));
    assert_eq!(scaled((1 << 35) + 1), Err(UnprojectError::DegenerateRay));
    assert_eq!(scaled(1 << 34), Ok(-1));
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(i128::from(high) - i128::from(low) + 1).expect("non-empty");
        let offset = i128::from(self.next() % span);
        i64::try_from(i128::from(low) + offset).expect("in range")
    }

    /// A nonzero value with a random bit width up to `bits` and a random sign.
    fn wide(&mut self, bits: u32) -> i64 {
        let width = 1 + u32::try_from(self.next() % u64::from(bits)).expect("small");
        let magnitude = (self.next() | 1) >> (53 - width.min(53));
        let magnitude = i64::try_from(magnitude).expect("below 2^53") << width.saturating_sub(53);
        if self.next() % 2 == 0 {
            magnitude.max(1)
        } else {
            -magnitude.max(1)
        }
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    fn size(&mut self) -> u32 {
        match self.next() % 4 {
            0 => u32::try_from(self.range(1, 4)).expect("small"),
            1 => 1 << self.range(10, 20),
            _ => u32::try_from(self.range(1, 2048)).expect("small"),
        }
    }
}

/// A projection: half from the builders, half raw matrices of either shape with entries of
/// random width and the signs that keep both perspective points ahead.
fn random_projection(seeds: &mut Seeds) -> FixedMat4 {
    match seeds.next() % 4 {
        0 => {
            let near = seeds.range(ONE >> 10, 4 * ONE);
            let far = near + seeds.range(1, 1024 * ONE);
            FixedMat4::try_perspective(
                fixed(seeds.range(ONE >> 4, 4 * ONE)),
                fixed(seeds.range(ONE >> 8, 64 * ONE)),
                fixed(near),
                fixed(far),
            )
            .expect("a projection")
        }
        1 => {
            let near = seeds.range(0, 4 * ONE);
            let far = near + seeds.range(1, 1024 * ONE);
            FixedMat4::try_orthographic(
                fixed(seeds.range(ONE >> 4, 64 * ONE)),
                fixed(seeds.range(ONE >> 4, 64 * ONE)),
                fixed(near),
                fixed(far),
            )
            .expect("a projection")
        }
        2 => {
            let s = seeds.wide(63);
            let r = if s < 0 { -ONE - seeds.wide(62).abs() } else { seeds.wide(62).abs() };
            perspective_raw(seeds.wide(63), seeds.wide(63), r, s)
        }
        _ => orthographic_raw(seeds.wide(63), seeds.wide(63), seeds.wide(63), seeds.wide(63)),
    }
}

/// A view: half from `try_look_at`, half affine matrices with entries of random width.
fn random_view(seeds: &mut Seeds) -> FixedMat4 {
    if seeds.next() % 2 == 0 {
        let eye = vec3(seeds.triple(1 << 40));
        let target = vec3(seeds.triple(1 << 40));
        if let Ok(view) = FixedMat4::try_look_at(eye, target, vec3(seeds.triple(1 << 34))) {
            return view;
        }
    }
    let mut rows = [[0; 4]; 4];
    for row in rows.iter_mut().take(3) {
        *row = [seeds.wide(63), seeds.wide(63), seeds.wide(63), seeds.wide(63)];
    }
    rows[3] = [0, 0, 0, ONE];
    matrix(rows)
}

/// A coordinate in the closed viewport: an edge or corner value one time in four.
fn random_coordinate(seeds: &mut Seeds, size: u32) -> i64 {
    let top = (i64::from(size) * ONE).min(i64::MAX);
    match seeds.next() % 8 {
        0 => 0,
        1 => top,
        _ => seeds.range(0, top),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_018_seeded_rays_match_the_general_inverse_oracle_bit_for_bit() {
    println!("CL-TP-018 seed {SEED:#018x}");
    let mut seeds = Seeds(SEED);
    let (mut matched, mut overflows, mut degenerate) = (0, 0, 0);
    for index in 0..1_500 {
        let (width, height) = (seeds.size(), seeds.size());
        let projection = random_projection(&mut seeds);
        let view = random_view(&mut seeds);
        let (x, y) = (random_coordinate(&mut seeds, width), random_coordinate(&mut seeds, height));
        let case = Case { width, height, x, y, projection: raw4(projection), view: raw4(view) };
        let (origin, direction) = case.exact_ray().expect("a supported, invertible case");
        let expected = (rounded(&origin), rounded(&direction));
        match unproject(viewport(width, height), (x, y), projection, view) {
            Ok(got) => {
                assert_eq!(
                    (Some(raw3(got.origin)), Some(raw3(got.direction))),
                    expected,
                    "case {index}: {case:?}"
                );
                matched += 1;
            }
            Err(UnprojectError::Overflow) => {
                assert!(expected.0.is_none() || expected.1.is_none(), "case {index}: {case:?}");
                overflows += 1;
            }
            Err(UnprojectError::DegenerateRay) => {
                assert_eq!(expected.1, Some([0; 3]), "case {index}: {case:?}");
                assert!(expected.0.is_some(), "case {index}: overflow is checked first");
                degenerate += 1;
            }
            Err(error) => panic!("case {index}: unexpected {error:?} for {case:?}"),
        }
    }
    println!("CL-TP-018: 1500 cases, {matched} matched bit for bit, {overflows} overflow, {degenerate} degenerate");
    assert!(
        matched >= 600 && overflows >= 100,
        "coverage: {matched} matched, {overflows} overflow"
    );
}

/// The exact view-space point of a raw world point, in units: `(A p + t 2^32) / 2^64`.
fn view_point(view: &Raw4, p: [i64; 3]) -> [Ratio; 3] {
    [0, 1, 2].map(|i| {
        let sum: i128 = (0..3).map(|j| i128::from(view[i][j]) * i128::from(p[j])).sum::<i128>()
            + (i128::from(view[i][3]) << 32);
        Ratio::frac(sum, 1 << 64)
    })
}

fn unit(raw: i64) -> Ratio {
    Ratio::frac(raw.into(), ONE.into())
}

/// The exact inverse of the view's linear part, in units, by cofactors.
fn inverse3(view: &Raw4) -> [[Ratio; 3]; 3] {
    let a = |i: usize, j: usize| unit(view[i][j]);
    let minor = |r0: usize, r1: usize, c0: usize, c1: usize| {
        a(r0, c0).mul(&a(r1, c1)).sub(&a(r0, c1).mul(&a(r1, c0)))
    };
    let det = a(0, 0)
        .mul(&minor(1, 2, 1, 2))
        .sub(&a(0, 1).mul(&minor(1, 2, 0, 2)))
        .add(&a(0, 2).mul(&minor(1, 2, 0, 1)));
    let cofactor = |i: usize, j: usize| {
        let (rows, cols) = (
            [0, 1, 2].into_iter().filter(|&k| k != i).collect::<Vec<_>>(),
            [0, 1, 2].into_iter().filter(|&k| k != j).collect::<Vec<_>>(),
        );
        let m = minor(rows[0], rows[1], cols[0], cols[1]);
        if (i + j) % 2 == 0 {
            m
        } else {
            m.neg()
        }
    };
    [0, 1, 2].map(|i| [0, 1, 2].map(|j| cofactor(j, i).div(&det)))
}

/// One round trip's measured distance from the world point to the ray, squared, raw units,
/// and its derived bound, squared.
fn round_trip(
    case_view: &Raw4,
    projection: &Raw4,
    size: (u32, u32),
    point: [i64; 3],
) -> Option<(Ratio, Ratio)> {
    let v = view_point(case_view, point);
    let [p, q, r, s] =
        [projection[0][0], projection[1][1], projection[2][2], projection[2][3]].map(unit);
    let perspective = projection[3][2] != 0;
    // `w` is the clip `w`: `-z` for a perspective matrix, 1 for an orthographic one.
    let w = if perspective { v[2].neg() } else { Ratio::int(1) };
    if w.sign() != Ordering::Greater {
        return None;
    }
    let ndc = [p.mul(&v[0]).div(&w), q.mul(&v[1]).div(&w)];
    let depth = r.mul(&v[2]).add(&s).div(&w);
    if depth.sign() == Ordering::Less || depth.compare(&Ratio::int(1)).is_gt() {
        return None;
    }
    let ndc_raw = ndc.map(|c| c.mul(&Ratio::int(ONE.into())).round());
    let vp = viewport(size.0, size.1);
    let pixel = vp.try_ndc_to_viewport(vec3([ndc_raw[0]?, ndc_raw[1]?, 0])).ok()?;
    let inside = |c: FixedQ32_32, n: u32| (0..=i64::from(n) * ONE).contains(&c.raw());
    if !inside(pixel.x, size.0) || !inside(pixel.y, size.1) {
        return None;
    }
    let coordinate = FixedVec2::new(pixel.x, pixel.y);
    let got = vp.try_unproject(coordinate, matrix(*projection), matrix(*case_view));
    let got = got.expect("a landed coordinate unprojects");
    let (o, d) = (raw3(got.origin), raw3(got.direction));
    // Distance squared from the point to the ray's line: |(P - o) x d|^2 / |d|^2, raw units.
    let e = [0, 1, 2].map(|i| Int::new(i128::from(point[i]) - i128::from(o[i])));
    let dd = d.map(|c| Int::new(c.into()));
    let cross = [
        e[1].mul(&dd[2]).sub(&e[2].mul(&dd[1])),
        e[2].mul(&dd[0]).sub(&e[0].mul(&dd[2])),
        e[0].mul(&dd[1]).sub(&e[1].mul(&dd[0])),
    ];
    let norm = |v: &[Int; 3]| v[0].mul(&v[0]).add(&v[1].mul(&v[1])).add(&v[2].mul(&v[2]));
    let distance = Ratio::new(norm(&cross), norm(&dd));
    // The bound: each viewport axis moved at most `size / 4 + 1` raw pixel units (half a raw
    // NDC unit times `size / 2`, then the floor); at the point's clip `w` that moves the view
    // point by `2 delta w / (size m)` units on that axis, carried to world by the inverse.
    let pixel_units = |size: u32| Ratio::frac(i128::from(size) + 4, 4 * i128::from(ONE));
    let shift = [(size.0, &p), (size.1, &q)].map(|(n, m)| {
        Ratio::int(2).mul(&pixel_units(n)).mul(&w).div(&Ratio::int(n.into()).mul(&m.abs()))
    });
    let inverse = inverse3(case_view);
    let moved = (0..3).fold(Ratio::int(0), |acc, i| {
        acc.add(&inverse[i][0].abs().mul(&shift[0])).add(&inverse[i][1].abs().mul(&shift[1]))
    });
    // The kernel's own rounding: half a raw unit per component of the origin and of the
    // direction, so under `1 + |t|` raw in length at the parameter `t` of the point's depth
    // on the exact ray: `t = (z - z_0) / (z_1 - z_0)` with the depth-0 and depth-1 view `z`.
    let z0 = s.div(&r).neg();
    let z1 = if perspective {
        s.div(&r.add(&Ratio::int(1))).neg()
    } else {
        Ratio::int(1).sub(&s).div(&r)
    };
    let t = v[2].sub(&z0).div(&z1.sub(&z0));
    let bound = moved.mul(&Ratio::int(ONE.into())).add(&Ratio::int(1)).add(&t.abs());
    Some((distance, bound.mul(&bound)))
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_019_world_points_round_trip_onto_their_ray_within_the_derived_bound() {
    let seed = SEED ^ 0x13;
    println!("CL-TP-019 seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let (mut cases, mut worst) = (0, Ratio::int(0));
    for _ in 0..400 {
        let (width, height) = (
            u32::try_from(seeds.range(1, 2048)).expect("small"),
            u32::try_from(seeds.range(1, 2048)).expect("small"),
        );
        let eye = seeds.triple(64 * ONE);
        let Ok(view) = FixedMat4::try_look_at(
            vec3(eye),
            vec3(seeds.triple(64 * ONE)),
            vec3(seeds.triple(ONE)),
        ) else {
            continue;
        };
        let aspect = fixed(i64::from(width) << 32)
            .checked_div(fixed(i64::from(height) << 32))
            .expect("an aspect");
        let tan = fixed(seeds.range(ONE >> 4, 4 * ONE));
        let perspective = seeds.next() % 2 == 0;
        let projection = if perspective {
            FixedMat4::try_perspective(tan, aspect, fixed(ONE >> 4), fixed(256 * ONE))
        } else {
            FixedMat4::try_orthographic(
                fixed(seeds.range(ONE >> 2, 16 * ONE)),
                tan,
                fixed(0),
                fixed(256 * ONE),
            )
        }
        .expect("a projection");
        let rows = raw4(view);
        for _ in 0..8 {
            // A point near the camera's view direction, at a random depth.
            let back = [rows[2][0], rows[2][1], rows[2][2]];
            let depth = seeds.range(ONE >> 2, 128 * ONE);
            // Inside the view: a lateral offset up to half the half height at that depth.
            let reach = i128::from(tan.raw())
                * if perspective { i128::from(depth) } else { i128::from(ONE) };
            let offset = seeds.triple(i64::try_from(reach >> 33).expect("small"));
            let point = [0, 1, 2].map(|i| {
                eye[i]
                    - i64::try_from((i128::from(back[i]) * i128::from(depth)) >> 32).expect("small")
                    + offset[i]
            });
            let Some((distance, bound)) =
                round_trip(&rows, &raw4(projection), (width, height), point)
            else {
                continue;
            };
            assert!(
                distance.compare(&bound).is_le(),
                "point {point:?}: distance^2 {distance:?} above bound^2 {bound:?}"
            );
            if worst.compare(&distance).is_lt() {
                worst = distance;
            }
            cases += 1;
        }
    }
    println!("CL-TP-019: {cases} round trips, worst squared distance {:?} raw^2", worst.round());
    assert!(cases >= 1_000, "only {cases} round trips");
}
