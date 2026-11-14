//! Unit tests of the SAH builder's internals: the depth cap and the cost bound.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use super::build_capped;
use super::cost::{leaf_cost, split_cost, Scale};
use crate::bvh::{BvhNode, SAH_MAX_LEAF_SIZE};

fn raw_box(min: i64, max: i64) -> FixedAabb3 {
    let corner =
        |raw| FixedVec3::new(FixedQ32_32::from_raw(raw), FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let mut bounds = FixedAabb3::new(corner(min), corner(max));
    bounds.max.y = FixedQ32_32::ONE;
    bounds.max.z = FixedQ32_32::ONE;
    bounds
}

fn depth_of_leaves(nodes: &[BvhNode], index: usize, depth: usize, out: &mut Vec<(usize, u32)>) {
    let node = nodes[index];
    if node.prim_count > 0 {
        out.push((depth, node.prim_count));
        return;
    }
    let child = node.first_child_or_prim_idx as usize;
    depth_of_leaves(nodes, child, depth + 1, out);
    depth_of_leaves(nodes, child + 1, depth + 1, out);
}

#[test]
fn bvh_tp_016_nodes_at_the_depth_cap_become_leaves_whatever_their_size() {
    let primitives: Vec<FixedAabb3> =
        (0..100).map(|i| raw_box(i << 32, (i << 32) + (1 << 30))).collect();
    let mut nodes =
        vec![BvhNode { bounds: primitives[0], first_child_or_prim_idx: 0, prim_count: 0 }; 199];
    let mut prim_indices = vec![0_u32; 100];
    let used = build_capped(&mut nodes, &mut prim_indices, &primitives, 3).expect("buffers fit");
    let mut leaves = Vec::new();
    depth_of_leaves(&nodes[..used], 0, 0, &mut leaves);
    assert!(leaves.iter().all(|&(depth, _)| depth <= 3), "{leaves:?}");
    assert!(leaves.iter().any(|&(depth, count)| depth == 3 && count as usize > SAH_MAX_LEAF_SIZE));
    assert_eq!(leaves.iter().map(|&(_, count)| count).sum::<u32>(), 100);
    // Without the cap, the same input has no leaf above the leaf size.
    let used =
        build_capped(&mut nodes, &mut prim_indices, &primitives, usize::MAX).expect("buffers fit");
    let mut leaves = Vec::new();
    depth_of_leaves(&nodes[..used], 0, 0, &mut leaves);
    assert!(leaves.iter().all(|&(_, count)| count as usize <= SAH_MAX_LEAF_SIZE));
}

#[test]
fn bvh_tp_017_costs_of_the_widest_boxes_and_largest_counts_do_not_overflow() {
    let full = FixedQ32_32::from_raw(i64::MAX);
    let empty = FixedQ32_32::from_raw(i64::MIN);
    let widest =
        FixedAabb3::new(FixedVec3::new(empty, empty, empty), FixedVec3::new(full, full, full));
    let scale = Scale::of(&widest);
    let count = usize::try_from(u32::MAX).expect("usize holds u32");
    let area = scale.area(&widest).expect("area fits");
    let weighted = scale.weighted(&widest, count).expect("weighted area fits");
    assert_eq!(area, 3 * ((1_u128 << 45) - 1).pow(2));
    assert!(split_cost(area, weighted, weighted).is_some());
    assert!(leaf_cost(weighted).is_some());
    // Below 2^45 raw units no shift applies, so areas are exact.
    let exact = raw_box(0, (1 << 45) - 1);
    assert_eq!(
        Scale::of(&exact).area(&exact),
        Some(((1_u128 << 45) - 1) * (1 << 32) * 2 + (1 << 64))
    );
}
