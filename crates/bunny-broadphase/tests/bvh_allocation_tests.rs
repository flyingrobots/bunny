//! Integration tests.

#![cfg(not(target_arch = "wasm32"))]

use bunny_broadphase::{build_bvh, intersect_aabb, intersect_ray, BvhNode};
use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

/// Runs `operation` and returns its result with the number of heap blocks the calling thread
/// allocated meanwhile. Allocations by other threads, including the test harness, are not counted.
fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let mut result = None;
    let counts = allocation_counter::measure(|| result = Some(operation()));
    (result.expect("the measured operation should have run"), counts.count_total)
}

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * bunny_num::fixed_q32_32::ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn aabb(min_x: i32, max_x: i32) -> FixedAabb3 {
    FixedAabb3::new(vec3(min_x, -1, -1), vec3(max_x, 1, 1))
}

fn fixture_primitives() -> [FixedAabb3; 4] {
    [aabb(0, 1), aabb(3, 4), aabb(6, 7), aabb(9, 10)]
}

fn empty_node() -> BvhNode {
    BvhNode {
        bounds: FixedAabb3::new(vec3(0, 0, 0), vec3(0, 0, 0)),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }
}

fn build_fixture_bvh() -> (usize, [BvhNode; 7], [u32; 4]) {
    let primitives = fixture_primitives();
    let mut nodes = [empty_node(); 7];
    let mut primitive_indices = [0_u32; 4];
    let node_count =
        build_bvh(&mut nodes, &mut primitive_indices, &primitives).expect("build should succeed");

    (node_count, nodes, primitive_indices)
}

fn warm_up_bvh_paths() {
    let (node_count, nodes, primitive_indices) = build_fixture_bvh();
    let query_box = FixedAabb3::new(vec3(-1, -2, -2), vec3(11, 2, 2));
    let mut overlap_count = 0;
    intersect_aabb(&nodes[..node_count], &primitive_indices, &query_box, |_| {
        overlap_count += 1;
    })
    .expect("AABB traversal should succeed");
    assert_eq!(overlap_count, 4);

    let ray = FixedRay3::new(vec3(-1, 0, 0), FixedUnitVec3::UNIT_X);
    let mut ray_count = 0;
    intersect_ray(&nodes[..node_count], &primitive_indices, &ray, |_| {
        ray_count += 1;
    })
    .expect("ray traversal should succeed");
    assert_eq!(ray_count, 4);
}

#[test]
fn build_bvh_and_traversal_allocate_zero_times_after_warm_up() {
    warm_up_bvh_paths();

    let primitives = fixture_primitives();
    let mut nodes = [empty_node(); 7];
    let mut primitive_indices = [0_u32; 4];

    let (node_count, build_blocks) =
        allocations_during(|| build_bvh(&mut nodes, &mut primitive_indices, &primitives));

    assert_eq!(build_blocks, 0);
    let node_count = node_count.expect("build should succeed");
    assert!(node_count > 0);

    let query_box = FixedAabb3::new(vec3(-1, -2, -2), vec3(11, 2, 2));
    let mut overlap_hits = [u32::MAX; 4];
    let ((overlap_result, overlap_count), overlap_blocks) = allocations_during(|| {
        let mut count = 0;
        let result = intersect_aabb(&nodes[..node_count], &primitive_indices, &query_box, |idx| {
            overlap_hits[count] = idx;
            count += 1;
        });
        (result, count)
    });
    assert_eq!(overlap_blocks, 0);
    assert_eq!(overlap_result, Ok(()));
    assert_eq!(overlap_count, 4);

    let ray = FixedRay3::new(vec3(-1, 0, 0), FixedUnitVec3::UNIT_X);
    let mut ray_hits = [u32::MAX; 4];
    let ((ray_result, ray_count), ray_blocks) = allocations_during(|| {
        let mut count = 0;
        let result = intersect_ray(&nodes[..node_count], &primitive_indices, &ray, |idx| {
            ray_hits[count] = idx;
            count += 1;
        });
        (result, count)
    });
    assert_eq!(ray_blocks, 0);
    assert_eq!(ray_result, Ok(()));
    assert_eq!(ray_count, 4);
}
