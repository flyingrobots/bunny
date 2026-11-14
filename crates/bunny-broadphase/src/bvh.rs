//! Flat array-backed Bounding Volume Hierarchy (BVH).

mod build;
mod sah;
mod split;

use bunny_geom::FixedAabb3;

/// A node in the flat array-backed bounding volume hierarchy (BVH).
#[derive(Clone, Copy, Debug)]
pub struct BvhNode {
    /// Bounding box enclosing this node's children or primitives.
    pub bounds: FixedAabb3,
    /// Left child index (if interior node) or starting primitive index (if leaf).
    pub first_child_or_prim_idx: u32,
    /// Number of primitives in this node. If 0, this is an interior node.
    pub prim_count: u32,
}

/// Builds a flat BVH in-place into the provided node and primitive index buffers.
///
/// Returns the number of nodes actually used, or `None` if buffers are too small.
#[must_use]
pub fn build_bvh(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    build::build_bvh(nodes, prim_indices, primitives)
}

/// Number of centroid bins per axis that [`build_bvh_sah`] evaluates split planes over.
pub const SAH_BIN_COUNT: usize = 16;

/// Largest node [`build_bvh_sah`] may keep as a leaf; larger nodes are always split
/// (unless they reach [`SAH_MAX_DEPTH`]).
pub const SAH_MAX_LEAF_SIZE: usize = 4;

/// Deepest level [`build_bvh_sah`] creates (the root is level 0).
///
/// A node at this depth is a leaf whatever its size. A tree this deep needs at most 61
/// entries of the 64-entry stack that `intersect_aabb`, `intersect_ray`, `closest_hit` and
/// `any_hit` use.
pub const SAH_MAX_DEPTH: usize = 60;

/// Weight of visiting a node in the cost [`build_bvh_sah`] minimizes.
pub const SAH_TRAVERSAL_COST: u128 = 1;

/// Weight of testing one primitive in the cost [`build_bvh_sah`] minimizes.
pub const SAH_INTERSECTION_COST: u128 = 1;

/// Builds a flat BVH with a binned surface area heuristic into the provided node and
/// primitive index buffers.
///
/// The buffers and the result are those of [`build_bvh`]: `nodes` needs room for
/// `2 * primitives.len() - 1` nodes and `prim_indices` for `primitives.len()` entries; the
/// root is node 0, an interior node's children are the adjacent pair starting at
/// `first_child_or_prim_idx`, and a leaf lists `prim_count` entries of `prim_indices`
/// starting there. Every traversal in this crate accepts the tree. The output differs from
/// [`build_bvh`]'s.
///
/// At each node, primitive centroids are sorted into [`SAH_BIN_COUNT`] equal-width bins
/// per axis and every plane between bins is costed as
/// `SAH_TRAVERSAL_COST * area(node) + SAH_INTERSECTION_COST * (area(left) * left_count +
/// area(right) * right_count)`, against a leaf cost of
/// `SAH_INTERSECTION_COST * area(node) * count`, with half surface areas in integer raw
/// units. The tree is a deterministic function of `primitives` alone:
///
/// - the cheapest plane wins; equal costs go to the lower axis (x, then y, then z) and
///   then the lower plane;
/// - a node of at most [`SAH_MAX_LEAF_SIZE`] primitives splits only when the split is
///   strictly cheaper than a leaf; a larger node always splits, at the cheapest plane, or
///   in half by primitive order when all its centroids are equal;
/// - a node of one primitive, or at depth [`SAH_MAX_DEPTH`], is a leaf;
/// - every node's entries in `prim_indices` are in ascending primitive order, so each leaf
///   lists its primitives in ascending index order.
///
/// Arithmetic is integer only and never overflows (areas are computed in `u128` after a
/// per-node shift that keeps extents below `2^45` raw units; the shift is zero, and areas
/// exact, for nodes narrower than 8 192 units). Nothing is allocated and the build does
/// not recurse.
///
/// Returns the number of nodes used, or `None` if the buffers are too small.
#[must_use]
pub fn build_bvh_sah(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    sah::build_bvh_sah(nodes, prim_indices, primitives)
}
