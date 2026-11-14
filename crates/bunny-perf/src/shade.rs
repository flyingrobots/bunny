//! Diffuse shading cases: the per-triangle and per-hit work of `bunny-diffuse-shading/1`.
//!
//! The inputs are 1 024 seeded triangles within 8 units of the origin, each hit by a ray from
//! a seeded origin toward its centroid, and one sampler point per hit (pair 1 of a 32 by 32
//! tile). Traversal is not included: these time the shading steps a renderer runs after a
//! closest-hit query returns.

use std::hint::black_box;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    Albedo, DirectionalLight, PixelSampler, ShadingFrame, SkyLight, SurfaceHit, TriangleSurface,
};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit};

use crate::cases::{case, Case};

const SEED: u64 = 0xB002_0160_5AD3_0001;
const SAMPLER_SEED: u64 = 0x0123_4567_89AB_CDEF;

/// One shot: the triangle, its prepared surface, the ray, the hit and a sampler point.
#[derive(Clone, Copy)]
struct Shot {
    triangle: TraceTriangle,
    surface: TriangleSurface,
    ray: TraceRay,
    hit: TriangleHit,
    sample: [FixedQ32_32; 2],
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 11
}

/// A raw value in `[-range, range)` units.
fn coordinate(state: &mut u64, range: i64) -> i64 {
    let span = (2 * range * ONE_RAW).unsigned_abs();
    i64::try_from(next(state) % span).unwrap_or(0) - range * ONE_RAW
}

fn point(state: &mut u64, range: i64) -> FixedVec3 {
    let mut c = || FixedQ32_32::from_raw(coordinate(state, range));
    FixedVec3::new(c(), c(), c())
}

/// A seeded triangle hit by a ray toward its centroid, or `None` when the draw is unusable.
fn hit(state: &mut u64, index: u32) -> Option<Shot> {
    let centre = point(state, 8);
    let corner = |state: &mut u64| centre + point(state, 1);
    let triangle = TraceTriangle::new(corner(state), corner(state), corner(state))?;
    let surface = TriangleSurface::new(&triangle)?;
    let [a, b, c] = triangle.vertices();
    let target = FixedVec3::new(
        (a.x + b.x + c.x) / FixedQ32_32::from_raw(3 * ONE_RAW),
        (a.y + b.y + c.y) / FixedQ32_32::from_raw(3 * ONE_RAW),
        (a.z + b.z + c.z) / FixedQ32_32::from_raw(3 * ONE_RAW),
    );
    let toward = target - point(state, 16);
    let largest = toward.x.raw().abs().max(toward.y.raw().abs()).max(toward.z.raw().abs());
    let scale = FixedQ32_32::from_raw(largest);
    let ray = TraceRay::new(target - toward, toward / scale)?;
    let hit = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX))?;
    let sample = PixelSampler::new(SAMPLER_SEED, index % 32, index / 32 % 32).point(index % 16, 1);
    Some(Shot { triangle, surface, ray, hit, sample })
}

fn hits() -> Vec<Shot> {
    let mut state = SEED;
    let mut hits = Vec::with_capacity(1024);
    let mut index = 0_u32;
    while hits.len() < 1024 && index < 1 << 16 {
        hits.extend(hit(&mut state, index));
        index += 1;
    }
    hits
}

pub(crate) fn shade_cases() -> Vec<Case> {
    let all = hits();
    let triangles: Vec<TraceTriangle> = all.iter().map(|h| h.triangle).collect();
    let frames: Vec<(ShadingFrame, [FixedQ32_32; 2])> =
        all.iter().map(|h| (h.surface.facing_frame(h.hit.front_face), h.sample)).collect();
    let one = FixedQ32_32::from_raw(ONE_RAW);
    let toward = FixedVec3::new(
        one / FixedQ32_32::from_raw(2 * ONE_RAW),
        one,
        one / FixedQ32_32::from_raw(4 * ONE_RAW),
    );
    let light = DirectionalLight::new(toward, [FixedQ32_32::from_raw(3 * ONE_RAW); 3]).ok();
    let sky = SkyLight::new([FixedQ32_32::from_raw(ONE_RAW / 3); 3]).ok();
    let albedo = Albedo::new([FixedQ32_32::from_raw(ONE_RAW * 7 / 10); 3]).ok();
    vec![
        // The per-triangle preparation: unit normal, frame and exact plane.
        case("shade/surface_new", move || {
            triangles
                .iter()
                .map(|t| {
                    black_box(TriangleSurface::new(black_box(t)))
                        .map_or(0, |s| s.frame().normal().x.raw())
                })
                .fold(0, i64::wrapping_add)
        }),
        // A cosine-weighted bounce direction from a held frame and a sampler point.
        case("shade/bounce_direction", move || {
            frames
                .iter()
                .map(|(frame, sample)| {
                    black_box(black_box(frame).cosine_direction(*sample)).map_or(0, |d| d.z.raw())
                })
                .fold(0, i64::wrapping_add)
        }),
        // The whole per-hit step: the surface hit (facing and start point), the light sample
        // with its shadow ray, the bounce ray, and the reflection of direct plus sky.
        case("shade/hit_shading", move || {
            let (Some(light), Some(sky), Some(albedo)) = (light, sky, albedo) else { return 0 };
            all.iter()
                .map(|h| {
                    let Some(surface_hit) = SurfaceHit::new(black_box(&h.surface), &h.ray, &h.hit)
                    else {
                        return 0;
                    };
                    let direct = black_box(surface_hit.light_sample(&light)).radiance(true);
                    let bounce = black_box(surface_hit.bounce_ray(h.sample));
                    let radiance = albedo.reflect(direct, sky.radiance());
                    let radiance = black_box(radiance).map_or(0, |r| r[0].raw());
                    radiance.wrapping_add(bounce.map_or(0, |r| r.direction().x.raw()))
                })
                .fold(0, i64::wrapping_add)
        }),
    ]
}
