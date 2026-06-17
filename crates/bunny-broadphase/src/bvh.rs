//! Flat array-backed Bounding Volume Hierarchy (BVH).

mod build;
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
