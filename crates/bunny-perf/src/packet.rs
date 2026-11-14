//! Ray packets on the benchmark scene: the 1 024 camera rays traced with `closest_hits` in
//! square tiles of 2 x 2, 4 x 4 and 8 x 8 neighbouring rays, and in packets of the same sizes
//! drawn from a seeded shuffle of the rays (the incoherent worst case).
//!
//! Every case traces all 1 024 rays, so its time is per ray and compares directly with
//! `trace/closest_hit`. Before timing, [`disagreements`] checks that every packet variant
//! returns exactly the hit `closest_hit` returns for each ray alone.

use std::hint::black_box;
use std::rc::Rc;

use bunny_broadphase::{closest_hits, BvhView, ClosestHit, PacketTests, RayPacket};
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};

use crate::cases::{case, Case};
use crate::scene::{scene, trace_closest, trace_rays, trace_triangles, Scene, RAYS_PER_SIDE};

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const SHUFFLE_SEED: u64 = 0xB002_0217_0000_0001;

/// The variants measured, with the name each case uses.
pub(crate) const VARIANTS: [(PacketTests, &str); 3] = [
    (PacketTests::PerRay, "per_ray"),
    (PacketTests::Cull, "cull"),
    (PacketTests::CullAndAccept, "accept"),
];

/// Case names by variant, then packet size (2 x 2, 4 x 4, 8 x 8), then ray order.
const NAMES: [[[&str; 2]; 3]; 3] = [
    [
        ["packet/per_ray_2x2", "packet/per_ray_shuffled_4"],
        ["packet/per_ray_4x4", "packet/per_ray_shuffled_16"],
        ["packet/per_ray_8x8", "packet/per_ray_shuffled_64"],
    ],
    [
        ["packet/cull_2x2", "packet/cull_shuffled_4"],
        ["packet/cull_4x4", "packet/cull_shuffled_16"],
        ["packet/cull_8x8", "packet/cull_shuffled_64"],
    ],
    [
        ["packet/accept_2x2", "packet/accept_shuffled_4"],
        ["packet/accept_4x4", "packet/accept_shuffled_16"],
        ["packet/accept_8x8", "packet/accept_shuffled_64"],
    ],
];

/// The scene with its prepared triangles and rays.
pub(crate) struct Rig {
    pub(crate) scene: Scene,
    pub(crate) triangles: Vec<TraceTriangle>,
    pub(crate) rays: Vec<TraceRay>,
}

impl Rig {
    pub(crate) fn new() -> Self {
        let scene = scene();
        let triangles = trace_triangles(&scene);
        Self { scene, triangles, rays: trace_rays() }
    }

    pub(crate) fn bvh(&self) -> BvhView<'_> {
        BvhView::new(&self.scene.nodes, &self.scene.prim_indices)
    }

    /// The benchmark's triangle test for one ray.
    pub(crate) fn distance(
        &self,
        ray: &TraceRay,
        primitive: u32,
        limit: FixedQ32_32,
    ) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }
}

/// Ray indices in tile order: each run of `side * side` indices is one square tile of the
/// row-major camera grid, tiles row by row.
pub(crate) fn tiles(side: usize) -> Vec<usize> {
    let grid = usize::try_from(RAYS_PER_SIDE).unwrap_or(0);
    let mut order = Vec::with_capacity(grid * grid);
    for top in (0..grid).step_by(side) {
        for left in (0..grid).step_by(side) {
            for row in top..top + side {
                order.extend((left..left + side).map(|column| row * grid + column));
            }
        }
    }
    order
}

/// A seeded permutation of the ray indices (Fisher-Yates over a fixed LCG).
pub(crate) fn shuffled() -> Vec<usize> {
    let grid = usize::try_from(RAYS_PER_SIDE).unwrap_or(0);
    let mut order: Vec<usize> = (0..grid * grid).collect();
    let mut state = SHUFFLE_SEED;
    for last in (1..order.len()).rev() {
        state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let pick = usize::try_from((state >> 33) % (last as u64 + 1)).unwrap_or(0);
        order.swap(last, pick);
    }
    order
}

/// The rays of `order` cut into packets of `N`.
pub(crate) fn packets<const N: usize>(
    rig: &Rig,
    order: &[usize],
) -> Vec<([usize; N], [TraceRay; N])> {
    order
        .chunks_exact(N)
        .filter_map(|chunk| {
            let indices: [usize; N] = chunk.try_into().ok()?;
            let rays: Vec<TraceRay> =
                indices.iter().map(|&index| rig.rays.get(index).copied()).collect::<Option<_>>()?;
            Some((indices, rays.try_into().ok()?))
        })
        .collect()
}

/// Traces one packet; entry `i` is the hit of ray `i`.
pub(crate) fn trace_packet<const N: usize>(
    rig: &Rig,
    rays: &[TraceRay; N],
    tests: PacketTests,
) -> [Option<ClosestHit>; N] {
    let packet = RayPacket::new(rays, FAR, tests);
    let found = closest_hits(rig.bvh(), &packet, |ray, primitive, limit| {
        rig.distance(rays.get(ray)?, primitive, limit)
    });
    found.unwrap_or([None; N])
}

fn checksum(hits: &[Option<ClosestHit>]) -> i64 {
    hits.iter().map(|hit| hit.map_or(0, |hit| hit.distance.raw())).fold(0, i64::wrapping_add)
}

fn timed<const N: usize>(
    rig: &Rc<Rig>,
    order: &[usize],
    tests: PacketTests,
    name: &'static str,
) -> Case {
    let packed = packets::<N>(rig, order);
    let rig = Rc::clone(rig);
    case(name, move || {
        packed
            .iter()
            .map(|(_, rays)| checksum(&black_box(trace_packet(&rig, black_box(rays), tests))))
            .fold(0, i64::wrapping_add)
    })
}

/// The packet cases, and `packet/single_shuffled`: `closest_hit` one ray at a time in the
/// shuffled order, the single-ray reference for the shuffled packets.
pub(crate) fn packet_cases() -> Vec<Case> {
    let rig = Rc::new(Rig::new());
    let shuffle = shuffled();
    let mut cases = vec![{
        let (rig, order) = (Rc::clone(&rig), shuffle.clone());
        case("packet/single_shuffled", move || {
            order
                .iter()
                .filter_map(|&index| rig.rays.get(index))
                .map(|ray| {
                    trace_closest(&rig.scene, &rig.triangles, ray).map_or(0, |h| h.distance.raw())
                })
                .fold(0, i64::wrapping_add)
        })
    }];
    for (&(tests, _), names) in VARIANTS.iter().zip(NAMES) {
        let [two, four, eight] = names;
        cases.push(timed::<4>(&rig, &tiles(2), tests, two[0]));
        cases.push(timed::<4>(&rig, &shuffle, tests, two[1]));
        cases.push(timed::<16>(&rig, &tiles(4), tests, four[0]));
        cases.push(timed::<16>(&rig, &shuffle, tests, four[1]));
        cases.push(timed::<64>(&rig, &tiles(8), tests, eight[0]));
        cases.push(timed::<64>(&rig, &shuffle, tests, eight[1]));
    }
    cases
}

fn disagreements_for<const N: usize>(
    rig: &Rig,
    order: &[usize],
    label: &str,
    bad: &mut Vec<String>,
) {
    for &(tests, name) in &VARIANTS {
        for (indices, rays) in packets::<N>(rig, order) {
            let found = trace_packet(rig, &rays, tests);
            for (index, got) in indices.iter().zip(found) {
                let alone = rig
                    .rays
                    .get(*index)
                    .and_then(|ray| trace_closest(&rig.scene, &rig.triangles, ray));
                if got != alone {
                    bad.push(format!("{name} {label}: ray {index}"));
                }
            }
        }
    }
}

/// Every ray on which a packet variant differs from `closest_hit` for that ray alone
/// (primitive or distance bits), for each packing; empty when all agree. Also reports a
/// packing that does not cover all 1 024 rays.
pub(crate) fn disagreements() -> Vec<String> {
    let rig = Rig::new();
    let shuffle = shuffled();
    let mut bad = Vec::new();
    for (side, order) in [(2, tiles(2)), (4, tiles(4)), (8, tiles(8))] {
        let mut sorted = order.clone();
        sorted.sort_unstable();
        if sorted != (0..rig.rays.len()).collect::<Vec<_>>() {
            bad.push(format!("tiles of {side} do not cover the rays"));
        }
    }
    disagreements_for::<4>(&rig, &tiles(2), "2x2", &mut bad);
    disagreements_for::<16>(&rig, &tiles(4), "4x4", &mut bad);
    disagreements_for::<64>(&rig, &tiles(8), "8x8", &mut bad);
    disagreements_for::<4>(&rig, &shuffle, "shuffled 4", &mut bad);
    disagreements_for::<16>(&rig, &shuffle, "shuffled 16", &mut bad);
    disagreements_for::<64>(&rig, &shuffle, "shuffled 64", &mut bad);
    bad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_packet_variant_agrees_with_closest_hit_on_every_ray() {
        assert_eq!(disagreements(), Vec::<String>::new());
    }

    #[test]
    fn the_shuffle_is_a_permutation_that_moves_most_rays() {
        let order = shuffled();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..1024).collect::<Vec<_>>());
        assert!(order.iter().enumerate().filter(|(i, r)| i != *r).count() > 1000);
    }

    #[test]
    fn a_tile_holds_neighbouring_rays() {
        assert_eq!(&tiles(2)[..8], &[0, 1, 32, 33, 2, 3, 34, 35]);
    }
}
