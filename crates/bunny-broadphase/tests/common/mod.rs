//! Shared helpers for the BVH builder tests: seeded inputs, a structural checker and an
//! exact SAH cost of a built tree.

#![allow(dead_code)]

use bunny_broadphase::bvh::{SAH_INTERSECTION_COST, SAH_MAX_DEPTH, SAH_TRAVERSAL_COST};
use bunny_broadphase::BvhNode;
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

pub(crate) const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;

/// A 64-bit linear congruential generator; every test names its seed.
pub(crate) struct Seeds(pub(crate) u64);

impl Seeds {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    pub(crate) fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    pub(crate) fn point(&mut self, bound: i64) -> FixedVec3 {
        vec3([self.signed(bound), self.signed(bound), self.signed(bound)])
    }
}

pub(crate) fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

pub(crate) fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(vec3(min), vec3(max))
}

pub(crate) fn empty_node() -> BvhNode {
    BvhNode { bounds: aabb([0; 3], [0; 3]), first_child_or_prim_idx: 0, prim_count: 0 }
}

/// A built tree, truncated to the nodes the builder used.
#[derive(Clone)]
pub(crate) struct Tree {
    pub(crate) nodes: Vec<BvhNode>,
    pub(crate) prim_indices: Vec<u32>,
}

pub(crate) type Builder = fn(&mut [BvhNode], &mut [u32], &[FixedAabb3]) -> Option<usize>;

pub(crate) fn build_with(builder: Builder, primitives: &[FixedAabb3]) -> Tree {
    let count = primitives.len();
    let mut nodes = vec![empty_node(); (2 * count).saturating_sub(1)];
    let mut prim_indices = vec![u32::MAX; count];
    let used =
        builder(&mut nodes, &mut prim_indices, primitives).expect("buffers of 2n - 1 nodes fit");
    nodes.truncate(used);
    Tree { nodes, prim_indices }
}

/// Every field of every node and every primitive index, as raw integers in order.
pub(crate) fn tree_words(tree: &Tree) -> Vec<i64> {
    let mut words = Vec::new();
    for node in &tree.nodes {
        let b = node.bounds;
        for value in [b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z] {
            words.push(value.raw());
        }
        words.push(i64::from(node.first_child_or_prim_idx));
        words.push(i64::from(node.prim_count));
    }
    words.extend(tree.prim_indices.iter().map(|&index| i64::from(index)));
    words
}

/// FNV-1a over the little-endian bytes of [`tree_words`].
pub(crate) fn fingerprint(tree: &Tree) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for word in tree_words(tree) {
        for byte in word.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
        }
    }
    hash
}

pub(crate) fn union(a: FixedAabb3, b: FixedAabb3) -> FixedAabb3 {
    bunny_broadphase::utils::union_aabb(a, b)
}

/// What the structural check saw.
#[derive(Debug, Default)]
pub(crate) struct Shape {
    pub(crate) leaves: usize,
    pub(crate) max_depth: usize,
    pub(crate) largest_leaf: usize,
}

/// Checks the tree's structure and returns its shape. Panics, naming the node, when:
/// a node is unreachable or reached twice; an interior node's children are out of range;
/// a leaf's range is out of the index buffer; a primitive is missing or appears twice; a
/// node's bounds are not exactly the union of its primitives' boxes (so they contain every
/// child and primitive); the tree is deeper than `SAH_MAX_DEPTH`; or, with `ascending`, a
/// leaf's indices are not ascending.
pub(crate) fn check_structure(tree: &Tree, primitives: &[FixedAabb3], ascending: bool) -> Shape {
    let count = primitives.len();
    assert_eq!(tree.prim_indices.len(), count);
    if count == 0 {
        assert!(tree.nodes.is_empty());
        return Shape::default();
    }
    assert!(tree.nodes.len() < 2 * count, "node count {} above 2n - 1", tree.nodes.len());
    let mut walk = Walk {
        tree,
        primitives,
        ascending,
        reached: vec![false; tree.nodes.len()],
        seen: vec![false; count],
        shape: Shape::default(),
    };
    let root = walk.subtree(0, 0);
    assert_eq!(root, tree.nodes[0].bounds);
    assert!(walk.reached.iter().all(|&r| r), "every node is reachable");
    assert!(walk.seen.iter().all(|&s| s), "every primitive appears in a leaf");
    walk.shape
}

struct Walk<'a> {
    tree: &'a Tree,
    primitives: &'a [FixedAabb3],
    ascending: bool,
    reached: Vec<bool>,
    seen: Vec<bool>,
    shape: Shape,
}

impl Walk<'_> {
    fn subtree(&mut self, index: usize, depth: usize) -> FixedAabb3 {
        let tree = self.tree;
        assert!(index < tree.nodes.len(), "node {index} out of range");
        assert!(!self.reached[index], "node {index} reached twice");
        assert!(depth <= SAH_MAX_DEPTH, "node {index} at depth {depth}");
        self.reached[index] = true;
        self.shape.max_depth = self.shape.max_depth.max(depth);
        let node = tree.nodes[index];
        let first = node.first_child_or_prim_idx as usize;
        let bounds = if node.prim_count == 0 {
            assert!(first + 1 < tree.nodes.len(), "node {index}: children {first} out of range");
            let left = self.subtree(first, depth + 1);
            let right = self.subtree(first + 1, depth + 1);
            union(left, right)
        } else {
            self.leaf(index, first, node.prim_count as usize)
        };
        assert_eq!(node.bounds, bounds, "node {index}: bounds are not the union of its primitives");
        bounds
    }

    fn leaf(&mut self, index: usize, first: usize, count: usize) -> FixedAabb3 {
        let end = first + count;
        assert!(end <= self.tree.prim_indices.len(), "node {index}: leaf range out of range");
        let slots = &self.tree.prim_indices[first..end];
        if self.ascending {
            assert!(
                slots.windows(2).all(|pair| pair[0] < pair[1]),
                "node {index}: leaf not ascending"
            );
        }
        self.shape.leaves += 1;
        self.shape.largest_leaf = self.shape.largest_leaf.max(count);
        let mut bounds = self.primitives[slots[0] as usize];
        for &slot in slots {
            assert!(!self.seen[slot as usize], "primitive {slot} appears twice");
            self.seen[slot as usize] = true;
            bounds = union(bounds, self.primitives[slot as usize]);
        }
        bounds
    }
}

/// Exact half surface area in raw units squared (test extents stay below 2^45).
pub(crate) fn half_area(bounds: &FixedAabb3) -> u128 {
    let extent = |min: FixedQ32_32, max: FixedQ32_32| u128::from(max.raw().abs_diff(min.raw()));
    let x = extent(bounds.min.x, bounds.max.x);
    let y = extent(bounds.min.y, bounds.max.y);
    let z = extent(bounds.min.z, bounds.max.z);
    x * y + y * z + z * x
}

/// The tree's SAH cost times the root's area: the traversal weight times the area of every
/// interior node plus the intersection weight times area times count of every leaf. Trees
/// over the same primitives share the root, so these values compare directly.
pub(crate) fn sah_cost(tree: &Tree) -> u128 {
    tree.nodes
        .iter()
        .map(|node| {
            let area = half_area(&node.bounds);
            if node.prim_count == 0 {
                SAH_TRAVERSAL_COST * area
            } else {
                SAH_INTERSECTION_COST * area * u128::from(node.prim_count)
            }
        })
        .sum()
}
