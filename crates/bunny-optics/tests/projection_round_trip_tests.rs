//! The projection round trip: points along a camera ray, carried through
//! `FixedMat4::try_look_at`, a projection matrix and `Viewport`, land on the ray's own sample.
//!
//! The camera and the matrices round differently, so the landing point is not exact. Each case
//! is checked against a bound derived from every step's stated rounding, evaluated in exact
//! rationals for that case (see `docs/topics/optics/README.md#projection-round-trip`), and that
//! bound is checked against a stated tolerance, so a convention the two sides disagree on (the
//! handedness, the direction of `y`, the pixel centre) fails as a bound far above it.

// Only the signed big integers of the shared oracle module are used here.
#[allow(dead_code)]
mod exact;

use std::cmp::Ordering;

use bunny_linalg::{FixedMat4, FixedVec3, Viewport};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    point_at, CameraSpec, OrthographicCamera, OrthographicSpec, PinholeCamera, PixelSample,
};
use exact::Signed;
use wasm_bindgen_test::wasm_bindgen_test;

const ROUND_TRIP_SEED: u64 = 0x0257_7219_0A11_0001;
const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE_RAW / 2;
const LAST_SUB: i64 = ONE_RAW - 1;
const NEG_Z: [i64; 3] = [0, 0, -ONE];
const POS_X: [i64; 3] = [ONE, 0, 0];
const POS_Y: [i64; 3] = [0, ONE, 0];
/// The stated tolerance: every derived bound, in raw pixel units, is at most `2^-10` pixel.
const TOLERANCE_RAW: i128 = 1 << 22;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low + 1).expect("non-empty range");
        low + i64::try_from(self.next() % span).expect("below the span")
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.range(0, i64::from(bound) - 1)).expect("below a u32 bound")
    }

    fn size(&mut self) -> u32 {
        u32::try_from(self.range(1, 2048)).expect("small")
    }
}

/// An exact rational with a positive denominator.
#[derive(Clone, Debug)]
struct Ratio {
    num: Signed,
    den: Signed,
}

impl Ratio {
    fn frac(num: i128, den: i128) -> Self {
        assert!(den > 0, "positive denominator");
        Self { num: Signed::new(num), den: Signed::new(den) }
    }

    fn int(value: i128) -> Self {
        Self::frac(value, 1)
    }

    /// A raw Q32.32 value, `raw / 2^32`.
    fn raw(raw: i128) -> Self {
        Self::frac(raw, 1 << 32)
    }

    fn add(&self, other: &Self) -> Self {
        let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
        Self { num, den: self.den.mul(&other.den) }
    }

    fn neg(&self) -> Self {
        Self { num: self.num.negated(), den: self.den.clone() }
    }

    fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    fn mul(&self, other: &Self) -> Self {
        Self { num: self.num.mul(&other.num), den: self.den.mul(&other.den) }
    }

    /// `self / other` for `other > 0`.
    fn div(&self, other: &Self) -> Self {
        assert_eq!(other.sign(), Ordering::Greater, "division by a positive value");
        Self { num: self.num.mul(&other.den), den: self.den.mul(&other.num) }
    }

    fn sign(&self) -> Ordering {
        self.num.sign()
    }

    fn abs(&self) -> Self {
        if self.sign() == Ordering::Less {
            self.neg()
        } else {
            self.clone()
        }
    }

    fn le(&self, other: &Self) -> bool {
        self.sub(other).sign() != Ordering::Greater
    }

    /// The largest integer at most `self`, for `0 <= self < 2^100`, by bisection.
    fn floor(&self) -> i128 {
        let (mut low, mut high) = (0_i128, 1_i128 << 100);
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if Self::int(middle).le(self) {
                low = middle;
            } else {
                high = middle;
            }
        }
        low
    }
}

type Vector = [Ratio; 3];

fn vector(raw: [i64; 3]) -> Vector {
    raw.map(|c| Ratio::raw(i128::from(c)))
}

fn dot(a: &Vector, b: &Vector) -> Ratio {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

/// `|v|_1` of a raw vector, in units.
fn l1(raw: [i64; 3]) -> Ratio {
    Ratio::raw(raw.iter().map(|c| i128::from(c.unsigned_abs())).sum())
}

/// `a + s b` per component.
fn axpy(a: &Vector, s: &Ratio, b: &Vector) -> Vector {
    [0, 1, 2].map(|i| a[i].add(&s.mul(&b[i])))
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// `numerator / denominator` rounded to nearest, ties to even, for `denominator > 0`.
const fn round_div(numerator: i128, denominator: i128) -> i128 {
    let (floor, remainder) = (numerator.div_euclid(denominator), numerator.rem_euclid(denominator));
    if 2 * remainder > denominator || (2 * remainder == denominator && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// The view matrix's rows as raw axes `[right', up', forward']` and translations.
struct View {
    axes: [[i64; 3]; 3],
    translation: [i64; 3],
}

impl View {
    /// `try_look_at(position, position + look, up)`, the camera spec's own inputs.
    fn look_at(position: FixedVec3, look: FixedVec3, up: FixedVec3) -> Self {
        let target = vec3([0, 1, 2].map(|i| raw3(position)[i] + raw3(look)[i]));
        let view = FixedMat4::try_look_at(position, target, up).expect("a view matrix");
        let rows = [view.row0(), view.row1(), view.row2()];
        let [r, u, back] = rows.map(|row| [row.0.raw(), row.1.raw(), row.2.raw()]);
        let row3 = view.row3();
        assert_eq!([row3.0, row3.1, row3.2, row3.3].map(FixedQ32_32::raw), [0, 0, 0, ONE]);
        Self { axes: [r, u, back.map(|c| -c)], translation: rows.map(|row| row.3.raw()) }
    }

    /// The exact view-space point of `q`, in units of `2^-64`.
    fn apply(&self, q: [i64; 3]) -> [i128; 3] {
        let row = |axis: [i64; 3], translation: i64| {
            let products: i128 = (0..3).map(|i| i128::from(axis[i]) * i128::from(q[i])).sum();
            products + (i128::from(translation) << 32)
        };
        let [r, u, f] = self.axes;
        let [tr, tu, tb] = self.translation;
        // The third row is `back = -forward'`.
        [row(r, tr), row(u, tu), row(f.map(|c| -c), tb)]
    }
}

/// The viewport coordinate, raw, of exact NDC numerators over a common positive denominator,
/// each rounded once to nearest raw unit first.
fn land(size: (u32, u32), numerators: [i128; 2], denominator: i128) -> [i128; 2] {
    let ndc = numerators.map(|n| i64::try_from(round_div(n, denominator)).expect("NDC fits"));
    let viewport = Viewport::try_new(size.0, size.1).expect("non-empty");
    let pixel = viewport.try_ndc_to_viewport(vec3([ndc[0], ndc[1], 0])).expect("on screen");
    [pixel.x.raw(), pixel.y.raw()].map(i128::from)
}

/// The sample's own viewport coordinate, raw: `(x + sub_x, y + sub_y)`.
fn target(sample: PixelSample) -> [i128; 2] {
    let one = i128::from(ONE);
    [
        i128::from(sample.x) * one + i128::from(sample.sub_x.raw()),
        i128::from(sample.y) * one + i128::from(sample.sub_y.raw()),
    ]
}

/// The doubled offsets `a`, `b` of the sample from the image centre, in pixels.
fn offsets(sample: PixelSample, size: (u32, u32)) -> [Ratio; 2] {
    let one = i128::from(ONE);
    let (w, h) = (i128::from(size.0), i128::from(size.1));
    let a = (2 * i128::from(sample.x) - w) * one + 2 * i128::from(sample.sub_x.raw());
    let b = (h - 2 * i128::from(sample.y)) * one - 2 * i128::from(sample.sub_y.raw());
    [Ratio::raw(a), Ratio::raw(b)]
}

/// The bound on one NDC axis: `|m - ideal| (|s| + M / w) + ideal M / w + 2^-33`, where the
/// exact NDC value is `m x / w` and its ideal is `ideal s`, with `|x - s w| <= M`.
fn ndc_bound(entry: i64, ideal: &Ratio, s: &Ratio, mismatch: &Ratio, w_low: &Ratio) -> Ratio {
    let ratio = mismatch.div(w_low);
    let entry_error = Ratio::raw(i128::from(entry)).sub(ideal).abs();
    entry_error.mul(&s.abs().add(&ratio)).add(&ideal.mul(&ratio)).add(&Ratio::frac(1, 1 << 33))
}

/// The pixel bound, raw, from an NDC bound on an axis of `size` pixels: `size / 2` times it,
/// plus the viewport's floor (below one raw unit).
fn pixel_bound(size: u32, ndc: &Ratio) -> Ratio {
    let half = Ratio::frac(i128::from(size), 2);
    half.mul(ndc).mul(&Ratio::int(1 << 32)).add(&Ratio::int(1))
}

/// Largest measured error and largest derived bound over a run, raw pixel units.
#[derive(Default)]
struct Worst {
    measured: i128,
    bound: Option<Ratio>,
    cases: u32,
}

impl Worst {
    fn record(&mut self, errors: [i128; 2], bounds: [Ratio; 2], label: &str) {
        let tolerance = Ratio::int(TOLERANCE_RAW);
        for (error, bound) in errors.into_iter().zip(bounds) {
            assert!(Ratio::int(error.abs()).le(&bound), "{label}: error {error} above its bound");
            assert!(bound.le(&tolerance), "{label}: bound {} above tolerance", bound.floor());
            self.measured = self.measured.max(error.abs());
            if self.bound.as_ref().is_none_or(|worst| worst.le(&bound)) {
                self.bound = Some(bound);
            }
        }
        self.cases += 1;
    }

    fn report(&self, name: &str) {
        let bound = self.bound.as_ref().map_or(0, Ratio::floor);
        println!(
            "{name}: {} cases, worst measured {} raw pixel units, worst derived bound {bound}",
            self.cases, self.measured
        );
    }
}

/// A pinhole camera's round trip at parameter `t_raw`: the landing error and its bound, raw.
fn pinhole_trip(spec: &CameraSpec, sample: PixelSample, t_raw: i64) -> ([i128; 2], [Ratio; 2]) {
    let camera = PinholeCamera::new(spec).expect("valid camera");
    let ray = camera.ray(sample).expect("in-range sample");
    let q = raw3(point_at(&ray, fixed(t_raw)).expect("representable point"));
    let view = View::look_at(spec.position, spec.look, spec.up);
    let aspect = fixed(i64::from(spec.width) << 32)
        .checked_div(fixed(i64::from(spec.height) << 32))
        .expect("an aspect ratio");
    let projection =
        FixedMat4::try_perspective(spec.tan_half_fov, aspect, fixed(ONE >> 10), fixed(1024 * ONE))
            .expect("a projection");
    let row3 = projection.row3();
    assert_eq!([row3.0, row3.1, row3.2, row3.3].map(FixedQ32_32::raw), [0, 0, -ONE, 0]);
    let [x, y, z] = view.apply(q);
    let w = -z;
    let clip = |v: i128, m: FixedQ32_32| v.checked_mul(i128::from(m.raw())).expect("clip fits");
    let numerators = [clip(x, projection.m00()), clip(y, projection.m11())];
    let landed = land((spec.width, spec.height), numerators, w);
    let errors = [0, 1].map(|i| landed[i] - target(sample)[i]);
    (errors, pinhole_bounds(&camera, &view, sample, t_raw, [projection.m00(), projection.m11()]))
}

/// The derived bound of the pinhole round trip on each axis, raw pixel units; see the chapter.
fn pinhole_bounds(
    camera: &PinholeCamera,
    view: &View,
    sample: PixelSample,
    t_raw: i64,
    entries: [FixedQ32_32; 2],
) -> [Ratio; 2] {
    let size = (camera.width(), camera.height());
    let [f, r, u] = [camera.forward(), camera.right(), camera.up()].map(|v| raw3(v.into_inner()));
    let [rp, up, fp] = view.axes;
    let tan = Ratio::raw(i128::from(camera.tan_half_fov().raw()));
    let height = Ratio::int(i128::from(size.1));
    let [a, b] = offsets(sample, size);
    let (alpha, beta) = (a.mul(&tan).div(&height), b.mul(&tan).div(&height));
    let p = axpy(&axpy(&vector(f), &alpha, &vector(r)), &beta, &vector(u));
    let t = Ratio::raw(i128::from(t_raw));
    let tau = t.div(&Ratio::int(1 << camera.scale_shift()));
    // Direction error (1 + (W + H) / 2^33) raw, times t, plus the half raw unit of `point_at`.
    let slack = Ratio::frac(i128::from(size.0) + i128::from(size.1), 1 << 33).add(&Ratio::int(1));
    let e_max = t.mul(&slack).mul(&Ratio::frac(1, 1 << 32)).add(&Ratio::frac(1, 1 << 33));
    // Each translation entry sums three products rounded to nearest: 3/2 raw.
    let eta = Ratio::frac(3, 1 << 33);
    let forward_p = dot(&vector(fp), &p);
    let w_low = tau.mul(&forward_p).sub(&l1(fp).mul(&e_max)).sub(&eta);
    assert_eq!(w_low.sign(), Ordering::Greater, "the point is ahead of the view");
    let mismatch = |axis: [i64; 3], s: &Ratio| {
        let d = dot(&vector(axis), &p).sub(&s.mul(&forward_p));
        let rounding = l1(axis).add(&s.abs().mul(&l1(fp))).mul(&e_max);
        tau.mul(&d.abs()).add(&rounding).add(&Ratio::int(1).add(&s.abs()).mul(&eta))
    };
    let width = Ratio::int(i128::from(size.0));
    let ideal_x = height.div(&tan.mul(&width));
    let ideal_y = Ratio::int(1).div(&tan);
    let ndc_x = ndc_bound(entries[0].raw(), &ideal_x, &alpha, &mismatch(rp, &alpha), &w_low);
    let ndc_y = ndc_bound(entries[1].raw(), &ideal_y, &beta, &mismatch(up, &beta), &w_low);
    [pixel_bound(size.0, &ndc_x), pixel_bound(size.1, &ndc_y)]
}

/// An orthographic camera's round trip at distance `t_raw`: the landing error and its bound.
fn orthographic_trip(
    spec: &OrthographicSpec,
    sample: PixelSample,
    t_raw: i64,
) -> ([i128; 2], [Ratio; 2]) {
    let camera = OrthographicCamera::new(spec).expect("valid camera");
    let ray = camera.ray(sample).expect("in-range sample");
    let q = raw3(point_at(&ray, fixed(t_raw)).expect("representable point"));
    let view = View::look_at(spec.position, spec.look, spec.up);
    let (h, w) = (i128::from(spec.half_height.raw()), i128::from(spec.width));
    let half_width = round_div(h * w, i128::from(spec.height));
    let half_width = fixed(i64::try_from(half_width).expect("a half width"));
    let projection =
        FixedMat4::try_orthographic(half_width, spec.half_height, fixed(0), fixed(1024 * ONE))
            .expect("a projection");
    let row3 = projection.row3();
    assert_eq!([row3.0, row3.1, row3.2, row3.3].map(FixedQ32_32::raw), [0, 0, 0, ONE]);
    assert_eq!([projection.m03(), projection.m13()].map(FixedQ32_32::raw), [0, 0]);
    let [x, y, _] = view.apply(q);
    let clip = |v: i128, m: FixedQ32_32| v.checked_mul(i128::from(m.raw())).expect("clip fits");
    let numerators = [clip(x, projection.m00()), clip(y, projection.m11())];
    let landed = land((spec.width, spec.height), numerators, 1 << 64);
    let errors = [0, 1].map(|i| landed[i] - target(sample)[i]);
    (
        errors,
        orthographic_bounds(&camera, &view, sample, t_raw, [projection.m00(), projection.m11()]),
    )
}

/// The derived bound of the orthographic round trip on each axis, raw pixel units.
fn orthographic_bounds(
    camera: &OrthographicCamera,
    view: &View,
    sample: PixelSample,
    t_raw: i64,
    entries: [FixedQ32_32; 2],
) -> [Ratio; 2] {
    let size = (camera.width(), camera.height());
    let [f, r, u] = [camera.forward(), camera.right(), camera.up()].map(|v| raw3(v.into_inner()));
    let [rp, up, _] = view.axes;
    let half = Ratio::raw(i128::from(camera.half_height().raw()));
    let pixel = half.div(&Ratio::int(i128::from(size.1)));
    let [a, b] = offsets(sample, size);
    let (alpha, beta) = (a.mul(&pixel), b.mul(&pixel));
    let t = Ratio::raw(i128::from(t_raw));
    let g = axpy(
        &axpy(&axpy(&[0, 0, 0].map(Ratio::int), &alpha, &vector(r)), &beta, &vector(u)),
        &t,
        &vector(f),
    );
    // Half a raw unit from the origin's rounding and half from `point_at`'s, and 3/2 raw from
    // the translation entry.
    let rounding =
        |axis: [i64; 3]| l1(axis).mul(&Ratio::frac(1, 1 << 32)).add(&Ratio::frac(3, 1 << 33));
    let mismatch =
        |axis: [i64; 3], s: &Ratio| dot(&vector(axis), &g).sub(s).abs().add(&rounding(axis));
    let one = Ratio::int(1);
    let ideal_x = one.div(&half.mul(&Ratio::frac(i128::from(size.0), i128::from(size.1))));
    let ideal_y = one.div(&half);
    let ndc_x = ndc_bound(entries[0].raw(), &ideal_x, &alpha, &mismatch(rp, &alpha), &one);
    let ndc_y = ndc_bound(entries[1].raw(), &ideal_y, &beta, &mismatch(up, &beta), &one);
    [pixel_bound(size.0, &ndc_x), pixel_bound(size.1, &ndc_y)]
}

fn sample(x: u32, y: u32, sub: (i64, i64)) -> PixelSample {
    PixelSample::new(x, y, fixed(sub.0), fixed(sub.1))
}

fn pinhole_spec(look: [i64; 3], up: [i64; 3], tan: i64, size: (u32, u32)) -> CameraSpec {
    CameraSpec {
        position: vec3([0, 0, 0]),
        look: vec3(look),
        up: vec3(up),
        tan_half_fov: fixed(tan),
        width: size.0,
        height: size.1,
    }
}

fn orthographic_spec(
    look: [i64; 3],
    up: [i64; 3],
    half: i64,
    size: (u32, u32),
) -> OrthographicSpec {
    OrthographicSpec {
        position: vec3([0, 0, 0]),
        look: vec3(look),
        up: vec3(up),
        half_height: fixed(half),
        width: size.0,
        height: size.1,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_318_axis_aligned_cameras_land_exactly_on_their_samples() {
    let mut moved = pinhole_spec(POS_X, POS_Y, HALF, (4, 2));
    moved.position = vec3([ONE, 2 * ONE, 3 * ONE]);
    let pinholes =
        [pinhole_spec(NEG_Z, POS_Y, ONE, (2, 2)), pinhole_spec(NEG_Z, POS_Y, ONE, (4, 2)), moved];
    let mut moved = orthographic_spec(POS_X, POS_Y, HALF, (4, 2));
    moved.position = vec3([ONE, 2 * ONE, 3 * ONE]);
    let orthographics = [orthographic_spec(NEG_Z, POS_Y, ONE, (2, 2)), moved];
    let samples = |(w, h): (u32, u32)| {
        [
            PixelSample::centre(0, 0),
            PixelSample::centre(w - 1, h - 1),
            sample(0, 0, (0, 0)),
            sample(w - 1, 0, (3 * ONE / 4, ONE / 4)),
        ]
    };
    for (index, spec) in pinholes.iter().enumerate() {
        let k = PinholeCamera::new(spec).expect("valid camera").scale_shift();
        for sample in samples((spec.width, spec.height)) {
            for t in [ONE, 2 * ONE, 16 * ONE] {
                let (errors, _) = pinhole_trip(spec, sample, t << k);
                assert_eq!(errors, [0, 0], "pinhole {index}, {sample:?}, t {t}");
            }
        }
    }
    for (index, spec) in orthographics.iter().enumerate() {
        for sample in samples((spec.width, spec.height)) {
            for t in [0, ONE, 16 * ONE] {
                let (errors, _) = orthographic_trip(spec, sample, t);
                assert_eq!(errors, [0, 0], "orthographic {index}, {sample:?}, t {t}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_319_pinhole_points_project_back_to_their_sample() {
    let seed = ROUND_TRIP_SEED ^ 319;
    println!("OPTICS-TP-319 seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let mut worst = Worst::default();
    for index in 0..200 {
        let spec = CameraSpec {
            position: vec3(seeds.triple(64 * ONE)),
            look: vec3(seeds.triple(1 << 36)),
            up: vec3(seeds.triple(1 << 36)),
            tan_half_fov: fixed(seeds.range(ONE >> 4, 4 * ONE)),
            width: seeds.size(),
            height: seeds.size(),
        };
        let Ok(camera) = PinholeCamera::new(&spec) else {
            continue;
        };
        for _ in 0..4 {
            let (x, y) = (seeds.below(spec.width), seeds.below(spec.height));
            let sample = sample(x, y, (seeds.range(0, LAST_SUB), seeds.range(0, LAST_SUB)));
            // Depths along forward of 1/16, 1, 16 and 128 units.
            for depth in [ONE >> 4, ONE, 16 * ONE, 128 * ONE] {
                let (errors, bounds) = pinhole_trip(&spec, sample, depth << camera.scale_shift());
                worst.record(errors, bounds, &format!("camera {index}, {sample:?}, {depth}"));
            }
        }
    }
    worst.report("OPTICS-TP-319");
    assert!(worst.cases >= 3_100, "only {} cases", worst.cases);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_320_orthographic_points_project_back_to_their_sample() {
    let seed = ROUND_TRIP_SEED ^ 320;
    println!("OPTICS-TP-320 seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let mut worst = Worst::default();
    for index in 0..200 {
        let (width, height) = (seeds.size(), seeds.size());
        // Half extents from 2^-4 to 4 units on the longer side.
        let longer = seeds.range(ONE >> 4, 4 * ONE);
        let half_height =
            (i128::from(longer) * i128::from(height) / i128::from(width.max(height))).max(1);
        let spec = OrthographicSpec {
            position: vec3(seeds.triple(64 * ONE)),
            look: vec3(seeds.triple(1 << 36)),
            up: vec3(seeds.triple(1 << 36)),
            half_height: fixed(i64::try_from(half_height).expect("small")),
            width,
            height,
        };
        if OrthographicCamera::new(&spec).is_err() {
            continue;
        }
        for _ in 0..4 {
            let (x, y) = (seeds.below(width), seeds.below(height));
            let sample = sample(x, y, (seeds.range(0, LAST_SUB), seeds.range(0, LAST_SUB)));
            for distance in [0, ONE >> 4, ONE, 16 * ONE, 128 * ONE] {
                let (errors, bounds) = orthographic_trip(&spec, sample, distance);
                worst.record(errors, bounds, &format!("camera {index}, {sample:?}, {distance}"));
            }
        }
    }
    worst.report("OPTICS-TP-320");
    assert!(worst.cases >= 3_900, "only {} cases", worst.cases);
}
