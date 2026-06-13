use bunny_broadphase::{build_bvh, BvhNode};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

fn zero_vec() -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO)
}

fn empty_aabb() -> FixedAabb3 {
    FixedAabb3::new(zero_vec(), zero_vec())
}

fn empty_node() -> BvhNode {
    BvhNode {
        bounds: empty_aabb(),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_builder_rejects_malformed_buffers_without_panic() {
    let primitives = [empty_aabb(), empty_aabb()];

    let mut too_few_nodes = [empty_node(); 2];
    let mut primitive_indices = [0_u32; 2];
    assert_eq!(
        build_bvh(&mut too_few_nodes, &mut primitive_indices, &primitives),
        None
    );

    let mut nodes = [empty_node(); 3];
    let mut too_few_primitive_indices = [0_u32; 1];
    assert_eq!(
        build_bvh(&mut nodes, &mut too_few_primitive_indices, &primitives),
        None
    );
}
