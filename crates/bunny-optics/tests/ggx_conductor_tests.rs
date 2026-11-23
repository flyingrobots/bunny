//! Evidence for `bunny-ggx-conductor/1`: hand-derived samples, the roughness range and the
//! refusals, absorption as a result, unit directions above the surface, the weight against an
//! interval oracle of `F G2 / G1`, the direction against Heitz's listing at 60 fractional bits,
//! the mirror limit at the minimum roughness, a white-furnace table, and the shared `reflect`.
//! The rounding of each private step is pinned by the unit tests beside the sources.
//!
//! No floating point. The oracles are integer arithmetic written here and in `ggx_reference`
//! from the chapter's formulas. Values marked "port" come from an integer Python port of the
//! chapter's steps; values marked "quadrature" from an integer quadrature of the reflectance
//! integral that does not use the sampler (`docs/topics/optics/test-plan.md`).

mod exact;
mod ggx_reference;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use bunny_optics::{
    reflect, schlick_fresnel, Albedo, GgxConductor, GgxScatter, SecondaryRayError, SurfaceHit,
    TriangleSurface, GGX_CONDUCTOR_ID, MIN_GGX_ROUGHNESS_RAW,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

use exact::{within, Signed};
use ggx_reference::{heitz, ratio_bounds, ModelInput, LIFT};

const ONE: i64 = ONE_RAW;
const WIDE_ONE: i128 = 1 << 32;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const GGX_TEST_SEED: u64 = 0xB002_0371_66C0_0000;
/// `2^32 sqrt(3) / 2` rounded: `3 719 550 786.6...`.
const HALF_SQRT3: i64 = 3_719_550_787;

type Raw3 = [i64; 3];
type Wide3 = [i128; 3];

/// The ground: a triangle in the plane `y = 0` whose front face looks up (`+Y`). Its frame is
/// tangent `+X`, bitangent `-Z`, normal `+Y`, exactly.
const GROUND: [Raw3; 3] = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];
/// Polished gold's base reflectance, as in the mirror's tests, with a red channel of 1.0.
const GOLD: Raw3 = [ONE, 3_694_871_552, 1_417_339_208];

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

fn ray(origin: Raw3, direction: Raw3) -> TraceRay {
    TraceRay::new(vec3(origin), vec3(direction)).expect("a valid trace ray")
}

fn conductor(roughness: i64, f0: Raw3) -> GgxConductor {
    let reflectance = Albedo::new(f0.map(fixed)).expect("channels in 0..=1");
    GgxConductor::new(fixed(roughness), reflectance).expect("a roughness in range")
}

/// The hit of `ray` on the triangle `v`, if the trace kernel reports one.
fn hit_of(v: [Raw3; 3], ray: &TraceRay) -> Option<SurfaceHit> {
    let triangle = TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2]))?;
    let surface = TriangleSurface::new(&triangle)?;
    let hit = ray.hit_triangle(&triangle, fixed(0), MAX)?;
    SurfaceHit::new(&surface, ray, &hit)
}

/// The hit of a ray that reaches the ground's origin along `direction`.
fn ground_hit(direction: Raw3) -> (SurfaceHit, TraceRay) {
    let incoming = ray(direction.map(|c| -2 * c), direction);
    (hit_of(GROUND, &incoming).expect("hits the ground"), incoming)
}

/// The sample's direction and throughput, raw, or `None` when it is absorbed. Also checks that
/// the `Option` form agrees with the `Result` form.
fn scatter(
    hit: &SurfaceHit,
    incoming: &TraceRay,
    metal: &GgxConductor,
    sample: [i64; 2],
) -> Option<(Raw3, Raw3)> {
    let sample = sample.map(fixed);
    let result = hit.try_ggx_reflection(incoming, metal, sample).expect("an accepted sample");
    assert_eq!(hit.ggx_reflection(incoming, metal, sample), Some(result));
    match result {
        GgxScatter::Reflected(reflected) => {
            assert_eq!(reflected.ray().origin(), hit.point());
            Some((raw3(reflected.ray().direction()), reflected.throughput().map(FixedQ32_32::raw)))
        }
        GgxScatter::Absorbed => None,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_140_hand_derived_samples_follow_the_visible_normal() {
    assert_eq!(GGX_CONDUCTOR_ID, "bunny-ggx-conductor/1");
    let (hit, down) = ground_hit([0, -ONE, 0]);
    let (quarter, half) = (ONE / 4, ONE / 2);
    // The disk's centre at normal incidence: the facet is the normal for every roughness, so
    // the sample is the mirror, G2 / G1 = 1 and the throughput is F0 exactly.
    for roughness in [MIN_GGX_ROUGHNESS_RAW, quarter, ONE] {
        let sample = scatter(&hit, &down, &conductor(roughness, GOLD), [0, 0]);
        assert_eq!(sample, Some(([0, ONE, 0], GOLD)));
    }
    // Roughness 1.0, u = (1/4, 1/4): the hemisphere point is (0, 1/2, sqrt(3)/2) on (tangent,
    // bitangent, normal), which is the facet; the view reflects to (0, sqrt(3)/2, 1/2) there,
    // world (0, 1/2, -sqrt(3)/2). With c_v = 1, c_l = 1/2 and s = 1: G2 / G1 = 2/3, raw
    // 2 863 311 530.67. The other channels are schlick(F0, sqrt(3)/2) * 2/3 (port).
    let sample = scatter(&hit, &down, &conductor(ONE, GOLD), [quarter, quarter]);
    let expected = ([0, half, -HALF_SQRT3], [2_863_311_531, 2_463_264_970, 944_975_610]);
    assert_eq!(sample, Some(expected));
    // Roughness 1/2, u = (1/2, 0) and (1/2, 1/2): the facet is along (+-1/2, 0, 1), so the
    // direction is (+-4/5, 0, 3/5) there, world (+-4/5, 3/5, 0): raw 3 435 973 836.8 and
    // 2 576 980 377.6. G2 / G1 = 1.2 / (0.6 + sqrt(0.52)) = 0.908327..., raw 3 901 234 387
    // (port).
    for (turn, sign) in [(0, 1), (half, -1)] {
        let sample = scatter(&hit, &down, &conductor(half, GOLD), [half, turn]);
        let (direction, throughput) = sample.expect("a reflected sample");
        assert_eq!(direction, [sign * 3_435_973_836, 2_576_980_379, 0], "port");
        assert!((direction[0] - sign * 3_435_973_837).abs() <= 2, "4/5: {direction:?}");
        assert!((direction[1] - 2_576_980_378).abs() <= 2, "3/5: {direction:?}");
        assert_eq!(throughput, [3_901_234_387, 3_356_158_421, 1_287_441_627], "port");
    }
    // An oblique view: roughness 1/2, 45 degrees, u = (1/2, 1/8) (port).
    let (hit, slant) = ground_hit([ONE, -ONE, 0]);
    let sample = scatter(&hit, &slant, &conductor(half, GOLD), [half, ONE / 8]);
    let direction = [3_494_730_689, 2_010_201_179, 1_480_774_360];
    assert_eq!(sample, Some((direction, [3_648_589_617, 3_143_137_861, 1_224_806_103])));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_141_roughness_is_accepted_in_its_closed_range_only() {
    assert_eq!(MIN_GGX_ROUGHNESS_RAW, 1 << 16);
    let white = Albedo::new([fixed(ONE); 3]).expect("white");
    for raw in [MIN_GGX_ROUGHNESS_RAW, MIN_GGX_ROUGHNESS_RAW + 1, ONE / 2, ONE - 1, ONE] {
        let metal = GgxConductor::new(fixed(raw), white).expect("in range");
        assert_eq!((metal.roughness().raw(), metal.reflectance()), (raw, white));
    }
    for raw in [i64::MIN, -ONE, -1, 0, 1, MIN_GGX_ROUGHNESS_RAW - 1, ONE + 1, i64::MAX] {
        let refusal = GgxConductor::new(fixed(raw), white).expect_err("out of range");
        assert_eq!(refusal.roughness().raw(), raw);
        let expected = format!("roughness raw {raw} is outside 65536..=4294967296");
        assert_eq!(refusal.to_string(), expected);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_142_a_bad_sample_and_a_ray_from_below_are_refused() {
    let (hit, down) = ground_hit([0, -ONE, 0]);
    let metal = conductor(ONE / 2, GOLD);
    for sample in [[-1, 0], [ONE, 0], [0, -1], [0, ONE], [ONE, ONE]] {
        let sample = sample.map(fixed);
        let expected = SecondaryRayError::SampleOutsideUnitInterval { sample };
        assert_eq!(hit.try_ggx_reflection(&down, &metal, sample), Err(expected));
        assert_eq!(hit.ggx_reflection(&down, &metal, sample), None);
    }
    // The largest representable sample is accepted.
    assert!(hit.try_ggx_reflection(&down, &metal, [fixed(ONE - 1); 2]).is_ok());
    // Up from below, and along the plane: d . N >= 0 for the face this hit faces.
    for d in [[0, ONE, 0], [ONE, 0, 0], [ONE, 1, -ONE]] {
        let incoming = ray([0, ONE, 0], d);
        let expected = SecondaryRayError::ArrivesBelowSurface { direction: vec3(d) };
        assert_eq!(hit.try_ggx_reflection(&incoming, &metal, [fixed(0); 2]), Err(expected));
        assert_eq!(hit.ggx_reflection(&incoming, &metal, [fixed(0); 2]), None);
    }
    // The sample is checked first.
    let both = hit.try_ggx_reflection(&ray([0, ONE, 0], [0, ONE, 0]), &metal, [fixed(ONE); 2]);
    assert!(matches!(both, Err(SecondaryRayError::SampleOutsideUnitInterval { .. })));
}

/// The grazing hit of the mirror's `OPTICS-TP-107`: the ray arrives from above the exact plane
/// (`d . N < 0`) but has `d . n > 0` against the rounded unit normal.
const GRAZING: [Raw3; 3] = [
    [-14_247_233_187, -16_518_692_768, 8_255_702_029],
    [14_994_902_859, -9_167_710_013, 11_680_992_187],
    [-2_466_095_447, 12_133_874_043, -15_050_646_404],
];

/// A triangle, a ray onto it and the direction the sampler's steps give for roughness 1/4 and
/// the sample `(3 940 717 691, 536 870 912)`: see `OPTICS-TP-143`.
const SKIMMING: [Raw3; 3] = [
    [-275_773_231_340, 157_612_746_986, -255_598_851_569],
    [-274_892_669_456, 156_963_921_614, -255_896_067_222],
    [-275_531_642_834, 157_018_898_567, -255_892_649_115],
];
const SKIMMING_ORIGIN: Raw3 = [-275_448_683_982, 157_172_721_299, -255_833_105_092];
const SKIMMING_DIRECTION: Raw3 = [3_142_686_984, 2_556_620_541, 1_426_167_593];
/// The same for the sample `(3 939 138 499, 1 073 741 824)` on another triangle.
const LEVEL: [Raw3; 3] = [
    [537_514_476_848, 255_741_212_457, -385_039_265_098],
    [527_554_986_546, 268_139_635_969, -387_588_900_864],
    [509_159_718_369, 282_352_449_570, -400_282_885_161],
];
const LEVEL_ORIGIN: Raw3 = [524_737_762_166, 268_692_495_923, -390_928_181_965];
const LEVEL_DIRECTION: Raw3 = [-2_919_729_111, 2_720_429_430, -1_587_825_464];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_143_a_direction_that_does_not_leave_the_surface_is_absorbed() {
    // Roughness 1.0 at normal incidence, u = (3/4, 1/4): the facet is (0, sqrt(3)/2, 1/2) on
    // (tangent, bitangent, normal), 60 degrees from the normal, so the view reflects to 120
    // degrees: below the surface.
    let (hit, down) = ground_hit([0, -ONE, 0]);
    let sample = [3 * (ONE / 4), ONE / 4].map(fixed);
    let result = hit.try_ggx_reflection(&down, &conductor(ONE, GOLD), sample);
    assert_eq!(result, Ok(GgxScatter::Absorbed));
    assert_eq!(
        hit.ggx_reflection(&down, &conductor(ONE, GOLD), sample),
        Some(GgxScatter::Absorbed)
    );
    // A view below the rounded normal's horizon: no facet is visible, for any sample.
    let d = [ONE, 441_121_301, 1_116_683_283];
    let grazing = ray([-4_867_775_887, -4_958_630_880, 511_999_321], d);
    let hit = hit_of(GRAZING, &grazing).expect("the kernel reports the hit");
    assert!(dot(d.map(i128::from), wide(hit.frame().normal())) > 0, "the case's premise");
    for sample in [[0, 0], [ONE / 2, ONE / 3], [ONE - 1, ONE - 1]] {
        let result = hit.try_ggx_reflection(&grazing, &conductor(ONE / 4, GOLD), sample.map(fixed));
        assert_eq!(result, Ok(GgxScatter::Absorbed));
    }
    // A direction above the rounded normal's horizon but not above the exact plane. Found by
    // a search with the plane test removed, where the kernel returned `SKIMMING_DIRECTION`:
    // its cosine against the frame normal rounds to at least one raw unit, yet it is along or
    // below the exact plane, so only the plane test absorbs it.
    let skimming = ray(SKIMMING_ORIGIN, [3_168_177_461, 1_651_269_774, 2_383_943_950]);
    let hit = hit_of(SKIMMING, &skimming).expect("the kernel reports the hit");
    let sub = |a: Raw3, b: Raw3| [0, 1, 2].map(|i| i128::from(a[i]) - i128::from(b[i]));
    let plane = cross(sub(SKIMMING[1], SKIMMING[0]), sub(SKIMMING[2], SKIMMING[0]));
    let facing = if dot(wide(skimming.direction()), plane) < 0 { plane } else { plane.map(|c| -c) };
    let direction = SKIMMING_DIRECTION.map(i128::from);
    assert!(round_raw(dot(direction, wide(hit.frame().normal()))) >= 1, "the case's premise");
    assert!(dot(direction, facing) <= 0, "the case's premise");
    let sample = [3_940_717_691, 536_870_912].map(fixed);
    let result = hit.try_ggx_reflection(&skimming, &conductor(ONE / 4, GOLD), sample);
    assert_eq!(result, Ok(GgxScatter::Absorbed));
    // The other way round: a direction above the exact plane whose cosine against the frame
    // normal rounds to zero. Found by a search with that test removed, where the kernel
    // returned `LEVEL_DIRECTION` with a throughput of zero.
    let level = ray(LEVEL_ORIGIN, [339_098_946, 3_323_951_545, -2_698_778_171]);
    let hit = hit_of(LEVEL, &level).expect("the kernel reports the hit");
    let plane = cross(sub(LEVEL[1], LEVEL[0]), sub(LEVEL[2], LEVEL[0]));
    let facing = if dot(wide(level.direction()), plane) < 0 { plane } else { plane.map(|c| -c) };
    let direction = LEVEL_DIRECTION.map(i128::from);
    assert_eq!(round_raw(dot(direction, wide(hit.frame().normal()))), 0, "the case's premise");
    assert!(dot(direction, facing) > 0, "the case's premise");
    let sample = [3_939_138_499, 1_073_741_824].map(fixed);
    let result = hit.try_ggx_reflection(&level, &conductor(ONE / 4, GOLD), sample);
    assert_eq!(result, Ok(GgxScatter::Absorbed));
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
        let span = u64::try_from(high - low + 1).expect("non-empty range");
        low + i64::try_from(self.next() % span).expect("below the span")
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    /// A raw value in `0..2^32`, a third of them within 64 raw units of an end.
    fn unit_raw(&mut self) -> i64 {
        match self.range(0, 2) {
            0 => self.range(0, 64),
            1 => ONE - 1 - self.range(0, 64),
            _ => self.range(0, ONE - 1),
        }
    }

    /// A raw base reflectance in `0..=2^32`.
    fn reflectance(&mut self) -> i64 {
        self.unit_raw() + self.range(0, 1)
    }

    /// A raw roughness in `2^16..=2^32`: a random bit length, and each end one time in eight.
    fn roughness(&mut self) -> i64 {
        match self.range(0, 7) {
            0 => MIN_GGX_ROUGHNESS_RAW,
            1 => ONE,
            _ => {
                let bits = self.range(16, 31);
                self.range(1 << bits, (1 << (bits + 1)) - 1)
            }
        }
    }

    /// A trace direction: a seeded vector with one axis set to exactly +-1.0.
    fn direction(&mut self) -> Raw3 {
        let mut d = self.triple(ONE);
        let axis = usize::try_from(self.range(0, 2)).expect("an axis");
        d[axis] = if self.range(0, 1) == 0 { ONE } else { -ONE };
        d
    }
}

/// The largest component magnitude of `v` scaled to exactly 1.0 (raw `2^32`), floored.
fn scaled_to_one(v: Wide3) -> Wide3 {
    let top = v.iter().map(|c| c.abs()).max().expect("three").max(1);
    v.map(|c| (c << 32) / top)
}

/// How a seeded ray meets its triangle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Any direction with one axis at exactly +-1.0.
    Free,
    /// Along an edge of the triangle, leaning toward the plane by a seeded small angle.
    Grazing,
    /// A free direction scaled down by a seeded power of two: a short trace direction.
    Short,
}

/// A seeded direction of the given shape for the triangle with edge `edge` and normal `plane`.
fn seeded_direction(seeds: &mut Seeds, shape: Shape, edge: Wide3, plane: Wide3) -> Raw3 {
    let narrow = |v: Wide3| v.map(|c| i64::try_from(c.clamp(-WIDE_ONE, WIDE_ONE)).expect("raw"));
    match shape {
        Shape::Free => seeds.direction(),
        Shape::Short => {
            let shift = seeds.range(4, 28);
            seeds.direction().map(|c| c >> shift)
        }
        Shape::Grazing => {
            let (along, normal) = (scaled_to_one(edge), scaled_to_one(plane));
            let bits = seeds.range(4, 28);
            let lean = i128::from(seeds.range(1, 1 << bits));
            let side = if seeds.range(0, 1) == 0 { 1 } else { -1 };
            narrow([0, 1, 2].map(|i| along[i] - side * ((normal[i] * lean) >> 32)))
        }
    }
}

/// A seeded scene: a triangle near a seeded centre and a ray that aims at a point inside it.
/// Returns the hit, the ray and the exact normal of the face the ray arrived on.
fn seeded_case(seeds: &mut Seeds) -> Option<(SurfaceHit, TraceRay, Wide3)> {
    let bits = seeds.range(4, 36);
    let centre = seeds.triple(TRACE_COORD_LIMIT_RAW / 2);
    let corner = |seeds: &mut Seeds| {
        let offset = seeds.triple(1 << bits);
        [0, 1, 2].map(|i| centre[i] + offset[i])
    };
    let v = [corner(seeds), corner(seeds), corner(seeds)];
    let weights = [0; 3].map(|_| i128::from(seeds.range(1, 1_000)));
    let target: Raw3 = [0, 1, 2].map(|i| {
        let sum: i128 = (0..3).map(|k| weights[k] * i128::from(v[k][i])).sum();
        i64::try_from(sum / weights.iter().sum::<i128>()).expect("inside the domain")
    });
    let sub = |a: Raw3, b: Raw3| [0, 1, 2].map(|i| i128::from(a[i]) - i128::from(b[i]));
    let plane = cross(sub(v[1], v[0]), sub(v[2], v[0]));
    let shape = [Shape::Free, Shape::Free, Shape::Grazing, Shape::Short]
        [usize::try_from(seeds.range(0, 3)).expect("a shape")];
    let d = seeded_direction(seeds, shape, sub(v[1], v[0]), plane);
    let reach = seeds.range(1, 1 << (bits + 2).min(38));
    let origin: Raw3 = [0, 1, 2]
        .map(|i| {
            let back = (scaled_to_one(d.map(i128::from))[i] * i128::from(reach)) >> 32;
            i64::try_from(i128::from(target[i]) - back).ok()
        })
        .into_iter()
        .collect::<Option<Vec<i64>>>()?
        .try_into()
        .ok()?;
    let ray = TraceRay::new(vec3(origin), vec3(d))?;
    let hit = hit_of(v, &ray)?;
    // The exact normal of the face the ray arrived on.
    let facing = if dot(wide(ray.direction()), plane) < 0 { plane } else { plane.map(|c| -c) };
    Some((hit, ray, facing))
}

/// `value / 2^32` rounded to nearest, ties to even.
fn round_raw(value: i128) -> i128 {
    let (floor, remainder) = (value >> 32, value & (WIDE_ONE - 1));
    floor + i128::from(remainder > WIDE_ONE / 2 || (remainder == WIDE_ONE / 2 && floor & 1 == 1))
}

/// The chapter's unit-vector rule for a trace direction: scaled by a power of two until its
/// largest component is in `[2^61, 2^62)`, then normalized by `FixedUnitVec3::new`.
fn unit_rule(direction: FixedVec3) -> Wide3 {
    let top = raw3(direction).map(i64::unsigned_abs).into_iter().max().expect("three");
    let shift = 61 - top.ilog2();
    let scaled = vec3(raw3(direction).map(|c| c << shift));
    wide(FixedUnitVec3::new(scaled).expect("a nonzero direction").into_inner())
}

/// What the seeded cases of one test reached, so that a generator that stops reaching the hard
/// cases fails the test instead of passing it quietly.
#[derive(Debug, Default)]
struct Reach {
    reflected: u32,
    absorbed: u32,
    /// View cosine below `2^-6`, and below `2^-16` (the minimum roughness).
    grazing: u32,
    very_grazing: u32,
    /// View cosine above `1 - 2^-6`.
    steep: u32,
    /// A direction whose largest component is below `2^-4`.
    short: u32,
    minimum_roughness: u32,
    maximum_roughness: u32,
    /// First sample coordinate within 64 raw units of 1.0.
    rim: u32,
}

impl Reach {
    /// Counts one case: its hit and ray, and its `(roughness, u)`.
    fn note(&mut self, hit: &SurfaceHit, incoming: &TraceRay, parameters: (i64, i64)) {
        let (roughness, u) = parameters;
        let d = incoming.direction();
        let cosine = round_raw(-dot(unit_rule(d), wide(hit.frame().normal())));
        self.grazing += u32::from(cosine < 1 << 26);
        self.very_grazing += u32::from(cosine < 1 << 16);
        self.steep += u32::from(cosine > WIDE_ONE - (1 << 26));
        self.short += u32::from(raw3(d).iter().all(|c| c.abs() < 1 << 28));
        self.minimum_roughness += u32::from(roughness == MIN_GGX_ROUGHNESS_RAW);
        self.maximum_roughness += u32::from(roughness == ONE);
        self.rim += u32::from(u >= ONE - 65);
    }

    /// Every class is reached at least `least` times.
    fn covers(&self, least: u32) -> bool {
        let classes = [
            self.reflected,
            self.absorbed,
            self.grazing,
            self.very_grazing,
            self.steep,
            self.short,
            self.minimum_roughness,
            self.maximum_roughness,
            self.rim,
        ];
        classes.iter().all(|&count| count >= least)
    }
}

/// The kernel's stated cosines for an incoming ray, a returned direction and the frame normal:
/// `c_v = -(unit(d) . n)` and `c_l = r . n`, each rounded once and clamped to `0..=1.0`.
fn stated_cosines(hit: &SurfaceHit, incoming: &TraceRay, direction: Raw3) -> (i128, i128) {
    let n = wide(hit.frame().normal());
    let view = round_raw(-dot(unit_rule(incoming.direction()), n)).clamp(0, WIDE_ONE);
    let light = round_raw(dot(direction.map(i128::from), n)).clamp(0, WIDE_ONE);
    (view, light)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_144_reflected_directions_are_unit_vectors_above_the_surface() {
    let seed = GGX_TEST_SEED ^ 0x0144;
    println!("OPTICS-TP-144 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut reach = Reach::default();
    for case in 0..12_000 {
        let Some((hit, incoming, facing)) = seeded_case(&mut seeds) else { continue };
        let roughness = seeds.roughness();
        let metal = conductor(roughness, [0; 3].map(|_| seeds.reflectance()));
        let sample = [seeds.unit_raw(), seeds.unit_raw()];
        reach.note(&hit, &incoming, (roughness, sample[0]));
        let Some((direction, throughput)) = scatter(&hit, &incoming, &metal, sample) else {
            reach.absorbed += 1;
            continue;
        };
        reach.reflected += 1;
        let context = format!("seed {seed:#x} case {case}: direction {direction:?}");
        let r = direction.map(i128::from);
        // A unit vector by Bunny's own definition, within 3 raw units of length 1.0, and a
        // valid trace direction.
        assert!(FixedUnitVec3::try_from_unit(vec3(direction)).is_some(), "{context}");
        assert!((dot(r, r) - (1 << 64)).abs() <= 3 << 33, "{context}");
        assert!(direction.iter().all(|c| c.abs() <= ONE), "{context}");
        // Strictly above the exact plane, and a cosine against the frame normal that rounds to
        // at least one raw unit.
        assert!(dot(r, facing) > 0, "{context}: along or below the plane");
        let cosine = round_raw(dot(r, wide(hit.frame().normal())));
        assert!(cosine >= 1, "{context}: at or below the horizon");
        assert!(throughput.iter().all(|t| (0..=ONE).contains(t)), "{context}: {throughput:?}");
    }
    println!("OPTICS-TP-144 {reach:?}");
    assert!(reach.covers(100), "the generator no longer reaches every class: {reach:?}");
}

/// `floor(numerator 2^32 / denominator)` and its remainder, for `numerator <= denominator <
/// 2^98`: long division in two 16-bit steps, so no product overflows.
fn scaled_quotient(numerator: u128, denominator: u128) -> (i128, u128) {
    let mut quotient = numerator / denominator;
    let mut remainder = numerator % denominator;
    for _ in 0..2 {
        remainder <<= 16;
        quotient = (quotient << 16) + remainder / denominator;
        remainder %= denominator;
    }
    (i128::try_from(quotient).expect("at most 2^32"), remainder)
}

/// Whether raw `ratio` is within `1/2 + 2^-11` raw units of the exact Smith ratio at raw
/// `alpha`, `c_v` and `c_l`: no more than that below its upper bound or above its lower bound.
fn ratio_within_rounding(ratio: i64, alpha: i128, view: i128, light: i128) -> bool {
    let [n_low, d_low, n_high, d_high] = ratio_bounds(alpha, view, light);
    // 4096 (ratio - q) -+ 2050 against 4096 remainder / denominator, cross-multiplied.
    let scaled = |quotient: i128| 4096 * (i128::from(ratio) - quotient);
    let wide_of = |value: u128| i128::try_from(value).expect("below 2^98");
    let (q_low, r_low) = scaled_quotient(n_low, d_low);
    let (q_high, r_high) = scaled_quotient(n_high, d_high);
    let near = |quotient: i128| (i128::from(ratio) - quotient).abs() <= 2;
    near(q_low)
        && near(q_high)
        && (scaled(q_low) + 2050) * wide_of(d_low) >= 4096 * wide_of(r_low)
        && (scaled(q_high) - 2050) * wide_of(d_high) <= 4096 * wide_of(r_high)
}

/// Raw bounds `(low, high)` of the Fresnel cosine `-(unit(d) . m)` of a reflected sample,
/// from the two directions alone: with `w = -(unit(d) . r)`, the cosine `c` of the view
/// against the facet has `c^2 = (1 + w) / 2` within 2 raw units, and the kernel's rounded
/// cosine is within 5 raw units of `c`.
fn fresnel_cosine_bounds(incoming: &TraceRay, direction: Raw3) -> (i64, i64) {
    let w = -dot(unit_rule(incoming.direction()), direction.map(i128::from));
    let square = i128::midpoint(1 << 64, w);
    let root = |value: i128| u128::try_from(value.max(0)).expect("non-negative").isqrt();
    let low = i128::try_from(root(square - (2 << 32))).expect("a root") - 5;
    let high = i128::try_from(root(square + (2 << 32))).expect("a root") + 6;
    let raw = |value: i128| i64::try_from(value.clamp(0, WIDE_ONE)).expect("in 0..=2^32");
    (raw(low), raw(high))
}

/// `fresnel * ratio`, rounded once to nearest, ties to even: the chapter's weight.
fn stated_weight(fresnel: FixedQ32_32, ratio: i64) -> i64 {
    i64::try_from(round_raw(i128::from(fresnel.raw()) * i128::from(ratio))).expect("in 0..=2^32")
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_145_the_weight_is_fresnel_times_the_smith_ratio() {
    let seed = GGX_TEST_SEED ^ 0x0145;
    println!("OPTICS-TP-145 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let (mut reflected, mut pinned) = (0_u32, 0_u32);
    for case in 0..6_000 {
        let Some((hit, incoming, _)) = seeded_case(&mut seeds) else { continue };
        let roughness = seeds.roughness();
        let f0 = [ONE, seeds.reflectance(), seeds.reflectance()];
        let sample = [seeds.unit_raw(), seeds.unit_raw()];
        let Some((direction, throughput)) =
            scatter(&hit, &incoming, &conductor(roughness, f0), sample)
        else {
            continue;
        };
        reflected += 1;
        let context = format!("seed {seed:#x} case {case}: roughness {roughness} f0 {f0:?}");
        // The channel with F0 = 1.0 has F = 1.0 exactly, so it is the Smith ratio itself.
        let (view, light) = stated_cosines(&hit, &incoming, direction);
        let ratio = throughput[0];
        assert!(
            ratio_within_rounding(ratio, i128::from(roughness), view, light),
            "{context}: ratio {ratio} at c_v {view} c_l {light}"
        );
        // The other channels: Schlick's term at the Fresnel cosine, times that ratio, rounded
        // once. The rounded term never rises with the cosine, so the bounds of the cosine
        // bound the weight; where they give one value the weight is pinned exactly.
        let (low, high) = fresnel_cosine_bounds(&incoming, direction);
        for channel in 1..3 {
            let least = stated_weight(schlick_fresnel(fixed(f0[channel]), fixed(high)), ratio);
            let most = stated_weight(schlick_fresnel(fixed(f0[channel]), fixed(low)), ratio);
            assert!(
                (least..=most).contains(&throughput[channel]),
                "{context}: channel {channel} {} outside {least}..={most}",
                throughput[channel]
            );
            pinned += u32::from(least == most);
        }
    }
    println!("OPTICS-TP-145 reflected {reflected}, channels pinned exactly {pinned}");
    assert!(reflected >= 2_000 && pinned >= 500, "reflected {reflected} pinned {pinned}");
}

/// The model's input for one kernel call.
fn model_input(
    hit: &SurfaceHit,
    incoming: &TraceRay,
    roughness: i64,
    sample: [i64; 2],
) -> ModelInput {
    let (sin, cos) = FixedAngle::from_turns(fixed(sample[1])).sin_cos();
    let (normal, d) = (wide(hit.frame().normal()), wide(incoming.direction()));
    ModelInput {
        normal,
        tangent: wide(hit.frame().tangent()),
        incoming: d,
        alpha: i128::from(roughness),
        u: i128::from(sample[0]),
        sin_cos: (i128::from(sin.raw()), i128::from(cos.raw())),
        parallel: cross(d, normal) == [0; 3],
    }
}

/// A structural threshold for the direction against the model, raw units per component:
/// `2^-12`, applied where `1 - u >= 2^-12`. It is not a derived bound: a wrong step moves the
/// direction by far more (the mutation table shows each such change fail), and the largest
/// differences this test sees are printed as measurements. Nearer the disk's rim the floored
/// root of a small number loses relative precision and nothing is asserted about the direction.
const MODEL_THRESHOLD: i128 = 1 << 20;
const MODEL_RIM: i64 = ONE - (1 << 20);

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_146_the_direction_follows_heitz_listing_at_60_bits() {
    let seed = GGX_TEST_SEED ^ 0x0146;
    println!("OPTICS-TP-146 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut reach = Reach::default();
    let (mut worst, mut worst_inner, mut worst_rim) = (0_i128, 0_i128, 0_i128);
    let margin = (MODEL_THRESHOLD + 16) << LIFT;
    for case in 0..12_000 {
        let Some((hit, incoming, _)) = seeded_case(&mut seeds) else { continue };
        let roughness = seeds.roughness();
        let sample = [seeds.unit_raw(), seeds.range(0, ONE - 1)];
        reach.note(&hit, &incoming, (roughness, sample[0]));
        let model = heitz(&model_input(&hit, &incoming, roughness, sample));
        let context =
            format!("seed {seed:#x} case {case}: roughness {roughness} sample {sample:?}");
        let rim = sample[0] > MODEL_RIM;
        let Some((direction, _)) = scatter(&hit, &incoming, &conductor(roughness, GOLD), sample)
        else {
            // Absorbed: the model's direction is not clearly above the surface.
            let clear = model.cosine > margin && model.facet_cosine > margin;
            assert!(rim || !clear, "{context}: absorbed, model cosine {}", model.cosine);
            reach.absorbed += 1;
            continue;
        };
        reach.reflected += 1;
        let error = (0..3)
            .map(|i| (i128::from(direction[i]) - (model.direction[i] >> LIFT)).abs())
            .max()
            .expect("three components");
        if rim {
            worst_rim = worst_rim.max(error);
            continue;
        }
        assert!(model.cosine > -margin, "{context}: reflected, model {}", model.cosine);
        assert!(error <= MODEL_THRESHOLD, "{context}: {error} raw units from the model");
        worst = worst.max(error);
        if sample[0] <= ONE / 2 {
            worst_inner = worst_inner.max(error);
        }
    }
    println!(
        "OPTICS-TP-146 {reach:?}; measured, largest difference in raw units: {worst_inner} for \
         u <= 1/2, {worst} for 1 - u >= 2^-12, {worst_rim} nearer the rim"
    );
    assert!(reach.covers(100), "the generator no longer reaches every class: {reach:?}");
}

/// The mirror limit's derived bounds at the minimum roughness `alpha = 2^-16`, for a sample
/// with `u <= 1/2` (`docs/topics/optics/README.md#the-mirror-limit` has the derivation):
/// each component of the direction is within `33 alpha` of the mirror's, and, where the
/// mirror's direction has a cosine of at least 1/4, each channel of the weight is within
/// `81 alpha` of the mirror's throughput. Raw units.
const MIRROR_DIRECTION_BOUND: i128 = 33 << 16;
const MIRROR_WEIGHT_BOUND: i64 = 81 << 16;
/// Where the mirror's direction has a cosine of at least `2^-9`, the sample is reflected.
const MIRROR_REFLECTS_ABOVE: i128 = 1 << 23;

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_147_at_the_minimum_roughness_the_sample_approaches_the_mirror() {
    let seed = GGX_TEST_SEED ^ 0x0147;
    println!("OPTICS-TP-147 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut reach = Reach::default();
    let (mut weighed, mut worst_direction, mut worst_weight) = (0_u32, 0_i128, 0_i64);
    for case in 0..12_000 {
        let Some((hit, incoming, _)) = seeded_case(&mut seeds) else { continue };
        let metal = conductor(MIN_GGX_ROUGHNESS_RAW, [0; 3].map(|_| seeds.reflectance()));
        let sample = [seeds.range(0, ONE / 2), seeds.range(0, ONE - 1)];
        let Some(mirror) = hit.specular_reflection(&incoming, &metal.reflectance()) else {
            continue;
        };
        reach.note(&hit, &incoming, (MIN_GGX_ROUGHNESS_RAW, sample[0]));
        let mirror_direction = raw3(mirror.ray().direction());
        let cosine = round_raw(dot(mirror_direction.map(i128::from), wide(hit.frame().normal())));
        let context = format!("seed {seed:#x} case {case}: sample {sample:?}");
        let Some((direction, throughput)) = scatter(&hit, &incoming, &metal, sample) else {
            assert!(cosine < MIRROR_REFLECTS_ABOVE, "{context}: absorbed at cosine {cosine}");
            reach.absorbed += 1;
            continue;
        };
        reach.reflected += 1;
        let error = (0..3)
            .map(|i| i128::from((direction[i] - mirror_direction[i]).abs()))
            .max()
            .expect("three components");
        assert!(error <= MIRROR_DIRECTION_BOUND, "{context}: {error} raw units from the mirror");
        worst_direction = worst_direction.max(error);
        if cosine < i128::from(ONE / 4) {
            continue;
        }
        for (weight, mirrored) in throughput.into_iter().zip(mirror.throughput()) {
            let difference = (weight - mirrored.raw()).abs();
            assert!(
                difference <= MIRROR_WEIGHT_BOUND,
                "{context}: a channel differs by {difference}"
            );
            worst_weight = worst_weight.max(difference);
        }
        weighed += 1;
    }
    println!(
        "OPTICS-TP-147 {reach:?}, weighed {weighed}; measured: largest direction difference \
         {worst_direction} raw units (bound {MIRROR_DIRECTION_BOUND}), largest weight difference \
         {worst_weight} (bound {MIRROR_WEIGHT_BOUND})"
    );
    assert!(reach.reflected >= 2_000 && weighed >= 500 && reach.grazing >= 100, "{reach:?}");
}

/// The expected albedo of a single-scattering GGX conductor with `F = 1`, raw Q32.32, for a
/// roughness and an incoming direction onto the ground: `(roughness, direction, albedo)`.
///
/// The mean weight of the sampler is this albedo, `Int f cos(theta_l) dw_l` with `f = D G2 /
/// (4 c_v c_l)`. It is below one because the model follows one reflection only: light that the
/// masking-shadowing term `G2` removes would, on a real surface, scatter again, and is lost
/// here; the rougher the surface, the more.
///
/// At roughness 1.0 the integral has a closed form, `1 - c_v ln(1 + 1 / c_v)` (`D = 1 / pi`
/// and `s = 1`, so `f cos = c_l / (2 pi (c_l + c_v))`): `1 - ln 2` at normal incidence. The
/// first three values are that form at `c_v = 1`, `1 / sqrt(2)` and `1 / sqrt(17)`, from an
/// integer series for the logarithm. The rest are "quadrature" (see the module comment); the
/// quadrature reproduces the three closed forms within 200 raw units, and its last two
/// refinements differ by less than `2^-18`.
const FURNACE: [(i64, Raw3, i64); 18] = [
    (ONE, [0, -ONE, 0], 1_317_922_824),
    (ONE, [ONE, -ONE, 0], 1_618_235_272),
    (ONE, [ONE, -ONE / 4, 0], 2_593_107_111),
    (3 * (ONE / 4), [0, -ONE, 0], 1_992_506_003),
    (3 * (ONE / 4), [ONE, -ONE, 0], 2_190_406_746),
    (3 * (ONE / 4), [ONE, -ONE / 4, 0], 2_942_664_288),
    (ONE / 2, [0, -ONE, 0], 2_954_286_810),
    (ONE / 2, [ONE, -ONE, 0], 2_932_492_309),
    (ONE / 2, [ONE, -ONE / 4, 0], 3_300_954_462),
    (ONE / 4, [0, -ONE, 0], 3_933_384_657),
    (ONE / 4, [ONE, -ONE, 0], 3_806_388_211),
    (ONE / 4, [ONE, -ONE / 4, 0], 3_611_475_051),
    (ONE / 16, [0, -ONE, 0], 4_276_448_506),
    (ONE / 16, [ONE, -ONE, 0], 4_266_052_645),
    (ONE / 16, [ONE, -ONE / 4, 0], 4_121_761_597),
    (MIN_GGX_ROUGHNESS_RAW, [0, -ONE, 0], ONE),
    (MIN_GGX_ROUGHNESS_RAW, [ONE, -ONE, 0], ONE),
    (MIN_GGX_ROUGHNESS_RAW, [ONE, -ONE / 4, 0], ONE),
];

/// The sum of the weights of the `2^16` samples at the centres of a 256 by 256 grid of the
/// unit square (one per stratum), with `F0 = 1`; an absorbed sample weighs zero.
fn furnace_sum(roughness: i64, direction: Raw3) -> i128 {
    let (hit, incoming) = ground_hit(direction);
    let metal = conductor(roughness, [ONE; 3]);
    let mut sum = 0_i128;
    for cell in 0..1_i64 << 16 {
        let sample = [cell >> 8, cell & 255].map(|k| fixed((2 * k + 1) << 23));
        let result = hit.try_ggx_reflection(&incoming, &metal, sample).expect("accepted");
        if let GgxScatter::Reflected(reflected) = result {
            sum += i128::from(reflected.throughput()[0].raw());
        }
    }
    sum
}

/// The furnace threshold, raw units of the mean: `2^-11`. It is a measured tolerance, not a
/// derived bound. Its parts: the table's accuracy (exact at roughness 1.0, `2^-18` measured
/// elsewhere), the weights' rounding (derived, below `2^-29`), and the error of a 256 by 256
/// midpoint rule on the weight, which is not derived; the largest measured difference is
/// 1 026 800 raw units (`2^-12.03`), at roughness 1/16, where it falls as the grid is refined.
const FURNACE_THRESHOLD: i128 = 1 << 21;

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_148_the_mean_weight_is_the_single_scattering_albedo() {
    let mut worst = 0_i128;
    for (roughness, direction, albedo) in FURNACE {
        let mean = furnace_sum(roughness, direction) >> 16;
        let difference = mean - i128::from(albedo);
        println!(
            "OPTICS-TP-148 roughness {roughness} direction {direction:?}: mean {mean}, albedo \
             {albedo}, difference {difference}"
        );
        assert!(
            difference.abs() <= FURNACE_THRESHOLD,
            "roughness {roughness} direction {direction:?}: mean {mean} against {albedo}"
        );
        worst = worst.max(difference.abs());
    }
    println!("OPTICS-TP-148 measured: largest difference {worst} raw units");
}

/// The exact reflection `d (n . n) - 2 (d . n) n` about the line of `n`.
fn exact_reflection(d: Wide3, n: Wide3) -> Wide3 {
    let (nn, dn) = (dot(n, n), dot(d, n));
    [0, 1, 2].map(|i| d[i] * nn - 2 * dn * n[i])
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_149_reflect_is_the_exact_reflection_normalized_once() {
    let reflected = |d: Raw3, n: Raw3| reflect(vec3(d), vec3(n)).map(raw3);
    // About an axis, for a normal of any length. (1, -1, 0) reflects to (1, 1, 0) / sqrt(2):
    // raw 3 037 000 499.97..., to nearest.
    assert_eq!(reflected([ONE, -ONE, 0], [0, ONE, 0]), Some([3_037_000_500, 3_037_000_500, 0]));
    assert_eq!(reflected([0, -ONE, 0], [0, 7, 0]), Some([0, ONE, 0]));
    assert_eq!(reflected([0, -3, 0], [0, ONE, 0]), Some([0, ONE, 0]));
    // (3, -4, 0) reflects to (3, 4, 0) / 5: raw 2 576 980 377.6 and 3 435 973 836.8, to nearest.
    assert_eq!(reflected([3, -4, 0], [0, ONE, 0]), Some([2_576_980_378, 3_435_973_837, 0]));
    // Straight down about the rounded (3/5, 4/5, 0): (24/25, 7/25, 0), raw 4 123 168 604.16
    // and 1 202 590 842.88, each within one raw unit.
    let tilted = reflected([0, -ONE, 0], [2_576_980_378, 3_435_973_837, 0]).expect("a reflection");
    assert!((tilted[0] - 4_123_168_604).abs() <= 1 && (tilted[1] - 1_202_590_843).abs() <= 1);
    assert_eq!(tilted[2], 0);
    // The largest components of a unit vector, 2^32 + 1, are accepted; one more is not.
    assert_eq!(reflected([ONE + 1, 0, 0], [ONE + 1, 0, 0]), Some([-ONE, 0, 0]));
    assert_eq!(reflected([ONE + 2, 0, 0], [ONE, 0, 0]), None);
    assert_eq!(reflected([ONE, 0, 0], [0, -ONE - 2, 0]), None);
    // A zero vector has no reflection.
    assert_eq!(reflected([0; 3], [0, ONE, 0]), None);
    assert_eq!(reflected([ONE, 0, 0], [0; 3]), None);

    let seed = GGX_TEST_SEED ^ 0x0149;
    println!("OPTICS-TP-149 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let (mut short, mut long_normals) = (0_u32, 0_u32);
    for case in 0..4_000 {
        // Directions and normals of every length: a random bit length each.
        let (d_bits, n_bits) = (seeds.range(0, 32), seeds.range(0, 32));
        let (d, n) = (seeds.triple(1 << d_bits), seeds.triple(1 << n_bits));
        let exact = exact_reflection(d.map(i128::from), n.map(i128::from));
        let result = reflected(d, n);
        if d == [0; 3] || n == [0; 3] {
            assert_eq!(result, None, "seed {seed:#x} case {case}");
            continue;
        }
        short += u32::from(d.iter().all(|c| c.abs() < 1 << 16));
        long_normals += u32::from(n.iter().any(|c| c.abs() > 1 << 31));
        let direction = result.unwrap_or_else(|| panic!("seed {seed:#x} case {case}: refused"));
        // Within one raw unit of the unit vector along the exact reflection, plus the
        // prescale's floor (below 2^-27 raw units).
        let top = exact.iter().map(|c| c.abs()).max().expect("three");
        let slack = Signed::new(top >> 27);
        assert!(
            within(direction, &exact.map(Signed::new), 1, &slack),
            "seed {seed:#x} case {case}: {direction:?} for d {d:?} n {n:?}"
        );
        assert!(direction.iter().all(|c| c.abs() <= ONE), "seed {seed:#x} case {case}");
    }
    println!("OPTICS-TP-149 short directions {short}, normals above 1/2 {long_normals}");
    assert!(short >= 500 && long_normals >= 100, "short {short} long {long_normals}");
}
