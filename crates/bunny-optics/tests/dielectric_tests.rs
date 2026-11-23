//! Evidence for `bunny-dielectric/1`: hand-derived boundaries, Snell's law and the Fresnel
//! reflectance against an exact oracle on seeded hits, total internal reflection at and
//! around the exact critical angle, the choice between the branches, the parallel slab, the
//! start point of the refracted ray, and the refusals.
//!
//! The oracle (`dielectric_oracle`) is exact big-integer arithmetic written from the
//! chapter's formulas; no floating point. Golden values marked "model" come from a separate
//! Python model (unbounded integers) written from the same formulas.

mod dielectric_oracle;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    Albedo, DielectricBoundary, DielectricReflectance, DielectricScatter, SecondaryRayError,
    SurfaceHit, TriangleSurface, DIELECTRIC_ID, MAX_REFRACTIVE_INDEX_RAW,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

use dielectric_oracle::{cross, dot as dot_int, int3, within, Int, Refraction, Wide3};

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const DIELECTRIC_TEST_SEED: u64 = 0xB002_0372_D1E1_0001;
/// The largest choice value: it refracts whenever the reflectance is below 1.0.
const LAST_CHOICE: i64 = ONE - 1;
/// 1.5, crown glass.
const GLASS: i64 = 3 * ONE / 2;
/// `2^32 / sqrt(2)` rounded: `3 037 000 499.976...`.
const HALF_SQRT2: i64 = 3_037_000_500;
/// `0.04 * 2^32 = 171 798 691.84`, rounded: the reflectance of glass at normal incidence,
/// `((1.5 - 1) / (1.5 + 1))^2`.
const GLASS_NORMAL_REFLECTANCE: i64 = 171_798_692;

type Raw3 = [i64; 3];

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

fn boundary(outside: i64, inside: i64) -> DielectricBoundary {
    DielectricBoundary::new(fixed(outside), fixed(inside)).expect("indices in 1.0..=8.0")
}

fn triangle(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("inside the trace domain")
}

/// The hit of `ray` on `triangle`, if the trace kernel reports one.
fn hit_on(triangle: &TraceTriangle, ray: &TraceRay) -> Option<SurfaceHit> {
    let surface = TriangleSurface::new(triangle)?;
    let hit = ray.hit_triangle(triangle, fixed(0), MAX)?;
    SurfaceHit::new(&surface, ray, &hit)
}

/// The exact plane of the face `hit` faces: the normal `(v1 - v0) x (v2 - v0)`, negated for
/// the back face, and its dot product with `v0`.
fn facing_plane(triangle: &TraceTriangle, hit: &SurfaceHit) -> (Wide3, i128) {
    let [v0, v1, v2] = triangle.vertices().map(wide);
    let (e1, e2) = ([0, 1, 2].map(|i| v1[i] - v0[i]), [0, 1, 2].map(|i| v2[i] - v0[i]));
    let sign = if hit.front_face() { 1 } else { -1 };
    let normal = [
        sign * (e1[1] * e2[2] - e1[2] * e2[1]),
        sign * (e1[2] * e2[0] - e1[0] * e2[2]),
        sign * (e1[0] * e2[1] - e1[1] * e2[0]),
    ];
    (normal, dot(normal, v0))
}

/// The raw indices `[incident, transmitted]` at `hit`.
fn ordered(boundary: &DielectricBoundary, hit: &SurfaceHit) -> [i64; 2] {
    let (outside, inside) = (boundary.outside().raw(), boundary.inside().raw());
    if hit.front_face() {
        [outside, inside]
    } else {
        [inside, outside]
    }
}

fn scatter(
    hit: &SurfaceHit,
    incoming: &TraceRay,
    boundary: &DielectricBoundary,
    choice: i64,
) -> Result<DielectricScatter, SecondaryRayError> {
    let result = hit.try_dielectric_scatter(incoming, boundary, fixed(choice));
    assert_eq!(hit.dielectric_scatter(incoming, boundary, fixed(choice)), result.ok());
    result
}

fn refracted(scatter: Result<DielectricScatter, SecondaryRayError>) -> TraceRay {
    match scatter {
        Ok(DielectricScatter::Refracted(sample)) => {
            assert_eq!(sample.throughput(), [fixed(ONE); 3]);
            *sample.ray()
        }
        other => panic!("expected a refracted ray, got {other:?}"),
    }
}

fn reflected(scatter: Result<DielectricScatter, SecondaryRayError>) -> TraceRay {
    match scatter {
        Ok(DielectricScatter::Reflected(sample)) => {
            assert_eq!(sample.throughput(), [fixed(ONE); 3]);
            *sample.ray()
        }
        other => panic!("expected a reflected ray, got {other:?}"),
    }
}

fn partial(raw: i64) -> DielectricReflectance {
    DielectricReflectance::Partial(fixed(raw))
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_183_hand_derived_boundaries_reflect_and_refract() {
    assert_eq!(DIELECTRIC_ID, "bunny-dielectric/1");
    let ground = triangle(GROUND);
    let glass = boundary(ONE, GLASS);
    // Straight down into glass: R = 0.04; the refracted ray goes straight on from one raw
    // unit below the plane and the reflected ray straight back from one raw unit above it.
    let down = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let hit = hit_on(&ground, &down).expect("hits the ground");
    assert!(hit.front_face());
    assert_eq!(raw3(hit.point()), [0, 1, 0]);
    assert_eq!(hit.dielectric_reflectance(&down, &glass), Ok(partial(GLASS_NORMAL_REFLECTANCE)));
    let through = refracted(scatter(&hit, &down, &glass, GLASS_NORMAL_REFLECTANCE));
    assert_eq!((raw3(through.origin()), raw3(through.direction())), ([0, -1, 0], [0, -ONE, 0]));
    let back = reflected(scatter(&hit, &down, &glass, GLASS_NORMAL_REFLECTANCE - 1));
    assert_eq!((raw3(back.origin()), raw3(back.direction())), ([0, 1, 0], [0, ONE, 0]));
    // Straight up out of the glass, through the back face: the same reflectance.
    let up = ray([ONE, -2 * ONE, 0], [0, ONE, 0]);
    let hit = hit_on(&ground, &up).expect("hits the ground from below");
    assert!(!hit.front_face());
    assert_eq!(hit.dielectric_reflectance(&up, &glass), Ok(partial(GLASS_NORMAL_REFLECTANCE)));
    let through = refracted(scatter(&hit, &up, &glass, LAST_CHOICE));
    assert_eq!((raw3(through.origin()), raw3(through.direction())), ([ONE, 1, 0], [0, ONE, 0]));
    // From index 1.5 into 2.0 along (4, -3) / 5: sin = 4/5 refracts to sin = 3/5, the unit
    // vector (0.6, -0.8, 0) rounded. This is Brewster's angle (tan = 2.0 / 1.5), so r_p = 0
    // and R = r_s^2 / 2 = (0.7 / 2.5)^2 / 2 = 0.0392, raw 168 362 718.0032.
    let dense = boundary(GLASS, 2 * ONE);
    let slant = ray([-4 * ONE, 3 * ONE, 0], [ONE, -3 * ONE / 4, 0]);
    let hit = hit_on(&ground, &slant).expect("hits the ground");
    assert_eq!(hit.dielectric_reflectance(&slant, &dense), Ok(partial(168_362_718)));
    let through = refracted(scatter(&hit, &slant, &dense, 168_362_718));
    assert_eq!(raw3(through.direction()), [2_576_980_378, -3_435_973_837, 0]);
    let back = reflected(scatter(&hit, &slant, &dense, 168_362_717));
    assert_eq!(raw3(back.direction()), [3_435_973_837, 2_576_980_378, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_184_equal_indices_pass_straight_through_and_the_range_is_inclusive() {
    let ground = triangle(GROUND);
    let d = [123_456_789, -987_654_321, 5_555];
    let incoming = ray([0, ONE, 0], d);
    let hit = hit_on(&ground, &incoming).expect("hits the ground");
    for index in [ONE, GLASS, MAX_REFRACTIVE_INDEX_RAW] {
        // Equal indices: q = a m exactly, so both amplitude ratios are zero and T = a (n.n) d.
        let matched = boundary(index, index);
        assert_eq!(hit.dielectric_reflectance(&incoming, &matched), Ok(partial(0)));
        let through = refracted(scatter(&hit, &incoming, &matched, 0));
        // The unit vector along d (model): raw (532 725 124, -4 261 801 030, 23 970).
        assert_eq!(raw3(through.direction()), [532_725_124, -4_261_801_030, 23_970]);
        let exact = int3(d.map(i128::from));
        assert!(within(raw3(through.direction()), &exact, 1, &Int::new(0)));
    }
    let new = |outside, inside| DielectricBoundary::new(fixed(outside), fixed(inside));
    let limit = MAX_REFRACTIVE_INDEX_RAW;
    assert!(new(ONE, limit).is_ok() && new(limit, ONE).is_ok());
    for (outside, inside, refused) in [
        (ONE - 1, GLASS, ONE - 1),
        (GLASS, ONE - 1, ONE - 1),
        (limit + 1, GLASS, limit + 1),
        (GLASS, limit + 1, limit + 1),
        (0, -ONE, 0),
    ] {
        let error = new(outside, inside).expect_err("an index outside the range");
        assert_eq!(error.index().raw(), refused);
        let message = format!("refractive index raw {refused} is outside {ONE}..={limit}");
        assert_eq!(error.to_string(), message);
    }
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

    /// A trace direction: a seeded vector with one axis set to exactly +-1.0.
    fn direction(&mut self) -> Raw3 {
        let mut d = self.triple(ONE);
        let axis = usize::try_from(self.range(0, 2)).expect("an axis");
        d[axis] = if self.range(0, 1) == 0 { ONE } else { -ONE };
        d
    }

    /// A refractive index: half of them a named value (1.0, 4/3, 1.5, 2.417, 8.0), half
    /// anywhere in the range.
    fn index(&mut self) -> i64 {
        const NAMED: [i64; 5] =
            [ONE, 5_726_623_061, GLASS, 10_380_936_151, MAX_REFRACTIVE_INDEX_RAW];
        match self.range(0, 9) {
            pick @ 0..=4 => NAMED[usize::try_from(pick).expect("an index")],
            _ => self.range(ONE, MAX_REFRACTIVE_INDEX_RAW),
        }
    }

    /// A boundary: one in eight has indices one raw unit apart.
    fn boundary(&mut self) -> DielectricBoundary {
        let outside = self.index();
        let inside = match self.range(0, 7) {
            0 => (outside + 1).min(MAX_REFRACTIVE_INDEX_RAW),
            _ => self.index(),
        };
        boundary(outside, inside)
    }
}

/// A seeded triangle, a ray the trace kernel reports a hit for, and that hit.
struct Scene {
    triangle: TraceTriangle,
    ray: TraceRay,
    hit: SurfaceHit,
}

/// A seeded scene: a triangle near a seeded centre and a ray that aims at a point inside it.
fn seeded_scene(seeds: &mut Seeds) -> Option<Scene> {
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
    let triangle = TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2]))?;
    let hit = hit_on(&triangle, &ray)?;
    Some(Scene { triangle, ray, hit })
}

/// The scene's ray, or for one case in four the same origin with the direction divided by a
/// seeded power of two (floored), when that still arrives on the face the hit faces: a short
/// direction. `None` when the shortened direction no longer arrives.
fn seeded_incoming(seeds: &mut Seeds, scene: &Scene) -> Option<TraceRay> {
    let shift = match seeds.range(0, 11) {
        0 => 8,
        1 => 20,
        2 => 31,
        _ => return Some(scene.ray),
    };
    let d = raw3(scene.ray.direction()).map(|c| c >> shift);
    let (plane, _) = facing_plane(&scene.triangle, &scene.hit);
    let arrives = dot(d.map(i128::from), plane) < 0;
    TraceRay::new(scene.ray.origin(), vec3(d)).filter(|_| arrives)
}

/// Checks a refracted direction `w` against Snell's law for `d` arriving on `n` from index `a`
/// into `b`. Returns a description of the first failure, or an empty string.
///
/// 1. Each component is within one raw unit (plus `2^-26`) of the unit vector along the
///    oracle's exact refracted vector.
/// 2. The law itself, from `w` alone: `|a^2 sin_i^2 - b^2 sin_t^2| <= 4 b^2 2^-32`, with the
///    squared sines taken from the cross products with `n`; `w` is within `2 * 2^-32` of the
///    plane of `d` and `n`; it is on the far side of `n`; and its part along the surface points
///    the way that of `d` does.
fn check_snell(d: Wide3, n: Wide3, indices: [i64; 2], w: Raw3) -> String {
    let oracle = Refraction::new(d, n, indices);
    let exact = oracle.refracted();
    let top = exact.iter().map(Int::abs).max().expect("three components");
    let slack = &top * &Int::new(1 << 38);
    if !within(w, &exact.clone().map(|c| &c * &Int::power_of_two(64)), 1, &slack) {
        return format!("direction {w:?} is not the oracle's for d {d:?} n {n:?} {indices:?}");
    }
    let (d, n, w) = (int3(d), int3(n), int3(w.map(i128::from)));
    let [a2, b2] = indices.map(|index| Int::new(i128::from(index) * i128::from(index)));
    let (dd, ww, nn) = (dot_int(&d, &d), dot_int(&w, &w), dot_int(&n, &n));
    let (sine_in, sine_out) = (cross(&d, &n), cross(&w, &n));
    let law = &(&(&a2 * &dot_int(&sine_in, &sine_in)) * &ww)
        - &(&(&b2 * &dot_int(&sine_out, &sine_out)) * &dd);
    let unit = &(&dd * &ww) * &nn;
    if &law.abs() * &Int::power_of_two(32) > &(&b2 * &unit) * &Int::new(4) {
        return format!("sines of {w:?} break Snell's law for d {d:?} n {n:?} {indices:?}");
    }
    let out_of_plane = dot_int(&w, &sine_in);
    if &(&out_of_plane * &out_of_plane) * &Int::power_of_two(64) > &unit * &Int::new(4) {
        return format!("{w:?} leaves the plane of incidence of d {d:?} n {n:?}");
    }
    let through = dot_int(&w, &n).is_negative();
    let along = !dot_int(&sine_out, &sine_in).is_negative();
    if through && along {
        String::new()
    } else {
        format!("{w:?} is on the wrong side for d {d:?} n {n:?}")
    }
}

/// Whether the cosine of the angle between `d` and the line of `n` is below 1/16.
fn is_grazing(d: Wide3, n: Wide3) -> bool {
    let (d, n) = (int3(d), int3(n));
    let along = dot_int(&d, &n);
    &(&along * &along) * &Int::new(256) < &dot_int(&d, &d) * &dot_int(&n, &n)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_185_refracted_directions_satisfy_snells_law_on_seeded_hits() {
    let seed = DIELECTRIC_TEST_SEED ^ 0x0162;
    println!("OPTICS-TP-185 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    // Refracted, reflected (total or a reflectance of 1.0), refused as grazing, no scene.
    let mut counts = [0_u32; 4];
    // Entering, leaving, indices exactly one raw unit apart, incidence cosine below 1/16,
    // directions shorter than 0.5, equal indices.
    let mut reach = [0_u32; 6];
    for case in 0..3_000 {
        let scene = seeded_scene(&mut seeds);
        let boundary = seeds.boundary();
        let Some(scene) = scene else {
            counts[3] += 1;
            continue;
        };
        let Some(incoming) = seeded_incoming(&mut seeds, &scene) else { continue };
        let (d, n) = (wide(incoming.direction()), wide(scene.hit.frame().normal()));
        let indices = ordered(&boundary, &scene.hit);
        reach[usize::from(!scene.hit.front_face())] += 1;
        reach[2] += u32::from((indices[0] - indices[1]).abs() == 1);
        reach[5] += u32::from(indices[0] == indices[1]);
        reach[3] += u32::from(is_grazing(d, n));
        reach[4] += u32::from(d.iter().all(|c| c.abs() < i128::from(ONE / 2)));
        let label = format!("seed {seed:#x} case {case}");
        let reflectance = scene.hit.dielectric_reflectance(&incoming, &boundary);
        let closed = reflectance.expect("arrives").probability().raw() == ONE;
        match scatter(&scene.hit, &incoming, &boundary, LAST_CHOICE) {
            Ok(DielectricScatter::Refracted(sample)) => {
                counts[0] += 1;
                assert!(!closed, "{label}: refracted through a reflectance of 1.0");
                let failure = check_snell(d, n, indices, raw3(sample.ray().direction()));
                assert!(failure.is_empty(), "{label}: {failure}");
            }
            Ok(DielectricScatter::Reflected(_))
            | Err(SecondaryRayError::ReflectionBelowSurface { .. }) => {
                counts[1] += 1;
                assert!(closed, "{label}: reflected by the last choice below 1.0");
            }
            Err(SecondaryRayError::RefractionAboveSurface { .. }) => counts[2] += 1,
            Err(other) => panic!("{label}: unexpected refusal {other:?}"),
        }
    }
    println!("OPTICS-TP-185 refracted, reflected, grazing-refused, no scene: {counts:?}");
    println!("OPTICS-TP-185 entering, leaving, one apart, grazing, short, equal: {reach:?}");
    assert!(counts[0] >= 1_000 && counts[1] >= 100, "too few cases: {counts:?}");
    assert!(reach.iter().all(|&count| count >= 50), "a kind of input is not reached: {reach:?}");
}

/// The exact critical direction of TP-163: with `a = 1.25` and `b = 1.0`,
/// `(a^2 - b^2) / b^2 = 9 / 16`, so on the ground (`n` exactly `(0, -1, 0)` from below) a
/// direction `(x, y, 0)` is totally reflected exactly when `3 |x| >= 4 |y|`.
const CRITICAL: Raw3 = [ONE, 3 * ONE / 4, 0];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_186_total_internal_reflection_is_decided_exactly() {
    let ground = triangle(GROUND);
    let dense = boundary(ONE, 5 * ONE / 4);
    let probe = ray([0, -ONE, 0], [0, ONE, 0]);
    let hit = hit_on(&ground, &probe).expect("hits the ground from below");
    assert_eq!(raw3(hit.frame().normal()), [0, -ONE, 0]);
    // One raw unit steeper than the critical direction refracts; the critical direction and
    // one raw unit shallower are totally reflected, for every choice.
    for (lift, total) in [(1, false), (0, true), (-1, true)] {
        let d = [CRITICAL[0], CRITICAL[1] + lift, 0];
        let incoming = ray([0, -ONE, 0], d);
        let got = hit.dielectric_reflectance(&incoming, &dense).expect("arrives");
        assert_eq!(got == DielectricReflectance::Total, total, "lift {lift}");
        let oracle =
            Refraction::new(d.map(i128::from), [0, -i128::from(ONE), 0], [5 * ONE / 4, ONE]);
        assert_eq!(oracle.is_total(), total, "the oracle at lift {lift}");
        let last = scatter(&hit, &incoming, &dense, LAST_CHOICE);
        assert_eq!(matches!(last, Ok(DielectricScatter::Reflected(_))), total, "lift {lift}");
    }
    // The same three with every low bit in play: (4 j, 3 j, 0) for an odd j is exactly
    // critical, so (d . d) is odd and no power of two divides the terms that cancel.
    let j = 1_000_000_007;
    for (lift, total) in [(1, false), (0, true), (-1, true)] {
        let incoming = ray([0, -ONE, 0], [4 * j, 3 * j + lift, 0]);
        let got = hit.dielectric_reflectance(&incoming, &dense).expect("arrives");
        assert_eq!(got == DielectricReflectance::Total, total, "odd critical, lift {lift}");
    }
    // Indices one raw unit apart, at both ends of the range, with x = 1.0. The critical lift
    // is the largest y with b^2 y^2 <= (a^2 - b^2) x^2 (exact integers, asserted here):
    // a = 1.0 + 1 raw, b = 1.0: y^2 <= 2^33 + 1, so y = 92 681 (92 682^2 = 8 589 953 124);
    // a = 8.0, b = 8.0 - 1 raw: (2^35 - 1)^2 y^2 <= (2^36 - 1) 2^64, so y = 32 768.
    for (a, b, critical) in [(ONE + 1, ONE, 92_681_i64), (8 * ONE, 8 * ONE - 1, 32_768)] {
        let (a2, b2) = (i128::from(a) * i128::from(a), i128::from(b) * i128::from(b));
        let fits = |y: i64| b2 * i128::from(y) * i128::from(y) <= (a2 - b2) << 64;
        assert!(fits(critical) && !fits(critical + 1), "the critical lift for {a} / {b}");
        let near = boundary(b, a);
        for (lift, total) in [(1, false), (0, true), (-1, true)] {
            let d = [ONE, critical + lift, 0];
            let incoming = ray([0, -ONE, 0], d);
            let got = hit.dielectric_reflectance(&incoming, &near).expect("arrives");
            let label = format!("indices {a} / {b}, lift {lift}");
            assert_eq!(got == DielectricReflectance::Total, total, "{label}");
            let oracle = Refraction::new(d.map(i128::from), [0, -i128::from(ONE), 0], [a, b]);
            assert_eq!(oracle.is_total(), total, "the oracle for {label}");
        }
    }
    // 45 degrees inside glass (sin = 0.707 > 1 / 1.5) reflects as a mirror does.
    let slant = ray([-ONE, -ONE, 0], [ONE, ONE, 0]);
    let glass = boundary(ONE, GLASS);
    assert_eq!(hit.dielectric_reflectance(&slant, &glass), Ok(DielectricReflectance::Total));
    let back = reflected(scatter(&hit, &slant, &glass, LAST_CHOICE));
    assert_eq!(raw3(back.direction()), [HALF_SQRT2, -HALF_SQRT2, 0]);
    let mirror = hit.try_specular_reflection(&slant, &Albedo::new([fixed(ONE); 3]).expect("white"));
    assert_eq!(Ok(back), mirror.map(|sample| *sample.ray()));
}

/// The largest `y` in `0..=2^32` with `b^2 y^2 <= (a^2 - b^2) x^2`: the floor of the critical
/// `|y|` for a direction `(x, y, 0)` leaving index `a` for `b < a` through the ground.
fn critical_lift(a: i64, b: i64, x: i64) -> i64 {
    let square = |v: i64| Int::new(i128::from(v) * i128::from(v));
    let bound = &(&square(a) - &square(b)) * &square(x);
    let (mut low, mut high) = (0_i64, ONE + 1);
    while high - low > 1 {
        let mid = i64::midpoint(low, high);
        if &square(b) * &square(mid) <= bound {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_187_total_internal_reflection_matches_the_oracle_around_the_critical_angle() {
    let seed = DIELECTRIC_TEST_SEED ^ 0x0164;
    println!("OPTICS-TP-187 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let ground = triangle(GROUND);
    let probe = ray([0, -ONE, 0], [0, ONE, 0]);
    let hit = hit_on(&ground, &probe).expect("hits the ground from below");
    let n = wide(hit.frame().normal());
    // Total and partial, among directions within three raw units of the critical one; cases
    // with equal indices (never total), and the smallest difference of unequal indices.
    let mut counts = [0_u32; 2];
    let (mut equal, mut closest) = (0_u32, i64::MAX);
    for case in 0..1_500 {
        let (first, second) = (seeds.index(), seeds.index());
        let (inside, outside) = (first.max(second), first.min(second));
        let x = seeds.range(1, ONE);
        let lift = critical_lift(inside, outside, x) + seeds.range(-3, 3);
        if !(1..=ONE).contains(&lift) {
            continue;
        }
        let d = [x, lift, 0];
        let incoming = ray([0, -ONE, 0], d);
        let boundary = boundary(outside, inside);
        equal += u32::from(inside == outside);
        closest = closest.min(if inside == outside { i64::MAX } else { inside - outside });
        let oracle = Refraction::new(d.map(i128::from), n, [inside, outside]);
        let got = hit.dielectric_reflectance(&incoming, &boundary).expect("arrives");
        let total = got == DielectricReflectance::Total;
        assert_eq!(total, oracle.is_total(), "seed {seed:#x} case {case}: d {d:?}");
        counts[usize::from(!total)] += 1;
    }
    // On seeded triangles and boundaries, where the normal is a rounded unit vector.
    let mut tilted = [0_u32; 2];
    for case in 0..3_000 {
        let scene = seeded_scene(&mut seeds);
        let boundary = seeds.boundary();
        let Some(scene) = scene else { continue };
        let Some(incoming) = seeded_incoming(&mut seeds, &scene) else { continue };
        let (d, n) = (wide(incoming.direction()), wide(scene.hit.frame().normal()));
        let oracle = Refraction::new(d, n, ordered(&boundary, &scene.hit));
        let got = scene.hit.dielectric_reflectance(&incoming, &boundary).expect("arrives");
        let total = got == DielectricReflectance::Total;
        let expected = oracle.arrives() && oracle.is_total();
        assert_eq!(total, expected, "seed {seed:#x} tilted case {case}: d {d:?} n {n:?}");
        tilted[usize::from(!total)] += 1;
    }
    println!("OPTICS-TP-187 total, partial: near critical {counts:?}, seeded {tilted:?}");
    println!("OPTICS-TP-187 near critical: {equal} equal pairs, closest unequal {closest}");
    assert!(counts.iter().chain(&tilted).all(|&count| count >= 200), "{counts:?} {tilted:?}");
}

/// Whether raw `value` is within half a raw unit plus `2^-8` of `numerator / denominator`.
fn within_half_a_unit(value: i64, numerator: &Int, denominator: &Int) -> bool {
    let scaled = numerator * &Int::power_of_two(32);
    let error = (&(&Int::new(i128::from(value)) * denominator) - &scaled).abs();
    &error * &Int::power_of_two(9) <= denominator * &Int::new((1 << 8) + 2)
}

/// Checks the reflectance of one incidence against the oracle; returns whether it was partial.
fn check_reflectance(hit: &SurfaceHit, incoming: &TraceRay, boundary: &DielectricBoundary) -> bool {
    let (d, n) = (wide(incoming.direction()), wide(hit.frame().normal()));
    let oracle = Refraction::new(d, n, ordered(boundary, hit));
    let got = hit.dielectric_reflectance(incoming, boundary).expect("arrives");
    if !oracle.arrives() {
        // Along or behind the plane of the rounded normal: a reflectance of exactly 1.0.
        assert_eq!(got, DielectricReflectance::Partial(fixed(ONE)), "d {d:?} n {n:?}");
        return false;
    }
    if oracle.is_total() {
        assert_eq!(got, DielectricReflectance::Total, "d {d:?} n {n:?}");
        return false;
    }
    let DielectricReflectance::Partial(value) = got else { panic!("total for d {d:?} n {n:?}") };
    let (numerator, denominator) = oracle.reflectance();
    assert!(
        (0..=ONE).contains(&value.raw())
            && within_half_a_unit(value.raw(), &numerator, &denominator),
        "reflectance {} for d {d:?} n {n:?} {boundary:?}",
        value.raw()
    );
    true
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_188_the_reflectance_is_the_exact_fresnel_formula_rounded_once() {
    let seed = DIELECTRIC_TEST_SEED ^ 0x0165;
    println!("OPTICS-TP-188 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut partial = 0;
    for _ in 0..3_000 {
        let scene = seeded_scene(&mut seeds);
        let boundary = seeds.boundary();
        let Some(scene) = scene else { continue };
        let Some(incoming) = seeded_incoming(&mut seeds, &scene) else { continue };
        partial += u32::from(check_reflectance(&scene.hit, &incoming, &boundary));
    }
    // Near the critical angle, where the reflectance climbs to 1.0 with an unbounded slope.
    let ground = triangle(GROUND);
    let probe = ray([0, -ONE, 0], [0, ONE, 0]);
    let hit = hit_on(&ground, &probe).expect("hits the ground from below");
    let mut steep = 0;
    for _ in 0..1_000 {
        let (first, second) = (seeds.index(), seeds.index());
        let (inside, outside) = (first.max(second), first.min(second));
        let x = seeds.range(1, ONE);
        let reach = 1 << seeds.range(0, 20);
        let lift = critical_lift(inside, outside, x) + seeds.range(1, reach);
        if lift > ONE {
            continue;
        }
        let incoming = ray([0, -ONE, 0], [x, lift, 0]);
        steep += u32::from(check_reflectance(&hit, &incoming, &boundary(outside, inside)));
    }
    println!("OPTICS-TP-188 partial reflectances checked: seeded {partial}, near critical {steep}");
    assert!(partial >= 1_000 && steep >= 300, "too few cases: {partial} {steep}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_189_the_two_branches_share_the_choice_values_and_each_weighs_one() {
    let seed = DIELECTRIC_TEST_SEED ^ 0x0166;
    println!("OPTICS-TP-189 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let white = Albedo::new([fixed(ONE); 3]).expect("white");
    // Cases with both branches, with reflection only, with refraction only.
    let mut counts = [0_u32; 3];
    for case in 0..3_000 {
        let scene = seeded_scene(&mut seeds);
        let boundary = seeds.boundary();
        let Some(Scene { ray, hit, .. }) = scene else { continue };
        let label = format!("seed {seed:#x} case {case}");
        let reflectance = hit.dielectric_reflectance(&ray, &boundary).expect("arrives");
        let r = reflectance.probability().raw();
        assert!((0..=ONE).contains(&r), "{label}: reflectance {r}");
        // Reflected exactly for the r choice values 0..r, refracted for the 2^32 - r values
        // r..2^32: the boundary values of each interval decide it, and r + (2^32 - r) = 2^32.
        let mirror = hit.try_specular_reflection(&ray, &white);
        for choice in [0, r - 1].into_iter().filter(|c| (0..r).contains(c)) {
            let got = scatter(&hit, &ray, &boundary, choice);
            assert_eq!(got, mirror.map(DielectricScatter::Reflected), "{label}: choice {choice}");
            if let Ok(sample) = mirror {
                assert_eq!(sample.throughput(), [fixed(ONE); 3], "{label}");
            }
        }
        for choice in [r, LAST_CHOICE].into_iter().filter(|c| (r..ONE).contains(c)) {
            match scatter(&hit, &ray, &boundary, choice) {
                Ok(DielectricScatter::Refracted(sample)) => {
                    assert_eq!(sample.throughput(), [fixed(ONE); 3], "{label}");
                }
                Err(SecondaryRayError::RefractionAboveSurface { .. }) => {}
                other => panic!("{label}: choice {choice} gave {other:?}"),
            }
        }
        counts[if r == ONE {
            1
        } else if r == 0 {
            2
        } else {
            0
        }] += 1;
    }
    println!("OPTICS-TP-189 both branches, reflection only, refraction only: {counts:?}");
    assert!(counts[0] >= 1_000 && counts[1] >= 100, "too few cases: {counts:?}");
}

/// The two faces of a slab: `top`, and the same triangle moved by `offset` to the far side
/// with its winding reversed, so both front faces look out of the slab.
fn slab(top: [Raw3; 3], offset: Raw3) -> Option<[TraceTriangle; 2]> {
    let moved = |v: Raw3| vec3([0, 1, 2].map(|i| v[i] + offset[i]));
    let top_face = TraceTriangle::new(vec3(top[0]), vec3(top[1]), vec3(top[2]))?;
    Some([top_face, TraceTriangle::new(moved(top[0]), moved(top[2]), moved(top[1]))?])
}

/// What a ray did at a slab.
enum Transit {
    /// It left through the far face along this direction; the bound on each component's
    /// distance from the unit vector along the original direction, in raw units.
    Through { direction: Raw3, bound: i128 },
    /// It was not refracted twice: reflected, refused as grazing, or it missed the far face.
    Stopped,
}

/// Sends `incoming` through the slab `faces` with `boundary` on both faces, always choosing
/// refraction, and checks the far-side rules on the way.
fn transit(
    faces: &[TraceTriangle; 2],
    incoming: &TraceRay,
    boundary: &DielectricBoundary,
) -> Transit {
    let Some(entry) = hit_on(&faces[0], incoming).filter(SurfaceHit::front_face) else {
        return Transit::Stopped;
    };
    let Ok(DielectricScatter::Refracted(inside)) = scatter(&entry, incoming, boundary, LAST_CHOICE)
    else {
        return Transit::Stopped;
    };
    check_start(&faces[0], &entry, incoming, inside.ray());
    let Some(exit) = hit_on(&faces[1], inside.ray()) else { return Transit::Stopped };
    assert!(!exit.front_face(), "the far face is met from inside");
    let Ok(DielectricScatter::Refracted(outside)) =
        scatter(&exit, inside.ray(), boundary, LAST_CHOICE)
    else {
        return Transit::Stopped;
    };
    check_start(&faces[1], &exit, inside.ray(), outside.ray());
    let d = wide(incoming.direction());
    let oracle = Refraction::new(d, wide(entry.frame().normal()), ordered(boundary, &entry));
    let [a, b] = ordered(boundary, &entry).map(i128::from);
    let (high, low) = (a.max(b), a.min(b));
    let inverse_cosine = oracle.inverse_rarer_cosine();
    let bound = 1 + (6 * high * inverse_cosine + low - 1) / low;
    Transit::Through { direction: raw3(outside.ray().direction()), bound }
}

/// Checks the start of a refracted `ray` that left `hit` on `face`, where `incoming` reached
/// it: it is the first point strictly on the far side of the exact plane, one to five raw
/// units from the hit's start point along the plane normal's dominant axis; it is within 6.5
/// raw units of the exact hit point on that axis and 1.5 on the others; and the ray does not
/// meet the face it left at any distance.
fn check_start(face: &TraceTriangle, hit: &SurfaceHit, incoming: &TraceRay, ray: &TraceRay) {
    let (plane, offset) = facing_plane(face, hit);
    let (start, origin) = (wide(hit.point()), wide(ray.origin()));
    let magnitudes = plane.map(i128::unsigned_abs);
    let axis = (0..3).rev().max_by_key(|&i| magnitudes[i]).expect("three axes");
    let mut step = [0_i128; 3];
    step[axis] = plane[axis].signum();
    let steps = (start[axis] - origin[axis]) * step[axis];
    assert_eq!(origin, [0, 1, 2].map(|i| start[i] - steps * step[i]), "one axis moves");
    assert!((1..=5).contains(&steps), "{steps} steps to the far side");
    assert!(dot(plane, origin) < offset, "the start is strictly on the far side");
    let previous = [0, 1, 2].map(|i| origin[i] + step[i]);
    assert!(dot(plane, previous) >= offset, "no nearer point is on the far side");
    assert_eq!(ray.hit_triangle(face, fixed(0), MAX), None, "the ray meets the face it left");
    // The exact hit point is o + d (offset - N . o) / (N . d): compare without dividing.
    let (o, d) = (wide(incoming.origin()), wide(incoming.direction()));
    let (rate, height) = (Int::new(dot(plane, d)), Int::new(offset - dot(plane, o)));
    for i in 0..3 {
        let moved = &(&Int::new(origin[i] - o[i]) * &rate) - &(&Int::new(d[i]) * &height);
        let halves = if i == axis { 13 } else { 3 };
        let near = &moved.abs() * &Int::new(2) <= &rate.abs() * &Int::new(halves);
        assert!(near, "axis {i}: the start {origin:?} is too far from the exact hit point");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_190_a_parallel_slab_returns_the_original_direction() {
    // A glass slab between y = 0 and y = -1, entered along (4, -3) / 5: cos = 0.6 outside,
    // so the bound is 1 + 6 * 1.5 / 0.6 = 16 raw units (17 with 1 / cos rounded up to 2).
    let faces = slab(GROUND, [0, -ONE, 0]).expect("inside the trace domain");
    let glass = boundary(ONE, GLASS);
    let incoming = ray([-ONE, 3 * ONE / 4, 0], [ONE, -3 * ONE / 4, 0]);
    let Transit::Through { direction, bound } = transit(&faces, &incoming, &glass) else {
        panic!("the hand-built slab is crossed");
    };
    // The unit vector (0.8, -0.6, 0) is raw (3 435 973 836.8, -2 576 980 377.6, 0); the
    // round trip gives (..837, ..377) (model), 0.6 raw units from it.
    assert_eq!((direction, bound), ([3_435_973_837, -2_576_980_377, 0], 19));
    let seed = DIELECTRIC_TEST_SEED ^ 0x0167;
    println!("OPTICS-TP-190 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    // Crossed with a dense slab, crossed with a rarer slab, the largest deviation from the
    // rounded unit vector along the original direction and the bound of that case.
    let (mut crossed, mut worst) = ([0_u32; 2], (0_i64, 0_i128));
    for case in 0..6_000 {
        let bits = seeds.range(30, 36);
        let centre = seeds.triple(TRACE_COORD_LIMIT_RAW / 4);
        let top = [0; 3].map(|_| {
            let offset = seeds.triple(1 << bits);
            [0, 1, 2].map(|i| centre[i] + offset[i])
        });
        let (thickness, boundary) = (seeds.range(1 << 16, 1 << 28), seeds.boundary());
        let d = seeds.direction();
        // Aim at the centroid from two units back along the direction.
        let target: Raw3 = [0, 1, 2].map(|i| (top[0][i] + top[1][i] + top[2][i]) / 3);
        let origin = [0, 1, 2].map(|i| target[i] - d[i] * 2);
        let Some(incoming) = TraceRay::new(vec3(origin), vec3(d)) else { continue };
        let Some(first) = hit_on(&triangle(top), &incoming).filter(SurfaceHit::front_face) else {
            continue;
        };
        // Move the far face against the front normal on every axis.
        let offset = raw3(first.frame().normal()).map(|c| -c.signum() * thickness);
        let Some(faces) = slab(top, offset) else { continue };
        let Transit::Through { direction, bound } = transit(&faces, &incoming, &boundary) else {
            continue;
        };
        if bound > 1 + 6 * 8 * 1_024 {
            continue; // The bound is stated for a cosine of at least 2^-10.
        }
        let exact = int3(d.map(i128::from));
        assert!(
            within(direction, &exact, bound, &Int::new(0)),
            "seed {seed:#x} case {case}: {direction:?} is not within {bound} of d {d:?}"
        );
        crossed[usize::from(boundary.outside() > boundary.inside())] += 1;
        let unit = raw3(FixedUnitVec3::new(vec3(d)).expect("a direction").into_inner());
        let deviation = (0..3).map(|i| (direction[i] - unit[i]).abs()).max().expect("three");
        if deviation > worst.0 {
            worst = (deviation, bound);
        }
    }
    println!(
        "OPTICS-TP-190 crossed dense, rarer: {crossed:?}; worst deviation, its bound: {worst:?}"
    );
    assert!(crossed[0] >= 1_000 && crossed[1] >= 150, "too few slabs crossed: {crossed:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_191_a_refracted_ray_starts_strictly_on_the_far_side() {
    let seed = DIELECTRIC_TEST_SEED ^ 0x0168;
    println!("OPTICS-TP-191 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    // Refracted rays by the number of raw-unit steps from the hit's start point.
    let mut steps = [0_u32; 6];
    for _ in 0..4_000 {
        let scene = seeded_scene(&mut seeds);
        let boundary = seeds.boundary();
        let Some(Scene { triangle, ray, hit }) = scene else { continue };
        let Ok(DielectricScatter::Refracted(sample)) = scatter(&hit, &ray, &boundary, LAST_CHOICE)
        else {
            continue;
        };
        check_start(&triangle, &hit, &ray, sample.ray());
        let moved = (0..3).map(|i| (raw3(hit.point())[i] - raw3(sample.ray().origin())[i]).abs());
        steps[usize::try_from(moved.max().expect("three axes")).expect("a step count")] += 1;
    }
    println!("OPTICS-TP-191 refracted rays by steps 0..=5: {steps:?}");
    assert!(steps.iter().sum::<u32>() >= 1_000 && steps[0] == 0, "{steps:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_192_a_dielectric_sample_names_each_refusal() {
    let ground = triangle(GROUND);
    let glass = boundary(ONE, GLASS);
    let down = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let hit = hit_on(&ground, &down).expect("hits the ground");
    // A choice outside [0, 1) is refused first, even for a ray that arrives from below.
    for (choice, d) in [(-1, [0, -ONE, 0]), (ONE, [0, -ONE, 0]), (ONE, [0, ONE, 0])] {
        let incoming = ray([0, ONE, 0], d);
        let expected = SecondaryRayError::ChoiceOutsideUnitInterval { choice: fixed(choice) };
        assert_eq!(scatter(&hit, &incoming, &glass, choice), Err(expected));
        assert_eq!(expected.to_string(), format!("choice raw {choice} is outside [0, 1)"));
    }
    // Up from below, and along the plane: d . N >= 0 for the face this hit faces.
    for d in [[0, ONE, 0], [ONE, 0, 0], [ONE, 1, -ONE]] {
        let incoming = ray([0, ONE, 0], d);
        let expected = SecondaryRayError::ArrivesBelowSurface { direction: vec3(d) };
        assert_eq!(scatter(&hit, &incoming, &glass, 0), Err(expected));
        assert_eq!(hit.dielectric_reflectance(&incoming, &glass), Err(expected));
    }
    // A face one raw unit inside the trace domain: the reflected ray starts inside it, the
    // refracted ray would start on its boundary.
    let floor = -(TRACE_COORD_LIMIT_RAW - 1);
    let edge = triangle(GROUND.map(|[x, _, z]| [x, floor, z]));
    let low = ray([0, floor + ONE, 0], [0, -ONE, 0]);
    let hit = hit_on(&edge, &low).expect("hits the lowest face");
    assert_eq!(raw3(hit.point()), [0, floor + 1, 0]);
    let start = vec3([0, -TRACE_COORD_LIMIT_RAW, 0]);
    let expected = SecondaryRayError::StartOutsideTraceDomain { start };
    assert_eq!(scatter(&hit, &low, &glass, LAST_CHOICE), Err(expected));
    assert_eq!(raw3(reflected(scatter(&hit, &low, &glass, 0)).origin()), [0, floor + 1, 0]);
}

/// Found by a search over seeded triangles and directions nearly along an edge: a ray that
/// arrives from above the exact plane (`d . N < 0`) at a boundary with equal indices, whose
/// refracted direction (the rounded unit vector along `d`) is not below the exact plane.
const GRAZING: [Raw3; 3] = [
    [16_112_059_177, 17_062_346_931, 248_015_783],
    [6_524_064_973, -6_801_004_144, -15_375_713_246],
    [-1_448_506_600, -8_576_697_931, 2_982_270_311],
];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_193_a_refraction_that_stays_above_the_plane_is_refused() {
    let d = [-1_198_499_278, -2_982_918_887, -1_952_966_126];
    let grazing = ray([8_261_038_461, 3_544_467_172, -2_095_509_591], d);
    let face = triangle(GRAZING);
    let hit = hit_on(&face, &grazing).expect("the kernel reports the hit");
    let (plane, _) = facing_plane(&face, &hit);
    assert!(dot(d.map(i128::from), plane) < 0, "the case's premise: the ray arrives");
    let matched = boundary(15_702_691_695, 15_702_691_695);
    assert_eq!(hit.dielectric_reflectance(&grazing, &matched), Ok(partial(0)));
    let direction = [-1_368_503_082, -3_406_037_671, -2_229_988_963];
    let expected = SecondaryRayError::RefractionAboveSurface { direction: vec3(direction) };
    assert_eq!(scatter(&hit, &grazing, &matched, 0), Err(expected));
    assert!(dot(direction.map(i128::from), plane) >= 0);
    assert!(expected.to_string().contains("does not go through the surface"));
}

/// Found by a search over index pairs and the continued-fraction convergents of the critical
/// slope (Python, unbounded integers): leaving index `a` for 1.0 through the ground, this
/// direction is short of the critical angle (`K > 0`), but so close that the reflectance
/// rounds to 1.0.
const NEARLY_CRITICAL: (i64, Raw3) = (8_870_361_974, [1_486_455_473, 2_686_100_064, 0]);

/// The grazing case of `OPTICS-TP-107`: a kernel hit whose ray arrives from above the exact
/// plane (`d . N < 0`) but has `d . n > 0` against the rounded unit normal.
const BEHIND_THE_ROUNDED_NORMAL: [Raw3; 3] = [
    [-14_247_233_187, -16_518_692_768, 8_255_702_029],
    [14_994_902_859, -9_167_710_013, 11_680_992_187],
    [-2_466_095_447, 12_133_874_043, -15_050_646_404],
];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_199_a_reflectance_of_one_has_no_refracted_ray() {
    // Short of the critical angle: a refracted ray exists in exact arithmetic, the reflectance
    // is `Partial`, not `Total`, and it is exactly 1.0, so every choice value reflects.
    let ground = triangle(GROUND);
    let (inside, d) = NEARLY_CRITICAL;
    let incoming = ray([0, -ONE, 0], d);
    let hit = hit_on(&ground, &incoming).expect("hits the ground from below");
    let oracle = Refraction::new(d.map(i128::from), wide(hit.frame().normal()), [inside, ONE]);
    assert!(oracle.arrives() && !oracle.is_total(), "the case's premise: short of critical");
    let dense = boundary(ONE, inside);
    assert_eq!(hit.dielectric_reflectance(&incoming, &dense), Ok(partial(ONE)));
    for choice in [0, ONE / 2, LAST_CHOICE] {
        let _ = reflected(scatter(&hit, &incoming, &dense, choice));
    }
    // Along or behind the plane of the rounded normal, but arriving against the exact face:
    // no refraction is computed, the reflectance is exactly 1.0, and the mirror's own refusal
    // of a reflection under the plane is the result for every choice value.
    let d = [ONE, 441_121_301, 1_116_683_283];
    let grazing = ray([-4_867_775_887, -4_958_630_880, 511_999_321], d);
    let face = triangle(BEHIND_THE_ROUNDED_NORMAL);
    let hit = hit_on(&face, &grazing).expect("the kernel reports the hit");
    let (plane, _) = facing_plane(&face, &hit);
    let (dw, n) = (d.map(i128::from), wide(hit.frame().normal()));
    assert!(dot(dw, plane) < 0 && dot(dw, n) > 0, "the case's premise");
    let glass = boundary(ONE, GLASS);
    assert_eq!(hit.dielectric_reflectance(&grazing, &glass), Ok(partial(ONE)));
    let mirror =
        hit.try_specular_reflection(&grazing, &Albedo::new([fixed(ONE); 3]).expect("white"));
    assert!(matches!(mirror, Err(SecondaryRayError::ReflectionBelowSurface { .. })));
    for choice in [0, LAST_CHOICE] {
        let got = scatter(&hit, &grazing, &glass, choice);
        assert_eq!(got, mirror.map(DielectricScatter::Reflected), "choice {choice}");
    }
}
