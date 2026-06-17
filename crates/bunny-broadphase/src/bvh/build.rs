use bunny_geom::FixedAabb3;

use super::split::{SplitChoice, SplitEvaluator, SplitSearch};
use super::BvhNode;
use crate::utils::union_aabb;

pub(super) fn build_bvh(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    let range = validated_range(nodes, prim_indices, primitives)?;
    if range.is_empty() {
        return Some(0);
    }

    init_prim_indices(prim_indices, range.count)?;
    BvhBuilder::new(nodes, prim_indices, primitives).finish(range)
}

fn validated_range(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    primitives: &[FixedAabb3],
) -> Option<PrimRange> {
    let count = primitives.len();
    if count == 0 {
        return Some(PrimRange::new(0, 0));
    }

    let required_nodes = count.checked_mul(2)?.checked_sub(1)?;
    if !buffers_fit(nodes.len(), prim_indices.len(), count, required_nodes)? {
        return None;
    }
    Some(PrimRange::new(0, count))
}

fn buffers_fit(
    nodes_len: usize,
    indices_len: usize,
    count: usize,
    required: usize,
) -> Option<bool> {
    let max = usize::try_from(u32::MAX).ok()?;
    Some(required <= nodes_len && indices_len >= count && count <= max && required <= max)
}

fn init_prim_indices(prim_indices: &mut [u32], count: usize) -> Option<()> {
    for (i, idx) in prim_indices.iter_mut().take(count).enumerate() {
        *idx = u32::try_from(i).ok()?;
    }
    Some(())
}

#[derive(Clone, Copy)]
pub(super) struct PrimRange {
    pub(super) first: usize,
    pub(super) count: usize,
}

impl PrimRange {
    pub(super) const fn new(first: usize, count: usize) -> Self {
        Self { first, count }
    }

    const fn is_empty(self) -> bool {
        self.count == 0
    }

    const fn is_single(self) -> bool {
        self.count == 1
    }

    pub(super) const fn slot(self, offset: usize) -> Option<usize> {
        self.first.checked_add(offset)
    }

    const fn end(self) -> Option<usize> {
        self.first.checked_add(self.count)
    }

    fn split(self, left_count: usize) -> Option<(Self, Self)> {
        let right_first = self.first.checked_add(left_count)?;
        let right_count = self.count.checked_sub(left_count)?;
        Some((Self::new(self.first, left_count), Self::new(right_first, right_count)))
    }
}

#[derive(Clone, Copy)]
struct Children {
    left: usize,
    right: usize,
}

#[derive(Clone, Copy)]
struct BuildNode {
    index: usize,
    range: PrimRange,
    bounds: FixedAabb3,
}

struct PartitionCursor {
    left: usize,
    right: usize,
}

impl PartitionCursor {
    fn new(range: PrimRange) -> Option<Self> {
        Some(Self { left: range.first, right: range.end()?.checked_sub(1)? })
    }

    const fn active(&self) -> bool {
        self.left <= self.right
    }

    fn advance_left(&mut self) -> Option<()> {
        self.left = self.left.checked_add(1)?;
        Some(())
    }

    fn retreat_right(&mut self) -> Option<()> {
        if self.right == 0 {
            return self.advance_left();
        }
        self.right -= 1;
        Some(())
    }

    const fn left_count(self, range: PrimRange) -> Option<usize> {
        self.left.checked_sub(range.first)
    }
}

struct BvhBuilder<'a> {
    nodes: &'a mut [BvhNode],
    prim_indices: &'a mut [u32],
    primitives: &'a [FixedAabb3],
    node_count: usize,
}

impl<'a> BvhBuilder<'a> {
    const fn new(
        nodes: &'a mut [BvhNode],
        prim_indices: &'a mut [u32],
        primitives: &'a [FixedAabb3],
    ) -> Self {
        Self { nodes, prim_indices, primitives, node_count: 1 }
    }

    fn finish(mut self, range: PrimRange) -> Option<usize> {
        self.build(0, range)?;
        Some(self.node_count)
    }

    fn build(&mut self, node_idx: usize, range: PrimRange) -> Option<()> {
        let bounds = self.bounds_for(range)?;
        let work = BuildNode { index: node_idx, range, bounds };
        if range.is_single() {
            return self.write_leaf(work);
        }

        match self.accepted_split(range, bounds)? {
            SplitSearch::Found(split) => self.write_split(work, split),
            SplitSearch::Empty => self.write_leaf(work),
        }
    }

    fn write_split(&mut self, work: BuildNode, split: SplitChoice) -> Option<()> {
        let left_count = self.partition(work.range, split)?;
        if invalid_split(work.range, left_count) {
            return self.write_leaf(work);
        }

        let children = self.reserve_children()?;
        self.write_interior(work.index, children, work.bounds)?;
        self.build_children(children, work.range, left_count)
    }

    fn build_children(
        &mut self,
        children: Children,
        range: PrimRange,
        left_count: usize,
    ) -> Option<()> {
        let (left_range, right_range) = range.split(left_count)?;
        self.build(children.left, left_range)?;
        self.build(children.right, right_range)
    }

    fn reserve_children(&mut self) -> Option<Children> {
        let left = self.node_count;
        self.node_count = self.node_count.checked_add(2)?;
        if self.node_count > self.nodes.len() {
            return None;
        }
        Some(Children { left, right: left.checked_add(1)? })
    }

    fn write_leaf(&mut self, work: BuildNode) -> Option<()> {
        self.write_node(
            work.index,
            BvhNode {
                bounds: work.bounds,
                first_child_or_prim_idx: u32::try_from(work.range.first).ok()?,
                prim_count: u32::try_from(work.range.count).ok()?,
            },
        )
    }

    fn write_interior(
        &mut self,
        node_idx: usize,
        children: Children,
        bounds: FixedAabb3,
    ) -> Option<()> {
        self.write_node(
            node_idx,
            BvhNode {
                bounds,
                first_child_or_prim_idx: u32::try_from(children.left).ok()?,
                prim_count: 0,
            },
        )
    }

    fn write_node(&mut self, node_idx: usize, node: BvhNode) -> Option<()> {
        *self.nodes.get_mut(node_idx)? = node;
        Some(())
    }

    fn primitive_at(&self, slot: usize) -> Option<FixedAabb3> {
        let primitive_idx = usize::try_from(*self.prim_indices.get(slot)?).ok()?;
        self.primitives.get(primitive_idx).copied()
    }

    fn bounds_for(&self, range: PrimRange) -> Option<FixedAabb3> {
        let mut bounds = self.primitive_at(range.first)?;
        for offset in 1..range.count {
            bounds = union_aabb(bounds, self.primitive_at(range.slot(offset)?)?);
        }
        Some(bounds)
    }

    fn accepted_split(&self, range: PrimRange, bounds: FixedAabb3) -> Option<SplitSearch> {
        SplitEvaluator::new(self.prim_indices, self.primitives).accepted_split(range, bounds)
    }

    fn partition(&mut self, range: PrimRange, split: SplitChoice) -> Option<usize> {
        let mut cursor = PartitionCursor::new(range)?;
        while cursor.active() {
            let primitive = self.primitive_at(cursor.left)?;
            if split.sends_left(&primitive) {
                cursor.advance_left()?;
            } else {
                self.swap_partitioned(&mut cursor)?;
            }
        }
        cursor.left_count(range)
    }

    fn swap_partitioned(&mut self, cursor: &mut PartitionCursor) -> Option<()> {
        if cursor.left >= self.prim_indices.len() || cursor.right >= self.prim_indices.len() {
            return None;
        }
        self.prim_indices.swap(cursor.left, cursor.right);
        cursor.retreat_right()
    }
}

const fn invalid_split(range: PrimRange, left_count: usize) -> bool {
    left_count == 0 || left_count == range.count
}
