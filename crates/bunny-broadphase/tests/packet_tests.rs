//! `closest_hits` against `closest_hit` ray by ray, and so against a test of every primitive.

use bunny_broadphase::{
    build_bvh, closest_hit, closest_hits, BvhNode, BvhView, ClosestHit, PacketTests, RayPacket,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const PACKET_SEED: u64 = 0xB002_0217_0000_0002;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const ALL_TESTS: [PacketTests; 3] =
    [PacketTests::PerRay, PacketTests::Cull, PacketTests::CullAndAccept];

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

    fn point(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }
}

fn vec3([x, y, z]: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

struct Scene {
    triangles: Vec<TraceTriangle>,
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
}

impl Scene {
    /// Small triangles scattered through a cube, a few that span it, and every tenth one
    /// duplicated so that ties between equal distances occur.
    fn seeded(seeds: &mut Seeds, count: usize) -> Self {
        let mut triangles: Vec<TraceTriangle> = (0..count)
            .filter_map(|index| {
                let reach = if index % 40 == 0 { 6 * ONE_RAW } else { ONE_RAW };
                let v0 = seeds.point(6 * ONE_RAW);
                let offset = |s: &mut Seeds| {
                    let d = s.point(reach);
                    vec3([v0[0] + d[0], v0[1] + d[1], v0[2] + d[2]])
                };
                let (v1, v2) = (offset(seeds), offset(seeds));
                TraceTriangle::new(vec3(v0), v1, v2)
            })
            .collect();
        let copies: Vec<_> = triangles.iter().step_by(10).copied().collect();
        triangles.extend(copies);
        let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
        let empty = BvhNode { bounds: boxes[0], first_child_or_prim_idx: 0, prim_count: 0 };
        let mut nodes = vec![empty; 2 * boxes.len()];
        let mut prim_indices = vec![0_u32; boxes.len()];
        let used = build_bvh(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
        nodes.truncate(used);
        Self { triangles, nodes, prim_indices }
    }

    fn bvh(&self) -> BvhView<'_> {
        BvhView::new(&self.nodes, &self.prim_indices)
    }

    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(usize::try_from(primitive).ok()?)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }

    /// The oracle: `closest_hit` for one ray alone.
    fn alone(&self, ray: &TraceRay, t_max: FixedQ32_32) -> Option<ClosestHit> {
        closest_hit(self.bvh(), ray, t_max, |p, limit| self.distance(ray, p, limit))
            .expect("well formed")
    }

    /// The oracle's oracle: a test of every triangle, nearest first, then lowest index.
    fn brute_force(&self, ray: &TraceRay, t_max: FixedQ32_32) -> Option<ClosestHit> {
        (0..self.triangles.len())
            .filter_map(|index| {
                let primitive = u32::try_from(index).ok()?;
                let distance = self.distance(ray, primitive, t_max)?;
                Some(ClosestHit { primitive, distance })
            })
            .min_by_key(|hit| (hit.distance, hit.primitive))
    }

    fn packet<const N: usize>(
        &self,
        rays: &[TraceRay; N],
        t_max: FixedQ32_32,
        tests: PacketTests,
    ) -> [Option<ClosestHit>; N] {
        let packet = RayPacket::new(rays, t_max, tests);
        closest_hits(self.bvh(), &packet, |ray, p, limit| self.distance(&rays[ray], p, limit))
            .expect("well formed")
    }

    /// Checks every variant against `closest_hit` ray by ray; returns the number of hits.
    fn check<const N: usize>(&self, rays: &[TraceRay; N], t_max: FixedQ32_32, what: &str) -> usize {
        let expected: Vec<_> = rays.iter().map(|ray| self.alone(ray, t_max)).collect();
        for tests in ALL_TESTS {
            let found = self.packet(rays, t_max, tests);
            assert_eq!(found.to_vec(), expected, "{what}, {tests:?}, {N} rays");
        }
        expected.iter().filter(|hit| hit.is_some()).count()
    }
}

/// `N` rays from one origin toward points around a seeded target: coherent when `spread`
/// is small, incoherent when it is large.
fn fan<const N: usize>(seeds: &mut Seeds, spread: i64) -> Option<[TraceRay; N]> {
    let origin = seeds.point(8 * ONE_RAW);
    let target = seeds.point(5 * ONE_RAW);
    let rays: Vec<TraceRay> = (0..N)
        .filter_map(|_| {
            let jitter = seeds.point(spread);
            let d = [0, 1, 2].map(|a| (target[a] + jitter[a] - origin[a]) / 32);
            TraceRay::new(vec3(origin), vec3(d))
        })
        .collect();
    rays.try_into().ok()
}

/// `N` unrelated rays, each with its own origin; some components zero or tiny.
fn scattered<const N: usize>(seeds: &mut Seeds) -> Option<[TraceRay; N]> {
    let rays: Vec<TraceRay> = (0..N)
        .filter_map(|_| {
            let origin = seeds.point(8 * ONE_RAW);
            let toward = seeds.point(6 * ONE_RAW);
            let mut d = [0, 1, 2].map(|a| (toward[a] - origin[a]) / 16);
            for c in &mut d {
                match seeds.next() % 10 {
                    0 => *c = 0,
                    1 => *c = seeds.signed(2),
                    _ => {}
                }
            }
            TraceRay::new(vec3(origin), vec3(d))
        })
        .collect();
    rays.try_into().ok()
}

fn check_size<const N: usize>(scene: &Scene, seeds: &mut Seeds) -> usize {
    let mut hits = 0;
    for round in 0..6 {
        let what = format!("seed={PACKET_SEED:#018x} size={N} round={round}");
        let spread = [ONE_RAW / 16, ONE_RAW, 12 * ONE_RAW][round % 3];
        if let Some(rays) = fan::<N>(seeds, spread) {
            hits += scene.check(&rays, FAR, &format!("{what} fan"));
        }
        if let Some(rays) = scattered::<N>(seeds) {
            hits += scene.check(&rays, FAR, &format!("{what} scattered"));
        }
    }
    hits
}

macro_rules! every_size {
    ($scene:expr, $seeds:expr, $($n:literal)*) => { 0 $(+ check_size::<$n>($scene, $seeds))* };
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_111_every_packet_size_agrees_with_closest_hit_on_every_ray() {
    let mut seeds = Seeds(PACKET_SEED);
    let scene = Scene::seeded(&mut seeds, 400);
    let hits = every_size!(&scene, &mut seeds,
        1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32
        33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61
        62 63 64);
    assert!(hits > 5_000, "corpus needs hits: {hits}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_112_the_oracle_equals_a_test_of_every_primitive_on_packet_rays() {
    let mut seeds = Seeds(PACKET_SEED ^ 1);
    let scene = Scene::seeded(&mut seeds, 400);
    for round in 0..40 {
        let Some(rays) = fan::<16>(&mut seeds, ONE_RAW) else { continue };
        for ray in &rays {
            assert_eq!(scene.alone(ray, FAR), scene.brute_force(ray, FAR), "round {round}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_113_degenerate_packets_agree_with_closest_hit() {
    let mut seeds = Seeds(PACKET_SEED ^ 2);
    let scene = Scene::seeded(&mut seeds, 400);
    let hit_ray = (0..)
        .find_map(|_| fan::<1>(&mut seeds, 0).filter(|[r]| scene.alone(r, FAR).is_some()))
        .expect("some ray hits")[0];
    // Duplicate rays, and the same ray with one copy shortened to below its hit.
    assert_eq!(scene.check(&[hit_ray; 9], FAR, "duplicates"), 9);
    let short = scene.alone(&hit_ray, FAR).expect("hits").distance;
    let short = FixedQ32_32::from_raw(short.raw() - 1);
    assert_eq!(scene.check(&[hit_ray; 3], short, "below the hit"), 0);
    // Every ray points away from the scene, so all miss.
    let away = [0, 1, 2, 3].map(|i| {
        let d = vec3([i * ONE_RAW / 8, ONE_RAW / 2, ONE_RAW]);
        TraceRay::new(vec3([0, 0, 7 * ONE_RAW]), d).expect("in domain")
    });
    assert_eq!(scene.check(&away, FAR, "all miss"), 0);
    // Zero, negative and inclusive limits.
    let rays = fan::<8>(&mut seeds, ONE_RAW).expect("in domain");
    for limit in [0, -1, i64::MIN] {
        assert_eq!(scene.check(&rays, FixedQ32_32::from_raw(limit), "limit"), 0);
    }
    let mixed: [TraceRay; 2] = [hit_ray, scattered::<1>(&mut seeds).expect("in domain")[0]];
    scene.check(&mixed, FAR, "two origins");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_114_an_empty_tree_misses_for_every_ray() {
    let mut seeds = Seeds(PACKET_SEED ^ 3);
    let rays = fan::<4>(&mut seeds, ONE_RAW).expect("in domain");
    let bvh = BvhView::new(&[], &[]);
    for tests in ALL_TESTS {
        let packet = RayPacket::new(&rays, FAR, tests);
        let found = closest_hits(bvh, &packet, |_, _, _| Some(FixedQ32_32::ZERO));
        assert_eq!(found, Ok([None; 4]));
    }
}
