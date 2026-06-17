//! Integration tests.

use bunny_broadphase::{build_bvh, intersect_aabb, intersect_ray, sweep_and_prune, BvhNode};
use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn test_bvh_build_and_traverse() {
    // 1. Create some primitives (AABBs) scattered in space
    let prims = [
        // Box 0: center (0,0,0)
        FixedAabb3::new(
            FixedVec3::new(
                FixedQ32_32::from_f32(-0.5),
                FixedQ32_32::from_f32(-0.5),
                FixedQ32_32::from_f32(-0.5),
            ),
            FixedVec3::new(
                FixedQ32_32::from_f32(0.5),
                FixedQ32_32::from_f32(0.5),
                FixedQ32_32::from_f32(0.5),
            ),
        ),
        // Box 1: center (5,0,0)
        FixedAabb3::new(
            FixedVec3::new(
                FixedQ32_32::from_f32(4.5),
                FixedQ32_32::from_f32(-0.5),
                FixedQ32_32::from_f32(-0.5),
            ),
            FixedVec3::new(
                FixedQ32_32::from_f32(5.5),
                FixedQ32_32::from_f32(0.5),
                FixedQ32_32::from_f32(0.5),
            ),
        ),
        // Box 2: center (10,0,0)
        FixedAabb3::new(
            FixedVec3::new(
                FixedQ32_32::from_f32(9.5),
                FixedQ32_32::from_f32(-0.5),
                FixedQ32_32::from_f32(-0.5),
            ),
            FixedVec3::new(
                FixedQ32_32::from_f32(10.5),
                FixedQ32_32::from_f32(0.5),
                FixedQ32_32::from_f32(0.5),
            ),
        ),
    ];

    // Allocate buffers
    let mut nodes = [BvhNode {
        bounds: FixedAabb3::new(
            FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
            FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        ),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }; 10];
    let mut prim_indices = [0_u32; 3];

    // Build tree
    let node_count =
        build_bvh(&mut nodes, &mut prim_indices, &prims).expect("build should succeed");
    assert!(node_count > 0);

    // 2. Query AABB intersection: overlap query around (5,0,0)
    let query_box = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(4.0),
            FixedQ32_32::from_f32(-1.0),
            FixedQ32_32::from_f32(-1.0),
        ),
        FixedVec3::new(
            FixedQ32_32::from_f32(6.0),
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(1.0),
        ),
    );

    let mut overlapped = Vec::new();
    intersect_aabb(&nodes[..node_count], &prim_indices, &query_box, |idx| {
        overlapped.push(idx);
    })
    .expect("AABB intersection traversal should not overflow stack");

    assert_eq!(overlapped.len(), 1);
    assert_eq!(overlapped[0], 1); // Should find Box 1 at center (5,0,0)

    // 3. Query Ray intersection: ray along X axis from (-2, 0, 0)
    let ray = FixedRay3::try_new(
        FixedVec3::new(FixedQ32_32::from_f32(-2.0), FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    )
    .unwrap();

    let mut hit_indices = Vec::new();
    intersect_ray(&nodes[..node_count], &prim_indices, &ray, |idx| {
        hit_indices.push(idx);
    })
    .expect("Ray intersection traversal should not overflow stack");

    // Should intersect all 3 boxes along the X axis
    assert_eq!(hit_indices.len(), 3);
    assert!(hit_indices.contains(&0));
    assert!(hit_indices.contains(&1));
    assert!(hit_indices.contains(&2));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_sweep_and_prune_solver() {
    let prims = [
        // Box 0
        FixedAabb3::new(
            FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
            FixedVec3::new(
                FixedQ32_32::from_f32(2.0),
                FixedQ32_32::from_f32(2.0),
                FixedQ32_32::from_f32(2.0),
            ),
        ),
        // Box 1
        FixedAabb3::new(
            FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ONE, FixedQ32_32::ONE),
            FixedVec3::new(
                FixedQ32_32::from_f32(3.0),
                FixedQ32_32::from_f32(3.0),
                FixedQ32_32::from_f32(3.0),
            ),
        ),
        // Box 2
        FixedAabb3::new(
            FixedVec3::new(
                FixedQ32_32::from_f32(5.0),
                FixedQ32_32::from_f32(5.0),
                FixedQ32_32::from_f32(5.0),
            ),
            FixedVec3::new(
                FixedQ32_32::from_f32(7.0),
                FixedQ32_32::from_f32(7.0),
                FixedQ32_32::from_f32(7.0),
            ),
        ),
        // Box 3
        FixedAabb3::new(
            FixedVec3::new(
                FixedQ32_32::from_f32(6.0),
                FixedQ32_32::from_f32(6.0),
                FixedQ32_32::from_f32(6.0),
            ),
            FixedVec3::new(
                FixedQ32_32::from_f32(8.0),
                FixedQ32_32::from_f32(8.0),
                FixedQ32_32::from_f32(8.0),
            ),
        ),
        // Box 4
        FixedAabb3::new(
            FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
            FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::ONE, FixedQ32_32::ONE),
        ),
    ];

    let mut pairs = [(0, 0); 10];
    let mut prim_indices = [0; 5];

    let pair_count = sweep_and_prune(&mut pairs, &mut prim_indices, &prims)
        .expect("sweep and prune should succeed");

    assert_eq!(pair_count, 4);

    // Pairs must be sorted lexicographically and in a < b order for each pair
    assert_eq!(pairs[0], (0, 1));
    assert_eq!(pairs[1], (0, 4));
    assert_eq!(pairs[2], (1, 4));
    assert_eq!(pairs[3], (2, 3));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_traversal_stack_overflow() {
    use bunny_broadphase::traversal::{intersect_aabb, TraversalError};
    use bunny_broadphase::BvhNode;
    use bunny_geom::FixedAabb3;
    use bunny_linalg::FixedVec3;

    let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);

    // Create a pathological chain of nodes to trigger stack overflow
    // For even indices i < 160, it's a non-leaf pointing to i+1 and i+2.
    // For odd indices or i >= 160, it's a leaf.
    let mut nodes = Vec::new();
    for i in 0..200 {
        if i % 2 == 0 && i < 160 {
            nodes.push(BvhNode {
                bounds: FixedAabb3::new(zero, zero),
                first_child_or_prim_idx: u32::try_from(i + 1).expect("test index fits u32"),
                prim_count: 0,
            });
        } else {
            nodes.push(BvhNode {
                bounds: FixedAabb3::new(zero, zero),
                first_child_or_prim_idx: 0,
                prim_count: 1, // leaf
            });
        }
    }

    let prim_indices = vec![0; 250];
    let query_box = FixedAabb3::new(zero, zero);

    let res = intersect_aabb(&nodes, &prim_indices, &query_box, |_idx| {});
    assert_eq!(res, Err(TraversalError::StackOverflow));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_traversal_rejects_malformed_bvh_without_panic() {
    use bunny_broadphase::traversal::{intersect_aabb, TraversalError};
    use bunny_broadphase::BvhNode;
    use bunny_geom::{FixedAabb3, FixedRay3};
    use bunny_linalg::FixedVec3;

    let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let bounds = FixedAabb3::new(zero, zero);
    let malformed_nodes = [BvhNode { bounds, first_child_or_prim_idx: 10, prim_count: 0 }];
    let prim_indices = [0_u32];
    let query_box = FixedAabb3::new(zero, zero);

    let aabb_result = std::panic::catch_unwind(|| {
        intersect_aabb(&malformed_nodes, &prim_indices, &query_box, |_idx| {})
    });
    assert_eq!(
        aabb_result.expect("malformed child index should not panic"),
        Err(TraversalError::InvalidNodeIndex)
    );

    let ray = FixedRay3::try_new(
        FixedVec3::new(FixedQ32_32::from_f32(-1.0), FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    )
    .unwrap();
    let ray_result = std::panic::catch_unwind(|| {
        intersect_ray(&malformed_nodes, &prim_indices, &ray, |_idx| {})
    });
    assert_eq!(
        ray_result.expect("malformed child index should not panic"),
        Err(TraversalError::InvalidNodeIndex)
    );

    let malformed_leaf_nodes = [BvhNode { bounds, first_child_or_prim_idx: 1, prim_count: 2 }];
    let leaf_result = std::panic::catch_unwind(|| {
        intersect_aabb(&malformed_leaf_nodes, &prim_indices, &query_box, |_idx| {})
    });
    assert_eq!(
        leaf_result.expect("malformed primitive range should not panic"),
        Err(TraversalError::InvalidPrimitiveRange)
    );
}
