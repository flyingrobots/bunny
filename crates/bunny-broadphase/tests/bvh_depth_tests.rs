//! How deep each builder's trees get, against the depth the traversals hold: `build_bvh_sah`
//! reaches its cap of 60 inside the trace domain (BVH-REQ-010), and `build_bvh`, which has no
//! cap, goes past 63 inside it (BVH-REQ-011).
//!
//! Both inputs are collinear point boxes at offsets `d_k` below a top coordinate, with
//! `d_0 = 0` and `d_(k+1) = ceil(r (d_k + 1))`: each offset is far enough past the previous one
//! that the split rule leaves only the farthest primitive on one side. Every half area is zero,
//! so every cost ties and the lowest plane wins. The depths follow from that rule by hand.

mod common;

use bunny_broadphase::{
    any_hit, build_bvh, build_bvh_sah, closest_hit, compact_any_hit, compact_bvh,
    compact_closest_hit, intersect_aabb, BvhView, ClosestHit, CompactNode, CompactView,
    TraversalError,
};
use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use common::{aabb, build_with, check_sah_structure, check_structure, vec3, Tree, ONE};
use wasm_bindgen_test::wasm_bindgen_test;

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const TOP: i64 = TRACE_COORD_LIMIT_RAW - 1;

/// `count` offsets with `d_0 = 0` and `d_(k+1) = ceil(num (d_k + 1) / den)`.
fn offsets(count: usize, num: i64, den: i64) -> Vec<i64> {
    let mut out = vec![0];
    while out.len() < count {
        let last = out[out.len() - 1];
        out.push(((last + 1) * num + den - 1) / den);
    }
    out
}

/// A point box at `x = top - d` on the x axis for each offset.
fn points(top: i64, offsets: &[i64]) -> Vec<FixedAabb3> {
    offsets.iter().map(|d| aabb([top - d, 0, 0], [top - d, 0, 0])).collect()
}

/// The depth of every leaf, with its primitives, in depth-first, left-first order.
fn leaves(tree: &Tree) -> Vec<(usize, Vec<u32>)> {
    let mut out = Vec::new();
    let mut pending = vec![(0_usize, 0_usize)];
    while let Some((index, depth)) = pending.pop() {
        let node = tree.nodes[index];
        let first = node.first_child_or_prim_idx as usize;
        if node.prim_count == 0 {
            pending.extend([(first + 1, depth + 1), (first, depth + 1)]);
        } else {
            out.push((depth, tree.prim_indices[first..][..node.prim_count as usize].to_vec()));
        }
    }
    out
}

/// What `closest_hit` returns and what a test of every primitive returns, with box entry as
/// the primitive test.
fn nearest(
    tree: &Tree,
    boxes: &[FixedAabb3],
    ray: &TraceRay,
) -> [Result<Option<u32>, TraversalError>; 2] {
    let view = BvhView::new(&tree.nodes, &tree.prim_indices);
    let found = closest_hit(view, ray, FAR, |p, limit| ray.box_entry(&boxes[p as usize], limit));
    let every = (0..boxes.len())
        .filter_map(|i| {
            ray.box_entry(&boxes[i], FAR).map(|t| (t, u32::try_from(i).expect("small")))
        })
        .min();
    [found.map(|hit| hit.map(|ClosestHit { primitive, .. }| primitive)), Ok(every.map(|(_, p)| p))]
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_022_seventy_collinear_points_reach_the_depth_cap() {
    // With r = 16/15, 15 d_(k+1) >= 16 (d_k + 1), so the doubled gap 2 (d_(k+1) - d_k) times 16
    // is at least 2 d_(k+1) + 32 > 2 d_(k+1) + 1, the bin width: a node's lowest point is alone
    // in bin 0 and plane 1 splits it off. 70 points lose one per level; 10 remain at depth 60.
    let offsets = offsets(70, 16, 15);
    assert!(offsets[69] < ONE, "all within one unit of the top: inside the trace domain");
    let primitives = points(TOP, &offsets);
    let tree = build_with(build_bvh_sah, &primitives);
    let shape = check_sah_structure(&tree, &primitives);
    assert_eq!(shape.max_depth, 60, "the cap, written as a literal: 61 of the 64 entries");
    let deepest: Vec<_> = leaves(&tree).into_iter().filter(|(depth, _)| *depth == 60).collect();
    // The node at depth 59 splits off point 10; the capped leaf keeps points 0 to 9, more than
    // SAH_MAX_LEAF_SIZE (4), which only the cap allows.
    assert_eq!(deepest, vec![(60, vec![10]), (60, (0..10).collect())]);

    let from_below = TraceRay::new(vec3([TOP - ONE, 0, 0]), vec3([ONE, 0, 0])).expect("a ray");
    let [found, every] = nearest(&tree, &primitives, &from_below);
    assert_eq!(found, every, "closest_hit equals a test of every point");
    assert_eq!(every, Ok(Some(69)), "the lowest point is entered first");
    let view = BvhView::new(&tree.nodes, &tree.prim_indices);
    assert_eq!(any_hit(view, &from_below, FAR, |_| true), Ok(true));
    let mut reached = 0;
    let everything = aabb([-TOP; 3], [TOP; 3]);
    assert_eq!(
        intersect_aabb(&tree.nodes, &tree.prim_indices, &everything, |_| reached += 1),
        Ok(())
    );
    assert_eq!(reached, 70);
    let mut out =
        vec![CompactNode { bounds: [primitives[0]; 2], children: Default::default() }; 69];
    let (root, written) = compact_bvh(&tree.nodes, &mut out).expect("converts").expect("a root");
    let compact =
        CompactView { root: Some(root), nodes: &out[..written], prim_indices: &tree.prim_indices };
    let entry = |p: u32, limit| from_below.box_entry(&primitives[p as usize], limit);
    let compact_found = compact_closest_hit(compact, &from_below, FAR, entry);
    assert_eq!(compact_found.map(|hit| hit.map(|hit| hit.primitive)), every);
    assert_eq!(compact_any_hit(compact, &from_below, FAR, |_| true), Ok(true));
}

/// A triangle in the plane `x = x` whose bounds are one raw unit thick in y and z.
fn thin_triangle(x: i64) -> TraceTriangle {
    TraceTriangle::new(vec3([x, 0, 0]), vec3([x, 1, 0]), vec3([x, 0, 1])).expect("in the domain")
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_024_build_bvh_exceeds_the_traversal_depth_inside_the_trace_domain() {
    // With r = 4/3, 3 d_(k+1) >= 4 (d_k + 1): the gap below the farthest point is more than a
    // quarter of the span, so the first quartile plane (which wins every zero-cost tie) leaves
    // only that point on its left. Nodes above four points always split: 94 points, 90 levels.
    let top = TOP - ONE;
    let offsets = offsets(94, 4, 3);
    assert!(top - offsets[93] > -TRACE_COORD_LIMIT_RAW, "inside the trace domain");
    let primitives = points(top, &offsets);
    let tree = build_with(build_bvh, &primitives);
    assert_eq!(check_structure(&tree, &primitives, false).max_depth, 90);
    // The same planes, as triangles: their bounds have area, and the recorded tree is 70 deep.
    let triangles: Vec<FixedAabb3> =
        offsets.iter().map(|d| thin_triangle(top - d).bounds()).collect();
    let thin = build_with(build_bvh, &triangles);
    assert_eq!(check_structure(&thin, &triangles, false).max_depth, 70);

    let from_above = TraceRay::new(vec3([TOP, 0, 0]), vec3([-ONE, 0, 0])).expect("a ray");
    let from_below = TraceRay::new(vec3([-TOP, 0, 0]), vec3([ONE, 0, 0])).expect("a ray");
    let overflow = TraversalError::StackOverflow;
    for (tree, boxes) in [(&tree, &primitives), (&thin, &triangles)] {
        // Down the cluster's side every level keeps its split-off point pending: refused.
        assert_eq!(nearest(tree, boxes, &from_above)[0], Err(overflow));
        let view = BvhView::new(&tree.nodes, &tree.prim_indices);
        assert_eq!(any_hit(view, &from_above, FAR, |_| true), Err(overflow));
        let everything = aabb([-TOP; 3], [TOP; 3]);
        assert_eq!(
            intersect_aabb(&tree.nodes, &tree.prim_indices, &everything, |_| ()),
            Err(overflow)
        );
        // From the other end each split-off point is nearer and is searched at once.
        let [found, every] = nearest(tree, boxes, &from_below);
        assert_eq!((found, every), (Ok(Some(93)), Ok(Some(93))));
    }
}

/// The most left edges on the path to any interior node: the right children the SAH build
/// keeps waiting there (it pushes the right child under the left one).
fn left_turns(tree: &Tree) -> usize {
    let mut most = 0;
    let mut pending = vec![(0_usize, 0_usize)];
    while let Some((index, lefts)) = pending.pop() {
        let node = tree.nodes[index];
        if node.prim_count == 0 {
            most = most.max(lefts);
            let first = node.first_child_or_prim_idx as usize;
            pending.extend([(first, lefts + 1), (first + 1, lefts)]);
        }
    }
    most
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_026_a_left_spine_to_the_cap_fills_the_sah_build_stack() {
    // The review's chains, extended to the raw range: point boxes at 17^k raw (k = 0..15) on
    // each axis in turn, outside the trace domain, then 2^14 identical boxes at the origin.
    // Each chain point is split off to the right and each halving of the identical boxes keeps
    // the second half waiting, so 59 right children wait above the deepest interior node,
    // which pushes two more: all 61 entries of the build's work stack.
    let mut primitives = Vec::new();
    for axis in 0..3 {
        for k in 0..16 {
            let mut raw = [0_i64; 3];
            raw[axis] = 17_i64.pow(k);
            primitives.push(aabb(raw, raw));
        }
    }
    primitives.resize(primitives.len() + (1 << 14), aabb([0; 3], [0; 3]));
    let tree = build_with(build_bvh_sah, &primitives);
    let shape = check_sah_structure(&tree, &primitives);
    assert_eq!((shape.max_depth, left_turns(&tree)), (60, 59));
    let mut reached = 0;
    let everything = aabb([i64::MIN; 3], [i64::MAX; 3]);
    assert_eq!(
        intersect_aabb(&tree.nodes, &tree.prim_indices, &everything, |_| reached += 1),
        Ok(())
    );
    assert_eq!(reached, primitives.len());
}
