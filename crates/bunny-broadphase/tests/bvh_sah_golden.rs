//! Golden vectors for BVH construction determinism: fingerprints of the exact tree bytes
//! that `build_bvh_sah` and `build_bvh` produce for seeded inputs. These run on every CI
//! platform (Linux, macOS, Windows and WASM), so a target that builds a different tree fails.

mod common;

use bunny_broadphase::{build_bvh, build_bvh_sah};
use bunny_geom::FixedAabb3;
use common::{build_with, fingerprint, Seeds, ONE};
use wasm_bindgen_test::wasm_bindgen_test;

const GOLDEN_SEED: u64 = 0x601D_E40B_0000_0213;

/// 500 boxes: mostly small, one in twenty long, and every 25th repeated.
fn golden_boxes() -> Vec<FixedAabb3> {
    let mut seeds = Seeds(GOLDEN_SEED);
    let mut boxes: Vec<FixedAabb3> = (0..500)
        .map(|index| {
            let reach = if index % 20 == 0 { 8 * ONE } else { ONE / 3 };
            let corner = seeds.point(10 * ONE);
            let size = seeds.point(reach);
            let far =
                corner + common::vec3([size.x.raw().abs(), size.y.raw().abs(), size.z.raw().abs()]);
            FixedAabb3::new(corner, far)
        })
        .collect();
    let copies: Vec<FixedAabb3> = boxes.iter().step_by(25).copied().collect();
    boxes.extend(copies);
    boxes
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_018_sah_tree_bytes_golden_vector() {
    let tree = build_with(build_bvh_sah, &golden_boxes());
    assert_eq!((tree.nodes.len(), fingerprint(&tree)), (983, 8_164_951_778_464_349_701));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_019_quartile_tree_bytes_golden_vector() {
    let tree = build_with(build_bvh, &golden_boxes());
    assert_eq!((tree.nodes.len(), fingerprint(&tree)), (999, 15_928_625_504_236_988_134));
}
