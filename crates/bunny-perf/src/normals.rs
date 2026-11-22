//! Vertex normals (`bunny-vertex-normals/1`): one whole build of the normals of a seeded 32 by
//! 32 height-field grid per batch (1 024 vertices, 1 922 triangles), so one call is one vertex.
//! The time includes the two allocations of a build (the exact sums and the normals).

use std::hint::black_box;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::VertexNormals;

use crate::cases::{Case, BATCH};

const SEED: u64 = 0xB002_0289_9E2F_0001;
/// Vertices per side; `SIDE * SIDE` is the batch size.
const SIDE: u32 = 32;

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 11
}

/// A grid a quarter unit apart in `x` and `z`, with seeded heights in `[-1, 1)` units.
fn grid() -> (Vec<FixedVec3>, Vec<[u32; 3]>) {
    let mut state = SEED;
    let span = (2 * ONE_RAW).unsigned_abs();
    let positions = (0..SIDE * SIDE)
        .map(|k| {
            let height = i64::try_from(next(&mut state) % span).unwrap_or(0) - ONE_RAW;
            let at = |c: u32| FixedQ32_32::from_raw(i64::from(c) * ONE_RAW / 4);
            FixedVec3::new(at(k % SIDE), FixedQ32_32::from_raw(height), at(k / SIDE))
        })
        .collect();
    let corner = |i: u32, j: u32| j * SIDE + i;
    let triangles = (0..SIDE - 1)
        .flat_map(|j| (0..SIDE - 1).map(move |i| (i, j)))
        .flat_map(|(i, j)| {
            let (a, b) = (corner(i, j), corner(i + 1, j));
            let (c, d) = (corner(i + 1, j + 1), corner(i, j + 1));
            [[a, c, b], [a, d, c]]
        })
        .collect();
    (positions, triangles)
}

pub(crate) fn normal_cases() -> Vec<Case> {
    let (positions, triangles) = grid();
    debug_assert_eq!(positions.len(), BATCH);
    let run = move || {
        black_box(VertexNormals::new(black_box(&positions), black_box(&triangles)))
            .ok()
            .and_then(|normals| normals.normals().first().copied().flatten())
            .map_or(0, |normal| normal.into_inner().y.raw())
    };
    vec![Case { name: "shade/vertex_normals", calls: BATCH, run: Box::new(run) }]
}
