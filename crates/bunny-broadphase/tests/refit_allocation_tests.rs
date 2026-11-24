//! `refit_bvh` allocates nothing: it takes the caller's buffers and needs no scratch.

#![cfg(not(target_arch = "wasm32"))]

mod common;

use bunny_broadphase::{build_bvh, build_bvh_sah, refit_bvh};
use bunny_geom::FixedAabb3;
use common::{aabb, build_with, Builder, ONE};

fn unit_box(x: i64, y: i64) -> FixedAabb3 {
    aabb([x * ONE, y * ONE, 0], [(x + 1) * ONE, (y + 1) * ONE, ONE])
}

#[test]
fn bvh_tp_056_refit_allocates_zero_times() {
    // Distinct boxes and identical ones (a halving split), moved by a shift and reversed.
    let mut primitives: Vec<FixedAabb3> =
        (0..200).map(|i| unit_box((i * 37) % 101, i % 13)).collect();
    primitives.extend(std::iter::repeat_n(unit_box(5, 5), 40));
    let moved: Vec<FixedAabb3> = primitives
        .iter()
        .rev()
        .map(|b| aabb([b.min.x.raw() + 3, 0, 0], [b.max.x.raw(), 9, 9]))
        .collect();
    let builders: [Builder; 2] = [build_bvh, build_bvh_sah];
    for builder in builders {
        let mut tree = build_with(builder, &primitives);
        // Only this thread's allocations are counted, so the harness cannot fail the assertion.
        let mut result = None;
        let counts = allocation_counter::measure(|| {
            result = Some(refit_bvh(&mut tree.nodes, &tree.prim_indices, &moved));
        });
        assert_eq!(counts.count_total, 0);
        assert_eq!(result, Some(Ok(())));
        // A refusal allocates nothing either.
        let counts = allocation_counter::measure(|| {
            result = Some(refit_bvh(&mut tree.nodes, &tree.prim_indices, &moved[1..]));
        });
        assert_eq!(counts.count_total, 0);
        assert!(matches!(result, Some(Err(_))));
    }
}
