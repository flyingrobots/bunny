//! Front-to-back BVH ray traversal: the closest hit, or whether anything is hit.
//!
//! Unlike [`crate::intersect_ray`], which reports every primitive whose box the ray's line
//! touches, these searches visit the nearer child first and skip every subtree that starts
//! beyond the best hit so far. The result does not depend on the shape of the tree: it is the
//! hit a test of every primitive would choose. See `docs/topics/ray-tracing/README.md`.

use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::bvh::BvhNode;
use crate::traversal::TraversalError;

const STACK_CAPACITY: usize = 64;

/// A node index and the bound on the distance at which the ray enters it, if it can.
type Child = (u32, Option<FixedQ32_32>);

/// The node and primitive-index buffers of a BVH, as filled by [`crate::build_bvh`].
#[derive(Clone, Copy, Debug)]
pub struct BvhView<'a> {
    /// The used nodes; the root is node 0.
    pub nodes: &'a [BvhNode],
    /// The primitive indices that leaves refer to.
    pub prim_indices: &'a [u32],
}

impl<'a> BvhView<'a> {
    /// Borrows a BVH's buffers.
    #[must_use]
    pub const fn new(nodes: &'a [BvhNode], prim_indices: &'a [u32]) -> Self {
        Self { nodes, prim_indices }
    }
}

/// The nearest primitive a ray hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosestHit {
    /// Index of the primitive, as passed to `build_bvh`.
    pub primitive: u32,
    /// Distance the primitive test reported.
    pub distance: FixedQ32_32,
}

/// Finds the nearest primitive hit by `ray` within `0..=t_max`.
///
/// `hit(primitive, limit)` tests one primitive and returns its hit distance, or `None` for a
/// miss; distances outside `0..=limit` are ignored. Of several primitives at the same
/// distance, the lowest index wins. The answer equals a test of every primitive, whatever
/// the tree's shape, when every reported distance is at least the floor of a distance at
/// which the ray is inside that primitive's box, as [`TraceRay::hit_triangle`] guarantees.
///
/// ```
/// use bunny_broadphase::{build_bvh, closest_hit, BvhNode, BvhView};
/// use bunny_geom::FixedAabb3;
/// use bunny_linalg::FixedVec3;
/// use bunny_num::{fixed_q32_32::ONE_RAW, FixedQ32_32};
/// use bunny_query::{TraceRay, TraceTriangle};
///
/// let units = |n: i64| FixedQ32_32::from_raw(n * ONE_RAW);
/// let at = |x, y, z| FixedVec3::new(units(x), units(y), units(z));
/// // Two triangles facing the ray, at z = 0 and z = -2.
/// let triangles: Vec<TraceTriangle> = [0, -2]
///     .map(|z| TraceTriangle::new(at(-1, -1, z), at(1, -1, z), at(0, 1, z)).expect("in domain"))
///     .to_vec();
/// let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
/// let empty = BvhNode { bounds: boxes[0], first_child_or_prim_idx: 0, prim_count: 0 };
/// let (mut nodes, mut prim_indices) = (vec![empty; 3], vec![0; 2]);
/// let used = build_bvh(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
///
/// let ray = TraceRay::new(at(0, 0, 5), at(0, 0, -1)).expect("in domain");
/// let bvh = BvhView::new(&nodes[..used], &prim_indices);
/// let found = closest_hit(bvh, &ray, units(100), |primitive, limit| {
///     let hit = ray.hit_triangle(&triangles[primitive as usize], FixedQ32_32::ZERO, limit);
///     hit.map(|hit| hit.distance)
/// });
/// let nearest = found.expect("well formed").expect("the ray hits");
/// assert_eq!((nearest.primitive, nearest.distance), (0, units(5)));
/// ```
///
/// # Errors
/// Returns `TraversalError::StackOverflow` if the tree is deeper than 64 levels, and
/// `TraversalError::InvalidNodeIndex` or `TraversalError::InvalidPrimitiveRange` if a node
/// the search visits is malformed. A buffer that is not a tree (a child index naming an
/// ancestor, or a subtree reached twice) is `InvalidNodeIndex` once the search has made more
/// than `nodes.len() / 2` interior visits, so it cannot loop.
pub fn closest_hit<F>(
    bvh: BvhView<'_>,
    ray: &TraceRay,
    t_max: FixedQ32_32,
    hit: F,
) -> Result<Option<ClosestHit>, TraversalError>
where
    F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
{
    Search::new(bvh, ray, t_max, false).run(hit)
}

/// Reports whether `ray` hits any primitive within `0..=t_max`, stopping at the first hit.
///
/// `hit(primitive)` tests one primitive within that range. It is not called again after it
/// first returns `true`.
///
/// # Errors
/// Returns the same errors as [`closest_hit`].
pub fn any_hit<F>(
    bvh: BvhView<'_>,
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
    bvh: BvhView<'a>,
    ray: &'a TraceRay,
    /// Subtrees that start beyond this distance cannot hold a better hit.
    limit: FixedQ32_32,
    best: Option<ClosestHit>,
    /// End the search at the first accepted hit.
    stop_at_first: bool,
    stack: [(u32, FixedQ32_32); STACK_CAPACITY],
    len: usize,
    /// Interior visits still allowed. A tree of `n` nodes has at most `n / 2` interior nodes
    /// and visits each once, so running out proves a node was reached twice.
    interior_visits_left: usize,
}

impl<'a> Search<'a> {
    const fn new(
        bvh: BvhView<'a>,
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
            stack: [(0, FixedQ32_32::ZERO); STACK_CAPACITY],
            len: 0,
            interior_visits_left: bvh.nodes.len() / 2,
        }
    }

    fn run<F>(mut self, mut hit: F) -> Result<Option<ClosestHit>, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        if !self.bvh.nodes.is_empty() {
            let root = self.entry(0)?;
            self.push((0, root))?;
        }
        let mut interior_visits_left = self.interior_visits_left;
        while let Some(node) = self.pop()? {
            if node.prim_count == 0 {
                interior_visits_left =
                    interior_visits_left.checked_sub(1).ok_or(TraversalError::InvalidNodeIndex)?;
            }
            if self.visit(node, &mut hit)? {
                break;
            }
        }
        Ok(self.best)
    }

    fn node(&self, index: u32) -> Result<&'a BvhNode, TraversalError> {
        self.bvh.nodes.get(index as usize).ok_or(TraversalError::InvalidNodeIndex)
    }

    /// Lower bound on the distance at which the ray enters a node, if it can within the limit.
    fn entry(&self, index: u32) -> Result<Option<FixedQ32_32>, TraversalError> {
        Ok(self.ray.box_entry(&self.node(index)?.bounds, self.limit))
    }

    /// Pushes a node the ray can enter; a node it cannot enter is dropped.
    fn push(&mut self, (index, enter): Child) -> Result<(), TraversalError> {
        let Some(enter) = enter else { return Ok(()) };
        *self.stack.get_mut(self.len).ok_or(TraversalError::StackOverflow)? = (index, enter);
        self.len += 1;
        Ok(())
    }

    /// The next node that can still hold a better hit. A node whose entry bound equals the
    /// best distance is kept: it may hold an equally near primitive with a lower index.
    fn pop(&mut self) -> Result<Option<&'a BvhNode>, TraversalError> {
        while let Some(len) = self.len.checked_sub(1) {
            self.len = len;
            let &(index, enter) = self.stack.get(len).ok_or(TraversalError::StackOverflow)?;
            if enter <= self.limit {
                return self.node(index).map(Some);
            }
        }
        Ok(None)
    }

    /// Visits a node; returns whether the search is over.
    fn visit<F>(&mut self, node: &BvhNode, hit: &mut F) -> Result<bool, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        if node.prim_count > 0 {
            return self.visit_leaf(node, hit);
        }
        let left = node.first_child_or_prim_idx;
        let right = left.checked_add(1).ok_or(TraversalError::InvalidNodeIndex)?;
        let (near, far) = nearer_first((left, self.entry(left)?), (right, self.entry(right)?));
        // The farther child goes underneath, so the nearer one is visited first.
        self.push(far)?;
        self.push(near)?;
        Ok(false)
    }

    fn visit_leaf<F>(&mut self, node: &BvhNode, hit: &mut F) -> Result<bool, TraversalError>
    where
        F: FnMut(u32, FixedQ32_32) -> Option<FixedQ32_32>,
    {
        let start = node.first_child_or_prim_idx as usize;
        let end = start
            .checked_add(node.prim_count as usize)
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
fn nearer_first(left: Child, right: Child) -> (Child, Child) {
    match (left.1, right.1) {
        (Some(l), Some(r)) if r < l => (right, left),
        _ => (left, right),
    }
}
