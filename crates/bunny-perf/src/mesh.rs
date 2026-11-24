//! Mesh quantization and the mesh verification hash of `bunny-mesh` on the seeded grid of
//! `grid.rs` (64 by 64 vertices, 7 938 triangles, seed `GRID_SEED`), built before timing.
//!
//! - `mesh/quantize_vertex`: `quantize_vertex` on 1 024 grid positions (every fourth vertex)
//!   against the grid's bounds, one call per position. Each result passes through
//!   `black_box` and its three components are summed into the checksum.
//! - `mesh/dequantize_vertex`: `dequantize_vertex` on those positions' quantized values,
//!   one call per vertex; each result passes through `black_box` and its `x`, `y` and `z`
//!   raw values are summed.
//! - `mesh/hash`: one `compute_mesh_hash` of the whole quantized grid (4 096 vertices,
//!   7 938 triangles as `Width32` indices) per call. The digest passes through `black_box`
//!   and its first eight bytes are the checksum, so no byte of the input can be skipped.

use std::hint::black_box;

use bunny_mesh::{compute_mesh_hash, dequantize_vertex, quantize_vertex, IndexBufferLayout};

use crate::cases::{Case, BATCH};
use crate::grid::{grid, quantized};

/// Every case of the `mesh` group.
pub(crate) fn mesh_cases() -> Vec<Case> {
    let grid = grid();
    let bounds = grid.bounds;
    let points: Vec<_> = grid.positions.iter().step_by(4).copied().collect();
    debug_assert_eq!(points.len(), BATCH);
    let codes: Vec<_> = points.iter().map(|&p| quantize_vertex(p, &bounds)).collect();
    let vertices = quantized(&grid);
    let triangles = grid.triangles;
    let quantize = move || {
        points
            .iter()
            .map(|&p| {
                let q = black_box(quantize_vertex(black_box(p), &bounds));
                i64::from(q.x) + i64::from(q.y) + i64::from(q.z)
            })
            .fold(0, i64::wrapping_add)
    };
    let dequantize = move || {
        codes
            .iter()
            .map(|&q| {
                let p = black_box(dequantize_vertex(black_box(q), &bounds));
                p.x.raw().wrapping_add(p.y.raw()).wrapping_add(p.z.raw())
            })
            .fold(0, i64::wrapping_add)
    };
    let hash = move || {
        let layout = IndexBufferLayout::Width32(black_box(&triangles));
        let digest = black_box(compute_mesh_hash(black_box(&vertices), layout, &bounds));
        let mut first = [0_u8; 8];
        first.copy_from_slice(&digest[..8]);
        i64::from_le_bytes(first)
    };
    vec![
        Case { name: "mesh/quantize_vertex", calls: BATCH, run: Box::new(quantize) },
        Case { name: "mesh/dequantize_vertex", calls: BATCH, run: Box::new(dequantize) },
        Case { name: "mesh/hash", calls: 1, run: Box::new(hash) },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::fnv1a;

    /// Each case runs once (no timing) and returns the same checksum on every run and on every
    /// construction of its inputs.
    #[test]
    fn every_mesh_case_runs_and_its_checksum_is_stable() {
        let (first, second) = (mesh_cases(), mesh_cases());
        let names: Vec<&str> = first.iter().map(|case| case.name).collect();
        assert_eq!(names, ["mesh/quantize_vertex", "mesh/dequantize_vertex", "mesh/hash"]);
        for (a, b) in first.iter().zip(&second) {
            let value = (a.run)();
            assert_eq!(value, (a.run)(), "{}", a.name);
            assert_eq!(value, (b.run)(), "{}", a.name);
            assert_ne!(value, 0, "{} does work", a.name);
        }
    }

    /// The quantized grid the hash reads is the same bytes on every construction, and these
    /// (a digest recorded from this generator).
    #[test]
    fn the_quantized_grid_is_the_same_on_every_construction() {
        let bytes = |v: Vec<bunny_mesh::QuantizedVertex>| -> Vec<u8> {
            v.iter().flat_map(|q| [q.x, q.y, q.z]).flat_map(u16::to_le_bytes).collect()
        };
        let (first, second) = (bytes(quantized(&grid())), bytes(quantized(&grid())));
        assert_eq!(first, second);
        assert_eq!(fnv1a(&first), QUANTIZED_DIGEST);
    }

    /// The quantize case does real work: its codes span the whole `u16` range on `y`.
    #[test]
    fn the_quantized_heights_span_the_range() {
        let grid = grid();
        let codes = quantized(&grid);
        assert_eq!(codes.iter().map(|q| q.y).min(), Some(0));
        assert_eq!(codes.iter().map(|q| q.y).max(), Some(u16::MAX));
    }

    const QUANTIZED_DIGEST: u64 = 0x3427_8AF6_737C_44DC;
}
