//! Evidence for `bunny-point-light/1`: hand-derived irradiance, irradiance against an exact
//! rational oracle on seeded inputs, the shadow ray that ends at the light, and the refusals.
//!
//! The oracles are exact big-integer arithmetic written here from the chapter's formulas; no
//! floating point and no value computed by the code under test.

// The shared oracle module; this file uses only its big-integer comparison of a product with a
// square root, so its unit-vector helpers are unused here.
#[allow(dead_code)]
mod exact;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    DirectionalLight, LightSample, PointLight, PointLightError, SurfaceHit, TriangleSurface,
    FRAC_1_PI, MAX_LIGHT_RAW, MIN_LIGHT_DISTANCE_RAW, POINT_LIGHT_ID, SHADOW_T_MAX,
};
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

use exact::{sqrt_product_at_most, Big, Signed};

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const POINT_LIGHT_TEST_SEED: u64 = 0xB002_0365_9011_0001;

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

fn light(position: Raw3, intensity: Raw3) -> PointLight {
    PointLight::new(vec3(position), intensity.map(fixed)).expect("a valid point light")
}

fn triangle(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("a trace triangle")
}

/// The hit on the ground of the ray from `(x, 2, z)` straight down.
fn ground_hit(x: i64, z: i64) -> SurfaceHit {
    let ray = TraceRay::new(vec3([x, 2 * ONE, z]), vec3([0, -ONE, 0])).expect("a trace ray");
    let ground = triangle(GROUND);
    let surface = TriangleSurface::new(&ground).expect("a surface");
    let hit = ray.hit_triangle(&ground, fixed(0), MAX).expect("the ray hits the ground");
    SurfaceHit::new(&surface, &ray, &hit).expect("a surface hit")
}

/// A square occluder in the plane `y = height`, wide enough to cover every shadow ray here.
fn occluder(height: i64) -> TraceTriangle {
    triangle([[-64 * ONE, height, -64 * ONE], [64 * ONE, height, -64 * ONE], [0, height, 64 * ONE]])
}

/// Whether the sample's shadow ray, queried as the chapter says, hits `blocker`.
fn shadowed(sample: &LightSample, blocker: &TraceTriangle) -> bool {
    let ray = sample.shadow_ray().expect("a shadow ray");
    ray.hit_triangle(blocker, fixed(0), sample.shadow_t_max()).is_some()
}

/// `value / 2^32` rounded to nearest, ties to even: the Q32.32 product rule, written here.
fn round_q(value: i128) -> i128 {
    let (floor, remainder) = (value >> 32, value & 0xFFFF_FFFF);
    let half = 1 << 31;
    floor + i128::from(remainder > half || (remainder == half && floor & 1 == 1))
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_120_hand_derived_irradiance_falls_off_with_the_square_of_distance() {
    assert_eq!(POINT_LIGHT_ID, "bunny-point-light/1");
    let up = vec3([0, ONE, 0]);
    // Straight above at 4 units: cosine 1, 1 / d^2 = 1/16; intensity (16, 1, 3) gives
    // (1.0, 1/16, 3/16) exactly.
    let above = light([0, 4 * ONE, 0], [16 * ONE, ONE, 3 * ONE]);
    let irradiance = above.irradiance(vec3([0; 3]), up).expect("lit");
    assert_eq!(irradiance.map(FixedQ32_32::raw), [ONE, ONE / 16, 3 * ONE / 16]);
    // Twice as far, a quarter of it.
    let far = light([0, 8 * ONE, 0], [16 * ONE, ONE, 3 * ONE]);
    let irradiance = far.irradiance(vec3([0; 3]), up).expect("lit");
    assert_eq!(irradiance.map(FixedQ32_32::raw), [ONE / 4, ONE / 64, 3 * ONE / 64]);
    // A normal facing away: the cosine clamps to zero, and so does the irradiance.
    let irradiance = above.irradiance(vec3([0; 3]), vec3([0, -ONE, 0])).expect("a point");
    assert_eq!(irradiance.map(FixedQ32_32::raw), [0; 3]);
    // At 2 units to the side and 2 up (d^2 = 8, cosine sqrt(1/2)), intensity 8. The cosine
    // rounds to raw 3 037 000 500 (the nearest to 2^32 / sqrt(2) = 3 037 000 499.98); the
    // division by d^2 gives 379 625 062.5, a tie, to the even 379 625 062; times 8 that is
    // 3 037 000 496. Ties upward would give 3 037 000 504.
    let side = light([2 * ONE, 2 * ONE, 0], [8 * ONE; 3]);
    let irradiance = side.irradiance(vec3([0; 3]), up).expect("lit");
    assert_eq!(irradiance.map(FixedQ32_32::raw), [3_037_000_496; 3]);
    // Through a hit: the start point is one raw unit above the ground, so a light 4 units up
    // is 2^34 - 1 raw away; 2^96 / (2^34 - 1)^2 rounds to 2^28 (1/16), intensity 16 gives 1.0,
    // and the radiance is FRAC_1_PI exactly (model: Python integers).
    let hit = ground_hit(0, 0);
    assert_eq!(raw3(hit.point()), [0, 1, 0]);
    let sample = hit.point_light_sample(&light([0, 4 * ONE, 0], [16 * ONE; 3])).expect("lit");
    assert_eq!(sample.radiance(true), [FRAC_1_PI; 3]);
    assert_eq!(sample.radiance(false), [FixedQ32_32::ZERO; 3]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_121_irradiance_is_within_the_stated_rounding_of_the_exact_value() {
    println!("OPTICS-TP-121 seed {POINT_LIGHT_TEST_SEED:#x}");
    let mut seeds = Seeds(POINT_LIGHT_TEST_SEED);
    let mut checked = 0;
    for case in 0..3_000 {
        let point = seeds.triple(TRACE_COORD_LIMIT_RAW - 1);
        // A third of the lights within 2^28 raw units (1/16 unit) of the point.
        let reach = if case % 3 == 0 { 1 << 28 } else { TRACE_COORD_LIMIT_RAW - 1 };
        let position = point.map(|c| {
            (c + seeds.range(-reach, reach))
                .clamp(1 - TRACE_COORD_LIMIT_RAW, TRACE_COORD_LIMIT_RAW - 1)
        });
        let normal = seeds.unit_vector();
        let intensity = [0; 3].map(|_| seeds.range(0, MAX_LIGHT_RAW));
        let lamp = light(position, intensity);
        let Ok(irradiance) = lamp.irradiance(vec3(point), vec3(normal)) else {
            continue;
        };
        let v = [0, 1, 2].map(|i| i128::from(position[i]) - i128::from(point[i]));
        for (e, i) in irradiance.iter().zip(intensity) {
            assert!(
                within_stated_rounding(e.raw(), i, dot(wide(vec3(normal)), v), v),
                "seed {POINT_LIGHT_TEST_SEED:#x} case {case}: light {position:?} point {point:?} \
                 normal {normal:?} intensity {i} gave {}",
                e.raw()
            );
        }
        checked += 1;
    }
    println!("OPTICS-TP-121 checked {checked} of 3000");
    assert!(checked > 2_900, "too few cases reached the oracle: {checked}");
}

/// Whether `e` (raw) is within `B = I (3 / d^2 + 1/2) + 1/2` raw units (with `I` and `d^2` in
/// units) of the exact irradiance `X = I max(n . v, 0) 2^32 / |v|^3` raw, decided exactly.
///
/// Every term is scaled by `2^33 Q` (`Q = v . v`, raw squared), so with `Bn = I (6 2^64 + Q) +
/// 2^32 Q` the claim is `(e 2^33 Q - Bn) sqrt(Q) <= I nv 2^65 <= (e 2^33 Q + Bn) sqrt(Q)`; the
/// upper side is decided as `-(e 2^33 Q + Bn) sqrt(Q) <= -I nv 2^65`.
fn within_stated_rounding(e: i64, intensity: i64, nv: i128, v: Wide3) -> bool {
    let q = dot(v, v);
    let scaled = Signed::product(i128::from(e), q, 33);
    let bound = Signed::product(i128::from(intensity), 6 * (1 << 64) + q, 0)
        .add(&Signed::product(q, 1, 32));
    let exact = Signed::product(i128::from(intensity), nv.max(0), 65);
    let big_q = Big::new(q.unsigned_abs());
    sqrt_product_at_most(&scaled.add(&bound.negated()), &big_q, &exact)
        && sqrt_product_at_most(&scaled.add(&bound).negated(), &big_q, &exact.negated())
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_122_the_shadow_ray_stops_at_the_light() {
    let hit = ground_hit(0, 0);
    let sample = hit.point_light_sample(&light([0, 4 * ONE, 0], [ONE; 3])).expect("lit");
    let ray = sample.shadow_ray().expect("a shadow ray");
    assert_eq!(ray.origin(), hit.point());
    assert_eq!(raw3(ray.direction()), [0, ONE, 0]);
    // The light is m = 4 units less the start point's height above the ground away.
    let m = 4 * ONE - hit.point().y.raw();
    assert_eq!(sample.shadow_t_max().raw(), m - 1);
    assert!(shadowed(&sample, &occluder(2 * ONE)), "between the hit and the light");
    assert!(shadowed(&sample, &occluder(4 * ONE - 1)), "one raw unit short of the light");
    assert!(!shadowed(&sample, &occluder(4 * ONE)), "in the light's plane");
    assert!(!shadowed(&sample, &occluder(6 * ONE)), "beyond the light");
    // A directional light's sample keeps SHADOW_T_MAX, and the same blocker beyond the point
    // light's position shadows it.
    let sun = DirectionalLight::new(vec3([0, ONE, 0]), [fixed(ONE); 3]).expect("a light");
    let sun_sample = hit.light_sample(&sun);
    assert_eq!(sun_sample.shadow_t_max(), SHADOW_T_MAX);
    assert!(shadowed(&sun_sample, &occluder(6 * ONE)));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_123_seeded_shadow_rays_reach_the_light_on_their_largest_axis() {
    let seed = POINT_LIGHT_TEST_SEED ^ 0x0123;
    println!("OPTICS-TP-123 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    let mut lit = 0;
    for case in 0..2_000 {
        let hit = ground_hit(seeds.range(-2 * ONE, 2 * ONE), seeds.range(0, 4 * ONE));
        let position = seeds.triple(TRACE_COORD_LIMIT_RAW - 1);
        let intensity = [0; 3].map(|_| seeds.range(0, MAX_LIGHT_RAW));
        let lamp = light(position, intensity);
        let Ok(sample) = hit.point_light_sample(&lamp) else {
            continue;
        };
        let v = [0, 1, 2].map(|i| i128::from(position[i]) - wide(hit.point())[i]);
        let context = format!("seed {seed:#x} case {case}: light {position:?}");
        // The ground's front face looks up: lit exactly when the light is above the start.
        let Some(ray) = sample.shadow_ray() else {
            assert!(v[1] <= 0, "{context}: dark above the surface");
            assert_eq!(sample.radiance(true), [FixedQ32_32::ZERO; 3], "{context}");
            assert_eq!(sample.shadow_t_max(), FixedQ32_32::ZERO, "{context}");
            continue;
        };
        assert!(v[1] > 0, "{context}: a shadow ray below the surface");
        check_shadow_ray(ray, sample.shadow_t_max(), v, &context);
        let normal = hit.frame().normal();
        let irradiance = lamp.irradiance(hit.point(), normal).expect("lit");
        let expected =
            irradiance.map(|e| round_q(i128::from(e.raw()) * i128::from(FRAC_1_PI.raw())));
        let radiance = sample.radiance(true).map(|r| i128::from(r.raw()));
        assert_eq!(radiance, expected, "{context}: radiance is irradiance times 1 / pi");
        assert_eq!(sample.radiance(false), [FixedQ32_32::ZERO; 3], "{context}");
        lit += 1;
    }
    println!("OPTICS-TP-123 lit {lit} of 2000");
    assert!(lit > 800, "too few lit samples: {lit}");
}

/// The ray's largest component is exactly 1.0 on the axis of `v`'s largest `m`, so at the
/// distance `m` it reaches the light on that axis; `t_max` is `m - 1`; every other component
/// is within half a raw unit of `v_i 2^32 / m`.
fn check_shadow_ray(ray: &TraceRay, t_max: FixedQ32_32, v: Wide3, context: &str) {
    let m = v.iter().map(|c| c.abs()).max().expect("three components");
    let axis = v.iter().position(|c| c.abs() == m).expect("the largest axis");
    let direction = wide(ray.direction());
    assert_eq!(direction[axis], v[axis].signum() * i128::from(ONE), "{context}");
    assert_eq!(i128::from(t_max.raw()), m - 1, "{context}");
    for i in 0..3 {
        let error = (direction[i] * m - v[i] * i128::from(ONE)).abs();
        assert!(2 * error <= m, "{context}: component {i} is not rounded to nearest");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_124_a_point_at_or_inside_the_minimum_distance_is_refused() {
    assert_eq!(MIN_LIGHT_DISTANCE_RAW, 1 << 25);
    let lamp = light([0; 3], [ONE; 3]);
    let up = vec3([0, ONE, 0]);
    for point in [[0; 3], [MIN_LIGHT_DISTANCE_RAW, 0, 0], [0, -MIN_LIGHT_DISTANCE_RAW, 0]] {
        let to_light = vec3(point.map(|c| -c));
        assert_eq!(
            lamp.irradiance(vec3(point), up),
            Err(PointLightError::TooNear { to_light }),
            "{point:?}"
        );
    }
    // One raw unit farther is accepted: 1 / d^2 is just below 2^14.
    let point = vec3([0, -(MIN_LIGHT_DISTANCE_RAW + 1), 0]);
    let irradiance = lamp.irradiance(point, up).expect("outside the minimum distance");
    assert!(irradiance.iter().all(|e| (0..1 << 46).contains(&e.raw())), "{irradiance:?}");
    // At a diagonal: (2^24, 2^24, 2^24) has d^2 = 3 2^48 <= 2^50 and is refused.
    let diagonal = vec3([1 << 24; 3]);
    assert!(matches!(lamp.irradiance(diagonal, up), Err(PointLightError::TooNear { .. })));
    // Through a surface hit: a light at the start point is refused, not divided by zero.
    let hit = ground_hit(0, 0);
    let at_hit = PointLight::new(hit.point(), [fixed(ONE); 3]).expect("a light");
    assert_eq!(
        hit.point_light_sample(&at_hit),
        Err(PointLightError::TooNear { to_light: vec3([0; 3]) })
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_125_lights_and_points_outside_the_domain_are_refused() {
    let limit = TRACE_COORD_LIMIT_RAW;
    let outside = vec3([0, limit, 0]);
    assert_eq!(
        PointLight::new(outside, [fixed(ONE); 3]),
        Err(PointLightError::PositionOutsideTraceDomain { position: outside })
    );
    assert!(PointLight::new(vec3([0, limit - 1, 0]), [fixed(ONE); 3]).is_ok());
    let origin = vec3([0; 3]);
    assert_eq!(
        PointLight::new(origin, [ONE, -1, ONE].map(fixed)),
        Err(PointLightError::NegativeIntensity)
    );
    assert_eq!(
        PointLight::new(origin, [ONE, MAX_LIGHT_RAW + 1, ONE].map(fixed)),
        Err(PointLightError::IntensityTooLarge)
    );
    assert!(PointLight::new(origin, [MAX_LIGHT_RAW; 3].map(fixed)).is_ok());
    let lamp = light([0; 3], [ONE; 3]);
    let far = vec3([-limit, 0, 0]);
    assert_eq!(
        lamp.irradiance(far, vec3([ONE, 0, 0])),
        Err(PointLightError::PointOutsideTraceDomain { point: far })
    );
    // A light along the surface's plane, or below it, gives a dark sample.
    let hit = ground_hit(0, 0);
    let level = light([4 * ONE, hit.point().y.raw(), 0], [ONE; 3]);
    let sample = hit.point_light_sample(&level).expect("far enough");
    assert!(sample.shadow_ray().is_none());
    assert_eq!(sample.radiance(true), [FixedQ32_32::ZERO; 3]);
    for error in [PointLightError::NegativeIntensity, PointLightError::TooNear { to_light: far }] {
        assert!(!error.to_string().is_empty());
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

    /// A unit normal, as a renderer's frame would give: `FixedUnitVec3::new` of a seeded
    /// nonzero vector (an input, not an oracle).
    fn unit_vector(&mut self) -> Raw3 {
        loop {
            let v = self.triple(ONE);
            if let Some(unit) = FixedUnitVec3::new(vec3(v.map(|c| c << 20))) {
                return raw3(unit.into_inner());
            }
        }
    }
}
