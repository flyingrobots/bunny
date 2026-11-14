//! The whole-packet test against `box_entry` ray by ray.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use super::frustum::{Frustum, PacketVerdict as Verdict};

const SEED: u64 = 0xB002_0217_F000_0001;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }
}

fn vec3([x, y, z]: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// A packet of rays from one origin: directions spread around a centre by `spread` raw
/// units per axis, sometimes with a zero or tiny component on an axis for every ray, or a
/// component that changes sign across the packet.
fn packet(seeds: &mut Seeds, size: usize, spread: i64) -> Vec<TraceRay> {
    let origin = [0; 3].map(|_| seeds.signed(8 * ONE_RAW));
    let centre = [0; 3].map(|_| seeds.signed(ONE_RAW / 2));
    let special = seeds.next() % 6;
    let axis = usize::try_from(seeds.next() % 3).expect("small");
    (0..size)
        .filter_map(|_| {
            let mut d = centre.map(|c| c + seeds.signed(spread));
            match special {
                0 => d[axis] = 0,
                1 => d[axis] = 1 + seeds.signed(1).abs(),
                _ => {}
            }
            TraceRay::new(vec3(origin), vec3(d))
        })
        .collect()
}

/// A box somewhere around the origins, from a point to a thin slab to a large box.
fn seeded_box(seeds: &mut Seeds) -> FixedAabb3 {
    let size =
        [0, ONE_RAW / 64, ONE_RAW, 8 * ONE_RAW][usize::try_from(seeds.next() % 4).expect("small")];
    let lo = [0; 3].map(|_| seeds.signed(10 * ONE_RAW));
    let hi = lo.map(|c| c + seeds.signed(size).abs());
    FixedAabb3::new(vec3(lo), vec3(hi))
}

/// A box around a point on the first ray, from 0 to 3 units ahead and up to 4 units wide,
/// so that whole packets often enter it.
fn along(rays: &[TraceRay], seeds: &mut Seeds) -> FixedAabb3 {
    let ray = rays.first().expect("rays");
    let ahead = i128::from(seeds.signed(3 * ONE_RAW).abs());
    let half = seeds.signed(4 * ONE_RAW).abs();
    let at = |o: FixedQ32_32, d: FixedQ32_32| {
        o.raw() + i64::try_from((i128::from(d.raw()) * ahead) >> 32).expect("small")
    };
    let (o, d) = (ray.origin(), ray.direction());
    let centre = [at(o.x, d.x), at(o.y, d.y), at(o.z, d.z)];
    FixedAabb3::new(vec3(centre.map(|c| c - half)), vec3(centre.map(|c| c + half)))
}

/// Limits per ray: unlimited, or seeded within a few units (sometimes negative).
fn limits(seeds: &mut Seeds, size: usize) -> Vec<FixedQ32_32> {
    let unlimited = seeds.next().is_multiple_of(3);
    (0..size)
        .map(|_| {
            let raw = if unlimited { i64::MAX } else { seeds.signed(40 * ONE_RAW) };
            FixedQ32_32::from_raw(raw)
        })
        .collect()
}

#[test]
fn the_packet_verdict_never_contradicts_box_entry_for_any_ray() {
    let mut seeds = Seeds(SEED);
    let (mut rejected, mut accepted, mut tested) = (0, 0, 0);
    for case in 0..20_000 {
        let size = 1 + usize::try_from(seeds.next() % 64).expect("small");
        let spread = [0, 1 << 16, 1 << 24, ONE_RAW / 2][case % 4];
        let rays = packet(&mut seeds, size, spread);
        let Some(frustum) = Frustum::new(&rays) else { continue };
        let limits = limits(&mut seeds, rays.len());
        let low = *limits.iter().min().expect("rays");
        let high = *limits.iter().max().expect("rays");
        let aabb = if case % 3 == 0 { along(&rays, &mut seeds) } else { seeded_box(&mut seeds) };
        let each: Vec<_> = rays.iter().zip(&limits).map(|(r, &l)| r.box_entry(&aabb, l)).collect();
        match frustum.classify(&aabb, low, high) {
            Verdict::Reject => {
                assert!(each.iter().all(Option::is_none), "case {case}: rejected {each:?}");
                rejected += 1;
            }
            Verdict::Accept(bound) => {
                assert!(each.iter().all(|e| e.is_some_and(|e| bound <= e)), "case {case}");
                accepted += 1;
            }
            Verdict::Test => tested += 1,
        }
    }
    assert!(
        rejected > 2_000 && accepted > 1_000 && tested > 1_000,
        "{rejected} {accepted} {tested}"
    );
}

#[test]
fn rays_with_different_origins_have_no_frustum() {
    let d = vec3([0, 0, -ONE_RAW]);
    let a = TraceRay::new(vec3([0; 3]), d).expect("in domain");
    let b = TraceRay::new(vec3([1, 0, 0]), d).expect("in domain");
    assert!(Frustum::new(&[a, b]).is_none());
    assert!(Frustum::new(&[a, a]).is_some());
    assert!(Frustum::new(&[]).is_none());
}

#[test]
fn a_box_the_packet_surrounds_is_accepted_at_distance_zero() {
    let rays = [[1, 2, 3], [-3, 1, 2]].map(|d| TraceRay::new(vec3([0; 3]), vec3(d)).expect("ok"));
    let frustum = Frustum::new(&rays).expect("one origin");
    let around = FixedAabb3::new(vec3([-ONE_RAW; 3]), vec3([ONE_RAW; 3]));
    // The x components differ in sign, so the packet cannot accept; it must not reject.
    let verdict = frustum.classify(&around, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    assert_eq!(verdict, Verdict::Test);
    let same = Frustum::new(&[rays[0], rays[0]]).expect("one origin");
    let verdict = same.classify(&around, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    assert_eq!(verdict, Verdict::Accept(FixedQ32_32::ZERO));
}
