//! `build_bvh_sah` allocates nothing.

#![cfg(not(target_arch = "wasm32"))]

use bunny_broadphase::{build_bvh_sah, BvhNode};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

fn unit_box(x: i64, y: i64) -> FixedAabb3 {
    let one = bunny_num::fixed_q32_32::ONE_RAW;
    let corner = |x: i64, y: i64, z: i64| {
        FixedVec3::new(
            FixedQ32_32::from_raw(x * one),
            FixedQ32_32::from_raw(y * one),
            FixedQ32_32::from_raw(z * one),
        )
    };
    FixedAabb3::new(corner(x, y, 0), corner(x + 1, y + 1, 1))
}

#[test]
fn bvh_tp_021_build_bvh_sah_allocates_zero_times() {
    // Distinct boxes, identical boxes (median splits) and enough of them to sort.
    let mut primitives: Vec<FixedAabb3> =
        (0..200).map(|i| unit_box((i * 37) % 101, i % 13)).collect();
    primitives.extend(std::iter::repeat_n(unit_box(5, 5), 40));
    let empty = BvhNode { bounds: unit_box(0, 0), first_child_or_prim_idx: 0, prim_count: 0 };
    let mut nodes = vec![empty; 2 * primitives.len() - 1];
    let mut prim_indices = vec![0_u32; primitives.len()];

    // Only this thread's allocations are counted, so the harness cannot fail the assertion.
    let mut used = None;
    let counts = allocation_counter::measure(|| {
        used = build_bvh_sah(&mut nodes, &mut prim_indices, &primitives);
    });

    assert_eq!(counts.count_total, 0);
    assert!(used.is_some_and(|used| used > 1));
}
