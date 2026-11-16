//! Evidence for `bunny-specular-reflection/1`: hand-derived mirrors, Schlick's term at its
//! rounding boundaries and against an exact rational oracle, the reflected direction against
//! the exact reflection on seeded hits, normal incidence, and the refusals.
//!
//! The oracles are exact big-integer arithmetic written here from the chapter's formulas; no
//! floating point. Golden values marked "model" come from a separate Python model (unbounded
//! integers) written from the same formulas.

mod exact;

use std::cmp::Ordering;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    schlick_fresnel, Albedo, SecondaryRayError, SurfaceHit, TriangleSurface, SPECULAR_REFLECTION_ID,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

use exact::{within, Big, Signed};

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const SPECULAR_TEST_SEED: u64 = 0xB002_0370_5EC0_0001;
/// `2^32 / sqrt(2)` rounded: `3 037 000 499.976...`.
const HALF_SQRT2: i64 = 3_037_000_500;

type Raw3 = [i64; 3];
type Wide3 = [i128; 3];

/// The ground: a triangle in the plane `y = 0` whose front face looks up (`+Y`).
const GROUND: [Raw3; 3] = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];

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

fn ray(origin: Raw3, direction: Raw3) -> TraceRay {
    TraceRay::new(vec3(origin), vec3(direction)).expect("a valid trace ray")
}

fn albedo(raw: Raw3) -> Albedo {
    Albedo::new(raw.map(fixed)).expect("channels in 0..=1")
}

/// The hit of `ray` on the triangle `v`, if the trace kernel reports one.
fn hit_of(v: [Raw3; 3], ray: &TraceRay) -> Option<SurfaceHit> {
    let triangle = TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2]))?;
    let surface = TriangleSurface::new(&triangle)?;
    let hit = ray.hit_triangle(&triangle, fixed(0), MAX)?;
    SurfaceHit::new(&surface, ray, &hit)
}

fn throughput_raw(sample: &bunny_optics::SpecularSample) -> Raw3 {
    sample.throughput().map(FixedQ32_32::raw)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_100_hand_derived_mirrors_reflect_about_the_normal() {
    assert_eq!(SPECULAR_REFLECTION_ID, "bunny-specular-reflection/1");
    let gold = albedo([ONE, 3_694_871_552, 1_417_339_208]);
    // Straight down onto the ground: straight back up, and the throughput is F0 exactly.
    let down = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let hit = hit_of(GROUND, &down).expect("hits the ground");
    let sample = hit.specular_reflection(&down, &gold).expect("a mirror sample");
    assert_eq!(raw3(sample.ray().direction()), [0, ONE, 0]);
    assert_eq!(sample.ray().origin(), hit.point());
    assert_eq!(sample.throughput(), gold.channels());
    // At 45 degrees: (1, -1, 0) reflects to (1, 1, 0) / sqrt(2). The cosine is the rounded
    // 1 / sqrt(2), raw 3 037 000 500, so m = 1 257 966 796 and w = 9 257 771 (model); with
    // F0 = 0.04 (raw 171 798 692) the throughput is 180 686 152 (model), against the exact
    // 0.0420690... (raw 180 686 151.93...).
    let slant = ray([-2 * ONE, 2 * ONE, 0], [ONE, -ONE, 0]);
    let hit = hit_of(GROUND, &slant).expect("hits the ground");
    let dielectric = albedo([171_798_692; 3]);
    let sample = hit.specular_reflection(&slant, &dielectric).expect("a mirror sample");
    assert_eq!(raw3(sample.ray().direction()), [HALF_SQRT2, HALF_SQRT2, 0]);
    assert_eq!(throughput_raw(&sample), [180_686_152; 3]);
    assert_eq!(schlick_fresnel(fixed(171_798_692), fixed(HALF_SQRT2)).raw(), 180_686_152);
    // The back face of the ground reflects downward.
    let up = ray([ONE, -2 * ONE, 0], [0, ONE, 0]);
    let hit = hit_of(GROUND, &up).expect("hits the ground from below");
    let sample = hit.specular_reflection(&up, &gold).expect("a mirror sample");
    assert_eq!(raw3(sample.ray().direction()), [0, -ONE, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_101_schlick_is_pinned_at_its_ends_and_rounding_boundaries() {
    let half = 1_i64 << 31;
    for f0 in [0, 1, 171_798_692, half, ONE - 1, ONE] {
        // Normal incidence gives F0 exactly; grazing incidence gives 1.0 exactly.
        assert_eq!(schlick_fresnel(fixed(f0), fixed(ONE)).raw(), f0);
        assert_eq!(schlick_fresnel(fixed(f0), fixed(0)).raw(), ONE);
    }
    // cos = 0.5: m^5 = 2^-5 exactly, so F = 0.5 + 0.5 / 32, raw 2^31 + 2^26.
    assert_eq!(schlick_fresnel(fixed(half), fixed(half)).raw(), half + (1 << 26));
    // The final rounding. With F0 = 0.5, F = 2^31 + w / 2 raw: a tie whenever w is odd.
    // cos = 2^31 + 2: w = 134 217 727 (model), exact 2 214 592 511.5, so 2 214 592 512 (even);
    // floor or ties toward zero would give ...511.
    assert_eq!(schlick_fresnel(fixed(half), fixed(half + 2)).raw(), 2_214_592_512);
    // cos = 2^31 + 9: w = 134 217 725 (model), exact 2 214 592 510.5, so 2 214 592 510 (even);
    // ties away from zero would give ...511.
    assert_eq!(schlick_fresnel(fixed(half), fixed(half + 9)).raw(), 2_214_592_510);
    // An exact weight: cos = 0.75 gives m^5 = 2^-10, w = 4 194 304. An inexact one: cos raw
    // 1 234 567 890 gives w = 788 952 942 (model) and F = 2 541 960 119. A floored weight
    // (...941) gives the same F here; OPTICS-TP-102 and OPTICS-TP-103 catch it.
    assert_eq!(schlick_fresnel(fixed(half), fixed(3 << 30)).raw(), 2_149_580_800);
    assert_eq!(schlick_fresnel(fixed(half), fixed(1_234_567_890)).raw(), 2_541_960_119);
    // Inputs are clamped to 0..=1.0.
    assert_eq!(schlick_fresnel(fixed(half), fixed(-ONE)).raw(), ONE);
    assert_eq!(schlick_fresnel(fixed(half), fixed(2 * ONE)).raw(), half);
    assert_eq!(schlick_fresnel(fixed(-ONE), fixed(ONE)).raw(), 0);
    assert_eq!(schlick_fresnel(fixed(3 * ONE), fixed(half)).raw(), ONE);
}

/// `2^shift` as a big integer.
fn power_of_two(shift: u32) -> Big {
    (0..shift / 64).fold(Big::new(1 << (shift % 64)), |b, _| b.mul(&Big::new(1 << 64)))
}

fn big(value: i128) -> Big {
    Big::new(u128::try_from(value).expect("non-negative"))
}

/// Schlick's exact value at raw `f0` and raw `cosine` (each in `0..=2^32`), in units of
/// `2^-192`: `f0 2^160 + (2^32 - f0) (2^32 - cosine)^5`.
fn schlick_exact(f0: i128, cosine: i128) -> Big {
    let one = i128::from(ONE);
    let m = big(one - cosine);
    let fifth = m.mul(&m).mul(&m).mul(&m).mul(&m);
    big(f0).mul(&power_of_two(160)).add(&big(one - f0).mul(&fifth))
}

/// Whether raw `value` is within one raw unit of the exact `target` (units of `2^-192`).
fn within_one_raw(value: i64, target: &Big) -> bool {
    let unit = power_of_two(160);
    let scaled = big(i128::from(value)).mul(&unit);
    scaled.compare(&target.add(&unit)) != Ordering::Greater
        && scaled.add(&unit).compare(target) != Ordering::Less
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

    /// A raw value in `0..=2^32`, a third of them within 64 raw units of an end.
    fn unit_raw(&mut self) -> i64 {
        match self.range(0, 2) {
            0 => self.range(0, 64),
            1 => ONE - self.range(0, 64),
            _ => self.range(0, ONE),
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

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_102_schlick_is_within_one_raw_unit_of_the_exact_formula() {
    println!("OPTICS-TP-102 seed {SPECULAR_TEST_SEED:#x}");
    let mut seeds = Seeds(SPECULAR_TEST_SEED);
    for case in 0..4_000 {
        let (f0, cosine) = (seeds.unit_raw(), seeds.unit_raw());
        let value = schlick_fresnel(fixed(f0), fixed(cosine)).raw();
        let exact = schlick_exact(i128::from(f0), i128::from(cosine));
        assert!(
            (0..=ONE).contains(&value) && within_one_raw(value, &exact),
            "seed {SPECULAR_TEST_SEED:#x} case {case}: f0 {f0} cosine {cosine} gave {value}"
        );
    }
}

/// The largest integer `k` in `0..=2^32` with `k |d| <= -(d . n)`: the floor of the exact
/// cosine `-(d . n) / |d|` in raw units (zero when `d . n >= 0`).
fn cosine_floor(d: Wide3, n: Wide3) -> i128 {
    let along = -dot(d, n);
    if along <= 0 {
        return 0;
    }
    let (target, length) = (big(along).mul(&big(along)), big(dot(d, d)));
    let (mut low, mut high) = (0_i128, i128::from(ONE) + 1);
    while high - low > 1 {
        let mid = i128::midpoint(low, high);
        let fits = big(mid * mid).mul(&length).compare(&target) != Ordering::Greater;
        if fits {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// Whether raw `value` lies within one raw unit of Schlick's exact value at some cosine in
/// `low..=high` (clamped to `0..=2^32`): the term decreases with the cosine.
fn schlick_between(value: i64, f0: i64, low: i128, high: i128) -> bool {
    let one = i128::from(ONE);
    let (low, high) = (low.clamp(0, one), high.clamp(0, one));
    let unit = power_of_two(160);
    let scaled = big(i128::from(value)).mul(&unit);
    let f0 = i128::from(f0);
    scaled.compare(&schlick_exact(f0, low).add(&unit)) != Ordering::Greater
        && scaled.add(&unit).compare(&schlick_exact(f0, high)) != Ordering::Less
}

/// The exact reflection `d (n . n) - 2 (d . n) n` about the line of `n`; each component is
/// below `2^100`.
fn exact_reflection(d: Wide3, n: Wide3) -> Wide3 {
    let (nn, dn) = (dot(n, n), dot(d, n));
    [0, 1, 2].map(|i| d[i] * nn - 2 * dn * n[i])
}

/// A seeded scene: a triangle near a seeded centre and a ray that aims at a point inside it.
fn seeded_case(seeds: &mut Seeds) -> Option<(SurfaceHit, TraceRay)> {
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
    let d = seeds.direction();
    let reach = seeds.range(1, 1 << (bits + 2).min(38));
    let origin: Raw3 = [0, 1, 2].map(|i| {
        let back = (i128::from(d[i]) * i128::from(reach)) >> 32;
        i64::try_from(i128::from(target[i]) - back).expect("a raw value")
    });
    let ray = TraceRay::new(vec3(origin), vec3(d))?;
    Some((hit_of(v, &ray)?, ray))
}

/// Checks one seeded hit: the `Option` and `Result` forms agree; an accepted sample starts at
/// the hit's start point, its direction is within one raw unit (plus `2^-27`) of the unit
/// vector along the exact reflection about the frame normal `n`, and each throughput channel
/// is within one raw unit of Schlick's exact value at a cosine within three raw units of the
/// exact `-(d . n) / |d|`; the only refusal a kernel hit may give is a grazing reflection.
/// Returns a description of the first failure, or an empty string. `counts` tallies accepted
/// and refused cases.
fn check_case(hit: &SurfaceHit, ray: &TraceRay, f0: Raw3, counts: &mut [u32; 3]) -> String {
    let reflectance = albedo(f0);
    let result = hit.try_specular_reflection(ray, &reflectance);
    assert_eq!(hit.specular_reflection(ray, &reflectance), result.ok());
    let (d, n) = (wide(ray.direction()), wide(hit.frame().normal()));
    match result {
        Ok(sample) => {
            counts[0] += 1;
            assert_eq!(sample.ray().origin(), hit.point());
            let exact = exact_reflection(d, n);
            let top = exact.iter().map(|c| c.abs()).max().unwrap_or(0);
            let slack = Signed::new(top >> 27);
            let r = exact.map(Signed::new);
            let direction = raw3(sample.ray().direction());
            if !within(direction, &r, 1, &slack) {
                return format!("direction {direction:?} for d {d:?} n {n:?}");
            }
            let k = cosine_floor(d, n);
            for (t, f) in throughput_raw(&sample).into_iter().zip(f0) {
                if !schlick_between(t, f, k - 2, k + 3) {
                    return format!("throughput {t} for f0 {f} near cosine {k}");
                }
            }
        }
        Err(SecondaryRayError::ReflectionBelowSurface { .. }) => counts[1] += 1,
        Err(other) => return format!("unexpected refusal {other:?}"),
    }
    String::new()
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_103_the_mirror_matches_the_exact_reflection_on_seeded_hits() {
    let seed = SPECULAR_TEST_SEED ^ 0x0103;
    println!("OPTICS-TP-103 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut counts = [0_u32; 3];
    for case in 0..3_000 {
        let Some((hit, ray)) = seeded_case(&mut seeds) else {
            counts[2] += 1;
            continue;
        };
        let f0 = [seeds.unit_raw(), seeds.unit_raw(), seeds.unit_raw()];
        let failure = check_case(&hit, &ray, f0, &mut counts);
        assert!(failure.is_empty(), "seed {seed:#x} case {case}: {failure}");
    }
    println!("OPTICS-TP-103 accepted, grazing-refused, missed: {counts:?}");
    assert!(counts[0] >= 1_000, "too few accepted cases: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_104_at_normal_incidence_the_throughput_is_the_base_reflectance() {
    let seed = SPECULAR_TEST_SEED ^ 0x0104;
    println!("OPTICS-TP-104 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut checked = 0;
    for case in 0..3_000 {
        let Some((hit, _)) = seeded_case(&mut seeds) else { continue };
        let n = raw3(hit.frame().normal());
        // Along -n: a trace direction when no component of n exceeds 1.0.
        let Some(incoming) = TraceRay::new(hit.point(), vec3(n.map(|c| -c))) else { continue };
        let f0 = albedo([seeds.unit_raw(), seeds.unit_raw(), seeds.unit_raw()]);
        let sample = hit.try_specular_reflection(&incoming, &f0);
        let sample = sample.unwrap_or_else(|e| panic!("seed {seed:#x} case {case}: {e}"));
        assert_eq!(sample.throughput(), f0.channels(), "seed {seed:#x} case {case}");
        checked += 1;
    }
    assert!(checked >= 1_000, "too few cases: {checked}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_105_a_ray_arriving_below_the_surface_is_refused() {
    let gold = albedo([ONE; 3]);
    let down = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let hit = hit_of(GROUND, &down).expect("hits the ground");
    // Up from below, and along the plane: d . N >= 0 for the face this hit faces.
    for d in [[0, ONE, 0], [ONE, 0, 0], [ONE, 1, -ONE]] {
        let incoming = ray([0, ONE, 0], d);
        let expected = SecondaryRayError::ArrivesBelowSurface { direction: vec3(d) };
        assert_eq!(hit.try_specular_reflection(&incoming, &gold), Err(expected));
        assert_eq!(hit.specular_reflection(&incoming, &gold), None);
        assert!(expected.to_string().contains("arrives from below the surface"));
    }
    // One raw unit downward is enough to arrive from above.
    let barely = ray([0, ONE, 0], [ONE, -1, 0]);
    assert!(!matches!(
        hit.try_specular_reflection(&barely, &gold),
        Err(SecondaryRayError::ArrivesBelowSurface { .. })
    ));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_106_every_seeded_reversed_ray_is_refused() {
    let seed = SPECULAR_TEST_SEED ^ 0x0106;
    println!("OPTICS-TP-106 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let gold = albedo([ONE; 3]);
    for case in 0..2_000 {
        let Some((hit, ray)) = seeded_case(&mut seeds) else { continue };
        let d = ray.direction();
        let reversed = TraceRay::new(ray.origin(), vec3(raw3(d).map(|c| -c))).expect("a ray");
        let expected = SecondaryRayError::ArrivesBelowSurface { direction: reversed.direction() };
        let refusal = hit.try_specular_reflection(&reversed, &gold);
        assert_eq!(refusal, Err(expected), "seed {seed:#x} case {case}");
    }
}

/// Found by a search over seeded triangles: a grazing ray that arrives from above the exact
/// plane (`d . N < 0`) but has `d . n > 0` against the rounded unit normal, so its reflection
/// about `n` stays below the plane.
const GRAZING: [Raw3; 3] = [
    [-14_247_233_187, -16_518_692_768, 8_255_702_029],
    [14_994_902_859, -9_167_710_013, 11_680_992_187],
    [-2_466_095_447, 12_133_874_043, -15_050_646_404],
];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_107_a_reflection_that_stays_below_the_plane_is_refused() {
    let d = [ONE, 441_121_301, 1_116_683_283];
    let grazing = ray([-4_867_775_887, -4_958_630_880, 511_999_321], d);
    let hit = hit_of(GRAZING, &grazing).expect("the kernel reports the hit");
    let sub = |a: Raw3, b: Raw3| [0, 1, 2].map(|i| i128::from(a[i]) - i128::from(b[i]));
    let (e1, e2) = (sub(GRAZING[1], GRAZING[0]), sub(GRAZING[2], GRAZING[0]));
    let plane = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let (dw, n) = (d.map(i128::from), wide(hit.frame().normal()));
    assert!(dot(dw, plane) < 0 && dot(dw, n) > 0, "the case's premise");
    let refusal = hit.try_specular_reflection(&grazing, &albedo([ONE; 3]));
    let direction = [4_136_382_980, 424_833_652, 1_075_451_664];
    let expected = SecondaryRayError::ReflectionBelowSurface { direction: vec3(direction) };
    assert_eq!(refusal, Err(expected));
    assert!(dot(direction.map(i128::from), plane) <= 0);
    assert!(expected.to_string().contains("does not leave the surface"));
    assert_eq!(hit.specular_reflection(&grazing, &albedo([ONE; 3])), None);
}
