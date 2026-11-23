//! Golden vectors, an exact integer oracle and every refusal of the orthographic camera
//! `bunny-orthographic-camera/1`: parallel rays along `forward`, each from its sample's point on
//! the view rectangle, rounded once per component to nearest, ties to even.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    OrthographicCamera, OrthographicError, OrthographicSpec, PixelSample, MAX_IMAGE_DIMENSION,
    ORTHOGRAPHIC_CAMERA_ID, ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW,
};
use bunny_query::TRACE_COORD_LIMIT_RAW;
use wasm_bindgen_test::wasm_bindgen_test;

const ORTHO_SEED: u64 = 0x0257_0E7A_C0DE_0001;
const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE_RAW / 2;
const LAST_SUB: i64 = ONE_RAW - 1;
const ORIGIN: [i64; 3] = [0, 0, 0];
const POS_X: [i64; 3] = [ONE, 0, 0];
const POS_Y: [i64; 3] = [0, ONE, 0];
const NEG_Z: [i64; 3] = [0, 0, -ONE];
const LIMIT: i64 = TRACE_COORD_LIMIT_RAW;

/// A spec edit and the construction result it must produce.
type RefusalCase = (fn(&mut OrthographicSpec), Result<(), OrthographicError>);

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

fn spec(look: [i64; 3], up: [i64; 3], half_height: i64, size: (u32, u32)) -> OrthographicSpec {
    OrthographicSpec {
        position: vec3(ORIGIN),
        look: vec3(look),
        up: vec3(up),
        half_height: fixed(half_height),
        width: size.0,
        height: size.1,
    }
}

fn camera(spec: &OrthographicSpec) -> OrthographicCamera {
    OrthographicCamera::new(spec).expect("valid camera")
}

fn sample(x: u32, y: u32, sub: (i64, i64)) -> PixelSample {
    PixelSample::new(x, y, fixed(sub.0), fixed(sub.1))
}

/// The ray's origin and direction, raw.
fn ray(camera: &OrthographicCamera, sample: PixelSample) -> ([i64; 3], [i64; 3]) {
    let ray = camera.ray(sample).expect("in-range sample");
    (raw3(ray.origin()), raw3(ray.direction()))
}

/// The exact origin oracle, written from the chapter's formula and sharing nothing with the
/// kernel: with `D = height 2^64` and `N_i = a h R_i + b h U_i` (raw `a`, `b`, `h`, `R`, `U`),
/// the offset `o_i - p_i` is `N_i / D` rounded to nearest, ties to even. Returns the error
/// `(o_i - p_i) D - N_i` per axis after checking it.
fn check_origin(camera: &OrthographicCamera, sample: PixelSample, origin: [i64; 3]) -> [i128; 3] {
    let (w, h) = (i128::from(camera.width()), i128::from(camera.height()));
    let one = i128::from(ONE);
    let a = (2 * i128::from(sample.x) - w) * one + 2 * i128::from(sample.sub_x.raw());
    let b = (h - 2 * i128::from(sample.y)) * one - 2 * i128::from(sample.sub_y.raw());
    let half = i128::from(camera.half_height().raw());
    let right = raw3(camera.right().into_inner());
    let up = raw3(camera.up().into_inner());
    let position = raw3(camera.position());
    let denominator = h << 64;
    let mut errors = [0; 3];
    for (axis, error) in errors.iter_mut().enumerate() {
        let numerator = a * half * i128::from(right[axis]) + b * half * i128::from(up[axis]);
        let offset = i128::from(origin[axis]) - i128::from(position[axis]);
        *error = offset * denominator - numerator;
        assert!(2 * error.abs() <= denominator, "axis {axis}: more than half a raw unit off");
        if 2 * error.abs() == denominator {
            assert_eq!(offset & 1, 0, "axis {axis}: a tie goes to the even raw value");
        }
    }
    errors
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_310_golden_orthographic_rays_lock_raw_outputs() {
    assert_eq!(ORTHOGRAPHIC_CAMERA_ID, "bunny-orthographic-camera/1");
    let square = camera(&spec(NEG_Z, POS_Y, ONE, (2, 2)));
    let thirds = camera(&spec(NEG_Z, POS_Y, ONE, (3, 3)));
    let mut moved_spec = spec(POS_X, POS_Y, HALF, (2, 2));
    moved_spec.position = vec3([ONE, 2 * ONE, 3 * ONE]);
    let moved = camera(&moved_spec);
    // round(2^33 / 3) = 2_863_311_531: two thirds of a unit, rounded to nearest.
    let two_thirds = 2_863_311_531;
    // (camera, sample, expected raw origin). Every direction is the camera's forward.
    let goldens = [
        // 2x2, half height 1: one pixel is one unit; the centre of (0, 0) is (-1/2, 1/2).
        (&square, PixelSample::centre(0, 0), [-HALF, HALF, 0]),
        (&square, PixelSample::centre(1, 1), [HALF, -HALF, 0]),
        // The top-left corner of the view rectangle.
        (&square, sample(0, 0, (0, 0)), [-ONE, ONE, 0]),
        // The last sub-pixel offset of the right column: one raw unit inside the edge.
        (&square, sample(1, 0, (LAST_SUB, 0)), [ONE - 1, ONE, 0]),
        // 3x3, half height 1: the centre of (0, 0) is (-2/3, 2/3), rounded once each.
        (&thirds, PixelSample::centre(0, 0), [-two_thirds, two_thirds, 0]),
        (&thirds, PixelSample::centre(1, 1), [0, 0, 0]),
        // Looking along +X with up +Y, right is +Z; a pixel is half a unit, so the centre of
        // (1, 0) is a quarter unit along right and along up from the position.
        (&moved, PixelSample::centre(1, 0), [ONE, 2 * ONE + ONE / 4, 3 * ONE + ONE / 4]),
    ];
    for (index, (camera, sample, expected)) in goldens.into_iter().enumerate() {
        let (origin, direction) = ray(camera, sample);
        assert_eq!(origin, expected, "golden {index}");
        assert_eq!(direction, raw3(camera.forward().into_inner()), "golden {index}");
        check_origin(camera, sample, origin);
    }
    assert_eq!(ray(&moved, PixelSample::centre(0, 0)).1, POS_X);
    assert_eq!(OrthographicCamera::new(&moved_spec), Ok(moved), "construction is deterministic");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_311_origin_ties_round_to_even() {
    // A 1x1 image of half height h raw looking down -z: the offset along right is
    // `(2 sub - 1) h` raw, so `sub = 3/4` gives `h / 2` and `sub = 1/4` gives `-h / 2`.
    let (quarter, three_quarters) = (ONE / 4, 3 * ONE / 4);
    // (half height raw, sub_x, expected origin x): 1/2 and -1/2 go to 0, 3/2 to 2, -3/2 to -2,
    // and 5/2 to 2.
    let cases = [
        (1, three_quarters, 0),
        (1, quarter, 0),
        (3, three_quarters, 2),
        (3, quarter, -2),
        (5, three_quarters, 2),
        (5, quarter, -2),
        (7, three_quarters, 4),
    ];
    for (index, (half_height, sub_x, expected)) in cases.into_iter().enumerate() {
        let camera = camera(&spec(NEG_Z, POS_Y, half_height, (1, 1)));
        let sample = sample(0, 0, (sub_x, HALF));
        let (origin, _) = ray(&camera, sample);
        assert_eq!(origin, [expected, 0, 0], "tie {index}");
        let errors = check_origin(&camera, sample, origin);
        assert_eq!(2 * errors[0].abs(), 1 << 64, "case {index} is an exact tie");
    }
}

fn seeded_spec(seeds: &mut Seeds) -> OrthographicSpec {
    let (width, height) = (seeds.range(1, 4096), seeds.range(1, 4096));
    let half_height = seeds.range(ONE >> 8, 16 * ONE);
    // A wide image's half width is the half height times up to 4096, so its half height is
    // divided by 4096: every half extent is then at most 16 units and every view fits.
    let reach = (half_height / 4096).max(1);
    OrthographicSpec {
        position: vec3(seeds.triple(64 * ONE)),
        look: vec3(seeds.triple(1 << 36)),
        up: vec3(seeds.triple(1 << 36)),
        half_height: fixed(if width > height { reach } else { half_height }),
        width: u32::try_from(width).expect("small"),
        height: u32::try_from(height).expect("small"),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_312_every_origin_is_the_exact_point_rounded_once() {
    let seed = ORTHO_SEED ^ 0x138;
    println!("OPTICS-TP-312 seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let (mut cameras, mut rays) = (0, 0);
    for _ in 0..1_000 {
        let spec = seeded_spec(&mut seeds);
        let Ok(camera) = OrthographicCamera::new(&spec) else {
            continue;
        };
        cameras += 1;
        for _ in 0..8 {
            let (x, y) = (seeds.below(spec.width), seeds.below(spec.height));
            let sample = sample(x, y, (seeds.range(0, LAST_SUB), seeds.range(0, LAST_SUB)));
            let (origin, direction) = ray(&camera, sample);
            assert_eq!(direction, raw3(camera.forward().into_inner()), "camera {cameras}");
            assert!(origin.iter().all(|c| c.abs() < LIMIT), "camera {cameras}: {origin:?}");
            check_origin(&camera, sample, origin);
            rays += 1;
        }
    }
    println!("OPTICS-TP-312: {cameras} cameras, {rays} rays");
    assert!(cameras >= 990, "only {cameras} of 1 000 seeded cameras constructed");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_313_construction_refusals_and_their_thresholds() {
    let base = spec(NEG_Z, POS_Y, ONE, (1, 1));
    let with = |edit: fn(&mut OrthographicSpec)| {
        let mut spec = base;
        edit(&mut spec);
        OrthographicCamera::new(&spec).map(|_| ())
    };
    // With a 1x1 image of half height 1 looking down -z, the view rectangle's corners are
    // the position plus and minus one unit along x and y.
    let cases: [RefusalCase; 28] = [
        (|_| {}, Ok(())),
        (|s| s.position = vec3([LIMIT - 1, 0, 0]), Err(OrthographicError::ViewOutsideTraceDomain)),
        (|s| s.position = vec3([0, 0, LIMIT]), Err(OrthographicError::PositionOutsideTraceDomain)),
        (|s| s.position = vec3([-LIMIT, 0, 0]), Err(OrthographicError::PositionOutsideTraceDomain)),
        // The corner at x = position + 1 unit: 2^40 - 1 raw is inside, 2^40 is not.
        (|s| s.position = vec3([LIMIT - 1 - ONE, 0, 0]), Ok(())),
        (
            |s| s.position = vec3([LIMIT - ONE, 0, 0]),
            Err(OrthographicError::ViewOutsideTraceDomain),
        ),
        (|s| s.position = vec3([0, 1 + ONE - LIMIT, 0]), Ok(())),
        (
            |s| s.position = vec3([0, ONE - LIMIT, 0]),
            Err(OrthographicError::ViewOutsideTraceDomain),
        ),
        // Along the view axis the rectangle has no extent: z may reach 2^40 - 1.
        (|s| s.position = vec3([0, 0, LIMIT - 1]), Ok(())),
        // A frame turned an eighth turn about z: right and up are (1, -1, 0) and (1, 1, 0)
        // over sqrt(2), so the corner (-1, +1) reaches sqrt(2) units along y, and the two
        // diagonal corners do not bound the view.
        (
            |s| {
                s.up = vec3([ONE, ONE, 0]);
                s.position = vec3([0, LIMIT - 2 * ONE, 0]);
            },
            Ok(()),
        ),
        (
            |s| {
                s.up = vec3([ONE, ONE, 0]);
                s.position = vec3([0, LIMIT - ONE, 0]);
            },
            Err(OrthographicError::ViewOutsideTraceDomain),
        ),
        (|s| s.width = 0, Err(OrthographicError::EmptyImage)),
        (|s| s.height = 0, Err(OrthographicError::EmptyImage)),
        // A 32768 by 1 image of half height 1/256 is 256 units wide: inside the domain.
        (
            |s| {
                s.width = MAX_IMAGE_DIMENSION;
                s.half_height = fixed(ONE >> 8);
            },
            Ok(()),
        ),
        (|s| s.width = MAX_IMAGE_DIMENSION, Err(OrthographicError::ViewOutsideTraceDomain)),
        (|s| s.width = MAX_IMAGE_DIMENSION + 1, Err(OrthographicError::ImageTooLarge)),
        (|s| s.height = MAX_IMAGE_DIMENSION + 1, Err(OrthographicError::ImageTooLarge)),
        (|s| s.half_height = fixed(0), Err(OrthographicError::HalfHeightNotPositive)),
        (|s| s.half_height = fixed(-1), Err(OrthographicError::HalfHeightNotPositive)),
        (|s| s.half_height = fixed(1), Ok(())),
        // At the limit the extent is accepted and the corners at +-256 units are refused.
        (
            |s| s.half_height = fixed(ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW),
            Err(OrthographicError::ViewOutsideTraceDomain),
        ),
        (|s| s.half_height = fixed(ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW - 1), Ok(())),
        (
            |s| s.half_height = fixed(ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW + 1),
            Err(OrthographicError::HalfHeightTooLarge),
        ),
        (|s| s.look = vec3(ORIGIN), Err(OrthographicError::ZeroLook)),
        (|s| s.up = vec3(ORIGIN), Err(OrthographicError::UpParallelToLook)),
        (|s| s.up = vec3([0, 0, 5 * ONE]), Err(OrthographicError::UpParallelToLook)),
        // The pinhole camera's parallel threshold: 2^17 raw off is accepted, 2^15 refused.
        (|s| s.up = vec3([0, 1 << 17, -ONE]), Ok(())),
        (|s| s.up = vec3([0, 1 << 15, -ONE]), Err(OrthographicError::UpParallelToLook)),
    ];
    assert_eq!(ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW, LIMIT, "the limit is the trace domain's");
    for (index, (edit, expected)) in cases.into_iter().enumerate() {
        assert_eq!(with(edit), expected, "case {index}");
    }
    // Checked in order: position, image, half height, look, up hint, view rectangle. Each
    // refusal is fixed in turn; the position is first moved to where only its view is outside.
    let mut all_bad = base;
    all_bad.position = vec3([LIMIT, 0, 0]);
    all_bad.width = 0;
    all_bad.half_height = fixed(0);
    all_bad.look = vec3(ORIGIN);
    all_bad.up = vec3(ORIGIN);
    let order = [
        OrthographicError::PositionOutsideTraceDomain,
        OrthographicError::EmptyImage,
        OrthographicError::HalfHeightNotPositive,
        OrthographicError::ZeroLook,
        OrthographicError::UpParallelToLook,
        OrthographicError::ViewOutsideTraceDomain,
    ];
    for expected in order {
        assert_eq!(OrthographicCamera::new(&all_bad).map(|_| ()), Err(expected));
        match expected {
            OrthographicError::PositionOutsideTraceDomain => {
                all_bad.position = vec3([LIMIT - 1, 0, 0]);
            }
            OrthographicError::EmptyImage => all_bad.width = 1,
            OrthographicError::HalfHeightNotPositive => all_bad.half_height = fixed(ONE),
            OrthographicError::ZeroLook => all_bad.look = vec3(NEG_Z),
            OrthographicError::UpParallelToLook => all_bad.up = vec3(POS_Y),
            _ => all_bad.position = vec3(ORIGIN),
        }
    }
    assert!(OrthographicCamera::new(&all_bad).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_314_samples_outside_the_image_are_refused() {
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

fn corner_samples(width: u32, height: u32) -> [PixelSample; 5] {
    let (right, bottom) = (width - 1, height - 1);
    [
        sample(0, 0, (0, 0)),
        sample(right, 0, (LAST_SUB, 0)),
        sample(0, bottom, (0, LAST_SUB)),
        sample(right, bottom, (LAST_SUB, LAST_SUB)),
        PixelSample::centre(width / 2, height / 2),
    ]
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_315_corner_samples_of_extreme_cameras_have_rays() {
    let largest = MAX_IMAGE_DIMENSION;
    // (half height raw, width, height): the largest images, a 32768 by 1 image whose view is
    // 256 units wide, a 1 by 32768 image and the smallest extent, one raw unit.
    let lenses = [(ONE, largest, largest), (ONE >> 8, largest, 1), (ONE, 1, largest), (1, 1, 1)];
    for look in [NEG_Z, [ONE, ONE, ONE], [-ONE, 2 * ONE, -3 * ONE]] {
        for (half_height, width, height) in lenses {
            let camera = camera(&spec(look, POS_Y, half_height, (width, height)));
            for (index, sample) in corner_samples(width, height).into_iter().enumerate() {
                let label = format!("look {look:?}, lens {half_height}/{width}x{height}, {index}");
                let (origin, _) = ray(&camera, sample);
                check_origin(&camera, sample, origin);
                assert!(origin.iter().all(|c| c.abs() < LIMIT), "{label}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_316_pixel_origin_is_top_left_and_rows_grow_downward() {
    let camera = camera(&spec(NEG_Z, POS_Y, 2 * ONE, (8, 4)));
    let (first, direction) = ray(&camera, PixelSample::centre(0, 0));
    let (last, _) = ray(&camera, PixelSample::centre(7, 3));
    assert_eq!(direction, NEG_Z, "every ray looks along forward");
    assert!(first[0] < 0 && first[1] > 0, "pixel (0, 0) is top-left: {first:?}");
    assert!(last[0] > 0 && last[1] < 0, "the last pixel is bottom-right: {last:?}");
    // The view rectangle is 4 units high over 4 rows: one unit per pixel on both axes, and
    // 8 units wide, the half height times the aspect ratio on each side.
    assert_eq!(first, [-7 * HALF, 3 * HALF, 0]);
    let (next_column, _) = ray(&camera, PixelSample::centre(1, 0));
    let (next_row, _) = ray(&camera, PixelSample::centre(0, 1));
    assert_eq!(next_column, [first[0] + ONE, first[1], first[2]]);
    assert_eq!(next_row, [first[0], first[1] - ONE, first[2]]);
    let basis = [camera.forward(), camera.right(), camera.up()].map(|v| raw3(v.into_inner()));
    assert_eq!(basis, [NEG_Z, POS_X, POS_Y], "the pinhole camera's frame");
    assert_eq!((camera.width(), camera.height(), camera.half_height().raw()), (8, 4, 2 * ONE));
}
