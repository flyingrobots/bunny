//! Oracles for the thin-lens camera `bunny-thin-lens/1`: bit-for-bit agreement with the pinhole
//! camera at aperture 0, an exact big-integer focus-plane oracle, the disc mapping's range,
//! radius and angle against exact integer bounds, stratification of sampler blocks on the
//! disc, totality on extreme lenses, and every construction refusal.

// Only the signed sums of the shared oracle module are used here.
#[allow(dead_code)]
mod exact;

use std::cmp::Ordering;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    lens_disc_point, CameraError, CameraSpec, PinholeCamera, PixelSample, PixelSampler,
    ThinLensCamera, ThinLensError, ThinLensSpec, THIN_LENS_ID,
};
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};
use exact::Signed;
use wasm_bindgen_test::wasm_bindgen_test;

const THIN_LENS_SEED: u64 = 0xB002_0364_7E45_0001;
const ONE: i64 = ONE_RAW;
const LAST: i64 = ONE_RAW - 1;
/// `2^32 (2^32 - 16)`: the raw-squared radius the disc mapping reaches as `u` tends to 1.
const DISC_RADIUS_SQUARED: i128 = (1 << 32) * ((1 << 32) - 16);
/// Allowed raw-squared error of `x^2 + y^2` against `u (2^32 - 16)`; see OPTICS-REQ-072.
const RADIUS_SQUARED_SLACK: i128 = 1 << 35;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low).expect("ordered range") + 1;
        low + i64::try_from(self.next() % span).expect("in range")
    }

    fn triple(&mut self, bound: i64) -> [i64; 3] {
        [(); 3].map(|()| self.range(-bound, bound))
    }

    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.next() % u64::from(bound)).expect("below a u32")
    }

    fn unit_pair(&mut self) -> [FixedQ32_32; 2] {
        [fixed(self.range(0, LAST)), fixed(self.range(0, LAST))]
    }
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> [i128; 3] {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

fn seeded_camera(seeds: &mut Seeds, position_bound: i64) -> CameraSpec {
    let (width, height) = (seeds.range(1, 512), seeds.range(1, 512));
    CameraSpec {
        position: vec3(seeds.triple(position_bound)),
        look: vec3(seeds.triple(1 << 36)),
        up: vec3(seeds.triple(1 << 36)),
        tan_half_fov: fixed(seeds.range(1 << 20, 4 * ONE)),
        width: u32::try_from(width).expect("positive"),
        height: u32::try_from(height).expect("positive"),
    }
}

fn seeded_sample(seeds: &mut Seeds, spec: &CameraSpec) -> PixelSample {
    let (x, y) = (seeds.below(spec.width), seeds.below(spec.height));
    PixelSample::new(x, y, fixed(seeds.range(0, LAST)), fixed(seeds.range(0, LAST)))
}

fn lens(camera: CameraSpec, aperture: i64, focus: i64) -> ThinLensSpec {
    ThinLensSpec { camera, aperture_radius: fixed(aperture), focus_distance: fixed(focus) }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_130_aperture_zero_gives_the_pinhole_rays_bit_for_bit() {
    let mut seeds = Seeds(THIN_LENS_SEED + 130);
    println!("seed {THIN_LENS_SEED:#x} + 130");
    let mut rays = 0;
    for case in 0..1_000 {
        let spec = seeded_camera(&mut seeds, TRACE_COORD_LIMIT_RAW - 1);
        let Ok(pinhole) = PinholeCamera::new(&spec) else { continue };
        let thin = ThinLensCamera::new(&lens(spec, 0, seeds.range(1, 1 << 40)))
            .expect("every pinhole spec with aperture 0 is accepted");
        assert_eq!(thin.scale_shift(), pinhole.scale_shift(), "case {case}");
        for _ in 0..8 {
            let sample = seeded_sample(&mut seeds, &spec);
            let expected = pinhole.ray(sample);
            assert!(expected.is_some(), "case {case}: pinhole refused {sample:?}");
            assert_eq!(thin.ray(sample, seeds.unit_pair()), expected, "case {case} {sample:?}");
            rays += 1;
        }
    }
    println!("{rays} rays compared");
    assert!(rays >= 7_900, "only {rays} rays compared");
}

/// The test plan's focus-plane oracle: `2 e` is within `f 2^k [-(3D + 2 slack), D + 2 slack]`.
// The short names are those of the oracle formula in the test plan.
#[allow(clippy::many_single_char_names)]
fn check_focus_point(
    camera: &ThinLensCamera,
    spec: &ThinLensSpec,
    sample: PixelSample,
    ray: &TraceRay,
) -> Result<(), String> {
    let (w, h) = (i128::from(spec.camera.width), i128::from(spec.camera.height));
    let k = camera.scale_shift();
    let one = i128::from(ONE);
    let a = (2 * i128::from(sample.x) - w) * one + 2 * i128::from(sample.sub_x.raw());
    let b = (h - 2 * i128::from(sample.y)) * one - 2 * i128::from(sample.sub_y.raw());
    let t = i128::from(spec.camera.tan_half_fov.raw());
    let [f, r, u] = [camera.forward(), camera.right(), camera.up()].map(|v| raw3(v.into_inner()));
    let tau = i128::from(spec.focus_distance.raw()) << k;
    let d_scale = h << (64 + k);
    let slack = ((w + h) * h) << (31 + k);
    let (origin, direction) = (raw3(ray.origin()), raw3(ray.direction()));
    let position = raw3(spec.camera.position);
    for axis in 0..3 {
        let exact = f[axis] * h * (1 << 64) + a * t * r[axis] + b * t * u[axis];
        let along = direction[axis] * d_scale - exact;
        let offset = Signed::product(origin[axis] - position[axis], d_scale, 32);
        let error = offset.add(&Signed::product(tau, along, 0));
        let twice = error.add(&error);
        let low = Signed::product(tau, 3 * d_scale + 2 * slack, 0).negated();
        let high = Signed::product(tau, d_scale + 2 * slack, 0);
        let inside = twice.add(&low.negated()).sign() != Ordering::Less
            && high.add(&twice.negated()).sign() != Ordering::Less;
        if !inside {
            return Err(format!("axis {axis}: origin {origin:?}, direction {direction:?}"));
        }
    }
    Ok(())
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_131_every_ray_passes_its_focus_plane_point() {
    let mut seeds = Seeds(THIN_LENS_SEED + 131);
    println!("seed {THIN_LENS_SEED:#x} + 131");
    let mut rays = 0;
    for case in 0..1_000 {
        let camera = seeded_camera(&mut seeds, 1 << 38);
        let spec = lens(camera, seeds.range(1, 4 * ONE), seeds.range(ONE >> 8, 64 * ONE));
        let Ok(thin) = ThinLensCamera::new(&spec) else { continue };
        for _ in 0..8 {
            let sample = seeded_sample(&mut seeds, &spec.camera);
            let ray = thin.ray(sample, seeds.unit_pair()).expect("an accepted sample has a ray");
            check_focus_point(&thin, &spec, sample, &ray)
                .unwrap_or_else(|e| panic!("case {case} {sample:?}: {e}"));
            rays += 1;
        }
    }
    println!("{rays} rays checked");
    assert!(rays >= 7_900, "only {rays} rays checked");
}

fn disc_raw(point: [FixedQ32_32; 2]) -> [i128; 2] {
    lens_disc_point(point).expect("inside [0, 1)^2").map(|c| i128::from(c.raw()))
}

/// The octant `0..8` of a nonzero point, counter-clockwise from the first axis; a point on a
/// boundary ray belongs to the octant that starts there.
fn octant([x, y]: [i128; 2]) -> u32 {
    let quadrant = match (x.signum(), y.signum()) {
        (1, 0 | 1) => 0,
        (0 | -1, 1) => 1,
        (-1, 0 | -1) => 2,
        _ => 3,
    };
    // Rotate the point into the first quadrant, then split it at the diagonal.
    let (p, q) = match quadrant {
        0 => (x, y),
        1 => (y, -x),
        2 => (-x, -y),
        _ => (-y, x),
    };
    quadrant * 2 + u32::from(q >= p)
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_132_disc_points_lie_inside_the_unit_disc_at_their_radius_and_angle() {
    let mut seeds = Seeds(THIN_LENS_SEED + 132);
    println!("seed {THIN_LENS_SEED:#x} + 132");
    let edges = [0, 1, ONE / 8, ONE / 4, ONE / 2, 3 * ONE / 4, LAST - 1, LAST];
    let mut points: Vec<[i64; 2]> =
        edges.iter().flat_map(|&u| edges.iter().map(move |&v| [u, v])).collect();
    points.extend((0..20_000).map(|_| [seeds.range(0, LAST), seeds.range(0, LAST)]));
    for [u, v] in points {
        let [x, y] = disc_raw([fixed(u), fixed(v)]);
        let squared = x * x + y * y;
        assert!(squared < 1 << 64, "u {u}, v {v}: ({x}, {y}) is outside the unit disc");
        let ideal = i128::from(u) * ((1 << 32) - 16);
        assert!((squared - ideal).abs() <= RADIUS_SQUARED_SLACK, "u {u}, v {v}: radius");
        let angle_octant = u32::try_from(v >> 29).expect("below 8");
        // Within 2^-24 turn of an octant boundary the rounding may cross it.
        let into_octant = v & ((1 << 29) - 1);
        let near_boundary = !((1 << 8)..=(1 << 29) - (1 << 8)).contains(&into_octant);
        if squared > 0 && !near_boundary {
            assert_eq!(octant([x, y]), angle_octant, "u {u}, v {v}: ({x}, {y})");
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_133_sampler_blocks_stratify_the_disc_into_rings_and_octants() {
    let mut seeds = Seeds(THIN_LENS_SEED + 133);
    println!("seed {THIN_LENS_SEED:#x} + 133; lens pair 2");
    for case in 0..200 {
        let sampler = PixelSampler::new(seeds.next(), seeds.below(4096), seeds.below(4096));
        let block = u32::try_from(seeds.range(0, 1 << 20)).expect("small") * 16;
        let mut rings = [0_u32; 16];
        let mut octants = [0_u32; 8];
        for m in block..block + 16 {
            let point = disc_raw(sampler.point(m, 2));
            let squared = point[0] * point[0] + point[1] * point[1];
            let ring = usize::try_from(squared * 16 / DISC_RADIUS_SQUARED).expect("ring");
            rings[ring.min(15)] += 1;
            if m < block + 8 {
                octants[octant(point) as usize] += 1;
            }
        }
        assert_eq!(rings, [1; 16], "case {case}: equal-area rings of block {block}");
        assert_eq!(octants, [1; 8], "case {case}: octants of block {block}");
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_134_extreme_lenses_give_a_forward_ray_for_every_sample() {
    let mut seeds = Seeds(THIN_LENS_SEED + 134);
    println!("seed {THIN_LENS_SEED:#x} + 134");
    let corners = [0, LAST];
    let mut rays = 0;
    for case in 0..300 {
        let camera = seeded_camera(&mut seeds, 1 << 30);
        // Aperture up to 64 times the focus distance: far past any real lens.
        let focus = seeds.range(1, 4 * ONE);
        let spec = lens(camera, seeds.range(1, focus.saturating_mul(64).min(1 << 38)), focus);
        let Ok(thin) = ThinLensCamera::new(&spec) else { continue };
        let forward = raw3(thin.forward().into_inner());
        for (x, y) in [(0, 0), (camera.width - 1, camera.height - 1)] {
            for sub in corners {
                for lens_u in [LAST, LAST - 1] {
                    for lens_v in [0, ONE / 8, 3 * ONE / 8, 5 * ONE / 8, LAST] {
                        let sample = PixelSample::new(x, y, fixed(sub), fixed(sub));
                        let ray = thin.ray(sample, [fixed(lens_u), fixed(lens_v)]);
                        let ray = ray.unwrap_or_else(|| panic!("case {case}: no ray"));
                        let d = raw3(ray.direction());
                        let ahead: i128 = (0..3).map(|i| d[i] * forward[i]).sum();
                        assert!(ahead > 0, "case {case}: direction {d:?} faces backward");
                        rays += 1;
                    }
                }
            }
        }
    }
    println!("{rays} extreme rays");
    assert!(rays >= 5_000, "only {rays} extreme rays");
}

fn level_camera() -> CameraSpec {
    CameraSpec {
        position: vec3([0, 0, 0]),
        look: vec3([0, 0, -ONE]),
        up: vec3([0, ONE, 0]),
        tan_half_fov: fixed(ONE),
        width: 4,
        height: 4,
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_135_invalid_lenses_and_lens_samples_are_refused() {
    let base = level_camera();
    let refuse = |spec: ThinLensSpec| ThinLensCamera::new(&spec).err();
    assert_eq!(refuse(lens(base, ONE, 0)), Some(ThinLensError::FocusDistanceNotPositive));
    assert_eq!(refuse(lens(base, ONE, -1)), Some(ThinLensError::FocusDistanceNotPositive));
    assert_eq!(refuse(lens(base, -1, ONE)), Some(ThinLensError::ApertureNegative));
    let empty = CameraSpec { width: 0, ..base };
    let camera_error = Some(ThinLensError::Camera(CameraError::EmptyImage));
    assert_eq!(refuse(lens(empty, -1, 0)), camera_error, "the pinhole spec is checked first");
    assert_eq!(refuse(lens(base, 0, -1)), Some(ThinLensError::FocusDistanceNotPositive));
    let near = CameraSpec { position: vec3([TRACE_COORD_LIMIT_RAW - 2 * ONE, 0, 0]), ..base };
    assert!(ThinLensCamera::new(&lens(near, ONE - 1, ONE)).is_ok(), "|p| + 2a = 2^40 - 2");
    assert_eq!(refuse(lens(near, ONE, ONE)), Some(ThinLensError::LensOutsideTraceDomain));
    assert_eq!(refuse(lens(base, 1 << 38, 1)), Some(ThinLensError::Unrepresentable));
    let camera = ThinLensCamera::new(&lens(base, ONE, ONE)).expect("a valid lens");
    let centre = PixelSample::centre(1, 1);
    for bad in [[-1, 0], [0, -1], [ONE, 0], [0, ONE]] {
        assert_eq!(camera.ray(centre, bad.map(fixed)), None, "lens sample {bad:?}");
        assert_eq!(lens_disc_point(bad.map(fixed)), None, "disc point {bad:?}");
    }
    assert_eq!(camera.ray(PixelSample::centre(4, 0), [fixed(0); 2]), None);
    assert_eq!(THIN_LENS_ID, "bunny-thin-lens/1");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_136_golden_lens_rays_of_a_level_camera() {
    // Aperture 1, focus 1, 4 by 4 image, tangent 1, looking down -z. The pinhole corners
    // (+-1, +-1, -1) fit at k = 0; the lens margin ceil(2 * 2^32 * 2^32 / (2^32 * 2^k)) =
    // 2^33 / 2^k raw leaves room for the corner 2^32 / 2^k only from k = 2.
    let camera = ThinLensCamera::new(&lens(level_camera(), ONE, ONE)).expect("a valid lens");
    assert_eq!(camera.scale_shift(), 2);
    let pinhole = PinholeCamera::new(&level_camera()).expect("valid");
    assert_eq!(pinhole.scale_shift(), 0);
    // u = 0 is the lens centre: the pinhole ray at a quarter of its length.
    let corner = PixelSample::centre(0, 0);
    let pin = raw3(pinhole.ray(corner).expect("a ray").direction());
    let ray = camera.ray(corner, [fixed(0), fixed(0)]).expect("a ray");
    assert_eq!((raw3(ray.origin()), raw3(ray.direction())), ([0; 3], pin.map(|c| c / 4)));
    // u = 1/4: r = isqrt(2^62 - 2^34) = 2^31 - 5 raw, since (2^31 - 4)^2 = 2^62 - 2^34 + 16.
    // The image centre's direction at k = 2 is (0, 0, -1/4); the lens correction along the
    // lens axis is round(r / (1 * 2^2)) = round(2^29 - 1.25) = 2^29 - 1 raw.
    let r = (1_i128 << 31) - 5;
    let correction = (1_i128 << 29) - 1;
    let centre = PixelSample::new(2, 2, fixed(0), fixed(0));
    let quarter = -i128::from(ONE / 4);
    // v = 0 is along `right` (+x).
    let ray = camera.ray(centre, [fixed(ONE / 4), fixed(0)]).expect("a ray");
    assert_eq!(raw3(ray.origin()), [r, 0, 0]);
    assert_eq!(raw3(ray.direction()), [-correction, 0, quarter]);
    // v = 1/4 is along `up` (+y).
    let ray = camera.ray(centre, [fixed(ONE / 4), fixed(ONE / 4)]).expect("a ray");
    assert_eq!(raw3(ray.origin()), [0, r, 0]);
    assert_eq!(raw3(ray.direction()), [0, -correction, quarter]);
}
