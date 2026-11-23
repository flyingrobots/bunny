//! Nearest-first BVH search for the closest point of a set of triangles to a query point.
//!
//! The search visits the child whose box is nearer first and skips every subtree whose box
//! is farther than the best point found so far. The box bound is never above the exact
//! distance and points are compared exactly, so the result is the one a test of every
//! primitive would choose, whatever the shape of the tree. See
//! `docs/topics/mesh-queries/README.md`.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TrianglePoint};

use crate::bvh::BvhNode;
use crate::closest_hit::BvhView;
use crate::traversal::TraversalError;

const STACK_CAPACITY: usize = 64;

/// The closest point of a set of primitives to a query point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosestPoint {
    /// Index of the primitive, as passed to the BVH builder.
    pub primitive: u32,
    /// [`TrianglePoint::point`] of that primitive: the exact closest point, rounded.
    pub point: FixedVec3,
    /// [`TrianglePoint::distance_squared`] of that primitive: the exact squared distance,
    /// rounded.
    pub distance_squared: FixedQ32_32,
}

/// Finds the primitive whose closest point to `query` is nearest.
///
/// `nearest(primitive)` returns that primitive's closest point, or `None` to leave it out.
/// Distances are compared exactly; of several primitives at the same exact distance, the
/// lowest index wins. The answer equals a test of every primitive, whatever the tree's shape
/// and leaf order, when every point the closure returns lies in that primitive's box and the
/// tree's node bounds contain the boxes below them, as `build_bvh` and `build_bvh_sah`
/// guarantee for triangle bounds and [`PointQuery::closest_on_triangle`].
///
/// # Errors
/// The errors of [`crate::closest_hit`]: `TraversalError::StackOverflow` when the search
/// needs more than its 64 stack entries (never for a tree of depth at most 63),
/// `TraversalError::InvalidNodeIndex` or `TraversalError::InvalidPrimitiveRange` for a
/// malformed node the search visits, and `InvalidNodeIndex` once the search has made more than
/// `nodes.len() / 2` interior visits, which only a buffer that is not a tree causes.
pub fn closest_point<F>(
    bvh: BvhView<'_>,
    query: &PointQuery,
    nearest: F,
) -> Result<Option<ClosestPoint>, TraversalError>
where
    F: FnMut(u32) -> Option<TrianglePoint>,
{
    let mut search = Search { bvh, query, best: None, stack: [(0, 0); STACK_CAPACITY], len: 0 };
    search.run(nearest)?;
    Ok(search.best.map(|(primitive, found)| ClosestPoint {
        primitive,
        point: found.point(),
        distance_squared: found.distance_squared(),
    }))
}

struct Search<'a> {
    bvh: BvhView<'a>,
    query: &'a PointQuery,
    best: Option<(u32, TrianglePoint)>,
    /// Node indices waiting, each with the bound on its box distance.
    stack: [(u32, u128); STACK_CAPACITY],
    len: usize,
}

impl<'a> Search<'a> {
    fn run<F>(&mut self, mut nearest: F) -> Result<(), TraversalError>
    where
        F: FnMut(u32) -> Option<TrianglePoint>,
    {
        if !self.bvh.nodes.is_empty() {
            let root = self.child(0)?;
            self.push(root)?;
        }
        // A tree of `n` nodes has at most `n / 2` interior nodes and visits each once, so
        // running out proves a node was reached twice (as in `closest_hit`).
        let mut interior_visits_left = self.bvh.nodes.len() / 2;
        while let Some(node) = self.pop()? {
            if node.prim_count > 0 {
                self.visit_leaf(node, &mut nearest)?;
                continue;
            }
            interior_visits_left =
                interior_visits_left.checked_sub(1).ok_or(TraversalError::InvalidNodeIndex)?;
            self.visit_interior(node)?;
        }
        Ok(())
    }

    fn node(&self, index: u32) -> Result<&'a BvhNode, TraversalError> {
        self.bvh.nodes.get(index as usize).ok_or(TraversalError::InvalidNodeIndex)
    }

    /// A node with the bound on the squared distance from the query point to its box.
    fn child(&self, index: u32) -> Result<(u32, u128), TraversalError> {
        Ok((index, self.query.box_distance_bound(&self.node(index)?.bounds)))
    }

    fn push(&mut self, entry: (u32, u128)) -> Result<(), TraversalError> {
        *self.stack.get_mut(self.len).ok_or(TraversalError::StackOverflow)? = entry;
        self.len += 1;
        Ok(())
    }

    /// The next node that can still hold a point as near as the best. A node whose bound
    /// equals the best distance is kept: it may hold an equally near primitive with a lower
    /// index.
    fn pop(&mut self) -> Result<Option<&'a BvhNode>, TraversalError> {
        while let Some(len) = self.len.checked_sub(1) {
            self.len = len;
            let &(index, bound) = self.stack.get(len).ok_or(TraversalError::StackOverflow)?;
            let beaten = self.best.is_some_and(|(_, best)| best.cmp_bound(bound).is_lt());
            if !beaten {
                return self.node(index).map(Some);
            }
        }
        Ok(None)
    }

    /// Pushes both children, the farther first, so the nearer is searched first; equal
    /// bounds search the left child first.
    fn visit_interior(&mut self, node: &BvhNode) -> Result<(), TraversalError> {
        let left = node.first_child_or_prim_idx;
        let right = left.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        let (left, right) = (self.child(left)?, self.child(right)?);
        let (near, far) = if right.1 < left.1 { (right, left) } else { (left, right) };
        self.push(far)?;
        self.push(near)
    }

    fn visit_leaf<F>(&mut self, node: &BvhNode, nearest: &mut F) -> Result<(), TraversalError>
    where
        F: FnMut(u32) -> Option<TrianglePoint>,
    {
        let start = node.first_child_or_prim_idx as usize;
        let end = start
            .checked_add(node.prim_count as usize)
            .ok_or(TraversalError::InvalidPrimitiveRange)?;
        let primitives =
            self.bvh.prim_indices.get(start..end).ok_or(TraversalError::InvalidPrimitiveRange)?;
        for &primitive in primitives {
            if let Some(found) = nearest(primitive) {
                self.accept(primitive, found);
            }
        }
        Ok(())
    }

    /// Keeps a point nearer than the best so far, or as near with a lower primitive index.
    fn accept(&mut self, primitive: u32, found: TrianglePoint) {
        let better = self.best.is_none_or(|(best_primitive, best)| {
            found.cmp_distance(&best).then(primitive.cmp(&best_primitive)).is_lt()
        });
        if better {
            self.best = Some((primitive, found));
        }
    }
}
