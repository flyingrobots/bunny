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
    });

    assert_eq!(overlapped.len(), 1);
    assert_eq!(overlapped[0], 1); // Should find Box 1 at center (5,0,0)

    // 3. Query Ray intersection: ray along X axis from (-2, 0, 0)
    let ray = FixedRay3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(-2.0),
            FixedQ32_32::ZERO,
            FixedQ32_32::ZERO,
        ),
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO), // Direction (1,0,0)
    );

    let mut hit_indices = Vec::new();
    intersect_ray(&nodes[..node_count], &prim_indices, &ray, |idx| {
        hit_indices.push(idx);
    });

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
            FixedVec3::new(
                FixedQ32_32::from_f32(2.0),
                FixedQ32_32::ONE,
                FixedQ32_32::ONE,
            ),
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
