//! The Bunny compressed mesh encoder: round trips through the real decoder, canonical bytes
//! pinned by an independent model, and the refusal and output-buffer contract.

use bunny_codec::{
    decode_compressed_mesh, encode_compressed_mesh, encode_compressed_mesh_into,
    encoded_compressed_mesh_len, CompressedIndexWidth, CompressedMesh, CompressedMeshEncodeError,
    CompressedTriangle,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{IndexBufferLayout, QuantizedVertex, Triangle16, Triangle32};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const CANONICAL_HEX: &str = include_str!("fixtures/canonical_compressed_triangle.bunny.hex");
const ONE_RAW: i64 = 1_i64 << 32;
const MAX_COUNT: usize = 1_000_000;
const WIDTH16_CAPACITY: usize = 65_536;

// ---------------------------------------------------------------------------------------------
// Independent oracle: the profile v1 layout of `docs/goalposts/v0.4.0-gp3.md`, written field by
// field by pushing onto a `Vec`. It shares no code with the encoder.
// ---------------------------------------------------------------------------------------------

enum OracleTriangles<'a> {
    W16(&'a [Triangle16]),
    W32(&'a [Triangle32]),
}

fn oracle_bytes(bounds: &FixedAabb3, vertices: &[QuantizedVertex], triangles: &OracleTriangles<'_>) -> Vec<u8> {
    let (width, count, stride) = match triangles {
        OracleTriangles::W16(faces) => (16_u8, faces.len(), 6_u64),
        OracleTriangles::W32(faces) => (32_u8, faces.len(), 12_u64),
    };
    let payload = vertices.len() as u64 * 6 + count as u64 * stride;
    let mut out = Vec::new();
    out.extend_from_slice(b"BUNNYQZ!");
    out.push(1);
    out.push(width);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32::try_from(vertices.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&u32::try_from(count).unwrap().to_le_bytes());
    out.extend_from_slice(&payload.to_le_bytes());
    for corner in [bounds.min, bounds.max] {
        for value in [corner.x, corner.y, corner.z] {
            out.extend_from_slice(&value.to_raw().to_le_bytes());
        }
    }
    for vertex in vertices {
        for value in [vertex.x, vertex.y, vertex.z] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    match triangles {
        OracleTriangles::W16(faces) => {
            for face in *faces {
                for index in [face.v0, face.v1, face.v2] {
                    out.extend_from_slice(&index.to_le_bytes());
                }
            }
        }
        OracleTriangles::W32(faces) => {
            for face in *faces {
                for index in [face.v0, face.v1, face.v2] {
                    out.extend_from_slice(&index.to_le_bytes());
                }
            }
        }
    }
    out
}

/// FNV-1a, 64 bits: the digest the Python model pins for the seeded meshes.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn parse_hex(input: &str) -> Vec<u8> {
    let digits: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    assert_eq!(digits.len() % 2, 0, "hex must contain byte pairs");
    digits
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn vec3(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

fn unit_bounds() -> FixedAabb3 {
    FixedAabb3::new(vec3(0, 0, 0), vec3(ONE_RAW, ONE_RAW, ONE_RAW))
}

fn fixture_vertices() -> [QuantizedVertex; 3] {
    [QuantizedVertex::new(0, 0, 0), QuantizedVertex::new(u16::MAX, 0, 0), QuantizedVertex::new(0, u16::MAX, 0)]
}

// ---------------------------------------------------------------------------------------------
// Round-trip helpers: decode with the real decoder and compare every record.
// ---------------------------------------------------------------------------------------------

fn assert_round_trip(bounds: &FixedAabb3, vertices: &[QuantizedVertex], triangles: IndexBufferLayout<'_>) -> Vec<u8> {
    let bytes = encode_compressed_mesh(bounds, vertices, triangles).expect("valid mesh encodes");
    assert_eq!(encoded_compressed_mesh_len(vertices, triangles), Ok(bytes.len()));
    let mesh = decode_compressed_mesh(&bytes).expect("encoded mesh decodes");
    assert_eq!(mesh.bounds(), *bounds);
    assert_eq!(mesh.vertex_count(), vertices.len());
    assert_eq!(mesh.triangle_count(), triangles.len());
    for (index, vertex) in vertices.iter().enumerate() {
        assert_eq!(mesh.vertex(index), Ok(*vertex), "vertex {index}");
    }
    match triangles {
        IndexBufferLayout::Width16(faces) => {
            assert_eq!(mesh.index_width(), CompressedIndexWidth::Width16);
            for (index, face) in faces.iter().enumerate() {
                assert_eq!(mesh.triangle(index), Ok(CompressedTriangle::Width16(*face)), "triangle {index}");
            }
        }
        IndexBufferLayout::Width32(faces) => {
            assert_eq!(mesh.index_width(), CompressedIndexWidth::Width32);
            for (index, face) in faces.iter().enumerate() {
                assert_eq!(mesh.triangle(index), Ok(CompressedTriangle::Width32(*face)), "triangle {index}");
            }
        }
    }
    let mut into = vec![0_u8; bytes.len()];
    assert_eq!(encode_compressed_mesh_into(bounds, vertices, triangles, &mut into), Ok(bytes.len()));
    assert_eq!(into, bytes, "the two entry points write the same bytes");
    bytes
}

/// Reads every record of a decoded view back out and encodes it again.
fn reencode(mesh: CompressedMesh<'_>) -> Vec<u8> {
    let vertices: Vec<QuantizedVertex> = (0..mesh.vertex_count()).map(|i| mesh.vertex(i).unwrap()).collect();
    let triangles: Vec<CompressedTriangle> = (0..mesh.triangle_count()).map(|i| mesh.triangle(i).unwrap()).collect();
    match mesh.index_width() {
        CompressedIndexWidth::Width16 => {
            let faces: Vec<Triangle16> = triangles
                .iter()
                .map(|t| match t {
                    CompressedTriangle::Width16(face) => *face,
                    CompressedTriangle::Width32(_) => panic!("width mismatch"),
                })
                .collect();
            encode_compressed_mesh(&mesh.bounds(), &vertices, IndexBufferLayout::Width16(&faces)).unwrap()
        }
        CompressedIndexWidth::Width32 => {
            let faces: Vec<Triangle32> = triangles
                .iter()
                .map(|t| match t {
                    CompressedTriangle::Width32(face) => *face,
                    CompressedTriangle::Width16(_) => panic!("width mismatch"),
                })
                .collect();
            encode_compressed_mesh(&mesh.bounds(), &vertices, IndexBufferLayout::Width32(&faces)).unwrap()
        }
    }
}

fn assert_canonical(bytes: &[u8]) {
    let mesh = decode_compressed_mesh(bytes).expect("stream decodes");
    assert_eq!(reencode(mesh), bytes, "decode then encode reproduces the accepted stream");
}

// ---------------------------------------------------------------------------------------------
// Seeded meshes. The generator and the mesh recipe are mirrored exactly by the Python model
// posted on the pull request, which pins each mesh's length and FNV-1a-64 digest.
// ---------------------------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn below(&mut self, bound: u64) -> u64 {
        (self.next() >> 32) % bound
    }
    fn coordinate(&mut self) -> u16 {
        u16::try_from(self.next() >> 48).unwrap()
    }
}

struct SeededMesh {
    bounds: FixedAabb3,
    vertices: Vec<QuantizedVertex>,
    width16: Vec<Triangle16>,
    width32: Vec<Triangle32>,
    wide: bool,
}

impl SeededMesh {
    fn layout(&self) -> IndexBufferLayout<'_> {
        if self.wide {
            IndexBufferLayout::Width32(&self.width32)
        } else {
            IndexBufferLayout::Width16(&self.width16)
        }
    }
    fn oracle(&self) -> Vec<u8> {
        if self.wide {
            oracle_bytes(&self.bounds, &self.vertices, &OracleTriangles::W32(&self.width32))
        } else {
            oracle_bytes(&self.bounds, &self.vertices, &OracleTriangles::W16(&self.width16))
        }
    }
}

fn seeded_mesh(seed: u64) -> SeededMesh {
    let mut rng = Lcg(seed);
    let vertex_count = 1 + rng.below(300);
    let triangle_count = 1 + rng.below(300);
    let wide = rng.next() >> 63 == 1;
    let mut min = [0_i64; 3];
    let mut max = [0_i64; 3];
    for (lo, hi) in min.iter_mut().zip(max.iter_mut()) {
        let a = rng.next() as i64;
        let b = rng.next() as i64;
        (*lo, *hi) = (a.min(b), a.max(b));
    }
    let bounds = FixedAabb3::new(vec3(min[0], min[1], min[2]), vec3(max[0], max[1], max[2]));
    let vertices = (0..vertex_count)
        .map(|_| {
            let (x, y, z) = (rng.coordinate(), rng.coordinate(), rng.coordinate());
            QuantizedVertex::new(x, y, z)
        })
        .collect();
    let mut width16 = Vec::new();
    let mut width32 = Vec::new();
    for _ in 0..triangle_count {
        let corners = [rng.below(vertex_count), rng.below(vertex_count), rng.below(vertex_count)];
        let [a, b, c] = corners.map(|index| u32::try_from(index).unwrap());
        width32.push(Triangle32::new(a, b, c));
        let [a, b, c] = corners.map(|index| u16::try_from(index).unwrap());
        width16.push(Triangle16::new(a, b, c));
    }
    SeededMesh { bounds, vertices, width16, width32, wide }
}

/// (seed, encoded length, FNV-1a-64 of the encoded bytes), computed by the Python model.
const SEEDED_PINS: [(u64, usize, u64); 8] = [
    (0x0000_0000_0000_0001, 1_066, 0x1FB6_4B76_2768_9F6B), // 149 vertices, 8 triangles, width 32
    (0x0000_0000_0000_007B, 2_770, 0xD637_0A10_0020_16C6), // 279 vertices, 85 triangles, width 32
    (0x0000_0000_0BAD_5EED, 4_396, 0xC8D0_9573_C564_5DAC), // 220 vertices, 250 triangles, width 32
    (0x0000_0000_DEAD_BEEF, 1_990, 0x34E8_7A9D_583D_6ABB), // 49 vertices, 135 triangles, width 32
    (0x0123_4567_89AB_CDEF, 3_424, 0x8128_14EF_CDED_08AA), // 232 vertices, 163 triangles, width 32
    (0x5EED_0000_0000_0124, 2_086, 0x0896_8D84_DCAB_944B), // 283 vertices, 52 triangles, width 16
    (0x8000_0000_0000_0000, 2_842, 0xF8F8_F387_5676_6C84), // 163 vertices, 298 triangles, width 16
    (0xFFFF_FFFF_FFFF_FFFF, 4_600, 0xFAED_CF85_BB30_04A3), // 178 vertices, 288 triangles, width 32
];

// ---------------------------------------------------------------------------------------------
// CMESH-TP-001: the committed fixture.
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_001_encoding_the_committed_fixture_records_reproduces_its_bytes() {
    let fixture = parse_hex(CANONICAL_HEX);
    let faces = [Triangle16::new(0, 1, 2)];
    let bytes = assert_round_trip(&unit_bounds(), &fixture_vertices(), IndexBufferLayout::Width16(&faces));
    assert_eq!(to_hex(&bytes), to_hex(&fixture));
    assert_canonical(&fixture);

    // The width-32 form of the same mesh, as `compressed_tests.rs` derives it from the fixture.
    let wide = [Triangle32::new(0, 1, 2)];
    let bytes = assert_round_trip(&unit_bounds(), &fixture_vertices(), IndexBufferLayout::Width32(&wide));
    let mut expected = fixture[..94].to_vec();
    expected[9] = 32;
    expected[20] = 30;
    expected.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    assert_eq!(bytes, expected);
    assert_canonical(&bytes);
}

// ---------------------------------------------------------------------------------------------
// CMESH-TP-002: constructed meshes at the extremes, pinned to hex from the Python model.
// ---------------------------------------------------------------------------------------------

const EXTREME_W32_HEX: &str = concat!(
    "42554e4e59515a21012000000200000002000000240000000000000000000000",
    "000000800000000000000080ffffffffffffffffffffffffffffff7f00000000",
    "000000800000000000000000ffff00800100341200ffffff0100000000000000",
    "01000000000000000000000000000000",
);
const FLAT_W16_HEX: &str = concat!(
    "42554e4e59515a210110000001000000010000000c0000000000000000000000",
    "ffffffff0700000000000000070000000000000000000000ffffffff07000000",
    "000000000800000000000000000000000000000000000000",
);

fn extreme_bounds() -> FixedAabb3 {
    FixedAabb3::new(vec3(i64::MIN, i64::MIN, -1), vec3(i64::MAX, i64::MIN, 0))
}

fn extreme_vertices() -> [QuantizedVertex; 2] {
    [QuantizedVertex::new(u16::MAX, 0x8000, 1), QuantizedVertex::new(0x1234, 0xFF00, u16::MAX)]
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_002_constructed_extremes_match_the_independent_model_byte_for_byte() {
    let vertices = extreme_vertices();
    let wide = [Triangle32::new(1, 0, 1), Triangle32::new(0, 0, 0)];
    let bytes = assert_round_trip(&extreme_bounds(), &vertices, IndexBufferLayout::Width32(&wide));
    assert_eq!(bytes, oracle_bytes(&extreme_bounds(), &vertices, &OracleTriangles::W32(&wide)));
    assert_eq!(to_hex(&bytes), EXTREME_W32_HEX);
    assert_canonical(&bytes);

    let flat = FixedAabb3::new(vec3(-ONE_RAW, 7, 7), vec3(-ONE_RAW, 7, 8));
    let vertex = [QuantizedVertex::new(0, 0, 0)];
    let faces = [Triangle16::new(0, 0, 0)];
    let bytes = assert_round_trip(&flat, &vertex, IndexBufferLayout::Width16(&faces));
    assert_eq!(bytes, oracle_bytes(&flat, &vertex, &OracleTriangles::W16(&faces)));
    assert_eq!(to_hex(&bytes), FLAT_W16_HEX);
    assert_canonical(&bytes);
}

// ---------------------------------------------------------------------------------------------
// CMESH-TP-003 and -004: seeded meshes round trip, match the oracle and the pins, and the
// accepted streams re-encode to themselves.
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_003_seeded_meshes_round_trip_and_match_the_pinned_model() {
    for (seed, length, digest) in SEEDED_PINS {
        let mesh = seeded_mesh(seed);
        println!(
            "seed {seed:#018x}: {} vertices, {} triangles, width {}",
            mesh.vertices.len(),
            mesh.layout().len(),
            if mesh.wide { 32 } else { 16 }
        );
        let bytes = assert_round_trip(&mesh.bounds, &mesh.vertices, mesh.layout());
        assert_eq!(bytes, mesh.oracle(), "seed {seed:#018x}");
        assert_eq!((bytes.len(), fnv1a64(&bytes)), (length, digest), "seed {seed:#018x}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_004_every_accepted_stream_tested_reencodes_to_itself() {
    for (seed, _, _) in SEEDED_PINS {
        let mesh = seeded_mesh(seed);
        // Both widths of every seeded mesh: its indices all fit u16 (fewer than 301 vertices).
        let narrow = oracle_bytes(&mesh.bounds, &mesh.vertices, &OracleTriangles::W16(&mesh.width16));
        let wide = oracle_bytes(&mesh.bounds, &mesh.vertices, &OracleTriangles::W32(&mesh.width32));
        assert_canonical(&narrow);
        assert_canonical(&wide);
        assert_ne!(narrow, wide, "the width is part of the record, so the streams differ");
    }
}

// ---------------------------------------------------------------------------------------------
// CMESH-TP-005 to -009: the domain and the refusals.
// ---------------------------------------------------------------------------------------------

fn points(count: usize) -> Vec<QuantizedVertex> {
    (0..count).map(|i| QuantizedVertex::new((i % 65_536) as u16, (i / 65_536) as u16, 7)).collect()
}

fn encode_error(bounds: &FixedAabb3, vertices: &[QuantizedVertex], triangles: IndexBufferLayout<'_>) -> CompressedMeshEncodeError {
    let first = encode_compressed_mesh(bounds, vertices, triangles).expect_err("refused");
    let mut sentinel = vec![0xA5_u8; 128];
    assert_eq!(encode_compressed_mesh_into(bounds, vertices, triangles, &mut sentinel), Err(first));
    assert!(sentinel.iter().all(|byte| *byte == 0xA5), "a refusal writes nothing");
    first
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_005_counts_from_one_to_one_million_are_accepted_and_others_refused() {
    let one = [Triangle16::new(0, 0, 0)];
    let bounds = unit_bounds();
    assert_eq!(
        encode_error(&bounds, &[], IndexBufferLayout::Width16(&one)),
        CompressedMeshEncodeError::InvalidVertexCount { count: 0 }
    );
    assert_eq!(
        encode_error(&bounds, &points(1), IndexBufferLayout::Width16(&[])),
        CompressedMeshEncodeError::InvalidTriangleCount { count: 0 }
    );
    assert_eq!(
        encoded_compressed_mesh_len(&[], IndexBufferLayout::Width16(&one)),
        Err(CompressedMeshEncodeError::InvalidVertexCount { count: 0 })
    );

    let many = points(MAX_COUNT + 1);
    assert_eq!(
        encode_error(&bounds, &many, IndexBufferLayout::Width32(&[Triangle32::new(0, 0, 0)])),
        CompressedMeshEncodeError::InvalidVertexCount { count: MAX_COUNT + 1 }
    );
    let faces = vec![Triangle32::new(0, 0, 0); MAX_COUNT + 1];
    assert_eq!(
        encode_error(&bounds, &points(1), IndexBufferLayout::Width32(&faces)),
        CompressedMeshEncodeError::InvalidTriangleCount { count: MAX_COUNT + 1 }
    );

    // The largest stream the profile admits: 76 + 6 * 10^6 + 12 * 10^6 bytes, fitting a 32-bit usize.
    let max_vertices = &many[..MAX_COUNT];
    let last = u32::try_from(MAX_COUNT - 1).unwrap();
    let max_faces: Vec<Triangle32> =
        (0..MAX_COUNT).map(|i| Triangle32::new(u32::try_from(i).unwrap(), last, 0)).collect();
    let layout = IndexBufferLayout::Width32(&max_faces);
    assert_eq!(encoded_compressed_mesh_len(max_vertices, layout), Ok(18_000_076));
    let bytes = assert_round_trip(&bounds, max_vertices, layout);
    assert_eq!(bytes.len(), 18_000_076);
    assert_eq!(bytes, oracle_bytes(&bounds, max_vertices, &OracleTriangles::W32(&max_faces)));
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_006_width16_holds_at_most_65536_vertices() {
    let bounds = unit_bounds();
    let at_capacity = points(WIDTH16_CAPACITY);
    let faces = [Triangle16::new(u16::MAX, 0, u16::MAX)];
    assert_round_trip(&bounds, &at_capacity, IndexBufferLayout::Width16(&faces));

    let over = points(WIDTH16_CAPACITY + 1);
    assert_eq!(
        encode_error(&bounds, &over, IndexBufferLayout::Width16(&faces)),
        CompressedMeshEncodeError::Width16VertexCapacity { vertex_count: WIDTH16_CAPACITY + 1 }
    );
    assert_eq!(
        encoded_compressed_mesh_len(&over, IndexBufferLayout::Width16(&faces)),
        Err(CompressedMeshEncodeError::Width16VertexCapacity { vertex_count: WIDTH16_CAPACITY + 1 })
    );
    let wide = [Triangle32::new(65_536, 0, 1)];
    assert_round_trip(&bounds, &over, IndexBufferLayout::Width32(&wide));
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_007_indices_below_the_vertex_count_are_accepted_and_the_first_other_refused() {
    let bounds = unit_bounds();
    let vertices = points(5);
    assert_round_trip(&bounds, &vertices, IndexBufferLayout::Width16(&[Triangle16::new(4, 4, 4)]));
    assert_round_trip(&bounds, &vertices, IndexBufferLayout::Width32(&[Triangle32::new(4, 0, 4)]));
    for corner in 0..3 {
        let mut idx = [1_u32, 2, 3];
        idx[corner] = 5;
        let wide = [Triangle32::new(0, 1, 2), Triangle32::new(idx[0], idx[1], idx[2])];
        assert_eq!(
            encode_error(&bounds, &vertices, IndexBufferLayout::Width32(&wide)),
            CompressedMeshEncodeError::IndexOutOfBounds { triangle: 1, index: 5 }
        );
        let narrow = [Triangle16::new(0, 1, 2), Triangle16::new(idx[0] as u16, idx[1] as u16, idx[2] as u16)];
        assert_eq!(
            encode_error(&bounds, &vertices, IndexBufferLayout::Width16(&narrow)),
            CompressedMeshEncodeError::IndexOutOfBounds { triangle: 1, index: 5 }
        );
    }
    // The first offending triangle wins, and within it the first offending corner.
    let wide = [Triangle32::new(0, 0, 0), Triangle32::new(0, 9, u32::MAX), Triangle32::new(7, 0, 0)];
    assert_eq!(
        encode_error(&bounds, &vertices, IndexBufferLayout::Width32(&wide)),
        CompressedMeshEncodeError::IndexOutOfBounds { triangle: 1, index: 9 }
    );
    let wide = [Triangle32::new(u32::MAX, 0, 0)];
    assert_eq!(
        encode_error(&bounds, &vertices, IndexBufferLayout::Width32(&wide)),
        CompressedMeshEncodeError::IndexOutOfBounds { triangle: 0, index: u32::MAX }
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_008_bounds_inverted_on_any_axis_are_refused_and_flat_or_extreme_accepted() {
    let vertices = points(1);
    let faces = [Triangle16::new(0, 0, 0)];
    for axis in 0..3 {
        let mut min = [5_i64, 5, 5];
        min[axis] = 6;
        let inverted = FixedAabb3::new(vec3(min[0], min[1], min[2]), vec3(5, 5, 5));
        assert_eq!(
            encode_error(&inverted, &vertices, IndexBufferLayout::Width16(&faces)),
            CompressedMeshEncodeError::InvalidBounds
        );
    }
    let flat = FixedAabb3::new(vec3(5, 5, 5), vec3(5, 5, 5));
    assert_round_trip(&flat, &vertices, IndexBufferLayout::Width16(&faces));
    let whole = FixedAabb3::new(vec3(i64::MIN, i64::MIN, i64::MIN), vec3(i64::MAX, i64::MAX, i64::MAX));
    assert_round_trip(&whole, &vertices, IndexBufferLayout::Width16(&faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_009_refusals_are_checked_in_the_documented_order() {
    let inverted = FixedAabb3::new(vec3(1, 0, 0), vec3(0, 0, 0));
    let bad_face = [Triangle16::new(9, 9, 9)];
    // Vertex count before triangle count.
    assert_eq!(
        encode_error(&inverted, &[], IndexBufferLayout::Width16(&[])),
        CompressedMeshEncodeError::InvalidVertexCount { count: 0 }
    );
    // Triangle count before the width-16 capacity.
    let over = points(WIDTH16_CAPACITY + 1);
    assert_eq!(
        encode_error(&inverted, &over, IndexBufferLayout::Width16(&[])),
        CompressedMeshEncodeError::InvalidTriangleCount { count: 0 }
    );
    // The width-16 capacity before the bounds.
    assert_eq!(
        encode_error(&inverted, &over, IndexBufferLayout::Width16(&bad_face)),
        CompressedMeshEncodeError::Width16VertexCapacity { vertex_count: WIDTH16_CAPACITY + 1 }
    );
    // The bounds before the indices.
    assert_eq!(
        encode_error(&inverted, &points(3), IndexBufferLayout::Width16(&bad_face)),
        CompressedMeshEncodeError::InvalidBounds
    );
    // The indices before the output size.
    let mut empty: [u8; 0] = [];
    assert_eq!(
        encode_compressed_mesh_into(&unit_bounds(), &points(3), IndexBufferLayout::Width16(&bad_face), &mut empty),
        Err(CompressedMeshEncodeError::IndexOutOfBounds { triangle: 0, index: 9 })
    );
}

// ---------------------------------------------------------------------------------------------
// CMESH-TP-010: the caller-owned output buffer.
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_010_the_output_buffer_must_hold_the_stream_and_its_tail_is_untouched() {
    let faces = [Triangle16::new(0, 1, 2)];
    let layout = IndexBufferLayout::Width16(&faces);
    let vertices = fixture_vertices();
    let fixture = parse_hex(CANONICAL_HEX);
    let required = fixture.len();
    for provided in [0, 1, 75, 76, required - 1] {
        let mut out = vec![0xA5_u8; provided];
        assert_eq!(
            encode_compressed_mesh_into(&unit_bounds(), &vertices, layout, &mut out),
            Err(CompressedMeshEncodeError::OutputTooSmall { required, provided })
        );
        assert!(out.iter().all(|byte| *byte == 0xA5), "a short buffer is not written");
    }
    for extra in [0, 1, 64] {
        let mut out = vec![0xA5_u8; required + extra];
        assert_eq!(encode_compressed_mesh_into(&unit_bounds(), &vertices, layout, &mut out), Ok(required));
        assert_eq!(out[..required], fixture[..]);
        assert!(out[required..].iter().all(|byte| *byte == 0xA5), "the tail is untouched");
    }
    // Earlier contents do not matter.
    let mut out = vec![0xFF_u8; required];
    encode_compressed_mesh_into(&unit_bounds(), &vertices, layout, &mut out).unwrap();
    assert_eq!(out, fixture);
}

#[wasm_bindgen_test(unsupported = test)]
fn cmesh_tp_011_errors_display_their_values() {
    let cases = [
        (CompressedMeshEncodeError::InvalidVertexCount { count: 0 }, "0"),
        (CompressedMeshEncodeError::InvalidTriangleCount { count: 1_000_001 }, "1000001"),
        (CompressedMeshEncodeError::Width16VertexCapacity { vertex_count: 65_537 }, "65537"),
        (CompressedMeshEncodeError::IndexOutOfBounds { triangle: 3, index: 9 }, "9"),
        (CompressedMeshEncodeError::OutputTooSmall { required: 100, provided: 99 }, "99"),
    ];
    for (error, needle) in cases {
        let text = error.to_string();
        assert!(text.contains(needle), "{text:?} names {needle}");
    }
    assert!(!CompressedMeshEncodeError::InvalidBounds.to_string().is_empty());
    assert!(!CompressedMeshEncodeError::LengthOverflow.to_string().is_empty());
}
