//! Refit: new boxes for the primitives of a built BVH, with its topology and primitive order
//! unchanged. See `docs/topics/bvh-construction/README.md#refit`.

use bunny_geom::FixedAabb3;

use crate::bvh::BvhNode;
use crate::utils::union_aabb;

/// Why [`refit_bvh`] refused a buffer. A refused refit writes nothing.
///
/// `node` is the index of the first node, in index order, that has the fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefitError {
    /// An interior node's children (`first_child_or_prim_idx` and the node after it) are not
    /// both inside the node slice.
    InvalidNodeIndex {
        /// The interior node.
        node: usize,
    },
    /// An interior node's first child is at or before the node itself. Both builders store
    /// children after their parent, so this is not a built tree; every cycle has such a node.
    ChildNotAfterParent {
        /// The interior node.
        node: usize,
    },
    /// A leaf's slots (`prim_count` entries from `first_child_or_prim_idx`) are not all
    /// inside the primitive-index slice.
    InvalidPrimitiveRange {
        /// The leaf.
        node: usize,
    },
    /// The slice does not have one more leaf than interior nodes, as every tree a builder
    /// writes has. This is a count, not a proof that the nodes form a tree.
    NotATree {
        /// Interior nodes in the slice.
        interior: usize,
        /// Leaves in the slice.
        leaves: usize,
    },
    /// The leaves list `listed` slots in all (saturating at `usize::MAX`), which is the
    /// number of primitives the tree was built over, but `boxes` boxes were given.
    PrimitiveCountMismatch {
        /// Slots the leaves list.
        listed: usize,
        /// Boxes given.
        boxes: usize,
    },
    /// A leaf lists a primitive index that has no box (at least `primitives.len()`).
    InvalidPrimitiveIndex {
        /// The leaf.
        node: usize,
    },
}

/// Replaces every node's bounds with the exact union of its new contents, keeping the tree.
///
/// `nodes` is the used node slice a builder filled (the root is node 0), `prim_indices` the
/// primitive-index buffer it filled, and `primitives` one new box per primitive, in the
/// order the tree was built over. Afterwards every leaf's bounds are the union of the boxes
/// of the primitives it lists and every interior node's bounds the union of its two
/// children's: per axis the least `min` and the greatest `max` of raw values, so nothing is
/// rounded. An inverted box (a `min` above its `max`) is taken as it is, as both builders
/// take it. Only `bounds` fields change: the topology and `prim_indices` are untouched, so
/// the result depends only on them and on `primitives`.
///
/// Both builders store an interior node's children after it (`first_child_or_prim_idx` is
/// greater than the node's own index), so one pass from the last node to the first finds
/// every child already updated: there is no stack, no recursion, no scratch buffer and no
/// allocation. Refit applies to this layout only; a compact tree is converted again with
/// [`crate::compact_bvh`].
///
/// A refit preserves correctness, not quality: the searches return the same answers on a
/// refitted tree as on a rebuilt one, but its boxes can overlap far more.
///
/// # Errors
/// The whole buffer is checked before anything is written, so a refusal leaves it exactly
/// as it was. In this order: each node from first to last, an interior node for
/// [`RefitError::InvalidNodeIndex`] and then [`RefitError::ChildNotAfterParent`], a leaf for
/// [`RefitError::InvalidPrimitiveRange`]; then [`RefitError::NotATree`]; then
/// [`RefitError::PrimitiveCountMismatch`]; then each leaf from first to last for
/// [`RefitError::InvalidPrimitiveIndex`]. A primitive listed twice is not detected.
pub fn refit_bvh(
    nodes: &mut [BvhNode],
    prim_indices: &[u32],
    primitives: &[FixedAabb3],
) -> Result<(), RefitError> {
    validate(nodes, prim_indices, primitives)?;
    write(nodes, prim_indices, primitives)
}

#[derive(Default)]
struct Tally {
    interior: usize,
    leaves: usize,
    listed: usize,
}

fn validate(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    primitives: &[FixedAabb3],
) -> Result<(), RefitError> {
    let tally = check_structure(nodes, prim_indices)?;
    let (interior, leaves) = (tally.interior, tally.leaves);
    if !nodes.is_empty() && interior.checked_add(1) != Some(leaves) {
        return Err(RefitError::NotATree { interior, leaves });
    }
    if tally.listed != primitives.len() {
        return Err(RefitError::PrimitiveCountMismatch {
            listed: tally.listed,
            boxes: primitives.len(),
        });
    }
    check_primitives(nodes, prim_indices, primitives.len())
}

fn check_structure(nodes: &[BvhNode], prim_indices: &[u32]) -> Result<Tally, RefitError> {
    let mut tally = Tally::default();
    for (index, node) in nodes.iter().enumerate() {
        if node.prim_count == 0 {
            check_children(index, node, nodes.len())?;
            tally.interior += 1;
        } else {
            let slots = leaf_slots(index, node, prim_indices)?;
            tally.leaves += 1;
            tally.listed = tally.listed.saturating_add(slots.len());
        }
    }
    Ok(tally)
}

/// The left child's index, once both children are in the slice and after `index`.
fn check_children(index: usize, node: &BvhNode, len: usize) -> Result<usize, RefitError> {
    let left = usize::try_from(node.first_child_or_prim_idx).ok();
    let right = left.and_then(|left| left.checked_add(1));
    let (Some(left), Some(right)) = (left, right) else {
        return Err(RefitError::InvalidNodeIndex { node: index });
    };
    if right >= len {
        return Err(RefitError::InvalidNodeIndex { node: index });
    }
    if left <= index {
        return Err(RefitError::ChildNotAfterParent { node: index });
    }
    Ok(left)
}

fn leaf_slots<'a>(
    index: usize,
    node: &BvhNode,
    prim_indices: &'a [u32],
) -> Result<&'a [u32], RefitError> {
    let fault = RefitError::InvalidPrimitiveRange { node: index };
    let start = usize::try_from(node.first_child_or_prim_idx).map_err(|_| fault)?;
    let count = usize::try_from(node.prim_count).map_err(|_| fault)?;
    let end = start.checked_add(count).ok_or(fault)?;
    prim_indices.get(start..end).ok_or(fault)
}

fn check_primitives(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    boxes: usize,
) -> Result<(), RefitError> {
    for (index, node) in nodes.iter().enumerate().filter(|(_, node)| node.prim_count > 0) {
        let in_range = |&slot: &u32| usize::try_from(slot).is_ok_and(|slot| slot < boxes);
        if !leaf_slots(index, node, prim_indices)?.iter().all(in_range) {
            return Err(RefitError::InvalidPrimitiveIndex { node: index });
        }
    }
    Ok(())
}

/// The writing pass, last node first. Validation has ruled out every error it can return;
/// they are kept so that no index can panic.
fn write(
    nodes: &mut [BvhNode],
    prim_indices: &[u32],
    primitives: &[FixedAabb3],
) -> Result<(), RefitError> {
    for index in (0..nodes.len()).rev() {
        let bounds = new_bounds(nodes, index, prim_indices, primitives)?;
        if let Some(node) = nodes.get_mut(index) {
            node.bounds = bounds;
        }
    }
    Ok(())
}

fn new_bounds(
    nodes: &[BvhNode],
    index: usize,
    prim_indices: &[u32],
    primitives: &[FixedAabb3],
) -> Result<FixedAabb3, RefitError> {
    let node = nodes.get(index).ok_or(RefitError::InvalidNodeIndex { node: index })?;
    if node.prim_count > 0 {
        return leaf_bounds(index, leaf_slots(index, node, prim_indices)?, primitives);
    }
    let left = check_children(index, node, nodes.len())?;
    let child = |at: usize| nodes.get(at).map(|child| child.bounds);
    let pair = child(left).zip(left.checked_add(1).and_then(child));
    let (left, right) = pair.ok_or(RefitError::InvalidNodeIndex { node: index })?;
    Ok(union_aabb(left, right))
}

fn leaf_bounds(
    index: usize,
    slots: &[u32],
    primitives: &[FixedAabb3],
) -> Result<FixedAabb3, RefitError> {
    let fault = RefitError::InvalidPrimitiveIndex { node: index };
    let lookup = |slot: &u32| usize::try_from(*slot).ok().and_then(|at| primitives.get(at));
    let (first, rest) = slots.split_first().ok_or(fault)?;
    let mut bounds = *lookup(first).ok_or(fault)?;
    for slot in rest {
        bounds = union_aabb(bounds, *lookup(slot).ok_or(fault)?);
    }
    Ok(bounds)
}
