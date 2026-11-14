//! Front-to-back ray traversal of the compact layout from [`crate::compact_bvh`].
//!
//! The search is [`crate::closest_hit`]'s, step for step: the same box tests in the same
//! order, the same nearer-first descent with ties to the left, the same pruning, and the same
//! primitive calls. Only where the boxes are read from differs: an interior visit reads one
//! [`CompactNode`] that holds both children's boxes, and a stack entry carries the child's
//! reference, so a leaf is visited without reading a node at all.

use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::closest_hit::ClosestHit;
use crate::compact::{CompactChild, CompactNode, CompactRoot};
use crate::traversal::TraversalError;

const STACK_CAPACITY: usize = 64;

/// A child and the bound on the distance at which the ray enters it, if it can.
type Entry = (CompactChild, Option<FixedQ32_32>);

/// The buffers of a compact BVH: the root from [`crate::compact_bvh`], the compact nodes it
/// wrote and the source tree's primitive-index buffer.
#[derive(Clone, Copy, Debug)]
pub struct CompactView<'a> {
    /// The root, or `None` for an empty tree.
    pub root: Option<CompactRoot>,
    /// The compact nodes; an interior root is node 0.
    pub nodes: &'a [CompactNode],
    /// The primitive indices that leaves refer to.
    pub prim_indices: &'a [u32],
}

/// Finds the nearest primitive hit by `ray` within `0..=t_max` in a compact tree.
///
/// Returns exactly what [`crate::closest_hit`] returns on the tree the compact one was
/// converted from, and calls `hit` with the same primitives and limits in the same order.
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the tree is deeper than 64 levels, and
/// `TraversalError::InvalidNodeIndex` or `TraversalError::InvalidPrimitiveRange` if a node
/// the search visits is malformed.
pub fn compact_closest_hit<F>(
    bvh: CompactView<'_>,
    ray: &TraceRay,
    t_max: FixedQ32_32,
    hit: F,
) -> Result<Option<ClosestHit>, TraversalError>
where
    F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
{
    Search::new(bvh, ray, t_max, false).run(hit)
}

/// Reports whether `ray` hits any primitive within `0..=t_max` in a compact tree, stopping
/// at the first hit; returns what [`crate::any_hit`] returns on the source tree.
///
/// # Errors
/// Returns the same errors as [`compact_closest_hit`].
pub fn compact_any_hit<F>(
    bvh: CompactView<'_>,
    ray: &TraceRay,
    t_max: FixedQ32_32,
    mut hit: F,
) -> Result<bool, TraversalError>
where
    F: FnMut(u32) -> bool,
{
    let search = Search::new(bvh, ray, t_max, true);
    let found = search.run(|primitive, _| hit(primitive).then_some(FixedQ32_32::ZERO))?;
    Ok(found.is_some())
}

struct Search<'a> {
    bvh: CompactView<'a>,
    ray: &'a TraceRay,
    limit: FixedQ32_32,
    best: Option<ClosestHit>,
    stop_at_first: bool,
    stack: [(CompactChild, FixedQ32_32); STACK_CAPACITY],
    len: usize,
}

impl<'a> Search<'a> {
    const fn new(
        bvh: CompactView<'a>,
        ray: &'a TraceRay,
        t_max: FixedQ32_32,
        stop_at_first: bool,
    ) -> Self {
        Self {
            bvh,
            ray,
            limit: t_max,
            best: None,
            stop_at_first,
            stack: [(CompactChild { first: 0, prim_count: 0 }, FixedQ32_32::ZERO); STACK_CAPACITY],
            len: 0,
        }
    }

    fn run<F>(mut self, mut hit: F) -> Result<Option<ClosestHit>, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        if let Some(root) = self.bvh.root {
            let enter = self.entry(&root.bounds);
            self.push((root.child, enter))?;
        }
        while let Some(child) = self.pop()? {
            if self.visit(child, &mut hit)? {
                break;
            }
        }
        Ok(self.best)
    }

    fn entry(&self, bounds: &FixedAabb3) -> Option<FixedQ32_32> {
        self.ray.box_entry(bounds, self.limit)
    }

    fn push(&mut self, (child, enter): Entry) -> Result<(), TraversalError> {
        let Some(enter) = enter else { return Ok(()) };
        *self.stack.get_mut(self.len).ok_or(TraversalError::StackOverflow)? = (child, enter);
        self.len += 1;
        Ok(())
    }

    /// The next child that can still hold a better hit; a bound equal to the limit is kept.
    fn pop(&mut self) -> Result<Option<CompactChild>, TraversalError> {
        while let Some(len) = self.len.checked_sub(1) {
            self.len = len;
            let &(child, enter) = self.stack.get(len).ok_or(TraversalError::StackOverflow)?;
            if enter <= self.limit {
                return Ok(Some(child));
            }
        }
        Ok(None)
    }

    /// Visits a child; returns whether the search is over.
    fn visit<F>(&mut self, child: CompactChild, hit: &mut F) -> Result<bool, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        if child.is_leaf() {
            return self.visit_leaf(child, hit);
        }
        let node =
            self.bvh.nodes.get(child.first as usize).ok_or(TraversalError::InvalidNodeIndex)?;
        let ([left, right], [left_box, right_box]) = (node.children, &node.bounds);
        let (left, right) = ((left, self.entry(left_box)), (right, self.entry(right_box)));
        let (near, far) = nearer_first(left, right);
        // The farther child goes underneath, so the nearer one is visited first.
        self.push(far)?;
        self.push(near)?;
        Ok(false)
    }

    fn visit_leaf<F>(&mut self, leaf: CompactChild, hit: &mut F) -> Result<bool, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        let start = leaf.first as usize;
        let end = start
            .checked_add(leaf.prim_count as usize)
            .ok_or(TraversalError::InvalidPrimitiveRange)?;
        let primitives =
            self.bvh.prim_indices.get(start..end).ok_or(TraversalError::InvalidPrimitiveRange)?;
        for &primitive in primitives {
            let accepted = hit(primitive, self.limit).is_some_and(|t| self.accept(primitive, t));
            if accepted && self.stop_at_first {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Keeps a hit in `0..=limit` that is nearer than the best so far, or as near with a lower
    /// index; returns whether it was kept.
    fn accept(&mut self, primitive: u32, distance: FixedQ32_32) -> bool {
        let candidate = (distance, primitive);
        let better = (FixedQ32_32::ZERO..=self.limit).contains(&distance)
            && self.best.is_none_or(|best| candidate < (best.distance, best.primitive));
        if better {
            self.best = Some(ClosestHit { primitive, distance });
            self.limit = distance;
        }
        better
    }
}

/// Orders two children by entry bound, nearer first; a tie keeps the left child first.
fn nearer_first(left: Entry, right: Entry) -> (Entry, Entry) {
    match (left.1, right.1) {
        (Some(l), Some(r)) if r < l => (right, left),
        _ => (left, right),
    }
}
