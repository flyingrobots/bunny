//! Golden vectors and determinism oracles for the pinhole camera: hand-computed rays for
//! axis-aligned power-of-two cameras, an exact integer image-plane oracle over seeded
//! cameras, the extremes of the accepted camera domain, and every construction refusal.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    point_at, CameraError, CameraSpec, PinholeCamera, PixelSample, MAX_IMAGE_DIMENSION,
    TAN_HALF_FOV_LIMIT_RAW,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const OPTICS_SEED: u64 = 0xB002_0971_C500_0001;
const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE_RAW / 2;
/// The largest sub-pixel offset, `1 - 2^-32`.
const LAST_SUB: i64 = ONE_RAW - 1;
const ORIGIN: [i64; 3] = [0, 0, 0];
const POS_X: [i64; 3] = [ONE, 0, 0];
const POS_Y: [i64; 3] = [0, ONE, 0];
const POS_Z: [i64; 3] = [0, 0, ONE];
const NEG_Z: [i64; 3] = [0, 0, -ONE];
/// Basis dot products must be below this many raw-squared units: a cosine below 2^-28.
const ORTHOGONALITY_LIMIT: i128 = 1 << 36;

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

fn wide3(v: FixedVec3) -> [i128; 3] {
    raw3(v).map(i128::from)
}

fn dot(a: [i128; 3], b: [i128; 3]) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn spec(look: [i64; 3], up: [i64; 3], tan: i64, size: (u32, u32)) -> CameraSpec {
    CameraSpec {
        position: vec3(ORIGIN),
        look: vec3(look),
        up: vec3(up),
        tan_half_fov: fixed(tan),
        width: size.0,
        height: size.1,
    }
}

fn camera(spec: &CameraSpec) -> PinholeCamera {
    PinholeCamera::new(spec).expect("valid camera")
}

fn sample(x: u32, y: u32, sub: (i64, i64)) -> PixelSample {
    PixelSample::new(x, y, fixed(sub.0), fixed(sub.1))
}

fn direction(camera: &PinholeCamera, sample: PixelSample) -> [i64; 3] {
    raw3(camera.ray(sample).expect("in-range sample").direction())
}

fn basis(camera: &PinholeCamera) -> [[i64; 3]; 3] {
    [camera.forward(), camera.right(), camera.up()].map(|v| raw3(v.into_inner()))
}

/// The exact image-plane oracle of the test plan: `-D - slack < d * D - S <= slack`.
fn check_image_plane(camera: &PinholeCamera, sample: PixelSample, d: [i64; 3]) -> Result<(), String> {
    let (w, h) = (i128::from(camera.width()), i128::from(camera.height()));
    let k = camera.scale_shift();
    let a = (2 * i128::from(sample.x) - w) * i128::from(ONE) + 2 * i128::from(sample.sub_x.raw());
    let b = (2 * i128::from(sample.y) - h) * i128::from(ONE) + 2 * i128::from(sample.sub_y.raw());
    let t = i128::from(camera.tan_half_fov().raw());
    let [f, r, u] = basis(camera).map(|v| v.map(i128::from));
    let denominator = h.checked_mul(1 << (64 + k)).ok_or("denominator overflows")?;
    let slack = ((w + h) * h).checked_mul(1 << (31 + k)).ok_or("slack overflows")?;
    for axis in 0..3 {
        let exact = f[axis] * h * (1 << 64) + a * t * r[axis] + b * t * u[axis];
        let scaled = i128::from(d[axis]).checked_mul(denominator).ok_or("product overflows")?;
        let error = scaled - exact;
        if error <= -denominator - slack || error > slack {
            return Err(format!("axis {axis}: d = {d:?}, error {error} over D = {denominator}"));
        }
    }
    Ok(())
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_001_golden_basis_is_right_handed() {
    // (look, up hint, expected [forward, right, up]).
    let tilted = [0, ONE, ONE];
    let goldens = [
        (NEG_Z, POS_Y, [NEG_Z, POS_X, POS_Y]),
        (POS_X, POS_Y, [POS_X, POS_Z, POS_Y]),
        (POS_Y, POS_Z, [POS_Y, POS_X, POS_Z]),
        // A hint tilted toward the viewer fixes the same roll as +Y.
        (NEG_Z, tilted, [NEG_Z, POS_X, POS_Y]),
        // A look vector of any length gives the same forward.
        ([0, 0, -5 * ONE], POS_Y, [NEG_Z, POS_X, POS_Y]),
    ];
    for (index, (look, up, expected)) in goldens.into_iter().enumerate() {
        let camera = camera(&spec(look, up, ONE, (2, 2)));
        assert_eq!(basis(&camera), expected, "golden {index}");
        let [forward, right, up] = [camera.forward(), camera.right(), camera.up()];
        assert_eq!(forward.into_inner().cross(up.into_inner()), right.into_inner(), "{index}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_002_golden_rays_lock_raw_outputs() {
    let down = |size| camera(&spec(NEG_Z, POS_Y, ONE, size));
    let square = down((2, 2));
    let wide = down((4, 2));
    let thirds = down((3, 3));
    let mut moved_spec = spec(POS_X, POS_Y, HALF, (2, 2));
    moved_spec.position = vec3([ONE, 2 * ONE, 3 * ONE]);
    let moved = camera(&moved_spec);
    let upward = camera(&spec(POS_Y, POS_Z, ONE, (2, 2)));
    assert_eq!(
        [&square, &wide, &thirds, &moved, &upward].map(PinholeCamera::scale_shift),
        [0, 1, 0, 0, 0]
    );
    // floor(2^65 / 3) / 2^32 = 2_863_311_530.67: two thirds of a unit, floored either way.
    let two_thirds = 2_863_311_530;
    // (camera, sample, expected raw direction).
    let goldens = [
        // 2x2, tan 1: centres at (-1/2, -1/2, -1) and (1/2, 1/2, -1); the corner sample of the
        // image rectangle is exactly (-1, -1, -1), on the direction limit.
        (&square, PixelSample::centre(0, 0), [-HALF, -HALF, -ONE]),
        (&square, PixelSample::centre(1, 1), [HALF, HALF, -ONE]),
        (&square, sample(0, 0, (0, 0)), [-ONE, -ONE, -ONE]),
        // x = floor((2^33 - 2) / 2): the last sub-pixel offset of the right column.
        (&square, sample(1, 0, (LAST_SUB, 0)), [ONE - 1, -ONE, -ONE]),
        // 4x2, tan 1: corners at x = +-2 need one halving, so directions are half the
        // image-plane point: (-3/2, -1/2, -1) / 2.
        (&wide, PixelSample::centre(0, 0), [-3 * ONE / 4, -ONE / 4, -HALF]),
        (&wide, PixelSample::centre(3, 1), [3 * ONE / 4, ONE / 4, -HALF]),
        // 3x3, tan 1: the step is floor(2^65 / 3) fine units, so -2/3 floors to one raw unit
        // below and +2/3 floors to the raw unit below its exact value.
        (&thirds, PixelSample::centre(0, 0), [-two_thirds - 1, -two_thirds - 1, -ONE]),
        (&thirds, PixelSample::centre(2, 0), [two_thirds, -two_thirds - 1, -ONE]),
        (&thirds, PixelSample::centre(1, 1), [0, 0, -ONE]),
        // Looking along +X with up +Y, right is +Z; tan 1/2 puts the centre of pixel (1, 0)
        // at (1, -1/4, 1/4).
        (&moved, PixelSample::centre(1, 0), [ONE, -ONE / 4, ONE / 4]),
        // Looking along +Y with up +Z, right is +X: pixel (0, 1) is up and to the left.
        (&upward, PixelSample::centre(0, 1), [-HALF, ONE, HALF]),
    ];
    for (index, (camera, sample, expected)) in goldens.into_iter().enumerate() {
        assert_eq!(direction(camera, sample), expected, "golden {index}");
        check_image_plane(camera, sample, expected).expect("golden is inside the bound");
    }
    let ray = moved.ray(PixelSample::centre(1, 0)).expect("in range");
    assert_eq!(raw3(ray.origin()), [ONE, 2 * ONE, 3 * ONE]);
    assert_eq!(PinholeCamera::new(&moved_spec), Ok(moved), "construction is deterministic");
}

fn seeded_spec(seeds: &mut Seeds) -> CameraSpec {
    let (width, height) = (seeds.range(1, 4096), seeds.range(1, 4096));
    CameraSpec {
        position: vec3(seeds.triple(TRACE_COORD_LIMIT_RAW - 1)),
        look: vec3(seeds.triple(1 << 36)),
        up: vec3(seeds.triple(1 << 36)),
        tan_half_fov: fixed(seeds.range(1 << 20, 16 * ONE)),
        width: u32::try_from(width).expect("small"),
        height: u32::try_from(height).expect("small"),
    }
}

fn check_basis(camera: &PinholeCamera, spec: &CameraSpec) {
    let [f, r, u] = basis(camera).map(|v| v.map(i128::from));
    for (a, b) in [(f, r), (f, u), (r, u)] {
        assert!(dot(a, b).abs() < ORTHOGONALITY_LIMIT, "basis is not orthogonal");
    }
    let right_handed = camera.forward().into_inner().cross(camera.up().into_inner());
    assert!(dot(wide3(right_handed), r) > 0, "right is forward x up");
    assert!(dot(u, wide3(spec.up)) > 0, "up is on the hint's side");
    assert!(dot(f, wide3(spec.look)) > 0, "forward follows the look direction");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_003_rays_pass_through_the_intended_image_plane_point() {
    let mut seeds = Seeds(OPTICS_SEED ^ 3);
    // A bounded number of attempts, so a camera that refuses everything fails instead of
    // looping forever; seeded look and up vectors are almost never degenerate.
    let mut cameras = 0;
    for _ in 0..2_000 {
        let spec = seeded_spec(&mut seeds);
        let Ok(camera) = PinholeCamera::new(&spec) else {
            continue;
        };
        cameras += 1;
        check_basis(&camera, &spec);
        for _ in 0..4 {
            let (x, y) = (seeds.below(spec.width), seeds.below(spec.height));
            let sample = sample(x, y, (seeds.range(0, LAST_SUB), seeds.range(0, LAST_SUB)));
            let ray = camera.ray(sample).expect("in-range sample");
            assert_eq!(raw3(ray.origin()), raw3(spec.position), "camera {cameras}");
            let d = raw3(ray.direction());
            assert!(dot(d.map(i128::from), wide3(camera.forward().into_inner())) > 0);
            check_image_plane(&camera, sample, d).expect("ray meets the image-plane point");
        }
    }
    assert!(cameras >= 1_990, "only {cameras} of 2 000 seeded cameras constructed");
}

fn corner_samples(width: u32, height: u32) -> [PixelSample; 9] {
    let (right, top) = (width - 1, height - 1);
    [
        sample(0, 0, (0, 0)),
        sample(0, 0, (LAST_SUB, LAST_SUB)),
        sample(right, 0, (LAST_SUB, 0)),
        sample(right, 0, (0, LAST_SUB)),
        sample(0, top, (0, LAST_SUB)),
        sample(0, top, (LAST_SUB, 0)),
        sample(right, top, (LAST_SUB, LAST_SUB)),
        sample(right, top, (0, 0)),
        PixelSample::centre(width / 2, height / 2),
    ]
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_004_corner_samples_of_extreme_cameras_stay_in_the_trace_domain() {
    let (widest, largest) = (TAN_HALF_FOV_LIMIT_RAW, MAX_IMAGE_DIMENSION);
    let lenses = [
        (widest, largest, largest),
        (widest, largest, 1),
        (widest, 1, largest),
        (1, largest, largest),
        (ONE, 1, 1),
        // Pixel-centre corners of this camera fit at scale shift 0; rectangle corners do not.
        (ONE + (1 << 12), 1024, 1024),
    ];
    for look in [NEG_Z, [ONE, ONE, ONE], [-ONE, 2 * ONE, -3 * ONE]] {
        for (tan, width, height) in lenses {
            let camera = camera(&spec(look, POS_Y, tan, (width, height)));
            let forward = wide3(camera.forward().into_inner());
            for (index, sample) in corner_samples(width, height).into_iter().enumerate() {
                let label = format!("look {look:?}, lens {tan}/{width}x{height}, sample {index}");
                let ray = camera.ray(sample).unwrap_or_else(|| panic!("no ray: {label}"));
                let d = raw3(ray.direction());
                assert!(d.iter().all(|c| c.unsigned_abs() <= ONE.unsigned_abs()), "{label}");
                assert!(dot(d.map(i128::from), forward) > 0, "faces forward: {label}");
                check_image_plane(&camera, sample, d).unwrap_or_else(|e| panic!("{label}: {e}"));
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_005_degenerate_cameras_are_refused() {
    let base = spec(NEG_Z, POS_Y, ONE, (2, 2));
    let with = |edit: fn(&mut CameraSpec)| {
        let mut spec = base;
        edit(&mut spec);
        PinholeCamera::new(&spec).map(|_| ())
    };
    let edge = TRACE_COORD_LIMIT_RAW;
    let cases: [(fn(&mut CameraSpec), Result<(), CameraError>); 22] = [
        (|_| {}, Ok(())),
        (|s| s.look = vec3(ORIGIN), Err(CameraError::ZeroLook)),
        (|s| s.up = vec3(ORIGIN), Err(CameraError::UpParallelToLook)),
        (|s| s.up = vec3(NEG_Z), Err(CameraError::UpParallelToLook)),
        (|s| s.up = vec3(POS_Z), Err(CameraError::UpParallelToLook)),
        (|s| s.up = vec3([0, 0, 5 * ONE]), Err(CameraError::UpParallelToLook)),
        // (1, 1, 0) raw normalizes to (1.0, 1.0, 0), which is not unit length.
        (|s| s.look = vec3([1, 1, 0]), Err(CameraError::Unnormalizable)),
        (|s| s.look = vec3([i64::MAX; 3]), Err(CameraError::Unnormalizable)),
        (|s| s.up = vec3([i64::MAX; 3]), Err(CameraError::Unnormalizable)),
        (|s| s.position = vec3([TRACE_COORD_LIMIT_RAW - 1, 0, 0]), Ok(())),
        (|s| s.position = vec3([0, 1 - TRACE_COORD_LIMIT_RAW, 0]), Ok(())),
        (|s| s.position = vec3([0, 0, TRACE_COORD_LIMIT_RAW]), Err(CameraError::PositionOutsideTraceDomain)),
        (|s| s.position = vec3([-TRACE_COORD_LIMIT_RAW, 0, 0]), Err(CameraError::PositionOutsideTraceDomain)),
        (|s| s.width = 0, Err(CameraError::EmptyImage)),
        (|s| s.height = 0, Err(CameraError::EmptyImage)),
        (|s| s.width = MAX_IMAGE_DIMENSION, Ok(())),
        (|s| s.width = MAX_IMAGE_DIMENSION + 1, Err(CameraError::ImageTooLarge)),
        (|s| s.height = MAX_IMAGE_DIMENSION + 1, Err(CameraError::ImageTooLarge)),
        (|s| s.tan_half_fov = fixed(0), Err(CameraError::TangentNotPositive)),
        (|s| s.tan_half_fov = fixed(-ONE), Err(CameraError::TangentNotPositive)),
        (|s| s.tan_half_fov = fixed(TAN_HALF_FOV_LIMIT_RAW), Ok(())),
        (|s| s.tan_half_fov = fixed(TAN_HALF_FOV_LIMIT_RAW + 1), Err(CameraError::TangentTooLarge)),
    ];
    assert_eq!(edge, 1 << 40, "the trace domain is below 256 units");
    for (index, (edit, expected)) in cases.into_iter().enumerate() {
        assert_eq!(with(edit), expected, "case {index}");
    }
    // The smallest tangent is one raw unit.
    assert_eq!(with(|s| s.tan_half_fov = fixed(1)), Ok(()));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_006_samples_outside_the_image_are_refused() {
    let camera = camera(&spec(NEG_Z, POS_Y, ONE, (4, 3)));
    let refused = [
        sample(4, 0, (0, 0)),
        sample(0, 3, (0, 0)),
        sample(u32::MAX, u32::MAX, (0, 0)),
        sample(0, 0, (ONE, 0)),
        sample(0, 0, (0, ONE)),
        sample(0, 0, (-1, 0)),
        sample(0, 0, (0, -1)),
        sample(0, 0, (i64::MIN, i64::MAX)),
    ];
    for (index, sample) in refused.into_iter().enumerate() {
        assert_eq!(camera.ray(sample), None, "refused {index}");
    }
    for sample in [sample(3, 2, (LAST_SUB, LAST_SUB)), sample(0, 0, (0, 0))] {
        assert!(camera.ray(sample).is_some());
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_007_pixel_origin_is_bottom_left_and_rows_grow_upward() {
    let camera = camera(&spec(NEG_Z, POS_Y, ONE, (8, 4)));
    let first = direction(&camera, PixelSample::centre(0, 0));
    let last = direction(&camera, PixelSample::centre(7, 3));
    assert!(first[0] < 0 && first[1] < 0, "pixel (0, 0) is bottom-left: {first:?}");
    assert!(last[0] > 0 && last[1] > 0, "the last pixel is top-right: {last:?}");
    let next_column = direction(&camera, PixelSample::centre(1, 0));
    let next_row = direction(&camera, PixelSample::centre(0, 1));
    // One pixel is 2 * tan / height = 1/2 unit on the plane one unit ahead; the scale shift
    // is 1 here (corners at x = +-2), so one pixel moves the direction by 1/4.
    assert_eq!(camera.scale_shift(), 1);
    assert_eq!(next_column, [first[0] + ONE / 4, first[1], first[2]]);
    assert_eq!(next_row, [first[0], first[1] + ONE / 4, first[2]]);
}

fn ray(origin: [i64; 3], direction: [i64; 3]) -> TraceRay {
    TraceRay::new(vec3(origin), vec3(direction)).expect("in the trace domain")
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_008_point_at_rounds_and_refuses_overflow() {
    let moved = ray([ONE, 2 * ONE, 3 * ONE], [HALF, -ONE / 4, ONE]);
    assert_eq!(point_at(&moved, fixed(2 * ONE)).map(raw3), Some([2 * ONE, 3 * ONE / 2, 5 * ONE]));
    assert_eq!(point_at(&moved, fixed(0)).map(raw3), Some([ONE, 2 * ONE, 3 * ONE]));
    // 1 raw * 0.5 and 3 raw * 0.5 are ties: they round to the even raw values 0 and 2.
    let tiny = ray(ORIGIN, [1, 3, -3]);
    assert_eq!(point_at(&tiny, fixed(HALF)).map(raw3), Some([0, 2, -2]));
    // A distance so large that origin + direction * t leaves Q32.32.
    let far = ray([TRACE_COORD_LIMIT_RAW - 1, 0, 0], POS_X);
    assert_eq!(point_at(&far, fixed(i64::MAX)), None);
    // A camera ray meets the plane z = -4. The distance is the floor of the true distance in
    // multiples of the direction, so the point is on the camera's side of the plane by less
    // than one raw unit of distance times |d_z| <= 1.0, plus the rounding of the product.
    let camera = camera(&spec(NEG_Z, POS_Y, ONE, (5, 3)));
    let plane = |x: i64, y: i64| [x * 64 * ONE, y * 64 * ONE, -4 * ONE];
    let triangle =
        TraceTriangle::new(vec3(plane(-1, -1)), vec3(plane(1, -1)), vec3(plane(0, 1)))
            .expect("in the trace domain");
    for (x, y) in [(0, 0), (4, 2), (2, 1), (1, 2)] {
        let ray = camera.ray(PixelSample::centre(x, y)).expect("in range");
        let hit = ray.hit_triangle(&triangle, fixed(0), fixed(i64::MAX)).expect("hits");
        let point = point_at(&ray, hit.distance).map(raw3).expect("representable");
        assert!((-4 * ONE - 1..=-4 * ONE + 2).contains(&point[2]), "pixel ({x}, {y}): {point:?}");
    }
}
