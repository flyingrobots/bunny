//! Shared pieces of the refit tests: seeded box corpora and motions, and the independent
//! validator. The validator shares no code with `refit_bvh`: it walks the tree from the root
//! by recursion and recomputes every node's box with raw `i64` minimum and maximum written
//! here, never with `bunny_broadphase::utils::union_aabb`.

#![allow(dead_code)]

use bunny_broadphase::{refit_bvh, RefitError};
use bunny_geom::FixedAabb3;

use crate::common::{aabb, Seeds, Tree, ONE};

/// The six raw coordinates of a box: the minimum corner, then the maximum corner.
pub(crate) fn raw_box(bounds: &FixedAabb3) -> [i64; 6] {
    let (min, max) = (bounds.min, bounds.max);
    [min.x.raw(), min.y.raw(), min.z.raw(), max.x.raw(), max.y.raw(), max.z.raw()]
}

/// The exact union of two raw boxes: the smaller minimum and the larger maximum per axis.
pub(crate) fn raw_union(a: [i64; 6], b: [i64; 6]) -> [i64; 6] {
    let mut out = a;
    for axis in 0..3 {
        out[axis] = a[axis].min(b[axis]);
        out[axis + 3] = a[axis + 3].max(b[axis + 3]);
    }
    out
}

/// Asserts the refit post-condition: walking from the root, every node's stored box equals,
/// exactly, the union of its two children's recomputed boxes (interior) or of the boxes in
/// `boxes` of the primitives it lists (leaf), and every node of the slice is reached once.
pub(crate) fn assert_refitted(tree: &Tree, boxes: &[FixedAabb3], context: &str) {
    if tree.nodes.is_empty() {
        assert!(boxes.is_empty(), "{context}: an empty tree has no primitives");
        return;
    }
    let mut walk = Walk { tree, boxes, reached: vec![false; tree.nodes.len()], context };
    walk.recompute(0);
    assert!(walk.reached.iter().all(|&seen| seen), "{context}: a node is unreachable");
}

struct Walk<'a> {
    tree: &'a Tree,
    boxes: &'a [FixedAabb3],
    reached: Vec<bool>,
    context: &'a str,
}

impl Walk<'_> {
    fn recompute(&mut self, index: usize) -> [i64; 6] {
        let context = self.context;
        assert!(!self.reached[index], "{context}: node {index} reached twice");
        self.reached[index] = true;
        let node = self.tree.nodes[index];
        let first = node.first_child_or_prim_idx as usize;
        let expected = if node.prim_count == 0 {
            let left = self.recompute(first);
            let right = self.recompute(first + 1);
            raw_union(left, right)
        } else {
            let slots = &self.tree.prim_indices[first..first + node.prim_count as usize];
            let mut raw = slots.iter().map(|&primitive| raw_box(&self.boxes[primitive as usize]));
            let head = raw.next().expect("a leaf lists a primitive");
            raw.fold(head, raw_union)
        };
        assert_eq!(
            raw_box(&node.bounds),
            expected,
            "{context}: node {index} is not the exact union"
        );
        expected
    }
}

/// Every node as `(first_child_or_prim_idx, prim_count)`: the topology refit must keep.
pub(crate) fn layout(tree: &Tree) -> Vec<(u32, u32)> {
    tree.nodes.iter().map(|node| (node.first_child_or_prim_idx, node.prim_count)).collect()
}

/// A copy of `tree` refitted to `boxes`.
pub(crate) fn refitted(tree: &Tree, boxes: &[FixedAabb3]) -> Result<Tree, RefitError> {
    let mut out = tree.clone();
    refit_bvh(&mut out.nodes, &out.prim_indices, boxes)?;
    Ok(out)
}

/// Small boxes scattered through a 12-unit cube, one in twenty spanning most of it.
pub(crate) fn scattered(seeds: &mut Seeds, count: usize) -> Vec<FixedAabb3> {
    (0..count)
        .map(|index| {
            let reach = if index % 20 == 0 { 6 * ONE } else { ONE / 2 };
            let corner = seeds.point(6 * ONE);
            let size = [seeds.next(), seeds.next(), seeds.next()]
                .map(|bits| i64::try_from(bits % 1_000).expect("small") * reach / 1_000);
            let min = [corner.x.raw(), corner.y.raw(), corner.z.raw()];
            aabb(min, [min[0] + size[0], min[1] + size[1], min[2] + size[2]])
        })
        .collect()
}

/// `count` identical unit boxes (every centroid equal).
pub(crate) fn identical(count: usize) -> Vec<FixedAabb3> {
    vec![aabb([0; 3], [ONE; 3]); count]
}

/// `count` point boxes on the x axis, one unit apart.
pub(crate) fn collinear(count: usize) -> Vec<FixedAabb3> {
    (0..count)
        .map(|index| {
            let x = i64::try_from(index).expect("small") * ONE;
            aabb([x, 0, 0], [x, 0, 0])
        })
        .collect()
}

/// Scattered boxes with every third one inverted on some axes (minimum above maximum), and
/// two boxes at the ends of the Q32.32 range: inputs both builders accept.
pub(crate) fn inverted_and_extreme(seeds: &mut Seeds, count: usize) -> Vec<FixedAabb3> {
    let mut boxes = scattered(seeds, count);
    for (index, bounds) in boxes.iter_mut().enumerate().filter(|(index, _)| index % 3 == 0) {
        let raw = raw_box(bounds);
        let axis = index % 4;
        let mut min = [raw[0], raw[1], raw[2]];
        let mut max = [raw[3], raw[4], raw[5]];
        for flip in 0..3 {
            if axis == 3 || axis == flip {
                std::mem::swap(&mut min[flip], &mut max[flip]);
                max[flip] -= 1;
            }
        }
        *bounds = aabb(min, max);
    }
    boxes.push(aabb([i64::MIN; 3], [i64::MAX; 3]));
    boxes.push(aabb([i64::MAX; 3], [i64::MAX; 3]));
    boxes
}

/// The named box corpora of the structural tests (`seed` fixes the seeded ones).
pub(crate) fn corpora(seed: u64) -> Vec<(&'static str, Vec<FixedAabb3>)> {
    let mut seeds = Seeds(seed);
    vec![
        ("one box", scattered(&mut seeds, 1)),
        ("two boxes", scattered(&mut seeds, 2)),
        ("scattered 37", scattered(&mut seeds, 37)),
        ("scattered 600", scattered(&mut seeds, 600)),
        ("identical 200", identical(200)),
        ("collinear 200", collinear(200)),
        ("inverted and extreme 90", inverted_and_extreme(&mut seeds, 90)),
    ]
}

/// The named motions: each maps a corpus to a new box per primitive, same count.
pub(crate) fn motions(
    seeds: &mut Seeds,
    boxes: &[FixedAabb3],
) -> Vec<(&'static str, Vec<FixedAabb3>)> {
    let jitter: Vec<FixedAabb3> = boxes
        .iter()
        .map(|bounds| {
            let raw = raw_box(bounds);
            let step = [seeds.signed(ONE), seeds.signed(ONE), seeds.signed(ONE)];
            let shift = |value: i64, axis: usize| value.saturating_add(step[axis]);
            aabb(
                [shift(raw[0], 0), shift(raw[1], 1), shift(raw[2], 2)],
                [shift(raw[3], 0), shift(raw[4], 1), shift(raw[5], 2)],
            )
        })
        .collect();
    let swapped: Vec<FixedAabb3> = boxes.iter().rev().copied().collect();
    let point = aabb([ONE / 3; 3], [ONE / 3; 3]);
    let collapsed = vec![point; boxes.len()];
    let spread: Vec<FixedAabb3> = boxes
        .iter()
        .map(|bounds| {
            let raw = raw_box(bounds).map(|value| value.saturating_mul(8));
            aabb([raw[0], raw[1], raw[2]], [raw[3], raw[4], raw[5]])
        })
        .collect();
    let inverted: Vec<FixedAabb3> = boxes
        .iter()
        .map(|bounds| {
            let raw = raw_box(bounds);
            aabb([raw[3], raw[4], raw[5]], [raw[0], raw[1], raw[2]])
        })
        .collect();
    vec![
        ("jitter", jitter),
        ("swap places", swapped),
        ("collapse to a point", collapsed),
        ("spread out", spread),
        ("invert", inverted),
    ]
}
