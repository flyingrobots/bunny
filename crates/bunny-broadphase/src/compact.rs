//! A second BVH layout for ray traversal: each interior node holds both of its children's
//! boxes, so one traversal step reads one node and tests two boxes from one place.
//!
//! [`compact_bvh`] converts a tree that [`crate::build_bvh`] filled into this layout, in a
//! caller-provided buffer. [`crate::compact_closest_hit`] and [`crate::compact_any_hit`]
//! search it and return what [`crate::closest_hit`] and [`crate::any_hit`] return on the
//! source tree. See `docs/topics/ray-tracing/README.md`.

use bunny_geom::FixedAabb3;

use crate::bvh::BvhNode;

/// Interior subtrees the conversion can queue at once. A tree of depth `d` queues at most `d`,
/// so every tree of depth at most 64 fits; this is not a bound on depth as such.
const STACK_CAPACITY: usize = 64;

/// What a child slot refers to: another compact node, or a leaf's primitive range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompactChild {
    /// Index of the child's [`CompactNode`] when `prim_count` is zero; otherwise the first
    /// slot of the leaf's primitives in the primitive-index buffer.
    pub first: u32,
    /// Number of primitives in a leaf; zero for an interior child.
    pub prim_count: u32,
}

impl CompactChild {
    /// Whether the child is a leaf.
    #[must_use]
    pub const fn is_leaf(self) -> bool {
        self.prim_count > 0
    }
}

/// An interior node: the boxes and references of its left and right children, in that order.
///
/// Leaves have no node of their own; their primitive ranges live in their parent. A node is
/// 112 bytes with 8-byte alignment; padding it to 128 bytes was measured and not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactNode {
    /// Bounds of the left and the right child.
    pub bounds: [FixedAabb3; 2],
    /// The left and the right child.
    pub children: [CompactChild; 2],
}

/// The root of a compact tree: its box and what it is (a leaf, or compact node 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactRoot {
    /// Bounds of the whole tree.
    pub bounds: FixedAabb3,
    /// The root itself: a leaf for a one-leaf tree, otherwise compact node 0.
    pub child: CompactChild,
}

/// Why a tree could not be converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactError {
    /// An interior node's children are outside the node slice.
    InvalidNodeIndex,
    /// The output buffer holds fewer nodes than the tree has interior nodes.
    OutputTooSmall,
    /// The nodes do not form a tree: more interior nodes are reached than a tree of this
    /// many nodes can have, so some node is reached twice.
    NotATree,
    /// More than 64 interior subtrees would wait to be written at once. A tree of depth `d`
    /// (the root at depth 0) queues at most `d`, so only a tree of depth 65 or more can cause
    /// this, and not every such tree does (a comb whose right child is the next level queues
    /// one).
    TooDeep,
}

/// Converts a BVH from [`crate::build_bvh`] into the compact layout.
///
/// `nodes` is the used node slice (the root is node 0). The compact nodes are written to the
/// front of `out` in depth-first order, each node followed by its left subtree and then its
/// right subtree. A tree of `n` nodes has `(n - 1) / 2` interior nodes; `out` needs at least
/// that many. Returns the root, or `None` for an empty tree, and the number of nodes written.
/// The conversion is integer-only and gives the same bytes on every target.
///
/// # Errors
/// Returns [`CompactError`] when a node index is out of range, `out` is too small, a node is
/// reached twice or more than 64 interior subtrees would wait at once (see
/// [`CompactError::TooDeep`]). Leaf primitive ranges are copied as
/// they are and checked by the traversal, as [`crate::closest_hit`] checks them.
pub fn compact_bvh(
    nodes: &[BvhNode],
    out: &mut [CompactNode],
) -> Result<Option<(CompactRoot, usize)>, CompactError> {
    let Some(root) = nodes.first() else { return Ok(None) };
    let child = reference(root, 0);
    if child.is_leaf() {
        return Ok(Some((CompactRoot { bounds: root.bounds, child }, 0)));
    }
    let written = Converter::new(nodes, out).run()?;
    Ok(Some((CompactRoot { bounds: root.bounds, child }, written)))
}

/// The reference a parent stores for `node`; an interior node gets compact index `index`.
const fn reference(node: &BvhNode, index: u32) -> CompactChild {
    if node.prim_count > 0 {
        CompactChild { first: node.first_child_or_prim_idx, prim_count: node.prim_count }
    } else {
        CompactChild { first: index, prim_count: 0 }
    }
}

/// A source interior node still to write, and the parent slot that must point at it.
#[derive(Clone, Copy)]
struct Pending {
    source: usize,
    parent: Option<(usize, usize)>,
}

struct Converter<'a> {
    nodes: &'a [BvhNode],
    out: &'a mut [CompactNode],
    /// A tree of `n` nodes has at most `n / 2` interior nodes.
    capacity: usize,
    written: usize,
    stack: [Pending; STACK_CAPACITY],
    len: usize,
}

impl<'a> Converter<'a> {
    const fn new(nodes: &'a [BvhNode], out: &'a mut [CompactNode]) -> Self {
        let empty = Pending { source: 0, parent: None };
        let capacity = nodes.len() / 2;
        Self { nodes, out, capacity, written: 0, stack: [empty; STACK_CAPACITY], len: 0 }
    }

    fn run(mut self) -> Result<usize, CompactError> {
        self.push(Pending { source: 0, parent: None })?;
        while let Some(len) = self.len.checked_sub(1) {
            self.len = len;
            let pending = *self.stack.get(len).ok_or(CompactError::TooDeep)?;
            self.write(pending)?;
        }
        Ok(self.written)
    }

    fn push(&mut self, pending: Pending) -> Result<(), CompactError> {
        *self.stack.get_mut(self.len).ok_or(CompactError::TooDeep)? = pending;
        self.len += 1;
        Ok(())
    }

    /// Writes the next compact node, points its parent's slot at it and queues its interior
    /// children, the right one underneath so the left subtree is written first.
    fn write(&mut self, pending: Pending) -> Result<(), CompactError> {
        let source = self.nodes.get(pending.source).ok_or(CompactError::InvalidNodeIndex)?;
        let left = source.first_child_or_prim_idx as usize;
        let right = left.checked_add(1).ok_or(CompactError::InvalidNodeIndex)?;
        let pair = [self.source(left)?, self.source(right)?];
        let index = self.claim()?;
        let node = CompactNode {
            bounds: pair.map(|child| child.bounds),
            children: pair.map(|child| reference(&child, 0)),
        };
        *self.out.get_mut(index).ok_or(CompactError::OutputTooSmall)? = node;
        self.link(pending.parent, index)?;
        for (side, (child, at)) in pair.iter().zip([left, right]).enumerate().rev() {
            if child.prim_count == 0 {
                self.push(Pending { source: at, parent: Some((index, side)) })?;
            }
        }
        Ok(())
    }

    fn source(&self, index: usize) -> Result<BvhNode, CompactError> {
        self.nodes.get(index).copied().ok_or(CompactError::InvalidNodeIndex)
    }

    /// The index of the next node to write.
    const fn claim(&mut self) -> Result<usize, CompactError> {
        let index = self.written;
        if index >= self.capacity {
            return Err(CompactError::NotATree);
        }
        if index >= self.out.len() {
            return Err(CompactError::OutputTooSmall);
        }
        self.written += 1;
        Ok(index)
    }

    /// Points slot `side` of compact node `parent` at compact node `index`.
    fn link(&mut self, parent: Option<(usize, usize)>, index: usize) -> Result<(), CompactError> {
        let Some((parent, side)) = parent else { return Ok(()) };
        let first = u32::try_from(index).map_err(|_| CompactError::OutputTooSmall)?;
        let node = self.out.get_mut(parent).ok_or(CompactError::OutputTooSmall)?;
        let slot = node.children.get_mut(side).ok_or(CompactError::InvalidNodeIndex)?;
        slot.first = first;
        Ok(())
    }
}
