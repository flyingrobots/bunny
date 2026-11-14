//! Counters for `closest_hits` on the benchmark scene.
//!
//! As in `profile.rs`, the core crates carry no counters: this module replays the packet
//! search in the harness with public items only (`RayPacket::verdict`, `TraceRay::box_entry`,
//! the node buffers and the benchmark's triangle closure). A test checks that the replica
//! makes the same primitive calls, with the same limits and in the same order, as
//! `closest_hits`, and returns the same hits, for every variant and packing.

use std::fmt::Write as _;

use bunny_broadphase::{BvhNode, ClosestHit, PacketTests, PacketVerdict, RayPacket};
use bunny_num::FixedQ32_32;

use crate::packet::{packets, shuffled, tiles, Rig, VARIANTS};

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

/// Event counts of one packet search, or a sum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PacketCounts {
    /// Stack entries popped.
    pub(crate) popped: u64,
    /// Interior nodes visited by the packet.
    pub(crate) interior: u64,
    /// Leaves visited by the packet.
    pub(crate) leaves: u64,
    /// Sum over visited nodes of the rays active there.
    pub(crate) ray_visits: u64,
    /// Whole-packet tests.
    pub(crate) packet_tests: u64,
    /// Whole-packet tests that rejected the node for every ray.
    pub(crate) packet_rejects: u64,
    /// Whole-packet tests whose accept replaced the per-ray tests.
    pub(crate) packet_accepts: u64,
    /// Calls to `box_entry` for single rays.
    pub(crate) box_calls: u64,
    /// Calls to the triangle test.
    pub(crate) tri_tests: u64,
}

impl PacketCounts {
    fn add(&mut self, o: &Self) {
        self.popped += o.popped;
        self.interior += o.interior;
        self.leaves += o.leaves;
        self.ray_visits += o.ray_visits;
        self.packet_tests += o.packet_tests;
        self.packet_rejects += o.packet_rejects;
        self.packet_accepts += o.packet_accepts;
        self.box_calls += o.box_calls;
        self.tri_tests += o.tri_tests;
    }
}

type Entry = (u32, u64, FixedQ32_32);
/// The scene and the packet a replica walks.
type Ctx<'a, 'r, const N: usize> = (&'a Rig, &'a RayPacket<'r, N>);

/// Replica of one `closest_hits` search with counters and a log of primitive calls.
pub(crate) struct PacketWalk<const N: usize> {
    pub(crate) counts: PacketCounts,
    pub(crate) hits: [Option<ClosestHit>; N],
    /// `(ray, primitive, limit)` of every primitive call, in order.
    pub(crate) calls: Vec<(usize, u32, FixedQ32_32)>,
    limits: [FixedQ32_32; N],
}

fn bits(mut mask: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        (mask != 0).then(|| {
            let bit = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            bit
        })
    })
}

impl<const N: usize> PacketWalk<N> {
    /// Replays the search of `closest_hits` for `packet`.
    pub(crate) fn run(rig: &Rig, packet: &RayPacket<'_, N>) -> Self {
        let (_, t_max, _) = packet.parts();
        let mut walk = Self {
            counts: PacketCounts::default(),
            hits: [None; N],
            calls: Vec::new(),
            limits: [t_max; N],
        };
        let full = if N >= 64 { u64::MAX } else { (1 << N) - 1 };
        let mut stack = vec![walk.enter((rig, packet), 0, full)];
        while let Some((index, rays, bound)) = stack.pop() {
            if rays == 0 {
                continue;
            }
            walk.counts.popped += 1;
            let live =
                bits(rays).filter(|&r| bound <= walk.limits[r]).fold(0_u64, |m, r| m | 1 << r);
            if live == 0 {
                continue;
            }
            walk.counts.ray_visits += u64::from(live.count_ones());
            let node = rig.scene.nodes[index as usize];
            if node.prim_count > 0 {
                walk.leaf((rig, packet), &node, live);
                continue;
            }
            walk.counts.interior += 1;
            let first = node.first_child_or_prim_idx;
            let left = walk.enter((rig, packet), first, live);
            let right = walk.enter((rig, packet), first + 1, live);
            let (near, far) = if right.1 != 0 && (left.1 == 0 || right.2 < left.2) {
                (right, left)
            } else {
                (left, right)
            };
            stack.push(far);
            stack.push(near);
        }
        walk
    }

    fn enter(&mut self, (rig, packet): Ctx<'_, '_, N>, index: u32, rays: u64) -> Entry {
        let (packet_rays, _, tests) = packet.parts();
        let bounds = rig.scene.nodes[index as usize].bounds;
        let low = bits(rays).map(|r| self.limits[r]).min().unwrap_or(FAR);
        let high =
            bits(rays).map(|r| self.limits[r]).max().unwrap_or(FixedQ32_32::from_raw(i64::MIN));
        let verdict = packet.verdict(&bounds, low, high);
        if verdict.is_some() {
            self.counts.packet_tests += 1;
        }
        match verdict {
            Some(PacketVerdict::Reject) => {
                self.counts.packet_rejects += 1;
                return (index, 0, FixedQ32_32::ZERO);
            }
            Some(PacketVerdict::Accept(bound)) if tests == PacketTests::CullAndAccept => {
                self.counts.packet_accepts += 1;
                return (index, rays, bound);
            }
            _ => {}
        }
        let (mut entered, mut nearest) = (0_u64, FAR);
        for r in bits(rays) {
            self.counts.box_calls += 1;
            if let Some(bound) = packet_rays[r].box_entry(&bounds, self.limits[r]) {
                entered |= 1 << r;
                nearest = nearest.min(bound);
            }
        }
        (index, entered, nearest)
    }

    fn leaf(&mut self, (rig, packet): Ctx<'_, '_, N>, node: &BvhNode, rays: u64) {
        self.counts.leaves += 1;
        let (packet_rays, _, _) = packet.parts();
        let start = node.first_child_or_prim_idx as usize;
        for &primitive in &rig.scene.prim_indices[start..start + node.prim_count as usize] {
            for r in bits(rays) {
                self.counts.tri_tests += 1;
                let limit = self.limits[r];
                self.calls.push((r, primitive, limit));
                let Some(distance) = rig.distance(&packet_rays[r], primitive, limit) else {
                    continue;
                };
                let better = (FixedQ32_32::ZERO..=limit).contains(&distance)
                    && self.hits[r]
                        .is_none_or(|b| (distance, primitive) < (b.distance, b.primitive));
                if better {
                    self.hits[r] = Some(ClosestHit { primitive, distance });
                    self.limits[r] = distance;
                }
            }
        }
    }
}

/// Counts summed over all packets of `order` cut into packets of `N`.
pub(crate) fn totals<const N: usize>(
    rig: &Rig,
    order: &[usize],
    tests: PacketTests,
) -> PacketCounts {
    let mut sum = PacketCounts::default();
    for (_, rays) in packets::<N>(rig, order) {
        sum.add(&PacketWalk::run(rig, &RayPacket::new(&rays, FAR, tests)).counts);
    }
    sum
}

#[allow(clippy::cast_precision_loss)]
fn per_ray(value: u64) -> f64 {
    value as f64 / 1024.0
}

fn row(out: &mut String, label: &str, c: &PacketCounts) {
    let _ = writeln!(
        out,
        "| {label} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
        per_ray(c.box_calls),
        per_ray(c.packet_tests),
        per_ray(c.packet_rejects),
        per_ray(c.packet_accepts),
        per_ray(c.interior + c.leaves),
        per_ray(c.ray_visits),
        per_ray(c.popped),
        per_ray(c.tri_tests),
    );
}

/// The `--stats` table for packets: every counter divided by the 1 024 rays.
pub(crate) fn report(rig: &Rig) -> String {
    let mut out = String::from(
        "| packing, variant | per-ray box tests | packet tests | packet rejects | packet accepts \
         | packet node visits | ray-node visits | pops | triangle tests |\n\
         | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    let shuffle = shuffled();
    for &(tests, name) in &VARIANTS {
        row(&mut out, &format!("2 x 2, {name}"), &totals::<4>(rig, &tiles(2), tests));
        row(&mut out, &format!("4 x 4, {name}"), &totals::<16>(rig, &tiles(4), tests));
        row(&mut out, &format!("8 x 8, {name}"), &totals::<64>(rig, &tiles(8), tests));
        row(&mut out, &format!("shuffled 4, {name}"), &totals::<4>(rig, &shuffle, tests));
        row(&mut out, &format!("shuffled 16, {name}"), &totals::<16>(rig, &shuffle, tests));
        row(&mut out, &format!("shuffled 64, {name}"), &totals::<64>(rig, &shuffle, tests));
    }
    out
}

#[cfg(test)]
mod tests {
    use bunny_broadphase::closest_hits;

    use super::*;

    fn agrees<const N: usize>(rig: &Rig, order: &[usize]) -> bool {
        packets::<N>(rig, order).iter().all(|(_, rays)| {
            VARIANTS.iter().all(|&(tests, _)| {
                let packet = RayPacket::new(rays, FAR, tests);
                let mut calls = Vec::new();
                let found = closest_hits(rig.bvh(), &packet, |r, p, limit| {
                    calls.push((r, p, limit));
                    rig.distance(&rays[r], p, limit)
                });
                let walk = PacketWalk::run(rig, &packet);
                found == Ok(walk.hits) && calls == walk.calls
            })
        })
    }

    #[test]
    fn the_packet_replica_makes_the_same_calls_as_closest_hits() {
        let rig = Rig::new();
        assert!(agrees::<4>(&rig, &tiles(2)));
        assert!(agrees::<16>(&rig, &tiles(4)));
        assert!(agrees::<64>(&rig, &tiles(8)));
        assert!(agrees::<16>(&rig, &shuffled()));
    }

    #[test]
    fn per_ray_packets_make_no_packet_tests_and_cull_never_accepts() {
        let rig = Rig::new();
        let per_ray = totals::<16>(&rig, &tiles(4), PacketTests::PerRay);
        assert_eq!((per_ray.packet_tests, per_ray.packet_accepts), (0, 0));
        let cull = totals::<16>(&rig, &tiles(4), PacketTests::Cull);
        assert_eq!(cull.packet_accepts, 0);
        assert!(cull.packet_rejects > 0 && cull.box_calls < per_ray.box_calls);
    }
}
