//! Integration tests.

use bunny_broadphase::{build_bvh, sweep_and_prune, BvhNode};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn x_aabb(min_x: i32, max_x: i32) -> FixedAabb3 {
    FixedAabb3::new(vec3(min_x, 0, 0), vec3(max_x, 1, 1))
}

fn empty_node() -> BvhNode {
    BvhNode { bounds: x_aabb(0, 0), first_child_or_prim_idx: 0, prim_count: 0 }
}

#[wasm_bindgen_test(unsupported = test)]
fn sweep_pair_order_golden_vector() {
    let primitives = [x_aabb(0, 2), x_aabb(1, 3), x_aabb(4, 5), x_aabb(4, 6)];
    let mut primitive_indices = [0_u32; 4];
    let mut pairs = [(0_u32, 0_u32); 4];

    let count = sweep_and_prune(&mut pairs, &mut primitive_indices, &primitives)
        .expect("sweep should fit pair buffer");

    assert_eq!(count, 2);
    assert_eq!(&pairs[..count], &[(0, 1), (2, 3)]);
    assert_eq!(primitive_indices, [0, 1, 2, 3]);
}

#[wasm_bindgen_test(unsupported = test)]
fn single_leaf_bvh_golden_vector() {
    let primitives = [x_aabb(-2, 2)];
    let mut primitive_indices = [u32::MAX; 1];
    let mut nodes = [empty_node(); 1];

    let node_count =
        build_bvh(&mut nodes, &mut primitive_indices, &primitives).expect("leaf build succeeds");

    assert_eq!(node_count, 1);
    assert_eq!(primitive_indices, [0]);
    assert_eq!(nodes[0].bounds, primitives[0]);
    assert_eq!(nodes[0].first_child_or_prim_idx, 0);
    assert_eq!(nodes[0].prim_count, 1);
}
