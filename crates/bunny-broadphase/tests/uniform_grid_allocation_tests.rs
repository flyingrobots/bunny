//! `uniform_grid_pairs` allocates nothing.

#![cfg(not(target_arch = "wasm32"))]

use bunny_broadphase::uniform_grid::{grid_key_count, uniform_grid_pairs, GridCellSize, GridKey};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn corner(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

#[test]
fn grid_tp_014_uniform_grid_pairs_allocates_zero_times() {
    // 300 boxes in a 10-cell cube, some spanning several cells, some identical.
    let boxes: Vec<FixedAabb3> = (0_i64..300)
        .map(|i| {
            let (x, y, z) = ((i * 37) % 10, (i * 11) % 7, i % 5);
            let extent = 1 + i % 3;
            FixedAabb3::new(
                corner(x * ONE, y * ONE, z * ONE),
                corner((x + extent) * ONE, (y + 1) * ONE, z * ONE + 5),
            )
        })
        .collect();
    let cell = GridCellSize::try_new(FixedQ32_32::from_raw(ONE)).expect("positive");
    let required = grid_key_count(&boxes, cell).expect("a small key count");
    let mut keys = vec![GridKey::EMPTY; required];
    let mut pairs = vec![(0_u32, 0_u32); 64];

    // Only this thread's allocations are counted, so the harness cannot fail the assertion.
    let mut report = None;
    let counts = allocation_counter::measure(|| {
        report = uniform_grid_pairs(&boxes, cell, &mut keys, &mut pairs).ok();
    });

    assert_eq!(counts.count_total, 0);
    let report = report.expect("the grid runs");
    assert!(report.total() > 64, "the pair buffer was cut short, so selection ran");
    assert_eq!(report.written(), 64);
}
