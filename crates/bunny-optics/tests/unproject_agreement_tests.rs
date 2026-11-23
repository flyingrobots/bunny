//! Agreement of matrix-side unprojection (`Viewport::try_unproject`, `bunny-unproject/1`) with
//! the cameras: for the same sample, the ray unprojected through `FixedMat4::try_look_at`, a
//! projection builder and the matching `Viewport` lies along `PinholeCamera::ray` or
//! `OrthographicCamera::ray`.
//!
//! The two sides round differently (the camera floors its direction, the matrices round their
//! entries and basis), so the comparison is within a tolerance. The tolerances below are
//! MEASURED, not proven: each is a power of two four times the worst case of the seeded run, and
//! the run prints the worst level it reached. A convention mismatch (the
//! handedness, the direction of `y`, the pixel centre) would miss them by many orders of
//! magnitude, as the round trip of `docs/topics/optics/README.md#projection-round-trip` does.

// Only the signed big integers of the shared oracle module are used here.
#[allow(dead_code)]
mod exact;

use std::cmp::Ordering;

use bunny_linalg::{FixedMat4, FixedVec2, FixedVec3, Viewport, ViewportRay};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{CameraSpec, OrthographicCamera, OrthographicSpec, PinholeCamera, PixelSample};
use bunny_query::TraceRay;
use exact::{Big, Signed};
use wasm_bindgen_test::wasm_bindgen_test;

const AGREEMENT_SEED: u64 = 0x0150_A61E_E000_0001;
const ONE: i64 = ONE_RAW;
const LAST_SUB: i64 = ONE_RAW - 1;
/// Measured tolerance: the camera direction's distance from the unprojected line is at most
/// `2^6` raw. The seeded runs reach `2^4` (pinhole) and `2^0` (orthographic).
const DIRECTION_LEVEL: u32 = 6;
/// Measured tolerance: the unprojected origin's distance from the camera ray is at most `2^6`
/// raw. The seeded runs reach `2^3` (pinhole) and `2^4` (orthographic).
const ORIGIN_LEVEL: u32 = 6;
/// Measured tolerance: the camera ray's origin (the eye, or its point on the view rectangle) is
/// at most `2^6` raw from the unprojected line. The runs reach `2^2` and `2^4`.
const EYE_LEVEL: u32 = 6;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low + 1).expect("non-empty range");
        low + i64::try_from(self.next() % span).expect("below the span")
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    fn size(&mut self) -> u32 {
        u32::try_from(self.range(1, 2048)).expect("small")
    }

    fn sample(&mut self, width: u32, height: u32) -> PixelSample {
        let below =
            |s: &mut Self, n: u32| u32::try_from(s.range(0, i64::from(n) - 1)).expect("u32");
        let (x, y) = (below(self, width), below(self, height));
        let (sub_x, sub_y) = (self.range(0, LAST_SUB), self.range(0, LAST_SUB));
        PixelSample::new(x, y, fixed(sub_x), fixed(sub_y))
    }
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> [i128; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()].map(i128::from)
}

/// The sample's continuous viewport coordinate `(x + sub_x, y + sub_y)`.
fn coordinate(sample: PixelSample) -> FixedVec2 {
    let axis = |pixel: u32, sub: FixedQ32_32| fixed(i64::from(pixel) * ONE + sub.raw());
    FixedVec2::new(axis(sample.x, sample.sub_x), axis(sample.y, sample.sub_y))
}

fn view_of(position: FixedVec3, look: FixedVec3, up: FixedVec3) -> FixedMat4 {
    let target = vec3([0, 1, 2].map(|i| {
        let [p, l] = [raw3(position)[i], raw3(look)[i]];
        i64::try_from(p + l).expect("target in range")
    }));
    FixedMat4::try_look_at(position, target, up).expect("a view matrix")
}

fn cross(a: [i128; 3], b: [i128; 3]) -> [Signed; 3] {
    let term = |x: i128, y: i128, z: i128, w: i128| {
        Signed::new(x).mul(&Signed::new(y)).add(&Signed::new(z).mul(&Signed::new(w)).negated())
    };
    [term(a[1], b[2], a[2], b[1]), term(a[2], b[0], a[0], b[2]), term(a[0], b[1], a[1], b[0])]
}

fn squared(v: &[Signed; 3]) -> Big {
    v.iter().fold(Big::new(0), |sum, c| sum.add(&c.square()))
}

/// The smallest `k` with `|point x line|^2 <= 4^k |line|^2`: the distance from `point` (a
/// displacement from a point on the line) to the line through the origin along `line` is at
/// most `2^k` raw.
fn level(point: [i128; 3], line: [i128; 3]) -> u32 {
    let (numerator, length) = (squared(&cross(point, line)), squared(&line.map(Signed::new)));
    (0..64)
        .find(|&k| numerator.compare(&length.mul(&Big::new(1 << (2 * k)))) != Ordering::Greater)
        .expect("below 2^63 raw")
}

fn difference(a: [i128; 3], b: [i128; 3]) -> [i128; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [i128; 3], b: [i128; 3]) -> Signed {
    (0..3).fold(Signed::new(0), |sum, i| sum.add(&Signed::new(a[i]).mul(&Signed::new(b[i]))))
}

/// The worst level of each measure over a run.
#[derive(Default)]
struct Worst {
    direction: u32,
    origin: u32,
    eye: u32,
    cases: u32,
}

impl Worst {
    /// Records one case: the unprojected ray against the camera ray.
    fn record(&mut self, ours: &ViewportRay, camera: &TraceRay, label: &str) {
        let (o, d) = (raw3(ours.origin), raw3(ours.direction));
        let (e, c) = (raw3(camera.origin()), raw3(camera.direction()));
        assert_eq!(dot(c, d).sign(), Ordering::Greater, "{label}: opposite directions");
        let levels = [level(c, d), level(difference(o, e), c), level(difference(e, o), d)];
        assert!(levels[0] <= DIRECTION_LEVEL, "{label}: direction level {}", levels[0]);
        assert!(levels[1] <= ORIGIN_LEVEL, "{label}: origin level {}", levels[1]);
        assert!(levels[2] <= EYE_LEVEL, "{label}: eye level {}", levels[2]);
        self.direction = self.direction.max(levels[0]);
        self.origin = self.origin.max(levels[1]);
        self.eye = self.eye.max(levels[2]);
        self.cases += 1;
    }

    fn report(&self, name: &str) {
        println!(
            "{name}: {} cases; worst measured distances at most 2^{} raw (direction), 2^{} raw \
             (origin), 2^{} raw (eye)",
            self.cases, self.direction, self.origin, self.eye
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_020_unprojected_rays_agree_with_the_pinhole_camera() {
    let seed = AGREEMENT_SEED ^ 0x01;
    println!("CL-TP-020 pinhole seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let mut worst = Worst::default();
    for index in 0..300 {
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
        let view = view_of(spec.position, spec.look, spec.up);
        let aspect = fixed(i64::from(spec.width) << 32)
            .checked_div(fixed(i64::from(spec.height) << 32))
            .expect("an aspect ratio");
        let projection = FixedMat4::try_perspective(
            spec.tan_half_fov,
            aspect,
            fixed(ONE >> 10),
            fixed(1024 * ONE),
        )
        .expect("a projection");
        let viewport = Viewport::try_new(spec.width, spec.height).expect("non-empty");
        for _ in 0..8 {
            let sample = seeds.sample(spec.width, spec.height);
            let ours = viewport.try_unproject(coordinate(sample), projection, view);
            let ours = ours.expect("an unprojected ray");
            let theirs = camera.ray(sample).expect("an in-range sample");
            worst.record(&ours, &theirs, &format!("camera {index}, {sample:?}"));
        }
    }
    worst.report("CL-TP-020 pinhole");
    assert!(worst.cases >= 2_000, "only {} cases", worst.cases);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_020_unprojected_rays_agree_with_the_orthographic_camera() {
    let seed = AGREEMENT_SEED ^ 0x02;
    println!("CL-TP-020 orthographic seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let mut worst = Worst::default();
    for index in 0..300 {
        let (width, height) = (seeds.size(), seeds.size());
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
        let Ok(camera) = OrthographicCamera::new(&spec) else {
            continue;
        };
        let view = view_of(spec.position, spec.look, spec.up);
        // Half the width, rounded to nearest: `half_height * width / height`.
        let (h, w) = (half_height, i128::from(width));
        let half_width = (2 * h * w + i128::from(height)) / (2 * i128::from(height));
        let half_width = fixed(i64::try_from(half_width).expect("small"));
        let projection =
            FixedMat4::try_orthographic(half_width, spec.half_height, fixed(0), fixed(1024 * ONE))
                .expect("a projection");
        let viewport = Viewport::try_new(width, height).expect("non-empty");
        for _ in 0..8 {
            let sample = seeds.sample(width, height);
            let ours = viewport.try_unproject(coordinate(sample), projection, view);
            let ours = ours.expect("an unprojected ray");
            let theirs = camera.ray(sample).expect("an in-range sample");
            worst.record(&ours, &theirs, &format!("camera {index}, {sample:?}"));
        }
    }
    worst.report("CL-TP-020 orthographic");
    assert!(worst.cases >= 2_000, "only {} cases", worst.cases);
}
