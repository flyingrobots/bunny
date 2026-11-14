//! Typed refusals of the secondary-ray steps of `bunny-diffuse-shading/1`: one constructed case
//! per reachable `SecondaryRayError` variant, and a seeded cross-version differential case that
//! the `Option` forms keep their 0.7.1 outputs and agree with the `try_` forms.

mod shading_reference;

use std::hash::{BuildHasher, Hasher};

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{point_at, SecondaryRayError, ShadingFrame, SurfaceHit, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit, TRACE_COORD_LIMIT_RAW};
use shading_reference::HitRefusal;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const REFUSAL_TEST_SEED: u64 = 0xB002_0280_5EC0_0001;

type Raw3 = [i64; 3];

/// The ground: a triangle in the plane `y = 0` whose front face looks up (`+Y`).
const GROUND: [Raw3; 3] = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
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

/// A ray from 2 units above the ground straight down, and the hit the trace kernel reports.
fn ground_hit() -> (TriangleSurface, TraceRay, TriangleHit) {
    let ray = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let hit = ray.hit_triangle(&triangle(GROUND), fixed(0), MAX).expect("hits the ground");
    (surface(GROUND), ray, hit)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_057_an_unrepresentable_hit_point_names_its_distance() {
    let (surface, ray, mut hit) = ground_hit();
    // The direction's y is -1.0 and -1.0 * (raw i64::MIN) is 2^63 raw, which Q32.32 cannot hold.
    hit.distance = fixed(i64::MIN);
    let refusal = SurfaceHit::try_new(&surface, &ray, &hit).err();
    let expected = SecondaryRayError::HitPointUnrepresentable { distance: fixed(i64::MIN) };
    assert_eq!(refusal, Some(expected));
    assert_eq!(
        expected.to_string(),
        "hit point at distance raw -9223372036854775808 is not representable in Q32.32"
    );
    assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_058_a_hit_point_beyond_the_domain_is_refused_before_the_plane_test() {
    let (surface, ray, mut hit) = ground_hit();
    // 1 024 units back along the ray: y = 2 + 1 024 units, far above the domain's 256.
    hit.distance = fixed(-1024 * ONE);
    let point = vec3([0, 1026 * ONE, 0]);
    let refusal = SurfaceHit::try_new(&surface, &ray, &hit).err();
    assert_eq!(refusal, Some(SecondaryRayError::HitPointOutsideTraceDomain { point }));
    assert_eq!(
        SecondaryRayError::HitPointOutsideTraceDomain { point }.to_string(),
        "hit point raw (0, 4406636445696, 0) is outside the trace domain"
    );
    assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None);
    // 2^30 units back: y is 2^62 + 2 units raw, and the ground's plane normal is 256 units
    // squared (2^72 raw), so the exact plane test of 0.7.1 would need 2^134. It is refused
    // before that test, in debug and release builds alike.
    hit.distance = fixed(-(1 << 62));
    let point = vec3([0, (1 << 62) + 2 * ONE, 0]);
    let refusal = SurfaceHit::try_new(&surface, &ray, &hit).err();
    assert_eq!(refusal, Some(SecondaryRayError::HitPointOutsideTraceDomain { point }));
    assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_059_a_hit_off_the_arrival_side_names_its_point() {
    let (surface, ray, mut hit) = ground_hit();
    // At distance zero the point is the ray origin, 2 units above the ground; with the face
    // flipped the arrival side is below, which two raw-unit steps down cannot reach.
    hit.distance = fixed(0);
    hit.front_face = false;
    let point = vec3([0, 2 * ONE, 0]);
    let refusal = SurfaceHit::try_new(&surface, &ray, &hit).err();
    assert_eq!(refusal, Some(SecondaryRayError::NotOnArrivalSide { point }));
    assert_eq!(
        SecondaryRayError::NotOnArrivalSide { point }.to_string(),
        "hit point raw (0, 8589934592, 0) is not within two steps of the arrival side"
    );
    assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_060_a_start_point_outside_the_domain_names_the_stepped_point() {
    // The plane 2x + 4y = 4Y touches y = Y, the top of the domain, at x = 0; the ray along that
    // line arrives exactly there, and the one step goes up the dominant y axis to y = 2^40 raw.
    let top = TRACE_COORD_LIMIT_RAW - 1;
    let edge = [[0, top, -ONE], [0, top, ONE], [2 * ONE, top - ONE, 0]];
    let ray = ray([2 * ONE, top, 0], [-ONE, 0, 0]);
    let hit = ray.hit_triangle(&triangle(edge), fixed(0), MAX).expect("hits the edge");
    let start = vec3([0, TRACE_COORD_LIMIT_RAW, 0]);
    let refusal = SurfaceHit::try_new(&surface(edge), &ray, &hit).err();
    assert_eq!(refusal, Some(SecondaryRayError::StartOutsideTraceDomain { start }));
    assert_eq!(
        SecondaryRayError::StartOutsideTraceDomain { start }.to_string(),
        "start point raw (0, 1099511627776, 0) is outside the trace domain"
    );
    assert_eq!(SurfaceHit::new(&surface(edge), &ray, &hit), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_061_a_sample_outside_the_unit_interval_is_named() {
    let (surface, ray, hit) = ground_hit();
    let shading = SurfaceHit::try_new(&surface, &ray, &hit).expect("a valid hit");
    let frame = shading.frame();
    let outside = [(ONE, 0), (-1, 0), (0, ONE), (0, -1), (i64::MIN, i64::MAX), (ONE - 1, ONE)];
    for (u, v) in outside {
        let sample = [fixed(u), fixed(v)];
        let expected = Some(SecondaryRayError::SampleOutsideUnitInterval { sample });
        assert_eq!(frame.try_cosine_direction(sample).err(), expected, "u = {u}, v = {v}");
        assert_eq!(shading.try_bounce_ray(sample).err(), expected, "u = {u}, v = {v}");
        assert_eq!(frame.cosine_direction(sample), None, "u = {u}, v = {v}");
        assert_eq!(shading.bounce_ray(sample), None, "u = {u}, v = {v}");
    }
    let sample = [fixed(ONE), fixed(-1)];
    assert_eq!(
        SecondaryRayError::SampleOutsideUnitInterval { sample }.to_string(),
        "sample raw (4294967296, -1) is outside [0, 1)"
    );
}

/// The defensive `BounceRayRefused` cannot be produced through the public API: a `SurfaceHit`'s
/// start point is inside the trace domain and every bounce direction is a valid trace direction
/// (`OPTICS-TP-062` and `OPTICS-TP-064` count that it never occurs). This case names the variant
/// and locks its values and text, built directly.
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_065_the_defensive_bounce_refusal_names_its_ray() {
    let (origin, direction) = (vec3([0, 1, 0]), vec3([0, 0, 0]));
    let refusal = SecondaryRayError::BounceRayRefused { origin, direction };
    assert!(matches!(
        refusal,
        SecondaryRayError::BounceRayRefused { origin: o, direction: d } if o == origin && d == direction
    ));
    assert_eq!(
        refusal.to_string(),
        "bounce ray from raw (0, 1, 0) along raw (0, 0, 0) is not a trace ray"
    );
}

/// `SplitMix64`: a seeded source of full 64-bit draws for the generated inputs.
struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = i128::from(high) - i128::from(low) + 1;
        let offset = i128::from(self.next()) % span;
        i64::try_from(i128::from(low) + offset).expect("inside the range")
    }

    fn raw(&mut self) -> i64 {
        i64::from_ne_bytes(self.next().to_ne_bytes())
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    /// A hit distance: the full raw range, large values of either sign (up to 8 192 units),
    /// values within `2^41` raw of either end of the raw range, one that puts the point within
    /// two raw units of the trace domain's boundary on the ray's dominant axis, or a few raw
    /// units from the reported distance.
    fn distance(&mut self, reported: FixedQ32_32, ray: &TraceRay) -> FixedQ32_32 {
        match self.range(0, 4) {
            0 => fixed(self.raw()),
            1 => fixed(self.range(-(1 << 45), 1 << 45)),
            2 => fixed(self.range(i64::MAX - (1 << 41), i64::MAX) ^ -self.range(0, 1)),
            3 => self.boundary_distance(ray),
            _ => fixed(reported.raw().saturating_add(self.range(-4, 4))),
        }
    }

    /// The distance at which the ray's dominant coordinate (direction exactly +-1.0, so the
    /// product is exact) has magnitude `TRACE_COORD_LIMIT_RAW - 2` to `TRACE_COORD_LIMIT_RAW + 1`.
    fn boundary_distance(&mut self, ray: &TraceRay) -> FixedQ32_32 {
        let (origin, direction) = (ray.origin(), ray.direction());
        let axes = [(origin.x, direction.x), (origin.y, direction.y), (origin.z, direction.z)];
        let (o, d) = axes.into_iter().find(|(_, d)| d.raw().abs() == ONE).expect("dominant");
        let side = if self.range(0, 1) == 0 { 1 } else { -1 };
        let target = side * (TRACE_COORD_LIMIT_RAW + self.range(-2, 1));
        fixed((target - o.raw()) * d.raw().signum())
    }

    /// A sample coordinate: inside `[0, 1)`, an edge value, or a wide value of either sign.
    fn coordinate(&mut self) -> FixedQ32_32 {
        const EDGES: [i64; 6] = [0, ONE - 1, ONE, -1, i64::MIN, i64::MAX];
        match self.range(0, 3) {
            0 | 1 => fixed(self.range(0, ONE - 1)),
            2 => fixed(EDGES[usize::try_from(self.range(0, 5)).expect("small")]),
            _ => fixed(self.range(-(1 << 34), 1 << 34)),
        }
    }
}

/// How often each outcome occurred: the corpus classification of `OPTICS-TP-062`.
#[derive(Debug, Default)]
struct Witness {
    hits: usize,
    unrepresentable: usize,
    beyond_reach: usize,
    overflow: usize,
    not_on_arrival_side: usize,
    outside: usize,
    directions: usize,
    samples_refused: usize,
    bounce_rays: usize,
    bounce_refused: usize,
}

/// A seeded triangle within about 64 units of the origin, at a scale from 2^-24 to 16 units,
/// and a ray to a seeded point on it, or `None` for a degenerate triangle or a ray that misses.
fn seeded_scene(seeds: &mut Seeds) -> Option<(TraceTriangle, TriangleSurface, TraceRay)> {
    let bound = 1_i64 << seeds.range(8, 36);
    let centre = seeds.triple(1 << 38);
    let mut corner = || {
        let offset = seeds.triple(bound);
        [0, 1, 2].map(|i| centre[i] + offset[i])
    };
    let vertices = [corner(), corner(), corner()];
    let triangle = TraceTriangle::new(vec3(vertices[0]), vec3(vertices[1]), vec3(vertices[2]))?;
    let surface = TriangleSurface::new(&triangle)?;
    // A point inside the triangle: v0 + a (v1 - v0) + b (v2 - v0), a + b <= 1, in 2^-16 steps.
    let (a, b) = (seeds.range(0, 1 << 16), seeds.range(0, 1 << 16));
    let (a, b) = if a + b > 1 << 16 { ((1 << 16) - a, (1 << 16) - b) } else { (a, b) };
    let target = [0, 1, 2].map(|i| {
        let along = |w: i64, v: Raw3| i128::from(w) * i128::from(v[i] - vertices[0][i]);
        i128::from(vertices[0][i]) + ((along(a, vertices[1]) + along(b, vertices[2])) >> 16)
    });
    let limit = i128::from(TRACE_COORD_LIMIT_RAW - 1);
    let offset = seeds.triple(1 << 38);
    let origin = [0, 1, 2].map(|i| (target[i] + i128::from(offset[i])).clamp(-limit, limit));
    let diff = [0, 1, 2].map(|i| target[i] - origin[i]);
    let largest = diff.iter().map(|c| c.abs()).max().filter(|m| *m > 0)?;
    let direction = diff.map(|c| c * i128::from(ONE) / largest);
    let narrow = |v: [i128; 3]| v.map(|c| i64::try_from(c).expect("inside the trace domain"));
    let ray = TraceRay::new(vec3(narrow(origin)), vec3(narrow(direction)))?;
    Some((triangle, surface, ray))
}

/// Whether a coordinate of `point` is at least `TRACE_COORD_LIMIT_RAW + 2` raw units in
/// magnitude: two raw-unit steps cannot bring it into the trace domain.
fn beyond_reach(point: FixedVec3) -> bool {
    let reach = TRACE_COORD_LIMIT_RAW.unsigned_abs() + 2;
    [point.x, point.y, point.z].iter().any(|c| c.raw().unsigned_abs() >= reach)
}

/// The refusal the new code must give where the reference refused: the reference's cause, except
/// that a hit point beyond reach of the domain is named as such before the plane test.
fn expected_refusal(refusal: HitRefusal, hit_point: Option<FixedVec3>) -> SecondaryRayError {
    match (refusal, hit_point) {
        (HitRefusal::Unrepresentable(distance), _) => {
            SecondaryRayError::HitPointUnrepresentable { distance }
        }
        (_, Some(point)) if beyond_reach(point) => {
            SecondaryRayError::HitPointOutsideTraceDomain { point }
        }
        (HitRefusal::NotOnArrivalSide(point), _) => SecondaryRayError::NotOnArrivalSide { point },
        (HitRefusal::Outside(start), _) => SecondaryRayError::StartOutsideTraceDomain { start },
        (HitRefusal::Overflow(point), _) => panic!("overflow at {point:?}, inside reach"),
    }
}

/// Checks one hit against the reference and returns the accepted hit.
fn check_hit(
    scene: &(TraceTriangle, TriangleSurface, TraceRay),
    hit: &TriangleHit,
    witness: &mut Witness,
    context: &str,
) -> Option<SurfaceHit> {
    let (triangle, surface, ray) = scene;
    let reference = shading_reference::surface_hit(surface, triangle, ray, hit);
    let old = SurfaceHit::new(surface, ray, hit);
    let new = SurfaceHit::try_new(surface, ray, hit);
    assert_eq!(old, new.ok(), "{context}: Some exactly when Ok, same value");
    match reference {
        Ok((point, frame)) => {
            let accepted = old.unwrap_or_else(|| panic!("{context}: 0.7.1 accepted this hit"));
            assert_eq!(accepted.point(), point, "{context}: start point");
            assert_eq!(accepted.frame(), frame, "{context}: frame");
            witness.hits += 1;
        }
        Err(refusal) => {
            let expected = expected_refusal(refusal, point_at(ray, hit.distance));
            assert_eq!(new.err(), Some(expected), "{context}: refusal");
            match expected {
                SecondaryRayError::HitPointUnrepresentable { .. } => witness.unrepresentable += 1,
                SecondaryRayError::HitPointOutsideTraceDomain { .. } => witness.beyond_reach += 1,
                SecondaryRayError::NotOnArrivalSide { .. } => witness.not_on_arrival_side += 1,
                SecondaryRayError::StartOutsideTraceDomain { .. } => witness.outside += 1,
                _ => panic!("{context}: {expected:?} is not a surface-hit refusal"),
            }
            if matches!(refusal, HitRefusal::Overflow(_)) {
                witness.overflow += 1;
            }
        }
    }
    old
}

/// Checks one sample's direction and bounce ray against the reference.
fn check_sample(hit: &SurfaceHit, sample: [FixedQ32_32; 2], witness: &mut Witness, context: &str) {
    let frame: ShadingFrame = hit.frame();
    let reference = shading_reference::cosine_direction(&frame, sample);
    let (old, new) = (frame.cosine_direction(sample), frame.try_cosine_direction(sample));
    assert_eq!(old, reference, "{context}: direction keeps its 0.7.1 bits");
    assert_eq!(old, new.ok(), "{context}: direction Some exactly when Ok");
    let reference_ray = reference.and_then(|d| TraceRay::new(hit.point(), d));
    let (old_ray, new_ray) = (hit.bounce_ray(sample), hit.try_bounce_ray(sample));
    assert_eq!(old_ray, reference_ray, "{context}: bounce ray keeps its 0.7.1 bits");
    assert_eq!(old_ray, new_ray.ok(), "{context}: bounce ray Some exactly when Ok");
    match (reference, reference_ray) {
        (None, _) => {
            let refusal = Some(SecondaryRayError::SampleOutsideUnitInterval { sample });
            assert_eq!(new.err(), refusal, "{context}: direction refusal");
            assert_eq!(new_ray.err(), refusal, "{context}: bounce refusal");
            witness.samples_refused += 1;
        }
        (Some(direction), None) => {
            let origin = hit.point();
            let refusal = Some(SecondaryRayError::BounceRayRefused { origin, direction });
            assert_eq!(new_ray.err(), refusal, "{context}: defensive bounce refusal");
            witness.bounce_refused += 1;
        }
        (Some(_), Some(_)) => {
            witness.directions += 1;
            witness.bounce_rays += 1;
        }
    }
}

/// Runs `OPTICS-TP-062`'s differential check over 1 500 seeded scenes from `seed`; every failure
/// message names the seed, the case and the variant.
fn check_agreement(seed: u64) {
    let mut seeds = Seeds(seed);
    let mut witness = Witness::default();
    for case in 0..1_500 {
        let Some(scene) = seeded_scene(&mut seeds) else { continue };
        let Some(reported) = scene.2.hit_triangle(&scene.0, fixed(0), MAX) else { continue };
        let mut variants = [reported; 5];
        for variant in &mut variants[1..4] {
            variant.distance = seeds.distance(reported.distance, &scene.2);
        }
        variants[4].front_face = !reported.front_face;
        for (index, hit) in variants.iter().enumerate() {
            let context = format!("seed {seed:#x}, case {case}, variant {index}");
            let Some(accepted) = check_hit(&scene, hit, &mut witness, &context) else { continue };
            for draw in 0..3 {
                let sample = [seeds.coordinate(), seeds.coordinate()];
                check_sample(&accepted, sample, &mut witness, &format!("{context}, draw {draw}"));
            }
        }
    }
    let counts = format!("seed {seed:#x}: {witness:?}");
    assert!(witness.hits >= 1_000, "{counts}");
    assert!(witness.unrepresentable >= 20, "{counts}");
    assert!(witness.beyond_reach >= 100 && witness.overflow >= 10, "{counts}");
    assert!(witness.not_on_arrival_side >= 100, "{counts}");
    assert!(witness.outside >= 20, "{counts}");
    assert!(witness.directions >= 1_000 && witness.bounce_rays >= 1_000, "{counts}");
    assert!(witness.samples_refused >= 1_000, "{counts}");
    assert_eq!(witness.bounce_refused, 0, "{counts}: the defensive refusal is never reached");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_062_option_and_result_forms_agree_with_the_released_code() {
    check_agreement(REFUSAL_TEST_SEED);
}

/// The same check from a seed that differs per run, so coverage accumulates across runs. The
/// seed is printed in every failure message; rerun a failure with it as `REFUSAL_TEST_SEED`.
/// (On `wasm32-unknown-unknown` the standard library's hash keys are fixed, so there the seed
/// is the same every run.)
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_064_option_and_result_forms_agree_from_a_fresh_seed() {
    // dojo: allow unordered-container -- RandomState only draws this per-run seed, printed on failure
    let seed = std::collections::hash_map::RandomState::new().build_hasher().finish();
    check_agreement(seed);
}
