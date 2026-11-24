//! The seeded height-field mesh that the `mesh` and `codec` groups time, and its encodings.
//!
//! 64 by 64 vertices a quarter unit apart in `x` and `z`, with seeded heights in `[-1, 1)`
//! units on a `2^-16` grid: 4 096 vertices and 7 938 triangles, the order of the committed
//! Stanford bunny fixture (1 889 vertices, 3 851 faces). Every coordinate is a multiple of
//! `2^-16`, so it is an exact Q32.32 value, an exact `f32` and an exact short decimal: each
//! encoding below holds the same mesh, and the tests check that each decoder reads it back.
//!
//! Everything here is built once, before timing, from the fixed seed [`GRID_SEED`].

#[cfg(any(feature = "codec", test))]
use std::fmt::Write as _;

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{quantize_vertex, QuantizedVertex, Triangle32};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

/// The generator's fixed seed.
pub(crate) const GRID_SEED: u64 = 0xB002_0130_9E2F_0001;
/// Vertices per side.
pub(crate) const SIDE: u32 = 64;
/// `SIDE * SIDE`.
#[cfg(test)]
pub(crate) const VERTICES: usize = 4096;
/// `2 * (SIDE - 1)^2`.
#[cfg(test)]
pub(crate) const TRIANGLES: usize = 7938;

pub(crate) struct Grid {
    pub(crate) positions: Vec<FixedVec3>,
    pub(crate) triangles: Vec<Triangle32>,
    /// The smallest box holding every position.
    pub(crate) bounds: FixedAabb3,
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// The grid, from [`GRID_SEED`].
pub(crate) fn grid() -> Grid {
    let mut state = GRID_SEED;
    let at = |c: u32| FixedQ32_32::from_raw(i64::from(c) * ONE_RAW / 4);
    let positions: Vec<FixedVec3> = (0..SIDE * SIDE)
        .map(|k| {
            // 17 seeded bits: a height of `[-2^16, 2^16)` steps of `2^-16`, so `[-1, 1)` units.
            let steps = i64::try_from(next(&mut state) >> 47).unwrap_or(0) - (1 << 16);
            FixedVec3::new(at(k % SIDE), FixedQ32_32::from_raw(steps << 16), at(k / SIDE))
        })
        .collect();
    let corner = |i: u32, j: u32| j * SIDE + i;
    let triangles = (0..SIDE - 1)
        .flat_map(|j| (0..SIDE - 1).map(move |i| (i, j)))
        .flat_map(|(i, j)| {
            let (a, b) = (corner(i, j), corner(i + 1, j));
            let (c, d) = (corner(i + 1, j + 1), corner(i, j + 1));
            [Triangle32::new(a, c, b), Triangle32::new(a, d, c)]
        })
        .collect();
    let bounds = bounds(&positions);
    Grid { positions, triangles, bounds }
}

fn bounds(positions: &[FixedVec3]) -> FixedAabb3 {
    let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let first = positions.first().copied().unwrap_or(zero);
    let (min, max) = positions.iter().fold((first, first), |(min, max), p| {
        (
            FixedVec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z)),
            FixedVec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z)),
        )
    });
    FixedAabb3::new(min, max)
}

/// The positions quantized against the grid's bounds by `bunny_mesh::quantize_vertex`.
pub(crate) fn quantized(grid: &Grid) -> Vec<QuantizedVertex> {
    grid.positions.iter().map(|&p| quantize_vertex(p, &grid.bounds)).collect()
}

/// The exact decimal of a raw value whose low 16 bits are zero, without trailing zeros.
#[cfg(any(feature = "codec", test))]
fn push_decimal(out: &mut String, raw: i64) {
    let magnitude = raw.unsigned_abs();
    if raw < 0 {
        out.push('-');
    }
    // `2^-16 = 152_587_890_625 / 10^16`, so 16 decimal digits hold the fraction exactly.
    let fraction = format!("{:016}", ((magnitude >> 16) & 0xFFFF) * 152_587_890_625);
    let fraction = fraction.trim_end_matches('0');
    let fraction = if fraction.is_empty() { "0" } else { fraction };
    let _ = write!(out, "{}.{fraction}", magnitude >> 32);
}

#[cfg(feature = "codec")]
fn push_point(out: &mut String, p: FixedVec3) {
    push_decimal(out, p.x.raw());
    out.push(' ');
    push_decimal(out, p.y.raw());
    out.push(' ');
    push_decimal(out, p.z.raw());
}

/// The grid as an ASCII PLY file (`bunny-ascii-ply-triangle-mesh/1`).
#[cfg(feature = "codec")]
pub(crate) fn ascii_ply(grid: &Grid) -> Vec<u8> {
    let mut text = String::new();
    let _ = write!(
        text,
        "ply\nformat ascii 1.0\ncomment bunny-perf seeded height field\n\
         element vertex {}\nproperty float x\nproperty float y\nproperty float z\n\
         element face {}\nproperty list uchar int vertex_indices\nend_header\n",
        grid.positions.len(),
        grid.triangles.len()
    );
    for &p in &grid.positions {
        push_point(&mut text, p);
        text.push('\n');
    }
    for t in &grid.triangles {
        let _ = writeln!(text, "3 {} {} {}", t.v0, t.v1, t.v2);
    }
    text.into_bytes()
}

/// The grid as an OBJ file: `v` records with the same decimals, then one-based `f` records.
#[cfg(feature = "codec")]
pub(crate) fn obj(grid: &Grid) -> String {
    let mut text = String::from("# bunny-perf seeded height field\n");
    for &p in &grid.positions {
        text.push_str("v ");
        push_point(&mut text, p);
        text.push('\n');
    }
    for t in &grid.triangles {
        let _ = writeln!(text, "f {} {} {}", t.v0 + 1, t.v1 + 1, t.v2 + 1);
    }
    text
}

/// The grid as a binary little-endian PLY file: `float x/y/z` from `FixedQ32_32::to_f32`
/// (exact here) and `list uchar int` faces.
#[cfg(feature = "codec")]
pub(crate) fn binary_ply(grid: &Grid) -> Vec<u8> {
    let header = format!(
        "ply\nformat binary_little_endian 1.0\nelement vertex {}\nproperty float x\n\
         property float y\nproperty float z\nelement face {}\n\
         property list uchar int vertex_indices\nend_header\n",
        grid.positions.len(),
        grid.triangles.len()
    );
    let mut bytes = header.into_bytes();
    for p in &grid.positions {
        for c in [p.x, p.y, p.z] {
            bytes.extend_from_slice(&c.to_f32().to_le_bytes());
        }
    }
    for t in &grid.triangles {
        bytes.push(3);
        for index in [t.v0, t.v1, t.v2] {
            bytes.extend_from_slice(&i32::try_from(index).unwrap_or(-1).to_le_bytes());
        }
    }
    bytes
}

/// The grid as a canonical Bunny compressed mesh (magic `BUNNYQZ!`, version 1, 16-bit
/// indices): the 76-byte header with the grid's bounds, the positions from [`quantized`],
/// then the triangles. Written here because `bunny-codec` has a decoder and no encoder.
#[cfg(feature = "codec")]
pub(crate) fn compressed(grid: &Grid) -> Vec<u8> {
    let vertices = quantized(grid);
    let vertex_count = u32::try_from(vertices.len()).unwrap_or(0);
    let triangle_count = u32::try_from(grid.triangles.len()).unwrap_or(0);
    let payload = 6 * (u64::from(vertex_count) + u64::from(triangle_count));
    let mut bytes = b"BUNNYQZ!".to_vec();
    bytes.extend_from_slice(&[1, 16, 0, 0]);
    bytes.extend_from_slice(&vertex_count.to_le_bytes());
    bytes.extend_from_slice(&triangle_count.to_le_bytes());
    bytes.extend_from_slice(&payload.to_le_bytes());
    let (min, max) = (grid.bounds.min, grid.bounds.max);
    for c in [min.x, min.y, min.z, max.x, max.y, max.z] {
        bytes.extend_from_slice(&c.raw().to_le_bytes());
    }
    for v in &vertices {
        for c in [v.x, v.y, v.z] {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    for t in &grid.triangles {
        for index in [t.v0, t.v1, t.v2] {
            bytes.extend_from_slice(&u16::try_from(index).unwrap_or(u16::MAX).to_le_bytes());
        }
    }
    bytes
}

/// FNV-1a over bytes: a digest for the determinism tests.
#[cfg(test)]
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// The grid's positions and triangles as bytes, for the determinism tests.
#[cfg(test)]
pub(crate) fn grid_bytes(grid: &Grid) -> Vec<u8> {
    let mut bytes = Vec::new();
    for p in &grid.positions {
        for c in [p.x, p.y, p.z] {
            bytes.extend_from_slice(&c.raw().to_le_bytes());
        }
    }
    for t in &grid.triangles {
        for index in [t.v0, t.v1, t.v2] {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_has_the_stated_size_and_steps() {
        let grid = grid();
        assert_eq!((grid.positions.len(), grid.triangles.len()), (VERTICES, TRIANGLES));
        for p in &grid.positions {
            for c in [p.x, p.y, p.z] {
                assert_eq!(c.raw() & 0xFFFF, 0, "every coordinate is a multiple of 2^-16");
            }
            assert!((-ONE_RAW..ONE_RAW).contains(&p.y.raw()));
        }
    }

    /// The grid is the same bytes on every construction, and these bytes (a digest recorded
    /// from this generator): a change to the generator, the seed or the platform shows here.
    #[test]
    fn the_grid_is_the_same_on_every_construction() {
        let (first, second) = (grid_bytes(&grid()), grid_bytes(&grid()));
        assert_eq!(first, second);
        assert_eq!(first.len(), VERTICES * 24 + TRIANGLES * 12);
        assert_eq!(fnv1a(&first), GRID_DIGEST);
    }

    #[test]
    fn decimals_are_exact_and_short() {
        let mut out = String::new();
        for raw in [0, ONE_RAW, -ONE_RAW / 4, 1 << 16, -(1 << 16), (3 * ONE_RAW) / 2] {
            out.clear();
            push_decimal(&mut out, raw);
            let parsed = exact_parse(&out);
            assert_eq!(parsed, raw, "{out}");
        }
        out.clear();
        push_decimal(&mut out, 1 << 16);
        assert_eq!(out, "0.0000152587890625");
        out.clear();
        push_decimal(&mut out, -ONE_RAW / 4);
        assert_eq!(out, "-0.25");
    }

    /// An exact decimal reader for the test above, sharing nothing with `push_decimal`:
    /// `value * 2^32` computed in `i128` from the digits, exact for at most 16 fractional
    /// digits of a multiple of `2^-16`.
    fn exact_parse(text: &str) -> i64 {
        let (negative, body) = text.strip_prefix('-').map_or((false, text), |rest| (true, rest));
        let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
        let whole: i128 = whole.parse().unwrap_or(0);
        let digits = u32::try_from(fraction.len()).unwrap_or(0);
        let numerator: i128 = if fraction.is_empty() { 0 } else { fraction.parse().unwrap_or(0) };
        let scaled = (whole << 32) + (numerator << 32) / 10_i128.pow(digits);
        i64::try_from(if negative { -scaled } else { scaled }).unwrap_or(0)
    }

    /// FNV-1a of [`grid_bytes`] of [`grid`], recorded from this generator.
    const GRID_DIGEST: u64 = 0xFBF7_7330_62D7_74A0;
}
