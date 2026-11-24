//! Decoding the seeded grid of `grid.rs` (64 by 64 vertices, 7 938 triangles, seed
//! `GRID_SEED`) from each format `bunny-codec` reads. The four inputs are encoded once, before
//! timing, from the same mesh: ASCII PLY (245 KB of text), binary PLY (153 KB), OBJ (253 KB of
//! text) and the Bunny compressed mesh (72 KB).
//!
//! One call is one whole decode: the parse, which validates every record, then reading every
//! vertex and every triangle back through the returned view, as a caller that loads the mesh
//! does. Every coordinate and index read is folded into the checksum, and the input and the
//! result pass through `black_box`, so neither the parse nor a read can be skipped.
//!
//! The committed fixtures are not used for timing: `cargo xtask perf` builds a copy of this
//! harness's `src/` files under `target/perf/`, where a path to `bunny-codec/tests/fixtures`
//! does not resolve. The tests below run the ASCII PLY decode on the committed Stanford bunny
//! fixture as well.

use std::hint::black_box;

use bunny_codec::{
    decode_compressed_mesh, parse_ascii_ply, parse_binary_ply, parse_obj_text, CompressedTriangle,
};
use bunny_mesh::Triangle32;

use crate::cases::Case;
use crate::grid::{ascii_ply, binary_ply, compressed, grid, obj};

/// What one decode read: the records, a checksum of every coordinate, and the index sum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Decoded {
    pub(crate) vertices: usize,
    pub(crate) triangles: usize,
    pub(crate) coordinates: i64,
    pub(crate) indices: u64,
}

impl Decoded {
    fn vertex(&mut self, [x, y, z]: [i64; 3]) {
        self.vertices += 1;
        let mixed = x ^ y.rotate_left(21) ^ z.rotate_left(42);
        self.coordinates = self.coordinates.rotate_left(5).wrapping_add(mixed);
    }

    fn triangle(&mut self, t: Triangle32) {
        self.triangles += 1;
        self.indices += u64::from(t.v0) + u64::from(t.v1) + u64::from(t.v2);
    }

    fn checksum(self) -> i64 {
        let counts = i64::try_from(self.vertices + self.triangles).unwrap_or(0);
        let indices = i64::from_ne_bytes(self.indices.to_ne_bytes());
        self.coordinates.wrapping_add(counts).wrapping_add(indices)
    }
}

/// `parse_ascii_ply`, then every vertex (Q32.32 raw values) and triangle.
pub(crate) fn decode_ascii_ply(bytes: &[u8]) -> Option<Decoded> {
    let mesh = parse_ascii_ply(bytes).ok()?;
    let mut decoded = Decoded::default();
    for vertex in mesh.vertices() {
        let v = vertex.ok()?;
        decoded.vertex([v.x.raw(), v.y.raw(), v.z.raw()]);
    }
    for triangle in mesh.triangles() {
        decoded.triangle(triangle.ok()?);
    }
    Some(decoded)
}

fn f32_bits(value: f32) -> i64 {
    i64::from(value.to_bits())
}

/// `parse_binary_ply`, then every vertex (`f32` bits) and triangle.
pub(crate) fn decode_binary_ply(bytes: &[u8]) -> Option<Decoded> {
    let mesh = parse_binary_ply(bytes).ok()?;
    let mut decoded = Decoded::default();
    for index in 0..mesh.vertex_count() {
        let v = mesh.vertex(index).ok()?;
        decoded.vertex([f32_bits(v.x), f32_bits(v.y), f32_bits(v.z)]);
    }
    for index in 0..mesh.face_count() {
        decoded.triangle(mesh.triangle(index).ok()?);
    }
    Some(decoded)
}

/// `parse_obj_text`, then every vertex (`f32` bits) and triangle through its iterators.
pub(crate) fn decode_obj(text: &str) -> Option<Decoded> {
    let mesh = parse_obj_text(text).ok()?;
    let mut decoded = Decoded::default();
    for vertex in mesh.vertices() {
        let v = vertex.ok()?;
        decoded.vertex([f32_bits(v.x), f32_bits(v.y), f32_bits(v.z)]);
    }
    for triangle in mesh.triangles() {
        decoded.triangle(triangle.ok()?);
    }
    Some(decoded)
}

/// `decode_compressed_mesh`, then every quantized vertex and triangle.
pub(crate) fn decode_compressed(bytes: &[u8]) -> Option<Decoded> {
    let mesh = decode_compressed_mesh(bytes).ok()?;
    let mut decoded = Decoded::default();
    for index in 0..mesh.vertex_count() {
        let q = mesh.vertex(index).ok()?;
        decoded.vertex([i64::from(q.x), i64::from(q.y), i64::from(q.z)]);
    }
    for index in 0..mesh.triangle_count() {
        let triangle = match mesh.triangle(index).ok()? {
            CompressedTriangle::Width16(t) => {
                Triangle32::new(u32::from(t.v0), u32::from(t.v1), u32::from(t.v2))
            }
            CompressedTriangle::Width32(t) => t,
        };
        decoded.triangle(triangle);
    }
    Some(decoded)
}

fn decode_case<T: ?Sized + 'static>(
    name: &'static str,
    input: Box<T>,
    decode: fn(&T) -> Option<Decoded>,
) -> Case {
    let run = move || black_box(decode(black_box(&*input))).map_or(-1, Decoded::checksum);
    Case { name, calls: 1, run: Box::new(run) }
}

/// Every case of the `codec` group.
pub(crate) fn codec_cases() -> Vec<Case> {
    let grid = grid();
    vec![
        decode_case(
            "codec/ply_ascii_decode",
            ascii_ply(&grid).into_boxed_slice(),
            decode_ascii_ply,
        ),
        decode_case(
            "codec/ply_binary_decode",
            binary_ply(&grid).into_boxed_slice(),
            decode_binary_ply,
        ),
        decode_case("codec/obj_decode", obj(&grid).into_boxed_str(), decode_obj),
        decode_case(
            "codec/compressed_decode",
            compressed(&grid).into_boxed_slice(),
            decode_compressed,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{fnv1a, quantized, TRIANGLES, VERTICES};

    /// The committed Stanford bunny (`bunny-codec`'s ASCII PLY fixture).
    const STANFORD: &[u8] =
        include_bytes!("../../bunny-codec/tests/fixtures/stanford-bunny-ascii/bun_zipper_res3.ply");

    fn index_sum(triangles: &[Triangle32]) -> u64 {
        triangles.iter().map(|t| u64::from(t.v0) + u64::from(t.v1) + u64::from(t.v2)).sum()
    }

    /// Every decoder reads the whole grid back: every vertex, every triangle, and the same
    /// triangles as the grid holds (by their index sum).
    #[test]
    fn every_decoder_reads_the_whole_grid() {
        let grid = grid();
        let want = index_sum(&grid.triangles);
        let decoded = [
            decode_ascii_ply(&ascii_ply(&grid)),
            decode_binary_ply(&binary_ply(&grid)),
            decode_obj(&obj(&grid)),
            decode_compressed(&compressed(&grid)),
        ];
        for (format, d) in ["ascii ply", "binary ply", "obj", "compressed"].iter().zip(decoded) {
            let d = d.unwrap_or_else(|| panic!("{format} decodes"));
            assert_eq!(
                (d.vertices, d.triangles, d.indices),
                (VERTICES, TRIANGLES, want),
                "{format}"
            );
        }
    }

    /// The decoded values are the grid's: ASCII PLY gives the exact Q32.32 positions, binary PLY
    /// and OBJ the exact `f32` of each, and the compressed mesh the quantized positions.
    #[test]
    fn every_decoder_returns_the_grid_values() {
        let grid = grid();
        let mut exact = Decoded::default();
        let mut float = Decoded::default();
        for p in &grid.positions {
            exact.vertex([p.x.raw(), p.y.raw(), p.z.raw()]);
            float.vertex([f32_bits(p.x.to_f32()), f32_bits(p.y.to_f32()), f32_bits(p.z.to_f32())]);
        }
        let mut codes = Decoded::default();
        for q in quantized(&grid) {
            codes.vertex([i64::from(q.x), i64::from(q.y), i64::from(q.z)]);
        }
        let coordinates = |d: Option<Decoded>| d.map(|d| d.coordinates);
        assert_eq!(coordinates(decode_ascii_ply(&ascii_ply(&grid))), Some(exact.coordinates));
        assert_eq!(coordinates(decode_binary_ply(&binary_ply(&grid))), Some(float.coordinates));
        assert_eq!(coordinates(decode_obj(&obj(&grid))), Some(float.coordinates));
        assert_eq!(coordinates(decode_compressed(&compressed(&grid))), Some(codes.coordinates));
    }

    /// The ASCII PLY decode also reads the committed Stanford bunny fixture whole.
    #[test]
    fn the_ascii_decode_reads_the_committed_stanford_fixture() {
        let d = decode_ascii_ply(STANFORD).expect("the Stanford fixture decodes");
        assert_eq!((d.vertices, d.triangles), (1889, 3851));
    }

    /// Each encoded input is the same bytes on every construction, and these (lengths and
    /// digests recorded from this generator).
    #[test]
    fn every_input_is_the_same_on_every_construction() {
        let inputs = |grid: &crate::grid::Grid| {
            [ascii_ply(grid), binary_ply(grid), obj(grid).into_bytes(), compressed(grid)]
        };
        let (first, second) = (inputs(&grid()), inputs(&grid()));
        assert_eq!(first, second);
        let found: Vec<(usize, u64)> = first.iter().map(|b| (b.len(), fnv1a(b))).collect();
        assert_eq!(found, INPUT_DIGESTS);
    }

    /// Each case runs once (no timing) and returns the same checksum on every run and every
    /// construction; none is the failure value `-1`.
    #[test]
    fn every_codec_case_runs_and_its_checksum_is_stable() {
        let (first, second) = (codec_cases(), codec_cases());
        let names: Vec<&str> = first.iter().map(|case| case.name).collect();
        assert_eq!(
            names,
            [
                "codec/ply_ascii_decode",
                "codec/ply_binary_decode",
                "codec/obj_decode",
                "codec/compressed_decode"
            ]
        );
        for (a, b) in first.iter().zip(&second) {
            let value = (a.run)();
            assert_eq!(value, (a.run)(), "{}", a.name);
            assert_eq!(value, (b.run)(), "{}", a.name);
            assert_ne!(value, -1, "{} decodes", a.name);
        }
    }

    /// `(length, FNV-1a)` of the ASCII PLY, binary PLY, OBJ and compressed inputs.
    const INPUT_DIGESTS: [(usize, u64); 4] = [
        (244_676, 0x2CFB_4600_E4B3_1C19),
        (152_521, 0x14A3_BC5A_85B8_F8A1),
        (252_717, 0x63C1_C72D_7B62_5F10),
        (72_280, 0x3098_4B6B_696C_A4E4),
    ];
}
