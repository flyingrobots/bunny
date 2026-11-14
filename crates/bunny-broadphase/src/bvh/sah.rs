//! Binned surface-area-heuristic (SAH) BVH builder.
//!
//! The build is iterative: a fixed array of pending nodes on the call stack replaces
//! recursion, and the tree's depth is capped at [`SAH_MAX_DEPTH`], so the work stack never
//! needs more than `SAH_MAX_DEPTH + 1` entries. Nothing is allocated.

mod bins;
mod cost;
mod partition;
#[cfg(test)]
mod tests;

use bunny_geom::FixedAabb3;

use super::build::{init_prim_indices, validated_range, PrimRange};
use super::{BvhNode, SAH_MAX_DEPTH, SAH_MAX_LEAF_SIZE};
use bins::{best_plane, Plane, Summary};
use cost::{leaf_cost, Scale};

const STACK_CAPACITY: usize = SAH_MAX_DEPTH + 1;

pub(super) fn build_bvh_sah(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    build_capped(nodes, prim_indices, primitives, SAH_MAX_DEPTH)
}

/// Builds with leaves forced at `max_depth` (at most `SAH_MAX_DEPTH`); the tests use a
/// smaller cap to reach it.
fn build_capped(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
    max_depth: usize,
) -> Option<usize> {
    let range = validated_range(nodes, prim_indices, primitives)?;
    if range.count == 0 {
        return Some(0);
    }
    init_prim_indices(prim_indices, range.count)?;
    let stack = WorkStack::new(Task { index: 0, range, depth: 0 });
    let max_depth = max_depth.min(SAH_MAX_DEPTH);
    SahBuilder { nodes, prim_indices, primitives, node_count: 1, max_depth, stack }.run()
}

#[derive(Clone, Copy)]
struct Task {
    index: usize,
    range: PrimRange,
    depth: usize,
}

struct WorkStack {
    tasks: [Task; STACK_CAPACITY],
    len: usize,
}

impl WorkStack {
    const fn new(root: Task) -> Self {
        Self { tasks: [root; STACK_CAPACITY], len: 1 }
    }

    fn push(&mut self, task: Task) -> Option<()> {
        *self.tasks.get_mut(self.len)? = task;
        self.len += 1;
        Some(())
    }

    fn pop(&mut self) -> Option<Task> {
        self.len = self.len.checked_sub(1)?;
        self.tasks.get(self.len).copied()
    }
}

/// What to do with one node.
enum Choice {
    Leaf,
    /// Partition by the plane.
    Plane(Plane),
    /// Keep the slots' ascending order and split it in half.
    Median,
}

struct SahBuilder<'a> {
    nodes: &'a mut [BvhNode],
    prim_indices: &'a mut [u32],
    primitives: &'a [FixedAabb3],
    node_count: usize,
    max_depth: usize,
    stack: WorkStack,
}

impl SahBuilder<'_> {
    fn run(mut self) -> Option<usize> {
        while let Some(task) = self.stack.pop() {
            self.build_node(task)?;
        }
        Some(self.node_count)
    }

    fn build_node(&mut self, task: Task) -> Option<()> {
        let summary = Summary::of(self.slots(task.range)?, self.primitives)?;
        let left_count = match self.choose(task, &summary)? {
            Choice::Leaf => return self.write(task.index, leaf(task.range, summary.bounds)?),
            Choice::Median => task.range.count / 2,
            Choice::Plane(plane) => self.partition(task.range, &plane)?,
        };
        self.split(task, summary.bounds, left_count)
    }

    /// One primitive, or a node at the depth cap, is a leaf. Otherwise the cheapest plane
    /// is taken when the node holds more than `SAH_MAX_LEAF_SIZE` primitives, or when its
    /// cost is strictly below the leaf cost (an equal cost makes a leaf). A node above the
    /// leaf size with no plane (every centroid equal) is split in half by slot order.
    fn choose(&self, task: Task, summary: &Summary) -> Option<Choice> {
        if task.range.count == 1 || task.depth >= self.max_depth {
            return Some(Choice::Leaf);
        }
        let plane = best_plane(summary, self.slots(task.range)?, self.primitives).ok()?;
        let forced = task.range.count > SAH_MAX_LEAF_SIZE;
        Some(match plane {
            Some(plane) if forced || plane.cost < node_leaf_cost(summary, task.range)? => {
                Choice::Plane(plane)
            }
            None if forced => Choice::Median,
            Some(_) | None => Choice::Leaf,
        })
    }

    /// Partitions by `plane` and checks that the count matches the sweep's.
    fn partition(&mut self, range: PrimRange, plane: &Plane) -> Option<usize> {
        let slots = self.prim_indices.get_mut(range.first..range.end()?)?;
        let left_count = partition::partition(slots, self.primitives, plane)?;
        (left_count == plane.left_count).then_some(left_count)
    }

    fn split(&mut self, task: Task, bounds: FixedAabb3, left_count: usize) -> Option<()> {
        let (left, right) = task.range.split(left_count)?;
        let child = self.node_count;
        self.node_count = child.checked_add(2)?;
        let first_child_or_prim_idx = u32::try_from(child).ok()?;
        self.write(task.index, BvhNode { bounds, first_child_or_prim_idx, prim_count: 0 })?;
        let depth = task.depth + 1;
        self.stack.push(Task { index: child.checked_add(1)?, range: right, depth })?;
        self.stack.push(Task { index: child, range: left, depth })
    }

    fn write(&mut self, index: usize, node: BvhNode) -> Option<()> {
        *self.nodes.get_mut(index)? = node;
        Some(())
    }

    fn slots(&self, range: PrimRange) -> Option<&[u32]> {
        self.prim_indices.get(range.first..range.end()?)
    }
}

fn node_leaf_cost(summary: &Summary, range: PrimRange) -> Option<u128> {
    leaf_cost(Scale::of(&summary.bounds).weighted(&summary.bounds, range.count)?)
}

fn leaf(range: PrimRange, bounds: FixedAabb3) -> Option<BvhNode> {
    Some(BvhNode {
        bounds,
        first_child_or_prim_idx: u32::try_from(range.first).ok()?,
        prim_count: u32::try_from(range.count).ok()?,
    })
}
