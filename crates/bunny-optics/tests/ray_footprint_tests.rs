//! Oracles for the ray footprint `bunny-ray-footprint/1`: closed forms on a plane facing the
//! camera and on tilted planes toward grazing, an exact rational oracle on seeded pinhole and
//! thin-lens cameras, pixels and triangles, the neighbours past the image edge, every refusal
//! and unbounded result one raw unit either side of its threshold, and the extremes.
//!
//! The rational oracle is written here from the chapter's definition with a different
//! construction from the kernel's: the point `P = O + t D` with `t = ((v0 - O) . N) / (D . N)`,
//! its barycentric weights `((P - v0) x e2) . N / (N . N)` and `(e1 x (P - v0)) . N / (N . N)`,
//! the texture coordinates they give, their difference, and a rounding found by bisection, all
//! in the shared exact big integers. Its rays are the cameras' public `ray` outputs.

// Only the signed big integers of the shared oracle module are used here.
#[allow(dead_code)]
mod exact;

use std::cmp::Ordering;

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    CameraSpec, FootprintAxis, PinholeCamera, PixelSample, RayFootprint, RayFootprintError,
    TextureDerivatives, TexturedTriangle, ThinLensCamera, ThinLensSpec, UnboundedReason,
    RAY_FOOTPRINT_ID,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use exact::Signed;
use wasm_bindgen_test::wasm_bindgen_test;

const FOOTPRINT_SEED: u64 = 0xB002_0383_F007_0001;
const ONE: i64 = ONE_RAW;
const LAST: i64 = ONE_RAW - 1;
const LIMIT: i64 = TRACE_COORD_LIMIT_RAW - 1;

type Raw3 = [i64; 3];
type Uvs = [[i64; 2]; 3];

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

    fn triple(&mut self, bound: i64) -> Raw3 {
        [(); 3].map(|()| self.range(-bound, bound))
    }

    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.next() % u64::from(bound)).expect("below a u32")
    }

    /// Any raw `i64`, from two draws.
    fn any(&mut self) -> i64 {
        let bits = (self.next() << 22) ^ self.next();
        i64::from_ne_bytes(bits.to_ne_bytes())
    }

    fn unit_pair(&mut self) -> [FixedQ32_32; 2] {
        [fixed(self.range(0, LAST)), fixed(self.range(0, LAST))]
    }
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn wide(v: Raw3) -> [i128; 3] {
    v.map(i128::from)
}

fn surface(vertices: [Raw3; 3], uvs: Uvs) -> TexturedTriangle {
    let [a, b, c] = vertices.map(vec3);
    let triangle = TraceTriangle::new(a, b, c).expect("vertices in the trace domain");
    let uvs = uvs.map(|[u, v]| FixedVec2::new(fixed(u), fixed(v)));
    TexturedTriangle::new(triangle, uvs).expect("a triangle with a plane")
}

fn spec(position: Raw3, look: Raw3, up: Raw3, tan: i64, size: [u32; 2]) -> CameraSpec {
    CameraSpec {
        position: vec3(position),
        look: vec3(look),
        up: vec3(up),
        tan_half_fov: fixed(tan),
        width: size[0],
        height: size[1],
    }
}

fn derivatives(footprint: RayFootprint) -> [[i64; 2]; 2] {
    let RayFootprint::Bounded(d) = footprint else { panic!("unbounded: {footprint:?}") };
    let pair = |v: FixedVec2| [v.x.raw(), v.y.raw()];
    [pair(d.dx()), pair(d.dy())]
}

// --- The exact rational oracle -------------------------------------------------------------

fn big(value: i128) -> Signed {
    Signed::new(value)
}

fn cross(a: [i128; 3], b: [i128; 3]) -> [i128; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [i128; 3], b: [i128; 3]) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [i128; 3], b: [i128; 3]) -> [i128; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn big_cross(a: &[Signed; 3], b: &[Signed; 3]) -> [Signed; 3] {
    let term = |i: usize, j: usize| a[i].mul(&b[j]).add(&a[j].mul(&b[i]).negated());
    [term(1, 2), term(2, 0), term(0, 1)]
}

fn big_dot(a: &[Signed; 3], b: [i128; 3]) -> Signed {
    a.iter().zip(b).fold(big(0), |sum, (x, y)| sum.add(&x.mul(&big(y))))
}

/// A ray and a textured triangle, as exact integers.
struct Scene {
    vertices: [[i128; 3]; 3],
    uvs: [[i128; 2]; 3],
}

impl Scene {
    fn new(vertices: [Raw3; 3], uvs: Uvs) -> Self {
        Self { vertices: vertices.map(wide), uvs: uvs.map(|t| t.map(i128::from)) }
    }

    fn normal(&self) -> [i128; 3] {
        let [v0, v1, v2] = self.vertices;
        cross(sub(v1, v0), sub(v2, v0))
    }

    /// `(v0 - O) . N`: `t (D . N)` for every ray from `O`.
    fn height(&self, origin: [i128; 3]) -> i128 {
        dot(sub(self.vertices[0], origin), self.normal())
    }

    /// The texture coordinates where the line of `(origin, direction)` meets the plane, as two
    /// numerators over one nonzero denominator. Requires `D . N != 0`.
    fn uv(&self, origin: [i128; 3], direction: [i128; 3]) -> ([Signed; 2], Signed) {
        let [v0, v1, v2] = self.vertices;
        let (e1, e2, n) = (sub(v1, v0), sub(v2, v0), self.normal());
        let dn = dot(direction, n);
        let h = self.height(origin);
        // (P - v0) dn = (O - v0) dn + h D.
        let q: [Signed; 3] = std::array::from_fn(|i| {
            big(origin[i] - v0[i]).mul(&big(dn)).add(&big(h).mul(&big(direction[i])))
        });
        let nn = big_dot(&n.map(big), n);
        let bu = big_dot(&big_cross(&q, &e2.map(big)), n);
        let bv = big_dot(&big_cross(&e1.map(big), &q), n);
        let den = big(dn).mul(&nn);
        let channel = |s: usize| {
            let [t0, t1, t2] = self.uvs.map(|t| t[s]);
            big(t0).mul(&den).add(&big(t1 - t0).mul(&bu)).add(&big(t2 - t0).mul(&bv))
        };
        ([channel(0), channel(1)], den)
    }
}

/// `round(x / y)` to nearest, ties to even, when it is an `i64`.
fn round_ratio(x: &Signed, y: &Signed) -> Option<i64> {
    let (x, y) = if y.sign() == Ordering::Less {
        (x.negated(), y.negated())
    } else {
        (x.clone(), y.clone())
    };
    let (top, two_y) = (x.add(&x).add(&y), y.add(&y));
    let r = floor_ratio(&top, &two_y)?;
    let tie = top.add(&big(r).mul(&two_y).negated()).sign() == Ordering::Equal;
    i64::try_from(if tie && r & 1 != 0 { r - 1 } else { r }).ok()
}

/// `floor(x / y)` for `y > 0` by bisection, when it is within `2^65` of zero.
fn floor_ratio(x: &Signed, y: &Signed) -> Option<i128> {
    let at_most = |r: i128| x.add(&big(r).mul(y).negated()).sign() != Ordering::Less;
    let (mut low, mut high) = (-(1_i128 << 65), 1_i128 << 65);
    if !at_most(low) || at_most(high) {
        return None;
    }
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if at_most(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(low)
}

/// The footprint the chapter defines for three rays (the hit ray, then right, then down).
fn oracle(rays: [&TraceRay; 3], scene: &Scene) -> Result<RayFootprint, RayFootprintError> {
    let origin = wide(raw3(rays[0].origin()));
    let [hit, right, down] = rays.map(|r| wide(raw3(r.direction())));
    let (n, h) = (scene.normal(), scene.height(origin));
    let dn = dot(hit, n);
    if dn == 0 {
        return Err(RayFootprintError::RayParallelToPlane);
    }
    if h == 0 || h.signum() != dn.signum() {
        return Err(RayFootprintError::PlaneNotAhead);
    }
    let (base, base_den) = scene.uv(origin, hit);
    let mut axes = Vec::new();
    for (axis, direction) in [(FootprintAxis::Right, right), (FootprintAxis::Down, down)] {
        let dn = dot(direction, n);
        let reason = if dn == 0 {
            Some(UnboundedReason::Parallel)
        } else if dn.signum() != h.signum() {
            Some(UnboundedReason::Behind)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Ok(RayFootprint::Unbounded { axis, reason });
        }
        let (other, other_den) = scene.uv(origin, direction);
        let y = other_den.mul(&base_den);
        let channel = |s: usize| {
            let x = other[s].mul(&base_den).add(&base[s].mul(&other_den).negated());
            round_ratio(&x, &y)
        };
        let (Some(du), Some(dv)) = (channel(0), channel(1)) else {
            return Ok(RayFootprint::Unbounded { axis, reason: UnboundedReason::TooLarge });
        };
        axes.push(FixedVec2::new(fixed(du), fixed(dv)));
    }
    Ok(RayFootprint::Bounded(TextureDerivatives::new(axes[0], axes[1])))
}

// --- Closed forms --------------------------------------------------------------------------

/// A camera at the origin looking down `-z` with up `+y` (so right is `+x`), tangent `tan`
/// raw, a `side` by `side` image.
fn level_camera(tan: i64, side: u32) -> PinholeCamera {
    let camera = spec([0; 3], [0, 0, -ONE], [0, ONE, 0], tan, [side, side]);
    PinholeCamera::new(&camera).expect("a valid camera")
}

/// The plane `z = -depth` with `u` and `v` the world `x` and `y` times `scale / 2^32`, through a
/// triangle of side 16 units around the view axis.
fn facing_plane(depth: i64, scale: i64) -> TexturedTriangle {
    let side = 8 * ONE;
    let corner = |x: i64, y: i64| [x, y, -depth];
    let uv = |x: i64| i64::try_from(i128::from(x) * i128::from(scale) >> 32).expect("small");
    let vertices = [corner(-side, -side), corner(side, -side), corner(-side, side)];
    surface(vertices, vertices.map(|v| [uv(v[0]), uv(v[1])]))
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_210_a_plane_facing_the_camera_gives_the_closed_form_footprint() {
    assert_eq!(RAY_FOOTPRINT_ID, "bunny-ray-footprint/1");
    // Tangent 1 over 4 pixels and 1/2 over 8: one pixel is 2 tan / height = 1/2 and 1/8 of the
    // depth. The rays are exact: ((2x + 1 - n) tan / (n / 2), -(2y + 1 - n) tan / (n / 2), -1).
    for (tan, side, per_pixel_shift) in [(ONE, 4_u32, 1_u32), (ONE / 2, 8, 3)] {
        let camera = level_camera(tan, side);
        assert_eq!(camera.scale_shift(), 0);
        for (depth, scale) in [(ONE, ONE), (3 * ONE, ONE), (ONE / 8, 3 * ONE), (100 * ONE, ONE / 4)]
        {
            let plane = facing_plane(depth, scale);
            // Hand: d (u) / dx = scale depth 2 tan / height, exactly, at every pixel.
            let step = i64::try_from(i128::from(depth) * i128::from(scale) >> 32).expect("fits")
                >> per_pixel_shift;
            for (x, y) in (0..side).flat_map(|x| (0..side).map(move |y| (x, y))) {
                let got = camera.footprint(PixelSample::centre(x, y), &plane).expect("a hit");
                assert_eq!(derivatives(got), [[step, 0], [0, -step]], "{tan} {depth} ({x}, {y})");
            }
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_211_a_half_raw_unit_footprint_rounds_to_even() {
    // Depth `d` raw and scale 1.0 on the 4 by 4 camera: the exact footprint is d / 2 raw.
    let camera = level_camera(ONE, 4);
    let expected = [(1, 0), (2, 1), (3, 2), (5, 2), (7, 4), (9, 4)];
    for (depth, step) in expected {
        let got = camera.footprint(PixelSample::centre(1, 2), &facing_plane(depth, ONE));
        assert_eq!(derivatives(got.expect("a hit")), [[step, 0], [0, -step]], "depth {depth}");
    }
}

/// `numerator / denominator` rounded to nearest, ties to even, for `denominator > 0`, written
/// for the hand closed forms (each fits `i128`).
fn round_closed(numerator: i128, denominator: i128) -> i128 {
    let (floor, rest) = (numerator.div_euclid(denominator), numerator.rem_euclid(denominator));
    match (2 * rest).cmp(&denominator) {
        Ordering::Greater => floor + 1,
        Ordering::Equal => floor + (floor & 1),
        Ordering::Less => floor,
    }
}

/// The plane `z = -depth + slope y` with `(u, v)` the world `(x, y)`.
fn tilted_plane(depth: i64, slope: i64) -> ([Raw3; 3], Uvs) {
    let side = 8 * ONE;
    let point = |x: i64, y: i64| [x, y, -depth + slope * (y / ONE)];
    let vertices = [point(-side, -side), point(side, -side), point(-side, side)];
    (vertices, vertices.map(|v| [v[0], v[1]]))
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_212_a_grazing_plane_stretches_the_footprint_as_the_closed_form_says() {
    // Pixel (1, 1) of the 4 by 4 level camera: rays (dx, dy, -1) with dx = -1/4, dy = 1/4; the
    // right neighbour has dx + 1/2 and the down neighbour dy - 1/2 = -1/4. On the plane
    // z = -d + g y the ray (dx, dy, -1) meets it at t = d / (1 + g dy), so with
    // m = 4 - g and p = 4 + g (raw, times 2^32):
    //   du/dx = d 2^33 / p,  dv/dx = 0,  du/dy = -d 2^33 g / (m p),  dv/dy = -d 2^67 / (m p).
    // The down neighbour is parallel to the plane at g = 4 and meets it behind past that.
    let camera = level_camera(ONE, 4);
    let sample = PixelSample::centre(1, 1);
    let depth = 2 * ONE;
    let (d, four) = (i128::from(depth), 4 * i128::from(ONE));
    // 0, 1, 2, then 4 - 2^-j for j = 0..=24 (from 3) and one raw unit short of 4.
    let mut slopes: Vec<i64> = vec![0, ONE, 2 * ONE];
    slopes.extend((0..=24).map(|j| 4 * ONE - (ONE >> j)));
    slopes.push(4 * ONE - 1);
    let mut longest = 0_u128;
    for slope in slopes {
        let (vertices, uvs) = tilted_plane(depth, slope);
        let g = i128::from(slope);
        let (m, p) = (four - g, four + g);
        let expected = [
            [round_closed(d << 33, p), 0],
            [round_closed(-(d << 33) * g, m * p), round_closed(-(d << 67), m * p)],
        ];
        let got = camera.footprint(sample, &surface(vertices, uvs)).expect("a hit");
        let rays = [(1, 1), (2, 1), (1, 2)]
            .map(|(x, y)| camera.ray(PixelSample::centre(x, y)).expect("in the image"));
        let expected_by_oracle = oracle([&rays[0], &rays[1], &rays[2]], &Scene::new(vertices, uvs));
        assert_eq!(Ok(got), expected_by_oracle, "slope {slope}");
        let fits = expected.iter().flatten().all(|c| i64::try_from(*c).is_ok());
        if !fits {
            let too_large = RayFootprint::Unbounded {
                axis: FootprintAxis::Down,
                reason: UnboundedReason::TooLarge,
            };
            assert_eq!(got, too_large, "slope {slope}");
            continue;
        }
        let narrow = expected.map(|row| row.map(|c| i64::try_from(c).expect("fits")));
        assert_eq!(derivatives(got), narrow, "slope {slope}");
        // The long axis is dy, and it grows with the slope.
        let length = narrow[1][1].unsigned_abs().into();
        assert!(length > longest && length >= narrow[0][0].unsigned_abs().into(), "slope {slope}");
        longest = length;
    }
    let at = |slope: i64| {
        let (vertices, uvs) = tilted_plane(depth, slope);
        camera.footprint(sample, &surface(vertices, uvs))
    };
    let unbounded = |reason| Ok(RayFootprint::Unbounded { axis: FootprintAxis::Down, reason });
    assert_eq!(at(4 * ONE), unbounded(UnboundedReason::Parallel));
    assert_eq!(at(4 * ONE + 1), unbounded(UnboundedReason::Behind));
    assert_eq!(at(5 * ONE), unbounded(UnboundedReason::Behind));
}

// --- Seeded cameras, pixels and triangles against the rational oracle ---------------------

fn seeded_spec(seeds: &mut Seeds, min_side: i64) -> CameraSpec {
    let size = [seeds.range(min_side, 512), seeds.range(min_side, 512)];
    let size = size.map(|s| u32::try_from(s).expect("positive"));
    let (position, look, up) =
        (seeds.triple(1 << 38), seeds.triple(1 << 36), seeds.triple(1 << 36));
    spec(position, look, up, seeds.range(1 << 20, 4 * ONE), size)
}

/// A sample whose right and down neighbours are inside the image (`width, height >= 2`).
fn interior_sample(seeds: &mut Seeds, spec: &CameraSpec) -> PixelSample {
    let (x, y) = (seeds.below(spec.width - 1), seeds.below(spec.height - 1));
    PixelSample::new(x, y, fixed(seeds.range(0, LAST)), fixed(seeds.range(0, LAST)))
}

fn neighbours(sample: PixelSample) -> [PixelSample; 3] {
    let PixelSample { x, y, sub_x, sub_y } = sample;
    [sample, PixelSample::new(x + 1, y, sub_x, sub_y), PixelSample::new(x, y + 1, sub_x, sub_y)]
}

fn add_scaled(origin: Raw3, direction: Raw3, scale: i64) -> Raw3 {
    let at =
        |o: i64, d: i64| o + i64::try_from(i128::from(d) * i128::from(scale) >> 32).expect("near");
    [at(origin[0], direction[0]), at(origin[1], direction[1]), at(origin[2], direction[2])]
}

fn add(a: Raw3, b: Raw3) -> Raw3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// A seeded triangle near where the hit ray reaches, a third of them built nearly edge-on to
/// one neighbouring ray, with texture coordinates within 16 units or, one time in eight,
/// anywhere in `i64`. `None` when a vertex leaves the trace domain or the triangle has no plane.
fn seeded_scene(seeds: &mut Seeds, rays: &[TraceRay; 3]) -> Option<([Raw3; 3], Uvs)> {
    let (origin, hit) = (raw3(rays[0].origin()), raw3(rays[0].direction()));
    let point = add_scaled(origin, hit, seeds.range(ONE >> 6, 16 * ONE));
    let reach = 1_i64 << seeds.range(20, 36);
    let (v0, e1) = if seeds.below(3) == 0 {
        let along = raw3(rays[1 + usize::try_from(seeds.below(2)).expect("small")].direction());
        let scale = seeds.range(ONE, 64 * ONE);
        (add(point, seeds.triple(4)), add(add_scaled([0; 3], along, scale), seeds.triple(4)))
    } else {
        (add(point, seeds.triple(reach)), seeds.triple(reach))
    };
    let vertices = [v0, add(v0, e1), add(v0, seeds.triple(reach))];
    let inside = vertices.iter().flatten().all(|c| c.unsigned_abs() <= LIMIT.unsigned_abs());
    let huge = seeds.below(8) == 0;
    let mut coordinate = || if huge { seeds.any() } else { seeds.range(-16 * ONE, 16 * ONE) };
    let uvs = [(); 3].map(|()| [coordinate(), coordinate()]);
    let planar = Scene::new(vertices, uvs).normal() != [0; 3];
    (inside && planar).then_some((vertices, uvs))
}

/// Counts of the outcomes a seeded test saw.
#[derive(Debug, Default)]
struct Tally {
    bounded: u32,
    parallel: u32,
    behind: u32,
    too_large: u32,
    refused: u32,
}

impl Tally {
    fn add(&mut self, outcome: &Result<RayFootprint, RayFootprintError>) {
        match outcome {
            Ok(RayFootprint::Bounded(_)) => self.bounded += 1,
            Ok(RayFootprint::Unbounded { reason: UnboundedReason::Parallel, .. }) => {
                self.parallel += 1
            }
            Ok(RayFootprint::Unbounded { reason: UnboundedReason::Behind, .. }) => self.behind += 1,
            Ok(RayFootprint::Unbounded { reason: UnboundedReason::TooLarge, .. }) => {
                self.too_large += 1;
            }
            Err(_) => self.refused += 1,
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_213_seeded_pinhole_footprints_equal_the_rational_oracle() {
    let mut seeds = Seeds(FOOTPRINT_SEED + 213);
    println!("seed {FOOTPRINT_SEED:#x} + 213");
    let mut tally = Tally::default();
    for case in 0..2_000 {
        let spec = seeded_spec(&mut seeds, 2);
        let Ok(camera) = PinholeCamera::new(&spec) else { continue };
        let sample = interior_sample(&mut seeds, &spec);
        let rays = neighbours(sample).map(|s| camera.ray(s).expect("inside the image"));
        let Some((vertices, uvs)) = seeded_scene(&mut seeds, &rays) else { continue };
        let got = camera.footprint(sample, &surface(vertices, uvs));
        let expected = oracle([&rays[0], &rays[1], &rays[2]], &Scene::new(vertices, uvs));
        assert_eq!(got, expected, "case {case}: {sample:?} {vertices:?} {uvs:?}");
        tally.add(&got);
    }
    println!("{tally:?}");
    assert!(tally.bounded >= 1_000 && tally.behind >= 20 && tally.too_large >= 20, "{tally:?}");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_214_seeded_thin_lens_footprints_hold_the_lens_point_and_equal_the_oracle() {
    let mut seeds = Seeds(FOOTPRINT_SEED + 214);
    println!("seed {FOOTPRINT_SEED:#x} + 214");
    let mut tally = Tally::default();
    for case in 0..1_000 {
        let camera = seeded_spec(&mut seeds, 2);
        let aperture = fixed(seeds.range(1, 4 * ONE));
        let focus_distance = fixed(seeds.range(ONE >> 8, 64 * ONE));
        let lens = ThinLensSpec { camera, aperture_radius: aperture, focus_distance };
        let Ok(thin) = ThinLensCamera::new(&lens) else { continue };
        let (sample, lens_sample) = (interior_sample(&mut seeds, &camera), seeds.unit_pair());
        let rays = neighbours(sample).map(|s| thin.ray(s, lens_sample).expect("accepted"));
        assert!(rays.iter().all(|r| r.origin() == rays[0].origin()), "case {case}");
        let Some((vertices, uvs)) = seeded_scene(&mut seeds, &rays) else { continue };
        let got = thin.footprint(sample, lens_sample, &surface(vertices, uvs));
        let expected = oracle([&rays[0], &rays[1], &rays[2]], &Scene::new(vertices, uvs));
        assert_eq!(got, expected, "case {case}: {sample:?} {lens_sample:?} {vertices:?}");
        tally.add(&got);
    }
    println!("{tally:?}");
    assert!(tally.bounded >= 500 && tally.behind >= 10 && tally.too_large >= 10, "{tally:?}");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_215_aperture_zero_gives_the_pinhole_footprint_bit_for_bit() {
    let mut seeds = Seeds(FOOTPRINT_SEED + 215);
    println!("seed {FOOTPRINT_SEED:#x} + 215");
    let mut compared = 0;
    for case in 0..500 {
        let camera = seeded_spec(&mut seeds, 1);
        let Ok(pinhole) = PinholeCamera::new(&camera) else { continue };
        let focus_distance = fixed(seeds.range(1, 1 << 40));
        let lens = ThinLensSpec { camera, aperture_radius: fixed(0), focus_distance };
        let thin = ThinLensCamera::new(&lens).expect("aperture 0 is accepted");
        // Any sample, edges included.
        let (x, y) = (seeds.below(camera.width), seeds.below(camera.height));
        let sample =
            PixelSample::new(x, y, fixed(seeds.range(0, LAST)), fixed(seeds.range(0, LAST)));
        let ray = pinhole.ray(sample).expect("inside");
        let rays = [ray, ray, ray];
        let Some((vertices, uvs)) = seeded_scene(&mut seeds, &rays) else { continue };
        let plane = surface(vertices, uvs);
        let got = thin.footprint(sample, seeds.unit_pair(), &plane);
        assert_eq!(got, pinhole.footprint(sample, &plane), "case {case}: {sample:?}");
        compared += 1;
    }
    println!("{compared} footprints compared");
    assert!(compared >= 450, "only {compared} compared");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_216_neighbours_past_the_image_edge_follow_the_image_plane_formula() {
    // A camera n by m with tangent k m raw and one (n + 2) by (m + 2) with tangent k (m + 2)
    // have the same per-pixel steps and centre; when they also have the same scale shift,
    // pixel (x + 1, y + 1) of the larger is pixel (x, y) of the smaller, bit for bit, and the
    // larger camera's rays are the smaller's neighbours past its last column and row.
    let mut seeds = Seeds(FOOTPRINT_SEED + 216);
    println!("seed {FOOTPRINT_SEED:#x} + 216");
    let (mut edges, mut shifts_differ) = (0, 0);
    for case in 0..1_500 {
        let size = [seeds.range(1, 64), seeds.range(1, 64)].map(|s| u32::try_from(s).expect("+"));
        let per_row = seeds.range(1 << 22, 1 << 26);
        let (position, look, up) =
            (seeds.triple(1 << 38), seeds.triple(1 << 36), seeds.triple(1 << 36));
        let tan = |rows: u32| per_row * i64::from(rows);
        let small = spec(position, look, up, tan(size[1]), size);
        let large = spec(position, look, up, tan(size[1] + 2), size.map(|s| s + 2));
        let (Ok(small), Ok(large)) = (PinholeCamera::new(&small), PinholeCamera::new(&large))
        else {
            continue;
        };
        if small.scale_shift() != large.scale_shift() {
            shifts_differ += 1;
            continue;
        }
        let (sub_x, sub_y) = (fixed(seeds.range(0, LAST)), fixed(seeds.range(0, LAST)));
        let (x, y) = match seeds.below(3) {
            0 => (size[0] - 1, seeds.below(size[1])),
            1 => (seeds.below(size[0]), size[1] - 1),
            _ => (size[0] - 1, size[1] - 1),
        };
        let sample = PixelSample::new(x, y, sub_x, sub_y);
        let shifted = |dx: u32, dy: u32| PixelSample::new(x + dx, y + dy, sub_x, sub_y);
        let rays = [(1, 1), (2, 1), (1, 2)].map(|(dx, dy)| large.ray(shifted(dx, dy)).expect("in"));
        assert_eq!(small.ray(sample), Some(rays[0]), "case {case}: the same base ray");
        let Some((vertices, uvs)) = seeded_scene(&mut seeds, &rays) else { continue };
        let got = small.footprint(sample, &surface(vertices, uvs));
        let expected = oracle([&rays[0], &rays[1], &rays[2]], &Scene::new(vertices, uvs));
        assert_eq!(got, expected, "case {case}: {sample:?} of {size:?}");
        edges += 1;
    }
    println!("{edges} edge footprints checked; {shifts_differ} camera pairs had different shifts");
    assert!(edges >= 1_000, "only {edges} checked");
}

// --- Thresholds one raw unit either side, refusals and extremes ----------------------------

/// A camera at the origin looking along `+x` with up `+z` (so right is `-y`), tangent 1 and a
/// 2 by 2 image. Pixel (0, 0) at sub-pixel offset `(sx, sy)` has the ray
/// `(1, 1 - sx, 1 - sy)` and pixel (0, 1) the ray `(1, 1 - sx, -sy)`, in raw units of `2^-32`.
fn horizon_camera() -> PinholeCamera {
    let camera = spec([0; 3], [ONE, 0, 0], [0, 0, ONE], ONE, [2, 2]);
    PinholeCamera::new(&camera).expect("a valid camera")
}

/// The plane through `(0, 0, height)` with normal `64 L (0, 1, 2^32 - 1)` (`L` 64 units):
/// `D . N` is `64 L ((1 - sx) + (2^32 - 1)(-sy))` raw for the ray `(1, 1 - sx, -sy)`.
fn steep_plane(height: i64, uvs: Uvs) -> ([Raw3; 3], Uvs) {
    let side = 64 * ONE;
    let v0 = [0, 0, height];
    let e2 = [0, (ONE - 1) * 64, -64];
    ([v0, add(v0, [side, 0, 0]), add(v0, e2)], uvs)
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_217_each_threshold_is_exact_one_raw_unit_either_side() {
    let camera = horizon_camera();
    let ray = |x: u32, y: u32, sx: i64, sy: i64| {
        let ray = camera.ray(PixelSample::new(x, y, fixed(sx), fixed(sy))).expect("in");
        raw3(ray.direction())
    };
    // The premises of the construction.
    assert_eq!(camera.scale_shift(), 0);
    assert_eq!([ray(0, 0, 5, 7), ray(0, 1, 5, 7)], [[ONE, ONE - 5, ONE - 7], [ONE, ONE - 5, -7]]);
    let small: Uvs = [[0, 0], [4, 0], [0, 4]];
    let unit: Uvs = [[0, 0], [ONE, 0], [0, ONE]];
    let measure = |x, y, sx, sy, (vertices, uvs): ([Raw3; 3], Uvs)| {
        let sample = PixelSample::new(x, y, fixed(sx), fixed(sy));
        let got = camera.footprint(sample, &surface(vertices, uvs));
        let rays = neighbours(sample).map(|s| camera.ray(s));
        if let [Some(a), Some(b), Some(c)] = rays {
            assert_eq!(
                got,
                oracle([&a, &b, &c], &Scene::new(vertices, uvs)),
                "({x}, {y}) {sx} {sy}"
            );
        }
        got
    };
    let down = |reason| Ok(RayFootprint::Unbounded { axis: FootprintAxis::Down, reason });
    // The down neighbour (0, 1) at sy = 1 raw has D . N = 64 L (1 - sx): +1, 0, -1 units of
    // 64 L at sx = 0, 1, 2. One unit ahead it meets the plane 2^32 times farther than the hit.
    let plane = steep_plane(ONE, small);
    assert!(matches!(measure(0, 0, 0, 1, plane), Ok(RayFootprint::Bounded(_))));
    assert_eq!(measure(0, 0, 0, 1, steep_plane(ONE, unit)), down(UnboundedReason::TooLarge));
    assert_eq!(measure(0, 0, 1, 1, plane), down(UnboundedReason::Parallel));
    assert_eq!(measure(0, 0, 2, 1, plane), down(UnboundedReason::Behind));
    // The hit ray itself: pixel (0, 1) at sy = 1 raw has D . N = 64 L (1 - sx) too.
    assert!(measure(0, 1, 0, 1, plane).is_ok());
    assert_eq!(measure(0, 1, 1, 1, plane), Err(RayFootprintError::RayParallelToPlane));
    assert_eq!(measure(0, 1, 2, 1, plane), Err(RayFootprintError::PlaneNotAhead));
    // The plane one raw unit above, through and one raw unit below the camera position.
    assert!(measure(0, 0, 0, 1, steep_plane(1, small)).is_ok());
    assert_eq!(measure(0, 0, 0, 1, steep_plane(0, small)), Err(RayFootprintError::PlaneNotAhead));
    assert_eq!(measure(0, 0, 0, 1, steep_plane(-1, small)), Err(RayFootprintError::PlaneNotAhead));
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_218_invalid_inputs_are_refused_in_order() {
    let triangle = |v: [Raw3; 3]| {
        TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the trace domain")
    };
    let uvs = [FixedVec2::new(fixed(0), fixed(0)); 3];
    let (a, b) = ([ONE, 2 * ONE, -3 * ONE], [5 * ONE, -ONE, 7]);
    let collinear = [a, b, [2 * b[0] - a[0], 2 * b[1] - a[1], 2 * b[2] - a[2]]];
    for vertices in [collinear, [a, a, b], [a, b, b], [a; 3]] {
        let got = TexturedTriangle::new(triangle(vertices), uvs);
        assert_eq!(got, Err(RayFootprintError::DegenerateTriangle), "{vertices:?}");
    }
    // The thinnest accepted triangle: a normal of one raw unit squared.
    let thin = [[0, 0, 0], [1 << 39, (1 << 39) - 1, 0], [1, 1, 0]];
    assert!(TexturedTriangle::new(triangle(thin), uvs).is_ok());

    let pinhole = level_camera(ONE, 4);
    let plane = facing_plane(ONE, ONE);
    let at = |x, y, sx, sy| PixelSample::new(x, y, fixed(sx), fixed(sy));
    for sample in [at(4, 0, 0, 0), at(0, 4, 0, 0), at(0, 0, ONE, 0), at(0, 0, 0, -1)] {
        assert_eq!(pinhole.footprint(sample, &plane), Err(RayFootprintError::SampleOutsideImage));
    }
    let lens = ThinLensSpec {
        camera: spec([0; 3], [0, 0, -ONE], [0, ONE, 0], ONE, [4, 4]),
        aperture_radius: fixed(ONE / 4),
        focus_distance: fixed(ONE),
    };
    let thin = ThinLensCamera::new(&lens).expect("a valid lens");
    let centre = PixelSample::centre(1, 1);
    let outside = [fixed(ONE), fixed(0)];
    assert!(thin.footprint(centre, [fixed(0); 2], &plane).is_ok());
    assert_eq!(
        thin.footprint(centre, outside, &plane),
        Err(RayFootprintError::LensSampleOutsideUnitSquare)
    );
    // The sample is checked before the lens sample and before the plane.
    assert_eq!(
        thin.footprint(at(4, 0, 0, 0), outside, &plane),
        Err(RayFootprintError::SampleOutsideImage)
    );
    let behind = facing_plane(-ONE, ONE);
    assert_eq!(
        pinhole.footprint(at(4, 0, 0, 0), &behind),
        Err(RayFootprintError::SampleOutsideImage)
    );
    assert_eq!(pinhole.footprint(centre, &behind), Err(RayFootprintError::PlaneNotAhead));
    let errors = [
        RayFootprintError::SampleOutsideImage,
        RayFootprintError::LensSampleOutsideUnitSquare,
        RayFootprintError::DegenerateTriangle,
        RayFootprintError::RayParallelToPlane,
        RayFootprintError::PlaneNotAhead,
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    for (i, message) in messages.iter().enumerate() {
        assert!(!message.is_empty() && !messages[..i].contains(message), "{message}");
        let _: &dyn std::error::Error = &errors[i];
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_219_extreme_cameras_triangles_and_coordinates_neither_wrap_nor_panic() {
    // The widest field of view on the most extreme image shapes, the camera at trace-domain
    // corners, every image corner with both sub-pixel extremes (neighbours past the edge), the
    // triangle at trace-domain corners and texture coordinates at the ends of `i64`. Overflow
    // checks are on in test builds, so any wrap in `i128` arithmetic panics here.
    let mut seeds = Seeds(FOOTPRINT_SEED + 219);
    println!("seed {FOOTPRINT_SEED:#x} + 219");
    let corners = [-LIMIT, LIMIT];
    let mut tally = Tally::default();
    for size in [[1, 1], [1 << 15, 1], [1, 1 << 15], [1 << 15, 1 << 15], [3, 2]] {
        for case in 0..40 {
            let position = [0; 3].map(|_| corners[usize::try_from(seeds.below(2)).expect("0, 1")]);
            let (look, up) = (seeds.triple(1 << 36), seeds.triple(1 << 36));
            let spec = spec(position, look, up, bunny_optics::TAN_HALF_FOV_LIMIT_RAW, size);
            let Ok(camera) = PinholeCamera::new(&spec) else { continue };
            let lens = ThinLensSpec {
                camera: spec,
                aperture_radius: fixed(seeds.range(0, 1 << 30)),
                focus_distance: fixed(seeds.range(1, 1 << 20)),
            };
            let thin = ThinLensCamera::new(&lens).ok();
            let vertices = [(); 3]
                .map(|()| [0; 3].map(|_| corners[usize::try_from(seeds.below(2)).expect("0, 1")]));
            let extremes = [i64::MIN, i64::MAX, 0, -1];
            let uvs = [(); 3]
                .map(|()| [(); 2].map(|()| extremes[usize::try_from(seeds.below(4)).expect("<4")]));
            let Ok(plane) = TexturedTriangle::new(
                triangle_of(vertices),
                uvs.map(|[u, v]| FixedVec2::new(fixed(u), fixed(v))),
            ) else {
                continue;
            };
            for (x, y) in [(0, 0), (size[0] - 1, 0), (0, size[1] - 1), (size[0] - 1, size[1] - 1)] {
                for sub in [0, LAST] {
                    let sample = PixelSample::new(x, y, fixed(sub), fixed(sub));
                    tally.add(&camera.footprint(sample, &plane));
                    if let Some(thin) = &thin {
                        tally.add(&thin.footprint(sample, [fixed(LAST); 2], &plane));
                    }
                }
            }
            let _ = case;
        }
    }
    println!("{tally:?}");
    let total = tally.bounded + tally.parallel + tally.behind + tally.too_large + tally.refused;
    assert!(total >= 1_000 && tally.bounded > 0 && tally.too_large > 0, "{tally:?}");
}

fn triangle_of(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the trace domain")
}
