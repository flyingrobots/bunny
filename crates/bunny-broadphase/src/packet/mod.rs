//! Closest hits for a packet of rays that walk the tree together.
//!
//! Each ray gets exactly the hit [`crate::closest_hit`] returns for it alone: the packet
//! changes the order in which nodes are visited and which rays test them, never which
//! primitive wins. See `docs/topics/ray-tracing/README.md#packet-traversal`.

mod frustum;
#[cfg(test)]
mod tests;

use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::bvh::BvhNode;
use crate::closest_hit::{BvhView, ClosestHit};
use crate::traversal::TraversalError;
use frustum::Frustum;
pub use frustum::PacketVerdict;
use PacketVerdict as Verdict;

/// The largest packet [`closest_hits`] accepts; one bit of a `u64` per ray.
pub const PACKET_CAPACITY: usize = 64;

const STACK_CAPACITY: usize = 64;

/// Which tests decide whether a packet enters a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketTests {
    /// Every active ray runs `box_entry` at every node: a shared descent, no packet test.
    PerRay,
    /// One whole-packet test first; a node it rejects is not tested ray by ray.
    Cull,
    /// As `Cull`, and a node every ray certainly enters is not tested ray by ray either.
    CullAndAccept,
}

/// Rays to trace together, with their common distance limit and the tests to use.
#[derive(Clone, Copy, Debug)]
pub struct RayPacket<'r, const N: usize> {
    rays: &'r [TraceRay; N],
    t_max: FixedQ32_32,
    tests: PacketTests,
    frustum: Option<Frustum>,
}

impl<'r, const N: usize> RayPacket<'r, N> {
    /// Prepares a packet. The whole-packet tests apply only when every ray has the same
    /// origin; otherwise the packet runs as [`PacketTests::PerRay`].
    #[must_use]
    pub fn new(rays: &'r [TraceRay; N], t_max: FixedQ32_32, tests: PacketTests) -> Self {
        // dojo: allow panic-path -- evaluated at compile time; a packet above 64 rays does not build
        const { assert!(N <= PACKET_CAPACITY, "a packet holds at most 64 rays") };
        let frustum = if tests == PacketTests::PerRay { None } else { Frustum::new(rays) };
        Self { rays, t_max, tests, frustum }
    }

    /// The whole-packet test of `aabb` for rays whose limits lie in `low..=high`, or `None`
    /// when the packet has no whole-packet test (the rays' origins differ, or the tests are
    /// [`PacketTests::PerRay`]). Exposed so that harnesses can count what the search does.
    #[must_use]
    pub fn verdict(
        &self,
        aabb: &bunny_geom::FixedAabb3,
        low: FixedQ32_32,
        high: FixedQ32_32,
    ) -> Option<PacketVerdict> {
        self.frustum.map(|frustum| frustum.classify(aabb, low, high))
    }

    /// The rays, the distance limit and the tests the packet was prepared with.
    #[must_use]
    pub const fn parts(&self) -> (&'r [TraceRay; N], FixedQ32_32, PacketTests) {
        (self.rays, self.t_max, self.tests)
    }
}

/// Finds, for every ray of the packet, the nearest primitive it hits within `0..=t_max`.
///
/// `hit(ray, primitive, limit)` tests one primitive for ray number `ray` and returns its hit
/// distance, or `None`; distances outside `0..=limit` are ignored. Entry `i` of the result
/// equals what [`crate::closest_hit`] returns for `rays[i]` with the same closure, for any
/// rays: coherent or not, with one origin or many. Only the speed depends on coherence.
///
/// # Errors
/// The errors of [`crate::closest_hit`], for any node the packet visits. The packet can
/// visit a node that one of its rays alone would have pruned, so a malformed node can be
/// reported where a single-ray search of every ray would not reach it.
pub fn closest_hits<F, const N: usize>(
    bvh: BvhView<'_>,
    packet: &RayPacket<'_, N>,
    hit: F,
) -> Result<[Option<ClosestHit>; N], TraversalError>
where
    F: FnMut(usize, u32, FixedQ32_32) -> Option<FixedQ32_32>,
{
    PacketSearch::new(bvh, packet).run(hit)
}

/// A stack entry: node, the rays that may enter it, and the smallest of their entry bounds.
type Entry = (u32, u64, FixedQ32_32);

struct PacketSearch<'a, const N: usize> {
    bvh: BvhView<'a>,
    packet: &'a RayPacket<'a, N>,
    limits: [FixedQ32_32; N],
    best: [Option<ClosestHit>; N],
    stack: [Entry; STACK_CAPACITY],
    len: usize,
}

impl<'a, const N: usize> PacketSearch<'a, N> {
    const fn new(bvh: BvhView<'a>, packet: &'a RayPacket<'a, N>) -> Self {
        Self {
            bvh,
            packet,
            limits: [packet.t_max; N],
            best: [None; N],
            stack: [(0, 0, FixedQ32_32::ZERO); STACK_CAPACITY],
            len: 0,
        }
    }

    fn run<F>(mut self, mut hit: F) -> Result<[Option<ClosestHit>; N], TraversalError>
    where
        F: FnMut(usize, u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        if !self.bvh.nodes.is_empty() {
            let root = self.enter(0, all_rays(N))?;
            self.push(root)?;
        }
        while let Some((index, rays)) = self.pop()? {
            let node = self.node(index)?;
            if node.prim_count > 0 {
                self.visit_leaf(node, rays, &mut hit)?;
            } else {
                self.visit_interior(node, rays)?;
            }
        }
        Ok(self.best)
    }

    fn node(&self, index: u32) -> Result<&'a BvhNode, TraversalError> {
        self.bvh.nodes.get(index as usize).ok_or(TraversalError::InvalidNodeIndex)
    }

    fn limit(&self, ray: usize) -> FixedQ32_32 {
        self.limits.get(ray).copied().unwrap_or(FixedQ32_32::ZERO)
    }

    /// The rays of `rays` that may enter node `index`, and the smallest of their bounds.
    fn enter(&self, index: u32, rays: u64) -> Result<Entry, TraversalError> {
        let bounds = &self.node(index)?.bounds;
        let verdict = self.packet.frustum.map_or(Verdict::Test, |frustum| {
            let (low, high) = self.limit_range(rays);
            frustum.classify(bounds, low, high)
        });
        Ok(match verdict {
            Verdict::Reject => (index, 0, FixedQ32_32::ZERO),
            Verdict::Accept(bound) if self.packet.tests == PacketTests::CullAndAccept => {
                (index, rays, bound)
            }
            _ => self.enter_each(index, bounds, rays),
        })
    }

    /// `box_entry` for each ray in `rays` with its own limit.
    fn enter_each(&self, index: u32, bounds: &bunny_geom::FixedAabb3, rays: u64) -> Entry {
        let (mut entered, mut nearest) = (0, FixedQ32_32::from_raw(i64::MAX));
        for ray in Bits(rays) {
            let Some(found) = self.packet.rays.get(ray) else { continue };
            if let Some(bound) = found.box_entry(bounds, self.limit(ray)) {
                entered |= 1 << ray;
                nearest = nearest.min(bound);
            }
        }
        (index, entered, nearest)
    }

    /// The smallest and largest limit over `rays`.
    fn limit_range(&self, rays: u64) -> (FixedQ32_32, FixedQ32_32) {
        let mut range = (FixedQ32_32::from_raw(i64::MAX), FixedQ32_32::from_raw(i64::MIN));
        for ray in Bits(rays) {
            let limit = self.limit(ray);
            range = (range.0.min(limit), range.1.max(limit));
        }
        range
    }

    fn push(&mut self, entry: Entry) -> Result<(), TraversalError> {
        if entry.1 == 0 {
            return Ok(());
        }
        *self.stack.get_mut(self.len).ok_or(TraversalError::StackOverflow)? = entry;
        self.len += 1;
        Ok(())
    }

    /// The next node with the rays that can still find a better hit in it: those whose
    /// limit is at least the entry's bound (a bound equal to the limit is kept, for ties).
    fn pop(&mut self) -> Result<Option<(u32, u64)>, TraversalError> {
        while let Some(len) = self.len.checked_sub(1) {
            self.len = len;
            let &(index, rays, bound) = self.stack.get(len).ok_or(TraversalError::StackOverflow)?;
            let live =
                Bits(rays).filter(|&ray| bound <= self.limit(ray)).fold(0_u64, |m, r| m | 1 << r);
            if live != 0 {
                return Ok(Some((index, live)));
            }
        }
        Ok(None)
    }

    fn visit_interior(&mut self, node: &BvhNode, rays: u64) -> Result<(), TraversalError> {
        let left = node.first_child_or_prim_idx;
        let right = left.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        let (left, right) = (self.enter(left, rays)?, self.enter(right, rays)?);
        // One order for the whole packet: nearer smallest bound first, ties to the left.
        let (near, far) = if right.1 != 0 && (left.1 == 0 || right.2 < left.2) {
            (right, left)
        } else {
            (left, right)
        };
        self.push(far)?;
        self.push(near)
    }

    fn visit_leaf<F>(
        &mut self,
        node: &BvhNode,
        rays: u64,
        hit: &mut F,
    ) -> Result<(), TraversalError>
    where
        F: FnMut(usize, u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        let start = node.first_child_or_prim_idx as usize;
        let end = start
            .checked_add(node.prim_count as usize)
            .ok_or(TraversalError::InvalidPrimitiveRange)?;
        let primitives =
            self.bvh.prim_indices.get(start..end).ok_or(TraversalError::InvalidPrimitiveRange)?;
        for &primitive in primitives {
            for ray in Bits(rays) {
                if let Some(distance) = hit(ray, primitive, self.limit(ray)) {
                    self.accept(ray, primitive, distance);
                }
            }
        }
        Ok(())
    }

    /// Keeps a hit in `0..=limit` that is nearer than the ray's best, or as near with a lower
    /// index, exactly as `closest_hit` does.
    fn accept(&mut self, ray: usize, primitive: u32, distance: FixedQ32_32) {
        let limit = self.limit(ray);
        let (Some(best), Some(slot)) = (self.best.get_mut(ray), self.limits.get_mut(ray)) else {
            return;
        };
        let better = (FixedQ32_32::ZERO..=limit).contains(&distance)
            && best.is_none_or(|b| (distance, primitive) < (b.distance, b.primitive));
        if better {
            *best = Some(ClosestHit { primitive, distance });
            *slot = distance;
        }
    }
}

/// A mask with the low `count` bits set.
const fn all_rays(count: usize) -> u64 {
    if count >= 64 {
        u64::MAX
    } else {
        (1 << count) - 1
    }
}

/// The set bit positions of a mask, lowest first.
struct Bits(u64);

impl Iterator for Bits {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let bit = self.0.trailing_zeros() as usize;
        self.0 &= self.0 - 1;
        Some(bit)
    }
}
