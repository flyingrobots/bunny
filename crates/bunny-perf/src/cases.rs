//! Benchmark cases: one closure per kernel, run over a fixed batch of seeded inputs.
//!
//! Every case uses only public Bunny APIs, so the same source builds against older
//! revisions for before/after comparisons. Inputs come from a fixed-seed generator, so
//! every run times the same work.

use std::hint::black_box;

#[cfg(feature = "query")]
use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

/// Calls per timed batch.
const BATCH: usize = 1024;
const PERF_SEED: u64 = 0xB002_9E2F_0000_0001;

/// A named kernel; `run` makes `calls` calls and returns a checksum. Each call's whole result
/// passes through `black_box` before the checksum reads part of it, so the optimizer cannot drop
/// work whose output the checksum ignores (such as the y and z divisions of `normalize`).
pub(crate) struct Case {
    pub(crate) name: &'static str,
    pub(crate) calls: usize,
    pub(crate) run: Box<dyn Fn() -> i64>,
}

/// Every case compiled into this build, in a stable order.
pub(crate) fn all() -> Vec<Case> {
    let mut cases = Vec::new();
    #[cfg(feature = "num")]
    cases.extend(num_cases());
    #[cfg(feature = "linalg")]
    cases.extend(linalg_cases());
    #[cfg(feature = "query")]
    cases.extend(query_cases());
    #[cfg(feature = "angle")]
    cases.extend(angle_cases());
    cases
}

fn case(name: &'static str, run: impl Fn() -> i64 + 'static) -> Case {
    Case { name, calls: BATCH, run: Box::new(run) }
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A value in `[-range, range)` Bunny units.
    #[allow(clippy::cast_possible_wrap)]
    fn fixed(&mut self, range: i64) -> FixedQ32_32 {
        let span = (2 * range * ONE_RAW).unsigned_abs();
        FixedQ32_32::from_raw((self.next() % span) as i64 - range * ONE_RAW)
    }

    fn vec3(&mut self, range: i64) -> FixedVec3 {
        FixedVec3::new(self.fixed(range), self.fixed(range), self.fixed(range))
    }

    fn batch<T>(&mut self, make: impl Fn(&mut Self) -> T) -> Vec<T> {
        (0..BATCH).map(|_| make(self)).collect()
    }
}

#[cfg(feature = "num")]
fn num_cases() -> Vec<Case> {
    let mut seeds = Seeds(PERF_SEED);
    let pairs = seeds.batch(|s| (s.fixed(1000), s.fixed(1000)));
    // Non-negative values spread up to about a million Bunny units.
    let positives = seeds.batch(|s| FixedQ32_32::from_raw((s.next() >> 12).cast_signed()));
    let (mul_pairs, div_pairs) = (pairs.clone(), pairs);
    vec![
        case("num/mul", move || {
            mul_pairs
                .iter()
                .map(|&(a, b)| black_box(a).checked_mul(b).map_or(0, FixedQ32_32::raw))
                .fold(0, i64::wrapping_add)
        }),
        case("num/div", move || {
            div_pairs
                .iter()
                .map(|&(a, b)| black_box(a).checked_div(b).map_or(0, FixedQ32_32::raw))
                .fold(0, i64::wrapping_add)
        }),
        case("num/sqrt", move || {
            positives
                .iter()
                .map(|&a| black_box(a).sqrt().map_or(0, FixedQ32_32::raw))
                .fold(0, i64::wrapping_add)
        }),
    ]
}

#[cfg(feature = "linalg")]
fn linalg_cases() -> Vec<Case> {
    let mut seeds = Seeds(PERF_SEED ^ 0x11A1);
    let vectors = seeds.batch(|s| s.vec3(100));
    let for_normalize = vectors.clone();
    vec![
        case("linalg/length", move || {
            vectors
                .iter()
                .map(|&v| black_box(v).length().map_or(0, FixedQ32_32::raw))
                .fold(0, i64::wrapping_add)
        }),
        case("linalg/normalize", move || {
            for_normalize
                .iter()
                .map(|&v| black_box(black_box(v).normalize()).map_or(0, |n| n.x.raw()))
                .fold(0, i64::wrapping_add)
        }),
    ]
}

#[cfg(feature = "query")]
fn query_cases() -> Vec<Case> {
    let mut seeds = Seeds(PERF_SEED ^ 0x0E77);
    let origins_and_directions = seeds.batch(|s| (s.vec3(10), s.vec3(10)));
    let toward_origin = seeds.batch(|s| ray_toward(s.vec3(10), s.vec3(1)));
    let away = seeds.batch(|s| ray_toward(s.vec3(10), s.vec3(1)).map(|r| reverse(&r)));
    let (sphere_rays, aabb_rays, triangle_rays) =
        (toward_origin.clone(), toward_origin.clone(), toward_origin);
    vec![
        case("query/ray_build", move || {
            origins_and_directions
                .iter()
                .map(|&(o, d)| {
                    black_box(FixedRay3::try_new(black_box(o), d))
                        .map_or(0, |r| r.direction.into_inner().x.raw())
                })
                .fold(0, i64::wrapping_add)
        }),
        case("query/ray_sphere_hit", move || {
            sum_hits(&sphere_rays, |r| {
                black_box(bunny_query::ray_intersects_sphere(r, &unit_sphere()))
                    .map(|(p, _)| p.x.raw())
            })
        }),
        case("query/ray_sphere_miss", move || {
            sum_hits(&away, |r| {
                black_box(bunny_query::ray_intersects_sphere(r, &unit_sphere()))
                    .map(|(p, _)| p.x.raw())
            })
        }),
        case("query/ray_aabb", move || {
            sum_hits(&aabb_rays, |r| {
                black_box(bunny_query::ray_intersects_aabb(r, &unit_box())).map(|(t, _)| t.raw())
            })
        }),
        case("query/ray_triangle", move || {
            let [a, b, c] = triangle();
            sum_hits(&triangle_rays, |r| {
                black_box(bunny_query::ray_intersects_triangle(r, a, b, c)).map(|p| p.x.raw())
            })
        }),
    ]
}

/// A ray from `origin` aimed at `target`, or `None` when they coincide.
#[cfg(feature = "query")]
fn ray_toward(origin: FixedVec3, target: FixedVec3) -> Option<FixedRay3> {
    FixedRay3::try_new(origin, target - origin).ok()
}

#[cfg(feature = "query")]
fn reverse(ray: &FixedRay3) -> FixedRay3 {
    let direction = ray.direction.into_inner();
    FixedRay3::try_new(ray.origin, FixedVec3::new(-direction.x, -direction.y, -direction.z))
        .unwrap_or(*ray)
}

#[cfg(feature = "query")]
fn sum_hits(rays: &[Option<FixedRay3>], hit: impl Fn(&FixedRay3) -> Option<i64>) -> i64 {
    rays.iter().flatten().map(|ray| hit(black_box(ray)).unwrap_or(0)).fold(0, i64::wrapping_add)
}

#[cfg(feature = "query")]
fn unit_sphere() -> FixedSphere3 {
    FixedSphere3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedQ32_32::ONE,
    )
}

#[cfg(feature = "query")]
fn unit_box() -> FixedAabb3 {
    let one = FixedQ32_32::ONE;
    FixedAabb3::new(FixedVec3::new(-one, -one, -one), FixedVec3::new(one, one, one))
}

#[cfg(feature = "query")]
fn triangle() -> [FixedVec3; 3] {
    let (one, zero) = (FixedQ32_32::ONE, FixedQ32_32::ZERO);
    [
        FixedVec3::new(-one, -one, zero),
        FixedVec3::new(one, -one, zero),
        FixedVec3::new(zero, one, zero),
    ]
}

#[cfg(feature = "angle")]
fn angle_cases() -> Vec<Case> {
    use bunny_num::FixedAngle;

    let mut seeds = Seeds(PERF_SEED ^ 0xA461);
    let angles = seeds.batch(|s| FixedAngle::from_raw(s.next()));
    let radians = seeds.batch(|s| s.fixed(100));
    // The same angles as f64 radians: a reference point for hardware-assisted float trig.
    #[allow(clippy::cast_precision_loss)]
    let float_angles: Vec<f64> = angles
        .iter()
        .map(|a| a.raw() as f64 / 18_446_744_073_709_551_616.0 * std::f64::consts::TAU)
        .collect();
    vec![
        case("angle/sin_cos", move || {
            angles
                .iter()
                .map(|&a| {
                    let (sin, cos) = black_box(a).sin_cos();
                    sin.raw() ^ cos.raw()
                })
                .fold(0, i64::wrapping_add)
        }),
        case("angle/from_radians", move || {
            radians
                .iter()
                .map(|&r| FixedAngle::from_radians(black_box(r)).raw().cast_signed())
                .fold(0, i64::wrapping_add)
        }),
        #[allow(clippy::cast_possible_truncation)]
        case("angle/f64_sin_cos_reference", move || {
            float_angles
                .iter()
                .map(|&a| {
                    let (sin, cos) = black_box(a).sin_cos();
                    (sin * 4_294_967_296.0) as i64 ^ (cos * 4_294_967_296.0) as i64
                })
                .fold(0, i64::wrapping_add)
        }),
    ]
}
