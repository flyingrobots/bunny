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
}

/// Traverses the BVH to find primitives that overlap with a query AABB.
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the stack size exceeds 64.
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

    stack[stack_ptr] = 0;
    stack_ptr += 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr] as usize;
        let node = &nodes[node_idx];

        if !aabbs_overlap(&node.bounds, query_box) {
            continue;
        }

        if node.prim_count > 0 {
            let start = node.first_child_or_prim_idx as usize;
            let count = node.prim_count as usize;
            for i in 0..count {
                overlap_leaf(prim_indices[start + i]);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child + 1;

            if stack_ptr + 2 > STACK_CAPACITY {
                return Err(TraversalError::StackOverflow);
            }

            stack[stack_ptr] = left_child;
            stack_ptr += 1;
            stack[stack_ptr] = right_child;
            stack_ptr += 1;
        }
    }
    Ok(())
}

/// Traverses the BVH to find primitives intersected by a ray.
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the stack size exceeds 64.
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

    stack[stack_ptr] = 0;
    stack_ptr += 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr] as usize;
        let node = &nodes[node_idx];

        if bunny_query::ray_intersects_aabb(ray, &node.bounds).is_none() {
            continue;
        }

        if node.prim_count > 0 {
            let start = node.first_child_or_prim_idx as usize;
            let count = node.prim_count as usize;
            for i in 0..count {
                intersect_leaf(prim_indices[start + i]);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child + 1;

            if stack_ptr + 2 > STACK_CAPACITY {
                return Err(TraversalError::StackOverflow);
            }

            stack[stack_ptr] = left_child;
            stack_ptr += 1;
            stack[stack_ptr] = right_child;
            stack_ptr += 1;
        }
    }
    Ok(())
}
