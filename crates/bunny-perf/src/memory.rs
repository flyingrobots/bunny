//! How much of a box test is memory access, on the benchmark scene's real visit sequence.
//!
//! The kernel replays in [`crate::profile`] time the box tests the search makes, in its order,
//! but each replayed call's node index is known in advance, so the core can start every load
//! long before it is needed. In the real search the next node comes from the previous visit's
//! result. The cases here separate the two:
//!
//! - `trace/kernel_box_load` reads the same six bound coordinates of the same nodes, in the
//!   same order, and does no arithmetic: the cost of the loads alone, overlapped.
//! - `trace/kernel_box_load_chain` makes each node index depend on the previous load's value,
//!   so the loads run one after another: their latency, with the search's cache behaviour.
//! - `trace/kernel_box_entry_addr_chain` is `box_entry` with each node index depending on the
//!   previous call's result: arithmetic and load latency in series, as in the search.
//! - `trace/closest_hit_shuffled` runs `closest_hit` on the same tree with its sibling pairs
//!   placed in a seeded random order. The search makes the same calls in the same order, so
//!   any difference from `trace/closest_hit` is placement in memory alone.
//!
//! It also holds a harness replica of `box_entry` that tests both children of a node at once
//! from per-ray values prepared once (`trace/kernel_box_pair`), with the same replica run one
//! box at a time (`trace/kernel_box_single`), so the gain of pairing and of hoisting the ray's
//! preparation can be told apart. A test checks that the replica returns `box_entry`'s bits on
//! every recorded call.

use std::hint::black_box;
use std::rc::Rc;

use bunny_broadphase::{closest_hit, BvhNode, BvhView, ClosestHit};
use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::cases::{case, Case};
use crate::profile::{walk, Mode, Rig};

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const SHUFFLE_SEED: u64 = 0x5EED_0215_0000_0001;

/// One recorded box test: ray index, node index, limit.
type BoxCall = (usize, u32, FixedQ32_32);

/// Every box test of the 1 024 `closest_hit` searches, in order, and the same calls grouped
/// as each ray's root test followed by the sibling pairs of its interior visits.
struct Calls {
    boxes: Vec<BoxCall>,
    /// `(ray, first child, limit)` of every interior visit; the root tests stay in `roots`.
    pairs: Vec<BoxCall>,
    roots: Vec<BoxCall>,
}

fn record(rig: &Rig) -> Calls {
    let mut calls = Calls { boxes: Vec::new(), pairs: Vec::new(), roots: Vec::new() };
    for (ray, path) in rig.rays.iter().enumerate() {
        let found = walk(rig, path, Mode::Closest).boxes;
        calls.boxes.extend(found.iter().map(|&(node, limit)| (ray, node, limit)));
        let Some((&(root, limit), rest)) = found.split_first() else { continue };
        calls.roots.push((ray, root, limit));
        calls.pairs.extend(rest.chunks_exact(2).map(|pair| (ray, pair[0].0, pair[0].1)));
    }
    calls
}

/// The box-load and box-latency replay cases, the pair replica and the shuffled tree.
pub(crate) fn memory_cases() -> Vec<Case> {
    let rig = Rc::new(Rig::new());
    let calls = Rc::new(record(&rig));
    let prepared: Rc<Vec<Prepared>> = Rc::new(rig.rays.iter().map(Prepared::new).collect());
    let shuffled = Rc::new(shuffled_tree(&rig.scene.nodes, SHUFFLE_SEED));
    vec![
        case("trace/kernel_box_load", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&calls));
            move || replay_loads(&rig.scene.nodes, &calls.boxes, false)
        }),
        case("trace/kernel_box_load_chain", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&calls));
            move || replay_loads(&rig.scene.nodes, &calls.boxes, true)
        }),
        case("trace/kernel_box_entry_addr_chain", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&calls));
            move || replay_entry_addr_chain(&rig, &calls.boxes)
        }),
        case("trace/kernel_box_single", {
            let (rig, calls, prepared) = (Rc::clone(&rig), Rc::clone(&calls), Rc::clone(&prepared));
            move || replay_single(&rig.scene.nodes, &prepared, &calls.boxes)
        }),
        case("trace/kernel_box_pair", {
            let (rig, calls, prepared) = (Rc::clone(&rig), Rc::clone(&calls), Rc::clone(&prepared));
            move || replay_pairs(&rig.scene.nodes, &prepared, &calls)
        }),
        case("trace/closest_hit_shuffled", move || {
            rig.rays
                .iter()
                .map(|ray| trace_on(&shuffled, &rig, ray).map_or(0, |hit| hit.distance.raw()))
                .fold(0, i64::wrapping_add)
        }),
    ]
}

fn bounds_sum(bounds: &FixedAabb3) -> i64 {
    let (lo, hi) = (bounds.min, bounds.max);
    [lo.x, lo.y, lo.z, hi.x, hi.y, hi.z].iter().fold(0_i64, |sum, c| sum.wrapping_add(c.raw()))
}

/// Reads the six bound coordinates of every recorded node. With `chain`, each node index
/// depends on the previous read, so the loads cannot overlap.
fn replay_loads(nodes: &[BvhNode], calls: &[BoxCall], chain: bool) -> i64 {
    let mut total = 0_i64;
    let mut previous = 0_i64;
    for &(_, node, _) in calls {
        let tie = if chain { u32::from(previous.to_le_bytes()[0]) & black_box(0) } else { 0 };
        let Some(node) = nodes.get((node | tie) as usize) else { continue };
        previous = black_box(bounds_sum(&node.bounds));
        total = total.wrapping_add(previous);
    }
    total
}

/// `box_entry` on every recorded call, each node index depending on the previous result.
fn replay_entry_addr_chain(rig: &Rig, calls: &[BoxCall]) -> i64 {
    let mut total = 0_i64;
    let mut previous = 0_i64;
    for &(ray, node, limit) in calls {
        let tie = u32::from(previous.to_le_bytes()[0]) & black_box(0);
        let (Some(ray), Some(node)) =
            (rig.rays.get(ray), rig.scene.nodes.get((node | tie) as usize))
        else {
            continue;
        };
        previous = black_box(ray.box_entry(&node.bounds, limit)).map_or(0, FixedQ32_32::raw);
        total = total.wrapping_add(previous);
    }
    total
}

fn replay_single(nodes: &[BvhNode], prepared: &[Prepared], calls: &[BoxCall]) -> i64 {
    let mut total = 0_i64;
    for &(ray, node, limit) in calls {
        let (Some(ray), Some(node)) = (prepared.get(ray), nodes.get(node as usize)) else {
            continue;
        };
        let found = black_box(ray.entry(&node.bounds, black_box(limit)));
        total = total.wrapping_add(found.map_or(0, FixedQ32_32::raw));
    }
    total
}

fn replay_pairs(nodes: &[BvhNode], prepared: &[Prepared], calls: &Calls) -> i64 {
    let mut total = replay_single(nodes, prepared, &calls.roots);
    for &(ray, first, limit) in &calls.pairs {
        let (Some(ray), Some(left), Some(right)) =
            (prepared.get(ray), nodes.get(first as usize), nodes.get(first as usize + 1))
        else {
            continue;
        };
        let [a, b] = black_box(ray.entry_pair([&left.bounds, &right.bounds], black_box(limit)));
        total = total
            .wrapping_add(a.map_or(0, FixedQ32_32::raw))
            .wrapping_add(b.map_or(0, FixedQ32_32::raw));
    }
    total
}

/// A ray's per-axis values for the replica box test, prepared once per ray.
///
/// The arithmetic is `TraceRay::box_entry`'s (`crates/bunny-query/src/trace.rs`), rebuilt
/// from the public origin and direction; the reciprocal is `floor(2^63 / |d|)`.
#[derive(Clone, Copy)]
pub(crate) struct Prepared {
    origin: [i128; 3],
    direction: [i64; 3],
    reciprocal: [u64; 3],
}

impl Prepared {
    pub(crate) fn new(ray: &TraceRay) -> Self {
        let (o, d) = (ray.origin(), ray.direction());
        let origin = [o.x, o.y, o.z].map(|c| i128::from(c.raw()));
        let direction = [d.x, d.y, d.z].map(FixedQ32_32::raw);
        let reciprocal = direction.map(|c| match c.unsigned_abs() {
            0 => 0,
            magnitude => (1_u64 << 63) / magnitude,
        });
        Self { origin, direction, reciprocal }
    }

    /// The replica of `box_entry` for one box.
    pub(crate) fn entry(&self, aabb: &FixedAabb3, t_max: FixedQ32_32) -> Option<FixedQ32_32> {
        let limit = u64::try_from(t_max.raw()).ok()?;
        let (lo, hi) = (aabb.min, aabb.max);
        let (mins, maxs) = ([lo.x, lo.y, lo.z], [hi.x, hi.y, hi.z]);
        let mut enter = 0_u64;
        let mut exit = u64::MAX;
        for axis in 0..3 {
            let (near, far) = self.slab(axis, mins[axis].raw(), maxs[axis].raw())?;
            enter = enter.max(near);
            exit = exit.min(far);
        }
        finish(enter, exit, limit)
    }

    /// The replica of two `box_entry` calls with the same limit, computed axis by axis for
    /// both boxes together.
    pub(crate) fn entry_pair(
        &self,
        boxes: [&FixedAabb3; 2],
        t_max: FixedQ32_32,
    ) -> [Option<FixedQ32_32>; 2] {
        let Ok(limit) = u64::try_from(t_max.raw()) else { return [None, None] };
        let mut enter = [0_u64; 2];
        let mut exit = [u64::MAX; 2];
        let mut alive = [true; 2];
        for axis in 0..3 {
            for (side, aabb) in boxes.iter().enumerate() {
                let (lo, hi) = (aabb.min, aabb.max);
                let (min, max) = ([lo.x, lo.y, lo.z][axis].raw(), [hi.x, hi.y, hi.z][axis].raw());
                match self.slab(axis, min, max) {
                    Some((near, far)) => {
                        enter[side] = enter[side].max(near);
                        exit[side] = exit[side].min(far);
                    }
                    None => alive[side] = false,
                }
            }
        }
        [0, 1].map(|side| if alive[side] { finish(enter[side], exit[side], limit) } else { None })
    }

    #[inline]
    fn slab(&self, axis: usize, min: i64, max: i64) -> Option<(u64, u64)> {
        let (origin, direction, reciprocal) =
            (self.origin[axis], self.direction[axis], self.reciprocal[axis]);
        let (min, max) = (i128::from(min), i128::from(max));
        if direction == 0 {
            return (min <= origin && origin <= max).then_some((0, u64::MAX));
        }
        let (near, far) =
            if direction > 0 { (min - origin, max - origin) } else { (origin - max, origin - min) };
        let far = u64::try_from(far).ok()?;
        let near = u64::try_from(near).unwrap_or(0);
        Some((scaled(near, reciprocal), scaled(far, reciprocal + 1).saturating_add(1)))
    }
}

#[inline]
fn scaled(value: u64, reciprocal: u64) -> u64 {
    u64::try_from((u128::from(value) * u128::from(reciprocal)) >> 31).unwrap_or(u64::MAX)
}

#[inline]
fn finish(enter: u64, exit: u64, limit: u64) -> Option<FixedQ32_32> {
    if enter > exit || enter > limit {
        return None;
    }
    i64::try_from(enter).ok().map(FixedQ32_32::from_raw)
}

/// The tree with its sibling pairs moved to seeded random slots; the root stays at node 0 and
/// every node keeps its bounds, its children's order and its primitive range.
pub(crate) fn shuffled_tree(nodes: &[BvhNode], seed: u64) -> Vec<BvhNode> {
    let pairs = nodes.len().saturating_sub(1) / 2;
    let mut slot: Vec<usize> = (0..pairs).collect();
    let mut state = seed;
    for i in (1..pairs).rev() {
        state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        slot.swap(i, usize::try_from(state >> 33).unwrap_or(0) % (i + 1));
    }
    // Old node `1 + 2p + s` moves to `1 + 2 slot[p] + s`.
    let moved = |old: usize| if old == 0 { 0 } else { 1 + 2 * slot[(old - 1) / 2] + (old - 1) % 2 };
    let mut out = nodes.to_vec();
    for (old, node) in nodes.iter().enumerate() {
        let mut node = *node;
        if node.prim_count == 0 {
            node.first_child_or_prim_idx =
                u32::try_from(moved(node.first_child_or_prim_idx as usize)).unwrap_or(u32::MAX);
        }
        out[moved(old)] = node;
    }
    out
}

fn trace_on(nodes: &[BvhNode], rig: &Rig, ray: &TraceRay) -> Option<ClosestHit> {
    let ray = black_box(ray);
    let bvh = BvhView::new(nodes, &rig.scene.prim_indices);
    let hit = closest_hit(bvh, ray, FAR, |primitive, limit| {
        let triangle = rig.triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    });
    black_box(hit).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_replica_box_test_returns_box_entry_bits_on_every_recorded_call() {
        let rig = Rig::new();
        let calls = record(&rig);
        let mut bad = Vec::new();
        for &(ray, node, limit) in &calls.boxes {
            let (path, aabb) = (&rig.rays[ray], &rig.scene.nodes[node as usize].bounds);
            if Prepared::new(path).entry(aabb, limit) != path.box_entry(aabb, limit) {
                bad.push((ray, node));
            }
        }
        for &(ray, first, limit) in &calls.pairs {
            let path = &rig.rays[ray];
            let [a, b] = [
                &rig.scene.nodes[first as usize].bounds,
                &rig.scene.nodes[first as usize + 1].bounds,
            ];
            if Prepared::new(path).entry_pair([a, b], limit)
                != [path.box_entry(a, limit), path.box_entry(b, limit)]
            {
                bad.push((ray, first));
            }
        }
        assert_eq!(bad, Vec::new());
        assert_eq!(calls.boxes.len(), calls.roots.len() + 2 * calls.pairs.len());
    }

    #[test]
    fn the_replica_box_test_matches_box_entry_on_axis_parallel_and_edge_rays() {
        use bunny_linalg::FixedVec3;
        use bunny_num::fixed_q32_32::ONE_RAW;
        let units = |n: i64| FixedQ32_32::from_raw(n * ONE_RAW);
        let at = |x, y, z| FixedVec3::new(units(x), units(y), units(z));
        let aabb = FixedAabb3::new(at(-1, -1, -1), at(1, 1, 1));
        let behind = FixedAabb3::new(at(-1, -1, 6), at(1, 1, 8));
        let directions = [at(0, 0, -1), at(1, 0, 0), at(0, 1, 0), at(1, 1, -1), at(-1, 0, 1)];
        let origins = [at(0, 0, 5), at(1, 1, 5), at(2, 0, 5), at(0, 0, 0), at(-5, 1, -1)];
        let limits = [units(0), units(3), units(100), FixedQ32_32::from_raw(-1), FAR];
        for origin in origins {
            for direction in directions {
                let ray = TraceRay::new(origin, direction).expect("in domain");
                for limit in limits {
                    let replica = Prepared::new(&ray);
                    let expected = [ray.box_entry(&aabb, limit), ray.box_entry(&behind, limit)];
                    assert_eq!(replica.entry(&aabb, limit), expected[0]);
                    assert_eq!(replica.entry_pair([&aabb, &behind], limit), expected);
                }
            }
        }
    }

    #[test]
    fn the_shuffled_tree_is_a_permutation_with_the_same_answers() {
        let rig = Rig::new();
        let shuffled = shuffled_tree(&rig.scene.nodes, SHUFFLE_SEED);
        let moved = shuffled
            .iter()
            .zip(&rig.scene.nodes)
            .filter(|(a, b)| a.first_child_or_prim_idx != b.first_child_or_prim_idx)
            .count();
        assert!(moved > rig.scene.nodes.len() / 2, "only {moved} nodes moved");
        for ray in &rig.rays {
            let expected = crate::scene::trace_closest(&rig.scene, &rig.triangles, ray);
            assert_eq!(trace_on(&shuffled, &rig, ray), expected);
        }
    }
}
