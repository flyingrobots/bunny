//! Frozen copies of the four plain BVH traversals as they were before flyingrobots/bunny#269
//! added the interior-visit bound: `traverse` is the body of `intersect_aabb` and
//! `intersect_ray`, `search` the body of `closest_hit` and `any_hit`. Same checks in the same
//! order, so the same results, errors and callback calls. Do not edit them to follow the
//! library: they are what the guarded functions are held to on buffers that are trees.

use bunny_broadphase::{BvhNode, BvhView, ClosestHit, TraversalError};
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

const STACK_CAPACITY: usize = 64;

fn leaf<'a>(node: &BvhNode, prim_indices: &'a [u32]) -> Result<&'a [u32], TraversalError> {
    let start = node.first_child_or_prim_idx as usize;
    let end =
        start.checked_add(node.prim_count as usize).ok_or(TraversalError::InvalidPrimitiveRange)?;
    prim_indices.get(start..end).ok_or(TraversalError::InvalidPrimitiveRange)
}

pub(crate) fn traverse(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    mut matches: impl FnMut(&BvhNode) -> bool,
    mut visit: impl FnMut(u32),
) -> Result<(), TraversalError> {
    let mut stack: Vec<u32> = Vec::new();
    if !nodes.is_empty() {
        stack.push(0);
    }
    while let Some(index) = stack.pop() {
        let node = nodes.get(index as usize).ok_or(TraversalError::InvalidNodeIndex)?;
        if !matches(node) {
            continue;
        }
        if node.prim_count > 0 {
            leaf(node, prim_indices)?.iter().for_each(|&primitive| visit(primitive));
            continue;
        }
        let left = node.first_child_or_prim_idx;
        let right = left.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        for child in [left, right] {
            if stack.len() >= STACK_CAPACITY {
                return Err(TraversalError::StackOverflow);
            }
            stack.push(child);
        }
    }
    Ok(())
}

type Child = (u32, Option<FixedQ32_32>);

/// `closest_hit` (`stop_at_first` false) and, with a closure that reports zero for a hit,
/// `any_hit` (`stop_at_first` true).
pub(crate) fn search(
    bvh: BvhView<'_>,
    (ray, t_max): (&TraceRay, FixedQ32_32),
    stop_at_first: bool,
    mut hit: impl FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
) -> Result<Option<ClosestHit>, TraversalError> {
    let node = |index: u32| bvh.nodes.get(index as usize).ok_or(TraversalError::InvalidNodeIndex);
    let entry =
        |index: u32, limit| Ok::<_, TraversalError>(ray.box_entry(&node(index)?.bounds, limit));
    let push = |stack: &mut Vec<(u32, FixedQ32_32)>, (index, enter): Child| {
        let Some(enter) = enter else { return Ok(()) };
        if stack.len() >= STACK_CAPACITY {
            return Err(TraversalError::StackOverflow);
        }
        stack.push((index, enter));
        Ok(())
    };
    let (mut limit, mut best, mut stack) = (t_max, None::<ClosestHit>, Vec::new());
    if !bvh.nodes.is_empty() {
        push(&mut stack, (0, entry(0, limit)?))?;
    }
    'search: loop {
        let current = loop {
            let Some((index, enter)) = stack.pop() else { break 'search };
            if enter <= limit {
                break node(index)?;
            }
        };
        if current.prim_count > 0 {
            for &primitive in leaf(current, bvh.prim_indices)? {
                let accepted = hit(primitive, limit).is_some_and(|distance| {
                    let better = (FixedQ32_32::ZERO..=limit).contains(&distance)
                        && best.is_none_or(|b: ClosestHit| {
                            (distance, primitive) < (b.distance, b.primitive)
                        });
                    if better {
                        best = Some(ClosestHit { primitive, distance });
                        limit = distance;
                    }
                    better
                });
                if accepted && stop_at_first {
                    break 'search;
                }
            }
            continue;
        }
        let left = current.first_child_or_prim_idx;
        let right = left.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        let (left, right): (Child, Child) =
            ((left, entry(left, limit)?), (right, entry(right, limit)?));
        let (near, far) = match (left.1, right.1) {
            (Some(l), Some(r)) if r < l => (right, left),
            _ => (left, right),
        };
        push(&mut stack, far)?;
        push(&mut stack, near)?;
    }
    Ok(best)
}
