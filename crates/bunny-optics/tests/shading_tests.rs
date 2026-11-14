//! Golden vectors for `bunny-diffuse-shading/1`: triangle normals and frames, cosine-weighted
//! directions, surface hits and the facing rule, light terms, refusals, and one composed path
//! per case of the reference order of operations. Values marked "model" come from the
//! independent Python reference model described in the test plan; the others are hand
//! computed in the comments beside them.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    Albedo, DirectionalLight, ShadingError, ShadingFrame, SkyLight, SurfaceHit, TriangleSurface,
    FRAC_1_PI, MAX_LIGHT_RAW, MIN_BOUNCE_COSINE_RAW, PI, SHADING_ID, SHADOW_T_MAX,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE / 2;
const QUARTER: i64 = ONE / 4;
const EIGHTH: i64 = ONE / 8;
const LAST: i64 = ONE - 1;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const ZERO3: [FixedQ32_32; 3] = [FixedQ32_32::ZERO; 3];

type Raw3 = [i64; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn rgb(raw: Raw3) -> [FixedQ32_32; 3] {
    raw.map(fixed)
}

fn rgb_raw(v: [FixedQ32_32; 3]) -> Raw3 {
    v.map(FixedQ32_32::raw)
}

/// `whole` units, raw.
const fn u(whole: i64) -> i64 {
    whole * ONE
}

/// The nearest raw value to `numerator / denominator` units, for golden inputs.
const fn ratio(numerator: i64, denominator: i64) -> i64 {
    (2 * numerator * ONE + denominator) / (2 * denominator)
}

fn triangle(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the trace domain")
}

fn surface(v: [Raw3; 3]) -> TriangleSurface {
    TriangleSurface::new(&triangle(v)).expect("not degenerate")
}

fn ray(origin: Raw3, direction: Raw3) -> TraceRay {
    TraceRay::new(vec3(origin), vec3(direction)).expect("a valid trace ray")
}

fn frame_raw(frame: &ShadingFrame) -> [Raw3; 3] {
    [raw3(frame.normal()), raw3(frame.tangent()), raw3(frame.bitangent())]
}

/// The ground: a triangle in the plane `y = 0` whose front face looks up (`+Y`).
const GROUND: [Raw3; 3] = [[u(-8), 0, u(8)], [u(8), 0, u(8)], [0, 0, u(-8)]];
/// A wall in the plane `x = 2` whose front face looks toward `-X`.
const WALL: [Raw3; 3] = [[u(2), 0, u(-8)], [u(2), 0, u(8)], [u(2), u(4), 0]];
/// A small horizontal occluder half a unit above the ground.
const OCCLUDER: [Raw3; 3] =
    [[ratio(1, 5), HALF, 0], [ratio(4, 5), HALF, 0], [HALF, HALF, ratio(3, 5)]];
const DIAGONAL: [Raw3; 3] = [[ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]];
const TILTED_DOWN: [Raw3; 3] = [[0, 0, 0], [ONE, 3 * QUARTER, -QUARTER], [QUARTER, -ONE, -HALF]];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_040_golden_surfaces_lock_normals_and_frames() {
    // (vertices, normal, tangent, bitangent). Axis-aligned cases are exact; for +Y the
    // frame is s = 1, q(y y) = 1: tangent (1, 0, 0), bitangent (0, 1 - 1, -1).
    let max = TRACE_COORD_LIMIT_RAW - 1;
    let cases: [([Raw3; 3], [Raw3; 3]); 10] = [
        (GROUND, [[0, ONE, 0], [ONE, 0, 0], [0, 0, -ONE]]),
        ([GROUND[0], GROUND[2], GROUND[1]], [[0, -ONE, 0], [ONE, 0, 0], [0, 0, ONE]]),
        ([[0; 3], [ONE, 0, 0], [0, ONE, 0]], [[0, 0, ONE], [ONE, 0, 0], [0, ONE, 0]]),
        ([[0; 3], [0, ONE, 0], [ONE, 0, 0]], [[0, 0, -ONE], [ONE, 0, 0], [0, -ONE, 0]]),
        ([[0; 3], [0, ONE, 0], [0, 0, ONE]], [[ONE, 0, 0], [0, 0, -ONE], [0, ONE, 0]]),
        // model: (1, 1, 1) / sqrt(3) and its frame.
        (
            DIAGONAL,
            [
                [2_479_700_525; 3],
                [3_387_333_910, -907_633_386, -2_479_700_525],
                [-907_633_386, 3_387_333_910, -2_479_700_525],
            ],
        ),
        // Edges one raw unit long: the cross product is one raw-squared unit.
        ([[0; 3], [1, 0, 0], [0, 1, 0]], [[0, 0, ONE], [ONE, 0, 0], [0, ONE, 0]]),
        // model: the corners of the trace domain, normal (-1, -1, 1) / sqrt(3).
        (
            [[-max; 3], [max, -max, max], [-max, max, max]],
            [
                [-2_479_700_525, -2_479_700_525, 2_479_700_525],
                [3_387_333_910, -907_633_386, 2_479_700_525],
                [-907_633_386, 3_387_333_910, 2_479_700_525],
            ],
        ),
        // model: a sliver 255 units long and 3 raw units wide.
        (
            [[0; 3], [255 * ONE, 1, 0], [0, 3, 1]],
            [[0, -1_358_187_913, 4_074_563_739], [ONE, 0, 0], [0, 4_074_563_739, 1_358_187_913]],
        ),
        // model: a normal with negative z, the other branch of the frame's sign.
        (
            TILTED_DOWN,
            [
                [-1_901_843_485, 1_331_290_440, -3_613_502_622],
                [3_837_608_454, 320_151_189, -1_901_843_485],
                [-320_151_189, -4_070_861_463, -1_331_290_440],
            ],
        ),
    ];
    for (index, (vertices, expected)) in cases.iter().enumerate() {
        let surface = surface(*vertices);
        assert_eq!(frame_raw(&surface.frame()), *expected, "case {index}");
        assert_eq!(surface.facing_frame(true), surface.frame(), "case {index}");
        assert_eq!(surface.facing_frame(false), surface.frame().flipped(), "case {index}");
    }
    let flipped = surface(GROUND).frame().flipped();
    assert_eq!(frame_raw(&flipped), [[0, -ONE, 0], [0, 0, -ONE], [ONE, 0, 0]]);
    // Zero-area triangles: collinear, duplicate and all-equal vertices.
    for degenerate in [
        [[0; 3], [ONE; 3], [2 * ONE; 3]],
        [[ONE, 2 * ONE, 3 * ONE], [ONE, 2 * ONE, 3 * ONE], [4 * ONE, 0, 0]],
        [[7; 3]; 3],
    ] {
        assert_eq!(TriangleSurface::new(&triangle(degenerate)), None, "{degenerate:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_043_golden_directions_lock_the_hemisphere_map() {
    // (u, v) and the angle's exact sine and cosine from FixedAngle's golden table:
    // v = 1/8 turn has sin = cos = 3 037 000 500.
    let samples = [
        (0, 0),
        (HALF, 0),
        (3 * QUARTER, QUARTER),
        (LAST, HALF),
        (HALF, EIGHTH),
        (QUARTER, 7 * EIGHTH),
        (LAST, 3 * QUARTER),
    ];
    // Around +Z: u = 0 is the normal; u = 1/2 gives floor(sqrt(1/2)) = 3 037 000 499 on both
    // the disk radius and the height; u = 3/4 gives radius floor(sqrt(3) * 2^31) = 3 719 550 786
    // and height exactly one half; u = 1 - 2^-32 gives height sqrt(2^-32) = 2^-16, raw 65 536,
    // the lowest possible; at v = 1/8 each disk component is
    // round(3 037 000 499 * 3 037 000 500 / 2^32) = 2 147 483 647.
    let plus_z: [Raw3; 7] = [
        [0, 0, ONE],
        [3_037_000_499, 0, 3_037_000_499],
        [0, 3_719_550_786, 2_147_483_648],
        [-4_294_967_295, 0, 65_536],
        [2_147_483_647, 2_147_483_647, 3_037_000_499],
        [1_518_500_250, -1_518_500_250, 3_719_550_786],
        [0, -4_294_967_295, 65_536],
    ];
    // model: the diagonal and tilted frames; the downward ground frame and the flipped +Z
    // frame are the +Z values permuted and negated.
    let diagonal: [Raw3; 7] = [
        [2_479_700_525; 3],
        [4_148_619_833, 1_111_619_334, 0],
        [453_816_693, 4_173_367_479, -907_633_385],
        [-3_387_296_072, 907_671_223, 2_479_738_362],
        [2_993_263_318, 2_993_263_318, -726_287_468],
        [3_665_983_898, 628_983_398, 2_147_483_648],
        [907_671_223, -3_387_296_072, 2_479_738_362],
    ];
    let tilted: [Raw3; 7] = [
        [-1_901_843_485, 1_331_290_440, -3_613_502_622],
        [1_368_792_536, 1_167_745_574, -3_899_938_632],
        [-1_228_180_805, -2_859_824_222, -2_959_682_652],
        [-3_837_637_473, -320_130_875, 1_901_788_347],
        [413_922_207, -933_990_639, -4_171_699_169],
        [-177_054_753, 2_705_388_752, -3_331_106_030],
        [320_122_169, 4_070_881_776, 1_331_235_302],
    ];
    let ground_down: [Raw3; 7] = plus_z.map(|[x, y, z]| [x, -z, y]);
    let flipped_z: [Raw3; 7] = plus_z.map(|[x, y, z]| [y, x, -z]);
    let plus_z_frame = surface([[0; 3], [ONE, 0, 0], [0, ONE, 0]]).frame();
    let frames = [
        (plus_z_frame, plus_z),
        (surface(DIAGONAL).frame(), diagonal),
        (surface([GROUND[0], GROUND[2], GROUND[1]]).frame(), ground_down),
        (plus_z_frame.flipped(), flipped_z),
        (surface(TILTED_DOWN).frame(), tilted),
    ];
    for (index, (frame, expected)) in frames.iter().enumerate() {
        for ((u, v), want) in samples.iter().zip(expected) {
            let direction = frame.cosine_direction([fixed(*u), fixed(*v)]).map(raw3);
            assert_eq!(direction, Some(*want), "frame {index}, u = {u}, v = {v}");
        }
    }
    // Coordinates outside [0, 1) are refused.
    for (u, v) in [(ONE, 0), (-1, 0), (0, ONE), (0, -1), (i64::MIN, i64::MAX)] {
        assert_eq!(plus_z_frame.cosine_direction([fixed(u), fixed(v)]), None, "u = {u}, v = {v}");
    }
    assert_eq!(MIN_BOUNCE_COSINE_RAW, 1 << 15);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_046_golden_surface_hits_lock_facing_and_start_points() {
    // (vertices, origin, direction, front face, start point, frame normal).
    // From above, the ground point (0, 0, 0) is exactly on the plane: one step up the dominant
    // axis puts it strictly above, at y = 1 raw. From below the same point steps down and the
    // frame flips. The oblique ray's floored point also lands on the plane and steps once; the
    // diagonal hits are strictly in front already (model).
    let cases: [([Raw3; 3], Raw3, Raw3, bool, Raw3, Raw3); 6] = [
        (GROUND, [0, u(2), 0], [0, -ONE, 0], true, [0, 1, 0], [0, ONE, 0]),
        (GROUND, [0, u(-2), 0], [0, ONE, 0], false, [0, -1, 0], [0, -ONE, 0]),
        (
            GROUND,
            [ONE / 3, ONE, ONE / 7],
            [HALF, -ONE, QUARTER],
            true,
            [3_579_139_413, 1, 1_687_308_580],
            [0, ONE, 0],
        ),
        (DIAGONAL, [0; 3], [ONE; 3], false, [1_431_655_765; 3], [-2_479_700_525; 3]),
        (
            DIAGONAL,
            [ONE / 5, ONE / 3, ONE / 7],
            [ONE, HALF, ONE / 3],
            false,
            [1_617_585_085, 1_810_951_578, 866_430_631],
            [-2_479_700_525; 3],
        ),
        (
            TILTED_DOWN,
            [ratio(2, 5), -ratio(2, 25), u(-3)],
            [0, 0, ONE],
            true,
            [1_717_986_918, -343_597_384, -1_030_792_151],
            [-1_901_843_485, 1_331_290_440, -3_613_502_622],
        ),
    ];
    for (index, (vertices, origin, direction, front, point, normal)) in cases.iter().enumerate() {
        let (surface, ray) = (surface(*vertices), ray(*origin, *direction));
        let hit = ray.hit_triangle(&triangle(*vertices), fixed(0), MAX).expect("hits");
        assert_eq!(hit.front_face, *front, "case {index}");
        let shading = SurfaceHit::new(&surface, &ray, &hit).expect("in the domain");
        assert_eq!(raw3(shading.point()), *point, "case {index}");
        assert_eq!(raw3(shading.frame().normal()), *normal, "case {index}");
        assert_eq!(shading.frame(), surface.facing_frame(*front), "case {index}");
    }
    // A start point stepped out of the trace domain is refused: the plane 2x + 4y = 4Y touches
    // y = Y, the top of the domain, at x = 0, and the ray along that line arrives exactly
    // there; the step goes up the dominant axis, to y = 2^40.
    let top = TRACE_COORD_LIMIT_RAW - 1;
    let edge = [[0, top, -ONE], [0, top, ONE], [2 * ONE, top - ONE, 0]];
    let ray = ray([2 * ONE, top, 0], [-ONE, 0, 0]);
    let hit = ray.hit_triangle(&triangle(edge), fixed(0), MAX).expect("hits the edge");
    assert_eq!(SurfaceHit::new(&surface(edge), &ray, &hit), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_048_golden_light_terms_lock_the_radiometric_convention() {
    assert_eq!(PI.raw(), 13_493_037_705); // round(pi * 2^32), pi * 2^32 = 13 493 037 704.52
    assert_eq!(FRAC_1_PI.raw(), 1_367_130_551); // round(2^32 / pi) = round(1 367 130 551.15)
    let irradiance = rgb([u(3), 3 * HALF, MAX_LIGHT_RAW]);
    // (toward light, surface normal, cosine, reflected radiance). Straight above: cosine 1,
    // weight FRAC_1_PI, and 3 * 1 367 130 551 = 4 101 391 653; 1.5 * w rounds 2 050 695 826.5
    // to even; 65536 * w = 1 367 130 551 * 2^16 exactly. The rest are model values.
    let up = [0, ONE, 0];
    let diagonal = [2_479_700_525; 3];
    let cases: [(Raw3, Raw3, i64, Raw3); 10] = [
        ([0, ONE, 0], up, ONE, [4_101_391_653, 2_050_695_826, 89_596_267_790_336]),
        ([0, ONE, 0], diagonal, 2_479_700_525, [2_367_939_576, 1_183_969_788, 51_728_429_350_912]),
        ([ONE, ONE, 0], up, 3_037_000_500, [2_900_121_849, 1_450_060_924, 63_354_128_498_688]),
        (
            [ONE, ONE, 0],
            diagonal,
            3_506_826_113,
            [3_348_772_263, 1_674_386_132, 73_155_046_342_656],
        ),
        (
            [HALF, ONE, QUARTER],
            up,
            3_748_954_808,
            [3_579_988_134, 1_789_994_067, 78_206_034_116_608],
        ),
        (
            [HALF, ONE, QUARTER],
            diagonal,
            3_787_805_119,
            [3_617_087_448, 1_808_543_724, 79_016_480_997_376],
        ),
        // Grazing: 2^-20 above the horizon.
        ([ONE, 1 << 12, 0], up, 4_096, [3_912, 1_956, 85_458_944]),
        // Along the surface, and below it: clamped to zero.
        ([ONE, 0, 0], up, 0, [0; 3]),
        ([-ONE, -ONE, ONE], up, 0, [0; 3]),
        ([-ONE, -ONE, ONE], diagonal, 0, [0; 3]),
    ];
    for (index, (toward, normal, cosine, radiance)) in cases.iter().enumerate() {
        let light = DirectionalLight::new(vec3(*toward), irradiance).expect("valid light");
        let actual = light.cosine(vec3(*normal));
        assert_eq!(actual.raw(), *cosine, "case {index}");
        assert_eq!(rgb_raw(light.reflected_radiance(actual)), *radiance, "case {index}");
    }
    // The light sample on the ground: a shadow ray from the start point along toward_light
    // when the light is strictly above the plane, none (and no light) along or below it.
    let ground = surface(GROUND);
    let down = ray([0, u(2), 0], [0, -ONE, 0]);
    let hit = down.hit_triangle(&triangle(GROUND), fixed(0), MAX).expect("hits");
    let shading = SurfaceHit::new(&ground, &down, &hit).expect("in the domain");
    let above = DirectionalLight::new(vec3([ONE, 1, 0]), irradiance).expect("valid");
    let sample = shading.light_sample(&above);
    let shadow = sample.shadow_ray().expect("the light is above the plane");
    assert_eq!((raw3(shadow.origin()), raw3(shadow.direction())), ([0, 1, 0], [ONE, 1, 0]));
    // The cosine is 2^-32 exactly and rounds the weight to zero: a shadow ray but no light.
    assert_eq!(sample.radiance(true), ZERO3);
    let straight = shading.light_sample(&DirectionalLight::new(vec3(up), irradiance).expect("ok"));
    assert_eq!(
        rgb_raw(straight.radiance(true)),
        [4_101_391_653, 2_050_695_826, 89_596_267_790_336]
    );
    assert_eq!(straight.radiance(false), ZERO3);
    for toward in [[ONE, 0, 0], [ONE, -1, 0], [0, -ONE, 0]] {
        let light = DirectionalLight::new(vec3(toward), irradiance).expect("valid");
        let sample = shading.light_sample(&light);
        assert_eq!((sample.shadow_ray(), sample.radiance(true)), (None, ZERO3), "{toward:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_052_lights_and_albedo_outside_the_domain_are_refused() {
    let white = rgb([ONE; 3]);
    let refused = |toward: Raw3| DirectionalLight::new(vec3(toward), white).err();
    assert_eq!(refused([ONE - 1, 0, 0]), Some(ShadingError::LightNotNormalized));
    assert_eq!(refused([ONE + 1, 0, 0]), Some(ShadingError::LightNotNormalized));
    assert_eq!(refused([0; 3]), Some(ShadingError::LightNotNormalized));
    assert_eq!(refused([HALF, HALF, HALF]), Some(ShadingError::LightNotNormalized));
    for accepted in [[ONE; 3], [-ONE, 0, 0], [1, -ONE, -1], [ONE, -ONE, ONE]] {
        assert_eq!(refused(accepted), None, "{accepted:?}");
    }
    let light: fn(Raw3) -> Option<ShadingError> =
        |e| DirectionalLight::new(vec3([0, ONE, 0]), rgb(e)).err();
    let sky: fn(Raw3) -> Option<ShadingError> = |e| SkyLight::new(rgb(e)).err();
    for check in [light, sky] {
        assert_eq!(check([0, -1, 0]), Some(ShadingError::NegativeRadiance));
        assert_eq!(check([MAX_LIGHT_RAW + 1, 0, 0]), Some(ShadingError::RadianceTooLarge));
        assert_eq!(check([-1, 0, MAX_LIGHT_RAW + 1]), Some(ShadingError::NegativeRadiance));
        assert_eq!(check([0, MAX_LIGHT_RAW, 0]), None);
    }
    assert_eq!(MAX_LIGHT_RAW, 65_536 * ONE);
    let albedo = |a: Raw3| Albedo::new(rgb(a)).err();
    assert_eq!(albedo([-1, 0, 0]), Some(ShadingError::AlbedoOutOfRange));
    assert_eq!(albedo([0, ONE + 1, 0]), Some(ShadingError::AlbedoOutOfRange));
    assert_eq!(albedo([0, ONE, HALF]), None);
    // The reflection: albedo * (direct + indirect), the sum exact and the product rounded to
    // nearest with ties to even; an unrepresentable sum is refused.
    let half = Albedo::new(rgb([HALF; 3])).expect("valid");
    let direct = rgb([4_101_391_653, 1, 3]);
    let sum = half.reflect(direct, rgb([1_073_741_824, 0, 0])).map(rgb_raw);
    // (4 101 391 653 + 2^30) / 2 = 2 587 566 738.5 -> even; 1 / 2 -> 0; 3 / 2 -> 2.
    assert_eq!(sum, Some([2_587_566_738, 0, 2]));
    assert_eq!(half.reflect(rgb([i64::MAX, 0, 0]), rgb([1, 0, 0])), None);
    assert_eq!(Albedo::new(white).expect("ok").reflect(white, ZERO3), Some(white));
}

/// The reference composition of one path (`docs/topics/optics/README.md#order-of-operations`),
/// with brute-force closest-hit and any-hit queries over a handful of triangles.
struct Scene {
    triangles: Vec<TraceTriangle>,
    surfaces: Vec<TriangleSurface>,
    albedo: Vec<Albedo>,
    light: DirectionalLight,
    sky: SkyLight,
}

impl Scene {
    fn new(parts: &[([Raw3; 3], Raw3)], toward: Raw3) -> Self {
        let triangles: Vec<_> = parts.iter().map(|(v, _)| triangle(*v)).collect();
        Self {
            surfaces: parts.iter().map(|(v, _)| surface(*v)).collect(),
            albedo: parts.iter().map(|(_, a)| Albedo::new(rgb(*a)).expect("valid")).collect(),
            triangles,
            light: DirectionalLight::new(vec3(toward), rgb([u(3); 3])).expect("valid"),
            sky: SkyLight::new(rgb([1_288_490_189, 1_717_986_918, 2_576_980_378])).expect("valid"),
        }
    }

    /// The lowest-index nearest hit, as `closest_hit` breaks ties.
    fn closest(&self, ray: &TraceRay) -> Option<(usize, SurfaceHit)> {
        let hits = self.triangles.iter().enumerate();
        let (index, hit) = hits
            .filter_map(|(i, t)| ray.hit_triangle(t, fixed(0), MAX).map(|hit| (i, hit)))
            .min_by_key(|(i, hit)| (hit.distance, *i))?;
        Some((index, SurfaceHit::new(&self.surfaces[index], ray, &hit).expect("in the domain")))
    }

    fn visible(&self, ray: &TraceRay) -> bool {
        !self.triangles.iter().any(|t| ray.hit_triangle(t, fixed(0), SHADOW_T_MAX).is_some())
    }

    fn direct(&self, hit: &SurfaceHit) -> [FixedQ32_32; 3] {
        let sample = hit.light_sample(&self.light);
        sample.radiance(sample.shadow_ray().is_some_and(|ray| self.visible(ray)))
    }

    fn radiance(&self, camera: &TraceRay, bounce: [i64; 2]) -> [FixedQ32_32; 3] {
        let Some((first, hit)) = self.closest(camera) else {
            return self.sky.radiance();
        };
        let direct = self.direct(&hit);
        let bounce_ray = hit.bounce_ray(bounce.map(fixed)).expect("a unit-square point");
        let indirect = match self.closest(&bounce_ray) {
            None => self.sky.radiance(),
            Some((second, next)) => {
                self.albedo[second].reflect(self.direct(&next), ZERO3).expect("bounded")
            }
        };
        self.albedo[first].reflect(direct, indirect).expect("bounded")
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_053_composed_paths_lock_the_order_of_operations() {
    assert_eq!(SHADING_ID, "bunny-diffuse-shading/1");
    let grey = [HALF; 3];
    let (sky_r, sky_g, sky_b) = (1_288_490_189, 1_717_986_918, 2_576_980_378);
    // Hand checked: the sun straight above gives 3 * FRAC_1_PI = 4 101 391 653 per channel; the
    // bounce with u = 0 goes straight up and sees the sky; L = 0.5 * (direct + sky) with ties to
    // even: 5 389 881 842 / 2, 5 819 378 571 / 2 -> 2 909 689 286, 6 678 372 031 / 2 ->
    // 3 339 186 016.
    let plain = Scene::new(&[(GROUND, grey)], [0, ONE, 0]);
    let down = ray([0, u(2), 0], [0, -ONE, 0]);
    let expected = [2_694_940_921, 2_909_689_286, 3_339_186_016];
    assert_eq!(rgb_raw(plain.radiance(&down, [0, 0])), expected);
    assert_eq!(2 * expected[0], 4_101_391_653 + sky_r);
    // model: the sun toward (-1, 1, 1/2) lights the ground and a wall; the bounce at
    // u = 1/2, v = 0 leaves at 45 degrees toward +X and hits the lit wall.
    let ground = (GROUND, [ratio(7, 10), HALF, ratio(3, 10)]);
    let wall = (WALL, [ratio(9, 10); 3]);
    let toward = [-ONE, ONE, HALF];
    let lit = Scene::new(&[ground, wall], toward);
    let down = ray([ONE, u(2), 0], [0, -ONE, 0]);
    assert_eq!(
        rgb_raw(lit.radiance(&down, [HALF, 0])),
        [3_636_567_264, 2_597_548_046, 1_558_528_828]
    );
    // model: an occluder above the ground point blocks the sun; the bounce still sees the wall.
    let shadowed = Scene::new(&[ground, wall, (OCCLUDER, [ratio(1, 5); 3])], toward);
    assert_eq!(
        rgb_raw(shadowed.radiance(&down, [HALF, 0])),
        [1_722_584_494, 1_230_417_496, 738_250_497]
    );
    // A camera ray that hits nothing sees the sky.
    let up = ray([ONE, u(2), 0], [0, ONE, 0]);
    assert_eq!(rgb_raw(shadowed.radiance(&up, [HALF, 0])), [sky_r, sky_g, sky_b]);
}
