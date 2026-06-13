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
    let end = start
        .checked_add(count)
        .ok_or(TraversalError::InvalidPrimitiveRange)?;
    prim_indices
        .get(start..end)
        .ok_or(TraversalError::InvalidPrimitiveRange)
}

fn push_stack(
    stack: &mut [u32; STACK_CAPACITY],
    stack_ptr: &mut usize,
    node_idx: u32,
) -> Result<(), TraversalError> {
    if *stack_ptr >= STACK_CAPACITY {
        return Err(TraversalError::StackOverflow);
    }
    *stack
        .get_mut(*stack_ptr)
        .ok_or(TraversalError::StackOverflow)? = node_idx;
    *stack_ptr += 1;
    Ok(())
}

fn pop_stack(stack: &[u32; STACK_CAPACITY], stack_ptr: &mut usize) -> Option<u32> {
    if *stack_ptr == 0 {
        return None;
    }
    *stack_ptr -= 1;
    stack.get(*stack_ptr).copied()
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
    mut overlap_leaf: F,
) -> Result<(), TraversalError>
where
    F: FnMut(u32),
{
    if nodes.is_empty() {
        return Ok(());
    }

    let mut stack = [0_u32; STACK_CAPACITY];
    let mut stack_ptr = 0;

    push_stack(&mut stack, &mut stack_ptr, 0)?;

    while let Some(node_idx) = pop_stack(&stack, &mut stack_ptr) {
        let node_idx = node_idx as usize;
        let node = nodes
            .get(node_idx)
            .ok_or(TraversalError::InvalidNodeIndex)?;

        if !aabbs_overlap(&node.bounds, query_box) {
            continue;
        }

        if node.prim_count > 0 {
            for &prim_idx in leaf_indices(node, prim_indices)? {
                overlap_leaf(prim_idx);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child
                .checked_add(1)
                .ok_or(TraversalError::InvalidNodeIndex)?;

            if STACK_CAPACITY - stack_ptr < 2 {
                return Err(TraversalError::StackOverflow);
            }

            push_stack(&mut stack, &mut stack_ptr, left_child)?;
            push_stack(&mut stack, &mut stack_ptr, right_child)?;
        }
    }
    Ok(())
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
    mut intersect_leaf: F,
) -> Result<(), TraversalError>
where
    F: FnMut(u32),
{
    if nodes.is_empty() {
        return Ok(());
    }

    let mut stack = [0_u32; STACK_CAPACITY];
    let mut stack_ptr = 0;

    push_stack(&mut stack, &mut stack_ptr, 0)?;

    while let Some(node_idx) = pop_stack(&stack, &mut stack_ptr) {
        let node_idx = node_idx as usize;
        let node = nodes
            .get(node_idx)
            .ok_or(TraversalError::InvalidNodeIndex)?;

        if bunny_query::ray_intersects_aabb(ray, &node.bounds).is_none() {
            continue;
        }

        if node.prim_count > 0 {
            for &prim_idx in leaf_indices(node, prim_indices)? {
                intersect_leaf(prim_idx);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child
                .checked_add(1)
                .ok_or(TraversalError::InvalidNodeIndex)?;

            if STACK_CAPACITY - stack_ptr < 2 {
                return Err(TraversalError::StackOverflow);
            }

            push_stack(&mut stack, &mut stack_ptr, left_child)?;
            push_stack(&mut stack, &mut stack_ptr, right_child)?;
        }
    }
    Ok(())
}
