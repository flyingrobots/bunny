//! BVH tree traversal queries (AABB overlap and ray intersection queries).

use bunny_geom::{FixedAabb3, FixedRay3};

use crate::bvh::BvhNode;
use crate::utils::aabbs_overlap;

const STACK_CAPACITY: usize = 64;

/// Traversal error type for BVH query operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalError {
    /// The traversal stack capacity (64) was exceeded.
    StackOverflow,
    /// A node references a child index that is outside the node slice.
    InvalidNodeIndex,
    /// A leaf node references primitives outside the primitive-index slice.
    InvalidPrimitiveRange,
}

fn leaf_indices<'a>(node: &BvhNode, prim_indices: &'a [u32]) -> Result<&'a [u32], TraversalError> {
    let start = node.first_child_or_prim_idx as usize;
    let count = node.prim_count as usize;
    let end = start.checked_add(count).ok_or(TraversalError::InvalidPrimitiveRange)?;
    prim_indices.get(start..end).ok_or(TraversalError::InvalidPrimitiveRange)
}

/// Traverses the BVH to find primitives that overlap with a query AABB.
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the stack size exceeds 64.
/// Returns `TraversalError::InvalidNodeIndex` or `TraversalError::InvalidPrimitiveRange`
/// if the provided BVH buffers are structurally malformed.
pub fn intersect_aabb<F>(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    query_box: &FixedAabb3,
    overlap_leaf: F,
) -> Result<(), TraversalError>
where
    F: FnMut(u32),
{
    traverse(nodes, prim_indices, |node| aabbs_overlap(&node.bounds, query_box), overlap_leaf)
}

/// Traverses the BVH to find primitives intersected by a ray.
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the stack size exceeds 64.
/// Returns `TraversalError::InvalidNodeIndex` or `TraversalError::InvalidPrimitiveRange`
/// if the provided BVH buffers are structurally malformed.
pub fn intersect_ray<F>(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    ray: &FixedRay3,
    intersect_leaf: F,
) -> Result<(), TraversalError>
where
    F: FnMut(u32),
{
    traverse(
        nodes,
        prim_indices,
        |node| bunny_query::ray_intersects_aabb(ray, &node.bounds).is_some(),
        intersect_leaf,
    )
}

fn traverse<Matches, Visit>(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    mut matches_node: Matches,
    mut visit_leaf: Visit,
) -> Result<(), TraversalError>
where
    Matches: FnMut(&BvhNode) -> bool,
    Visit: FnMut(u32),
{
    let mut stack = TraversalStack::new(nodes)?;
    while let Some(node_idx) = stack.pop() {
        let node = nodes.get(node_idx as usize).ok_or(TraversalError::InvalidNodeIndex)?;
        if matches_node(node) {
            visit_matching_node(node, prim_indices, &mut stack, &mut visit_leaf)?;
        }
    }
    Ok(())
}

fn visit_matching_node<Visit>(
    node: &BvhNode,
    prim_indices: &[u32],
    stack: &mut TraversalStack,
    visit_leaf: &mut Visit,
) -> Result<(), TraversalError>
where
    Visit: FnMut(u32),
{
    if node.prim_count > 0 {
        for &prim_idx in leaf_indices(node, prim_indices)? {
            visit_leaf(prim_idx);
        }
        return Ok(());
    }
    stack.push_children(node)
}

struct TraversalStack {
    values: [u32; STACK_CAPACITY],
    len: usize,
}

impl TraversalStack {
    fn new(nodes: &[BvhNode]) -> Result<Self, TraversalError> {
        let mut stack = Self { values: [0_u32; STACK_CAPACITY], len: 0 };
        if !nodes.is_empty() {
            stack.push(0)?;
        }
        Ok(stack)
    }

    fn push(&mut self, node_idx: u32) -> Result<(), TraversalError> {
        if self.len >= STACK_CAPACITY {
            return Err(TraversalError::StackOverflow);
        }
        *self.values.get_mut(self.len).ok_or(TraversalError::StackOverflow)? = node_idx;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<u32> {
        self.len = self.len.checked_sub(1)?;
        self.values.get(self.len).copied()
    }

    fn push_children(&mut self, node: &BvhNode) -> Result<(), TraversalError> {
        let left_child = node.first_child_or_prim_idx;
        let right_child = left_child.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        self.push(left_child)?;
        self.push(right_child)
    }
}
