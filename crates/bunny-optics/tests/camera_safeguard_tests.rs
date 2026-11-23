//! The two safeguards of camera construction, each with the evidence the chapter cites: the
//! second orthogonalization of the basis against its proven bound, at up hints just inside the
//! parallel threshold; the positive-forward corner check, unreachable for the pinhole camera
//! (its proven margin is checked at the extremes) and load-bearing for the thin-lens camera.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    CameraSpec, PinholeCamera, PixelSample, ThinLensCamera, ThinLensError, ThinLensSpec,
    MAX_IMAGE_DIMENSION, TAN_HALF_FOV_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

const SAFEGUARD_SEED: u64 = 0x0257_5AFE_6A4D_0001;
const ONE: i64 = ONE_RAW;
const LAST_SUB: i64 = ONE_RAW - 1;
const NEG_Z: [i64; 3] = [0, 0, -ONE];
const POS_Y: [i64; 3] = [0, ONE, 0];

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
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn wide3(v: FixedVec3) -> [i128; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()].map(i128::from)
}

fn dot(a: [i128; 3], b: [i128; 3]) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn spec(look: [i64; 3], up: [i64; 3], tan: i64, size: (u32, u32)) -> CameraSpec {
    CameraSpec {
        position: vec3([0, 0, 0]),
        look: vec3(look),
        up: vec3(up),
        tan_half_fov: fixed(tan),
        width: size.0,
        height: size.1,
    }
}

fn basis(camera: &PinholeCamera) -> [[i128; 3]; 3] {
    [camera.forward(), camera.right(), camera.up()].map(|v| wide3(v.into_inner()))
}

/// The proven bound of the chapter's Camera Frame: every pairwise dot product of the basis is
/// below `(3/2) sqrt(3) (1 + 2^-28) 2^32` raw-squared units, under `2.6 * 2^32`.
fn within_proven_bound(dot: i128) -> bool {
    5 * dot.abs() < 13 * (1_i128 << 32)
}

/// An up hint `2^-shift` of a unit off `look`: `look` plus a seeded perpendicular-ish offset
/// whose components are below `|look|_max >> shift`.
fn near_hint(seeds: &mut Seeds, look: [i64; 3], shift: u32) -> [i64; 3] {
    let largest = look.iter().map(|c| c.abs()).max().expect("three components");
    let offset = seeds.triple(largest >> shift);
    [0, 1, 2].map(|i| look[i] + offset[i])
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_321_the_basis_is_orthogonal_within_its_proven_bound() {
    let seed = SAFEGUARD_SEED ^ 0x141;
    println!("OPTICS-TP-321 seed {seed:#018x}");
    let mut seeds = Seeds(seed);
    let (mut cameras, mut worst) = (0, 0_i128);
    // Hints 2^-12 to 2^-15 of a unit off the look direction: the side vector `forward x
    // unit(up)` is then near the parallel threshold of 2^16 raw, where `first_right` is
    // orthogonal to `forward` only to about 2^-16, and only the second round of cross products
    // brings the basis within the bound. Also the fixed hints of OPTICS-TP-005.
    let mut cases: Vec<([i64; 3], [i64; 3])> = vec![
        (NEG_Z, [0, 1 << 17, -ONE]),
        (NEG_Z, [1 << 17, 0, -ONE]),
        (NEG_Z, [1 << 16, 1 << 16, -ONE]),
    ];
    for index in 0..2_000 {
        let look = seeds.triple(1 << 36);
        let shift = 12 + index % 4;
        cases.push((look, near_hint(&mut seeds, look, shift)));
    }
    for (index, (look, up)) in cases.into_iter().enumerate() {
        let Ok(camera) = PinholeCamera::new(&spec(look, up, ONE, (2, 2))) else {
            continue;
        };
        cameras += 1;
        let [f, r, u] = basis(&camera);
        for (name, value) in [("f.r", dot(f, r)), ("f.u", dot(f, u)), ("r.u", dot(r, u))] {
            assert!(within_proven_bound(value), "case {index}: {name} = {value}");
            worst = worst.max(value.abs());
        }
    }
    println!("OPTICS-TP-321: {cameras} cameras, largest |dot| {worst} raw-squared units");
    assert!(cameras >= 1_500, "only {cameras} of 2 003 cameras constructed");
}

fn corner_samples(width: u32, height: u32) -> [PixelSample; 4] {
    let (right, bottom) = (width - 1, height - 1);
    let last = fixed(LAST_SUB);
    [
        PixelSample::new(0, 0, fixed(0), fixed(0)),
        PixelSample::new(right, 0, last, fixed(0)),
        PixelSample::new(0, bottom, fixed(0), last),
        PixelSample::new(right, bottom, last, last),
    ]
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_322_pinhole_corner_directions_keep_the_proven_forward_margin() {
    let (widest, largest) = (TAN_HALF_FOV_LIMIT_RAW, MAX_IMAGE_DIMENSION);
    let lenses = [
        (widest, largest, largest),
        (widest, largest, 1),
        (widest, 1, largest),
        (1, largest, largest),
        (ONE, 1, 1),
    ];
    let looks = [NEG_Z, [ONE, ONE, ONE], [-ONE, 2 * ONE, -3 * ONE]];
    // A hint just inside the parallel threshold, where the basis is least orthogonal.
    let hints = [POS_Y, [0, 1 << 17, -ONE]];
    let mut least = i128::MAX;
    for (look, up) in looks.into_iter().flat_map(|look| hints.map(|up| (look, up))) {
        for (tan, width, height) in lenses {
            let camera = PinholeCamera::new(&spec(look, up, tan, (width, height)))
                .expect("every extreme camera is accepted");
            let k = camera.scale_shift();
            assert!(k <= 24, "the chapter's bound on the scale shift: {k}");
            let forward = wide3(camera.forward().into_inner());
            let l1: i128 = forward.iter().map(|c| c.abs()).sum();
            for sample in corner_samples(width, height) {
                let ray = camera.ray(sample).expect("in-range sample");
                let d_dot_f = dot(wide3(ray.direction()), forward);
                // Proven: d . F >= 2^(64 - k) (1 - 2^-7) - 2^33, and so above 2^39 for k <= 24.
                assert!(128 * (d_dot_f + (1 << 33)) >= 127 * (1_i128 << (64 - k)), "{look:?}");
                assert!(d_dot_f > 1 << 39);
                least = least.min(d_dot_f / (2 * l1));
            }
        }
    }
    println!("OPTICS-TP-322: smallest d . F over 2 |F|_1 at the corners: {least}");
    // The check `d . F > 2 |F|_1` passes with at least a factor of 32 to spare.
    assert!(least >= 32);
}

fn lens(aperture: i64) -> ThinLensSpec {
    ThinLensSpec {
        camera: spec(NEG_Z, POS_Y, ONE, (2, 2)),
        aperture_radius: fixed(aperture),
        focus_distance: fixed(1),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_323_the_thin_lens_forward_check_decides_extreme_apertures() {
    // A level 2x2 camera with tangent 1 and focus distance 1 raw. Its corner directions at
    // shift k are (+-1, +-1, -1) 2^(32 - k) raw, so d . F = 2^(64 - k); the component margin
    // 2a 2^32 / 2^k first fits at k = 29 for a = 2^28 - 1 and at k = 30 for a = 2^28.
    let accepted = ThinLensCamera::new(&lens((1 << 28) - 1)).expect("fits at shift 29");
    assert_eq!(accepted.scale_shift(), 29);
    // At k = 30 the forward check needs 2^34 > 2 |F|_1 + s_30 = 2^33 + 2^33 + 2^32: refused,
    // by the forward check, although the components fit.
    assert_eq!(ThinLensCamera::new(&lens(1 << 28)), Err(ThinLensError::Unrepresentable));
    // Every ray of the accepted camera exists and faces forward.
    let forward = wide3(accepted.forward().into_inner());
    for sample in corner_samples(2, 2) {
        for lens_sample in [[0, 0], [LAST_SUB, 0], [LAST_SUB, ONE / 4], [LAST_SUB, ONE / 2]] {
            let ray = accepted.ray(sample, lens_sample.map(fixed)).expect("a ray");
            assert!(dot(wide3(ray.direction()), forward) > 0, "{sample:?}, {lens_sample:?}");
        }
    }
}
