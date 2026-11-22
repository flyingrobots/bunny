//! Property oracles for `bunny-diffuse-shading/1`: normals and light cosines against a
//! floating-point oracle, exact orthonormality of the frame, the hemisphere map's bounds, a
//! distribution guard over stratified sampler points, energy sanity, the self-intersection rule
//! on constructed scenes, and the shadow-ray limit at the corners of the trace domain.
//!
//! Floating point appears only in these test oracles, never in the crate. Its error (about
//! `2^-53` relative) is far below the one-raw-unit (`2^-32`) scale of every bound checked.

// Frame algebra reads best as t, b and n for tangent, bitangent and normal.
#![allow(clippy::many_single_char_names)]

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    Albedo, DirectionalLight, PixelSampler, ShadingFrame, SkyLight, SurfaceHit, TriangleSurface,
    MIN_BOUNCE_COSINE_RAW, PI, SHADOW_T_MAX,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const WIDE_ONE: i128 = 1 << 32;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const SHADING_TEST_SEED: u64 = 0xB002_0160_D1FF_0001;
const SAMPLER_SEED: u64 = 0x0123_4567_89AB_CDEF;

/// Frame residuals, in raw units of Q32.32 (so `2^-32` each): pairwise dot products, squared
/// lengths minus one, and `tangent x bitangent - normal` per component.
const FRAME_DOT_LIMIT: i128 = 8;
const FRAME_LENGTH_LIMIT: i128 = 8;
const FRAME_CROSS_LIMIT: i128 = 8;
/// The hemisphere map's stated bound: each component within 4 raw units of the ideal map.
const DIRECTION_LIMIT: f64 = 4.0;
/// A bounce direction's length is within this many raw units of 1.0.
const DIRECTION_LENGTH_LIMIT: i128 = 8;
/// The light cosine's stated bound, in raw units, against the normal it is given.
const COSINE_LIMIT: f64 = 3.0;

type Raw3 = [i64; 3];
type Wide3 = [i128; 3];

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

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    /// A seeded triangle: a random scale from one raw unit to the whole domain.
    fn triangle(&mut self) -> [Raw3; 3] {
        let bits = self.range(0, 39);
        let bound = (1_i64 << bits).max(1);
        let centre = self.triple(TRACE_COORD_LIMIT_RAW / 2);
        let corner = |seeds: &mut Self| {
            let offset = seeds.triple(bound);
            [0, 1, 2].map(|i| centre[i] + offset[i])
        };
        [corner(self), corner(self), corner(self)]
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

fn wide(v: FixedVec3) -> Wide3 {
    raw3(v).map(i128::from)
}

fn dot(a: Wide3, b: Wide3) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Wide3, b: Wide3) -> Wide3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn sub(a: Raw3, b: Raw3) -> Wide3 {
    [0, 1, 2].map(|i| i128::from(a[i]) - i128::from(b[i]))
}

/// The nearest `f64`: exact below `2^53`, within `2^-53` relative above it.
#[allow(clippy::cast_precision_loss)] // The oracle's error is far below the bounds checked.
fn float(value: i128) -> f64 {
    value as f64
}

fn to_f64(v: Wide3) -> [f64; 3] {
    v.map(float)
}

/// `value` rounded to the nearest integer, for oracle inputs well inside `i64`.
#[allow(clippy::cast_possible_truncation)] // Every caller passes a value below 2^33.
fn round_raw(value: f64) -> i64 {
    value.round() as i64
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).expect("a small index")
}

fn trace_triangle(v: [Raw3; 3]) -> Option<TraceTriangle> {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2]))
}

/// The exact plane normal `(v1 - v0) x (v2 - v0)`.
fn plane(v: [Raw3; 3]) -> Wide3 {
    cross(sub(v[1], v[0]), sub(v[2], v[0]))
}

/// A unit normal from a seeded vector, including the special cases of the frame's sign rule.
fn seeded_normals(seeds: &mut Seeds, count: usize) -> Vec<FixedUnitVec3> {
    let special: [Raw3; 10] = [
        [0, 0, ONE],
        [0, 0, -ONE],
        [ONE, 0, 0],
        [0, -ONE, 0],
        [ONE, 1, 0],
        [ONE, 0, -1],
        [1, 1, -ONE],
        [-1, 1, ONE],
        [ONE, ONE, -1],
        [ONE, -ONE, 1],
    ];
    let mut normals: Vec<_> = special.iter().filter_map(|&v| FixedUnitVec3::new(vec3(v))).collect();
    while normals.len() < count {
        if let Some(n) = FixedUnitVec3::new(vec3(seeds.triple(ONE))) {
            normals.push(n);
        }
    }
    normals
}

fn frame_vectors(frame: &ShadingFrame) -> [Wide3; 3] {
    [wide(frame.tangent()), wide(frame.bitangent()), wide(frame.normal())]
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_041_normals_are_within_one_raw_unit_of_the_exact_unit_normal() {
    let mut seeds = Seeds(SHADING_TEST_SEED);
    let mut checked = 0;
    for case in 0..4_000 {
        let vertices = seeds.triangle();
        let Some(triangle) = trace_triangle(vertices) else { continue };
        let exact = plane(vertices);
        let surface = TriangleSurface::new(&triangle);
        if exact == [0; 3] {
            assert_eq!(surface, None, "case {case}: zero area");
            continue;
        }
        let normal = wide(surface.expect("nonzero area").frame().normal());
        let [x, y, z] = to_f64(exact);
        let length = (x * x + y * y + z * z).sqrt();
        for (axis, &component) in to_f64(exact).iter().enumerate() {
            let ideal = component / length * float(WIDE_ONE);
            let error = (float(normal[axis]) - ideal).abs();
            assert!(error <= 1.0, "case {case} axis {axis}: {error} raw units");
        }
        checked += 1;
    }
    assert!(checked >= 3_900, "only {checked} triangles checked");
}

/// Unit normals whose frame division `x y / (s + z)` is an exact tie (`z = 0`, the 2-adic
/// valuations of `x` and `y` adding up to 31): the review's `(2^31, 3 719 550 789, 0)` and the
/// four triangle normals of `OPTICS-TP-094`, each a tie with an even or an odd truncation.
const TIE_NORMALS: [Raw3; 5] = [
    [1 << 31, 3_719_550_789, 0],
    [607_322_112, 4_251_811_840, 0],
    [-607_322_112, 4_251_811_840, 0],
    [4_171_038_720, 1_024_294_912, 0],
    [-4_171_038_720, 1_024_294_912, 0],
];

/// `numerator / denominator` rounded to the nearest integer, ties to the even one: the floor
/// of the signed quotient, moved up when twice the remainder exceeds the divisor or equals it
/// with an odd floor.
fn nearest_even(numerator: i128, denominator: i128) -> i128 {
    let (n, d) =
        if denominator < 0 { (-numerator, -denominator) } else { (numerator, denominator) };
    let (floor, twice) = (n.div_euclid(d), 2 * n.rem_euclid(d));
    if twice > d || (twice == d && floor % 2 != 0) {
        floor + 1
    } else {
        floor
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_042_frames_are_orthonormal_and_follow_the_stated_formula() {
    let mut seeds = Seeds(SHADING_TEST_SEED ^ 0x42);
    let one_squared = WIDE_ONE * WIDE_ONE;
    let mut normals = seeded_normals(&mut seeds, 4_000);
    normals.extend(TIE_NORMALS.map(|v| FixedUnitVec3::try_from_unit(vec3(v)).expect("unit")));
    let mut ties = 0;
    for (case, normal) in normals.into_iter().enumerate() {
        for frame in [ShadingFrame::new(normal), ShadingFrame::new(normal).flipped()] {
            let [t, b, n] = frame_vectors(&frame);
            for (name, residual, limit) in [
                ("t.n", dot(t, n), FRAME_DOT_LIMIT),
                ("b.n", dot(b, n), FRAME_DOT_LIMIT),
                ("t.b", dot(t, b), FRAME_DOT_LIMIT),
                ("|t|^2", dot(t, t) - one_squared, FRAME_LENGTH_LIMIT),
                ("|b|^2", dot(b, b) - one_squared, FRAME_LENGTH_LIMIT),
            ] {
                assert!(residual.abs() <= limit * WIDE_ONE, "case {case} {name}: {residual}");
            }
            let handed = cross(t, b);
            for axis in 0..3 {
                let residual = handed[axis] - n[axis] * WIDE_ONE;
                assert!(residual.abs() <= FRAME_CROSS_LIMIT * WIDE_ONE, "case {case} axis {axis}");
            }
        }
        // Each component is the formula on this normal rounded to nearest with ties to even,
        // exactly: the nearest integer to the rational e / d, the even one on a tie.
        let [x, y, z] = wide(normal.into_inner());
        let sign = if z >= 0 { 1 } else { -1 };
        let d = sign * WIDE_ONE + z;
        let [t, b, _] = frame_vectors(&ShadingFrame::new(normal));
        let rationals = [
            (t[0], WIDE_ONE * d - sign * x * x),
            (t[1], -sign * x * y),
            (b[0], -x * y),
            (b[1], sign * WIDE_ONE * d - y * y),
        ];
        for (index, (component, numerator)) in rationals.into_iter().enumerate() {
            assert_eq!(component, nearest_even(numerator, d), "case {case} rational {index}");
            ties += usize::from((2 * numerator) % d == 0 && (2 * numerator / d) % 2 != 0);
        }
        assert_eq!((t[2], b[2]), (-sign * x, -y), "case {case}");
    }
    // Two tied quotients (q(x y) in the tangent and the bitangent) per tie normal.
    assert!(ties >= 2 * TIE_NORMALS.len(), "only {ties} exact ties checked");
}

/// The ideal map on `frame` for `[u, v]`, in raw units, by floating point.
fn ideal_direction(frame: &ShadingFrame, u: i64, v: i64) -> [f64; 3] {
    let (u, v) = (float(i128::from(u)) / float(WIDE_ONE), float(i128::from(v)) / float(WIDE_ONE));
    let angle = v * std::f64::consts::TAU;
    let weights = [u.sqrt() * angle.cos(), u.sqrt() * angle.sin(), (1.0 - u).sqrt()];
    let [t, b, n] = frame_vectors(frame).map(to_f64);
    [0, 1, 2].map(|i| weights[0] * t[i] + weights[1] * b[i] + weights[2] * n[i])
}

fn check_direction(frame: &ShadingFrame, sample: [i64; 2], label: &str) {
    let direction = frame.cosine_direction(sample.map(fixed)).expect("a unit-square point");
    let d = wide(direction);
    let n = wide(frame.normal());
    assert!(d.iter().all(|c| c.abs() <= WIDE_ONE), "{label}: {d:?} outside the trace domain");
    let cosine = dot(d, n);
    assert!(cosine >= i128::from(MIN_BOUNCE_COSINE_RAW) * WIDE_ONE, "{label}: cosine {cosine}");
    let squared = u128::try_from(dot(d, d)).expect("non-negative");
    let length = i128::try_from(squared.isqrt()).expect("fits");
    assert!((length - WIDE_ONE).abs() <= DIRECTION_LENGTH_LIMIT, "{label}: length {length}");
    let ideal = ideal_direction(frame, sample[0], sample[1]);
    for axis in 0..3 {
        let error = (float(d[axis]) - ideal[axis]).abs();
        assert!(error <= DIRECTION_LIMIT, "{label} axis {axis}: {error} raw units");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_044_bounce_directions_are_above_the_surface_and_near_the_ideal_map() {
    let mut seeds = Seeds(SHADING_TEST_SEED ^ 0x44);
    let edges = [0, 1, ONE / 2, ONE / 4, ONE - 1, ONE - (1 << 16)];
    for (case, normal) in seeded_normals(&mut seeds, 600).into_iter().enumerate() {
        let sampler = PixelSampler::new(SAMPLER_SEED, index_u32(case), 7);
        for frame in [ShadingFrame::new(normal), ShadingFrame::new(normal).flipped()] {
            for sample in 0..16 {
                let [u, v] = sampler.point(sample, 1).map(FixedQ32_32::raw);
                check_direction(&frame, [u, v], &format!("case {case} sample {sample}"));
            }
            for &u in &edges {
                for &v in &edges {
                    check_direction(&frame, [u, v], &format!("case {case} edge ({u}, {v})"));
                }
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_045_directions_follow_the_cosine_distribution() {
    // A regression guard with stated thresholds over stratified sampler points, not a
    // statistical test. Exact cosine-weighted values: E[cos] = 2/3; P(cos < c) = c^2, so the
    // bands [0, 1/4), [1/4, 1/2), [1/2, 3/4), [3/4, 1] hold 1/16, 3/16, 5/16, 7/16; each
    // quarter turn of azimuth holds 1/4.
    let mut seeds = Seeds(SHADING_TEST_SEED ^ 0x45);
    let mut frames = vec![ShadingFrame::new(FixedUnitVec3::UNIT_Z)];
    frames.extend(seeded_normals(&mut seeds, 13).into_iter().skip(9).map(ShadingFrame::new));
    for (case, frame) in frames.iter().enumerate() {
        let [t, b, n] = frame_vectors(frame);
        let sampler = PixelSampler::new(SAMPLER_SEED, 3, 5 + index_u32(case));
        let directions: Vec<Wide3> = (0..4_096)
            .map(|s| wide(frame.cosine_direction(sampler.point(s, 1)).expect("in range")))
            .collect();
        let total: i128 = directions.iter().map(|d| dot(*d, n)).sum();
        let mean = float(total) / 4_096.0 / float(WIDE_ONE * WIDE_ONE);
        assert!((mean - 2.0 / 3.0).abs() <= 1.0 / 4_096.0, "frame {case}: mean cosine {mean}");
        // The first 1 024 samples are a (0, 10, 2)-net: u and v are each stratified in 1 024
        // equal intervals, and the band and quadrant edges fall on interval edges.
        let mut bands = [0_i32; 4];
        let mut quadrants = [0_i32; 4];
        for d in &directions[..1_024] {
            let cosine = dot(*d, n) / WIDE_ONE;
            bands[usize::try_from((cosine * 4 / WIDE_ONE).min(3)).expect("non-negative")] += 1;
            let (x, y) = (dot(*d, t), dot(*d, b));
            let quadrant = match (x.signum(), y.signum()) {
                (1, 0 | 1) => 0,
                (-1 | 0, 1) => 1,
                (-1, -1 | 0) => 2,
                _ => 3,
            };
            quadrants[quadrant] += 1;
        }
        for (band, (&count, want)) in bands.iter().zip([64, 192, 320, 448]).enumerate() {
            assert!((count - want).abs() <= 1, "frame {case} band {band}: {count}");
        }
        for (quadrant, &count) in quadrants.iter().enumerate() {
            assert!((count - 256).abs() <= 2, "frame {case} quadrant {quadrant}: {count}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_049_light_cosines_are_within_three_raw_units() {
    let mut seeds = Seeds(SHADING_TEST_SEED ^ 0x49);
    let normals = seeded_normals(&mut seeds, 400);
    let white = [FixedQ32_32::ONE; 3];
    for case in 0..400 {
        // A light direction with one component exactly +-1.0 and the others within 1.0.
        let mut toward = seeds.triple(ONE);
        let axis = usize::try_from(seeds.range(0, 2)).expect("small");
        toward[axis] = if seeds.range(0, 1) == 0 { ONE } else { -ONE };
        let light = DirectionalLight::new(vec3(toward), white).expect("a normalized light");
        let l = to_f64(toward.map(i128::from));
        let l_length = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        for normal in &normals {
            let n = to_f64(wide(normal.into_inner()));
            let exact = (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / l_length;
            let expected = exact.clamp(0.0, float(WIDE_ONE));
            let actual = float(i128::from(light.cosine(normal.into_inner()).raw()));
            assert!(
                (actual - expected).abs() <= COSINE_LIMIT,
                "case {case}: {actual} vs {expected}"
            );
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_050_white_surfaces_conserve_energy() {
    // A white ground under a uniform sky of radiance L and nothing else: every bounce ray
    // escapes, so every sample is exactly L, with no pi anywhere in the bounce step.
    let ground = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];
    let triangle = trace_triangle(ground).expect("in the domain");
    let surface = TriangleSurface::new(&triangle).expect("not degenerate");
    let white = Albedo::new([FixedQ32_32::ONE; 3]).expect("valid");
    let sky = SkyLight::new([fixed(ONE / 3), fixed(ONE), fixed(12_345 * ONE)]).expect("valid");
    let down = TraceRay::new(vec3([ONE / 5, 2 * ONE, ONE / 7]), vec3([0, -ONE, 0])).expect("ray");
    let hit = SurfaceHit::new(
        &surface,
        &down,
        &down.hit_triangle(&triangle, fixed(0), MAX).expect("hit"),
    )
    .expect("in the domain");
    let sampler = PixelSampler::new(SAMPLER_SEED, 1, 2);
    let mut sums = [0_i128; 3];
    for sample in 0..1_024 {
        let bounce = hit.bounce_ray(sampler.point(sample, 1)).expect("in range");
        assert!(bounce.hit_triangle(&triangle, fixed(0), MAX).is_none(), "sample {sample}");
        let radiance = white.reflect([FixedQ32_32::ZERO; 3], sky.radiance()).expect("bounded");
        for (sum, channel) in sums.iter_mut().zip(radiance) {
            *sum += i128::from(channel.raw());
        }
    }
    assert_eq!(sums.map(|s| s / 1_024), sky.radiance().map(|c| i128::from(c.raw())));
    // The same sky as 1 024 directional lights, one per equal-solid-angle band of a uniform
    // hemisphere (cosines at band midpoints, azimuths by the golden angle), each delivering
    // irradiance L * 2 pi / 1024 to a perpendicular surface: the white surface reflects
    // sum(E * cos / pi) = L * (2 / 1024) * sum(cos) = L. A missing or extra pi is off by 3x.
    let radiance = 1_000 * i128::from(ONE);
    let irradiance = fixed(i64::try_from(radiance / 1_024).expect("fits")) * (PI + PI);
    let mut total = 0_i128;
    for band in 0..1_024 {
        let cosine = (f64::from(band) + 0.5) / 1_024.0;
        let azimuth = f64::from(band) * 2.399_963_229_728_653;
        let sine = (1.0 - cosine * cosine).sqrt();
        let toward = [sine * azimuth.cos(), cosine, sine * azimuth.sin()];
        let largest = toward.iter().fold(0.0_f64, |m, c| m.max(c.abs()));
        let raw = toward.map(|c| round_raw(c / largest * float(WIDE_ONE)));
        let light = DirectionalLight::new(vec3(raw), [irradiance; 3]).expect("normalized");
        let sample = hit.light_sample(&light);
        total += i128::from(
            white.reflect(sample.radiance(true), [FixedQ32_32::ZERO; 3]).expect("ok")[0].raw(),
        );
    }
    let relative = float(total - radiance) / float(radiance);
    assert!(relative.abs() <= 1.0 / 65_536.0, "direct furnace: relative error {relative}");
}

/// Every triangle of `scene` that `ray` hits at a distance in `0..=t_max`.
fn hits(scene: &[TraceTriangle], ray: &TraceRay, t_max: FixedQ32_32) -> Vec<usize> {
    (0..scene.len()).filter(|&i| ray.hit_triangle(&scene[i], fixed(0), t_max).is_some()).collect()
}

/// The lowest-index nearest hit of `ray` in `scene`.
fn closest(scene: &[TraceTriangle], ray: &TraceRay) -> Option<(usize, bunny_query::TriangleHit)> {
    (0..scene.len())
        .filter_map(|i| ray.hit_triangle(&scene[i], fixed(0), MAX).map(|hit| (i, hit)))
        .min_by_key(|(i, hit)| (hit.distance, *i))
}

/// Shoots seeded camera rays at `targets` in `scene` (from above), and checks that no
/// secondary ray from a hit meets any triangle of the scene: the start point is strictly on the
/// arrival side of every coplanar triangle, and bounce and shadow rays leave into that side.
/// Returns how many hits were checked.
fn check_no_self_hits(vertices: &[[Raw3; 3]], targets: &[Raw3], above: Raw3, seed: u64) -> usize {
    let scene: Vec<_> =
        vertices.iter().map(|&v| trace_triangle(v).expect("in the domain")).collect();
    let surfaces: Vec<_> =
        scene.iter().map(|t| TriangleSurface::new(t).expect("not degenerate")).collect();
    let lights = [[0, ONE, 0], [ONE, 1 << 20, -ONE / 3], [-ONE / 7, ONE / 1_000, ONE]]
        .map(|toward| DirectionalLight::new(vec3(toward), [FixedQ32_32::ONE; 3]).expect("light"));
    let mut seeds = Seeds(seed);
    let mut checked = 0;
    for (index, target) in targets.iter().enumerate() {
        let offset = seeds.triple(ONE / 2);
        let origin = [0, 1, 2].map(|i| target[i] + above[i] + offset[i]);
        let toward: Wide3 = sub(*target, origin);
        let largest = toward.iter().map(|c| c.abs()).max().expect("three");
        let direction = toward.map(|c| i64::try_from(c * WIDE_ONE / largest).expect("fits"));
        let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("a ray");
        let Some((primitive, hit)) = closest(&scene, &ray) else { continue };
        let shading = SurfaceHit::new(&surfaces[primitive], &ray, &hit).expect("in the domain");
        let sampler = PixelSampler::new(SAMPLER_SEED, index_u32(index), 0);
        for sample in 0..8 {
            let bounce = shading.bounce_ray(sampler.point(sample, 1)).expect("in range");
            assert_eq!(
                hits(&scene, &bounce, MAX),
                Vec::<usize>::new(),
                "target {index} bounce {sample}"
            );
        }
        for light in &lights {
            if let Some(shadow) = shading.light_sample(light).shadow_ray() {
                assert_eq!(
                    hits(&scene, shadow, SHADOW_T_MAX),
                    Vec::<usize>::new(),
                    "target {index}"
                );
            }
        }
        checked += 1;
    }
    checked
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_047_secondary_rays_never_hit_their_own_or_a_coplanar_surface() {
    // Two coplanar triangles sharing the diagonal of a square ground, a tilted coplanar pair
    // sharing an edge, an exact duplicate with the opposite winding, and a convex ridge.
    let s = 8 * ONE;
    let square = [[[-s, 0, -s], [s, 0, s], [s, 0, -s]], [[-s, 0, -s], [-s, 0, s], [s, 0, s]]];
    // The plane x + 3y + 2z = 0 through the origin, as two triangles sharing (p, q).
    let (p, q) = ([3 * ONE, -ONE, 0], [-2 * ONE, 0, ONE]);
    let tilted = [[p, q, [0, 2 * ONE, -3 * ONE]], [q, p, [ONE, -3 * ONE, 4 * ONE]]];
    let ridge = [[[-s, 0, -s], [0, ONE, s], [0, ONE, -s]], [[0, ONE, -s], [0, ONE, s], [s, 0, s]]];
    let mut seeds = Seeds(SHADING_TEST_SEED ^ 0x47);
    // Targets on the shared edges and vertices exactly, and seeded points near them.
    let along = |a: Raw3, b: Raw3, k: i64| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) / 16 * k);
    let mut on_square: Vec<Raw3> = (0..=16).map(|k| along([-s, 0, -s], [s, 0, s], k)).collect();
    let mut on_tilted: Vec<Raw3> = (0..=16).map(|k| along(p, q, k)).collect();
    let mut on_ridge: Vec<Raw3> = (0..=16).map(|k| along([0, ONE, -s], [0, ONE, s], k)).collect();
    for _ in 0..200 {
        let t = seeds.range(-s / 2, s / 2);
        let jitter = seeds.triple(16);
        on_square.push([t + jitter[0], 0, t + jitter[2]]);
        let k = seeds.range(0, 16);
        let base = along(p, q, k);
        on_tilted.push([0, 1, 2].map(|i| base[i] + jitter[i]));
        on_ridge.push([jitter[0], ONE, t]);
    }
    let mut checked = check_no_self_hits(&square, &on_square, [ONE / 3, 3 * ONE, ONE / 5], 1);
    checked += check_no_self_hits(&tilted, &on_tilted, [ONE / 2, 3 * ONE / 2, ONE], 2);
    let duplicate = [square[0], [square[0][0], square[0][2], square[0][1]]];
    checked += check_no_self_hits(&duplicate, &on_square, [-ONE / 3, 2 * ONE, ONE / 9], 3);
    checked += check_no_self_hits(&ridge, &on_ridge, [ONE / 9, 2 * ONE, 0], 4);
    assert!(checked >= 700, "only {checked} hits checked");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_054_nearby_surfaces_are_not_skipped() {
    // A cover 2^-20 units (4 096 raw) above the ground, starting at x = 0. A ground point 2^-17
    // to its left sees the sun toward (1, 2^-4, 0) through the cover: the shadow ray reaches
    // the cover's height at x = 2^-17 > 0. A rule with a distance epsilon of 2^-10 would miss
    // it. A cover only 4 raw units above the ground still shadows a point under it.
    let s = 8 * ONE;
    let ground = [[-s, 0, s], [s, 0, s], [0, 0, -s]];
    let cover = |height: i64| [[0, height, -s], [0, height, s], [s, height, 0]];
    for (height, x, toward) in [(1 << 12, -(1 << 15), [ONE, 1 << 28, 0]), (4, ONE, [0, ONE, 0])] {
        let scene = [ground, cover(height)].map(|v| trace_triangle(v).expect("in the domain"));
        let surface = TriangleSurface::new(&scene[0]).expect("not degenerate");
        // A camera ray from between the two surfaces, straight down onto the ground.
        let down = TraceRay::new(vec3([x, height / 2, 0]), vec3([0, -ONE, 0])).expect("ray");
        let hit = down.hit_triangle(&scene[0], fixed(0), MAX).expect("hits the ground");
        let shading = SurfaceHit::new(&surface, &down, &hit).expect("in the domain");
        assert!(shading.point().y.raw() < height, "height {height}: start {:?}", shading.point());
        let light = DirectionalLight::new(vec3(toward), [FixedQ32_32::ONE; 3]).expect("light");
        let shadow = *shading.light_sample(&light).shadow_ray().expect("light above the ground");
        assert_eq!(hits(&scene, &shadow, SHADOW_T_MAX), vec![1], "height {height}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_051_the_shadow_limit_covers_the_trace_domain() {
    // From one corner of the trace domain toward the opposite corner, with a direction whose
    // largest component is 1.0: the occluder is just below 512 units away, inside SHADOW_T_MAX
    // (2^41 raw), and a limit of half that misses it.
    let max = TRACE_COORD_LIMIT_RAW - 1;
    let far = [[max, -max, -max], [max, max, -max], [max, 0, max]];
    let occluder = trace_triangle(far).expect("in the domain");
    let origin = vec3([-max, 0, 0]);
    for toward in [[ONE, 0, 0], [ONE, ONE / 1_024, -ONE / 2_048]] {
        let ray = TraceRay::new(origin, vec3(toward)).expect("ray");
        let hit = ray.hit_triangle(&occluder, fixed(0), SHADOW_T_MAX).expect("within the limit");
        assert!(hit.distance.raw() > (1 << 41) - 4 * ONE, "{toward:?}: {}", hit.distance.raw());
        let half = fixed(SHADOW_T_MAX.raw() / 2);
        assert!(ray.hit_triangle(&occluder, fixed(0), half).is_none());
    }
    assert_eq!(SHADOW_T_MAX.raw(), 1 << 41);
}
