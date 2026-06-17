//! Integration tests.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Mutex;

use bunny_codec::{decode_compressed_mesh, parse_binary_ply, parse_obj_text, ObjError};

const HEADER: &str = concat!(
    "ply\n",
    "format binary_little_endian 1.0\n",
    "element vertex 3\n",
    "property float x\n",
    "property float y\n",
    "property float z\n",
    "element face 1\n",
    "property list uchar int vertex_indices\n",
    "end_header\n",
);
const VERTEX_BYTES: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    128, 63, 0, 0, 0, 0,
];
const FACE_BYTES: &[u8] = &[3, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0];
const OBJ_TRIANGLE: &str = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
";
const COMPRESSED_TRIANGLE_HEX: &str =
    include_str!("fixtures/canonical_compressed_triangle.bunny.hex");

static MEASUREMENT_LOCK: Mutex<()> = Mutex::new(());

#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

fn canonical_triangle_ply() -> Vec<u8> {
    let mut bytes = Vec::from(HEADER.as_bytes());
    bytes.extend_from_slice(VERTEX_BYTES);
    bytes.extend_from_slice(FACE_BYTES);
    bytes
}

fn canonical_compressed_triangle() -> Vec<u8> {
    parse_hex(COMPRESSED_TRIANGLE_HEX)
}

fn parse_hex(input: &str) -> Vec<u8> {
    let nybbles: Vec<_> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    assert_eq!(nybbles.len() % 2, 0, "hex fixture must contain byte pairs");
    nybbles.chunks_exact(2).map(|pair| (hex_value(pair[0]) << 4) | hex_value(pair[1])).collect()
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("fixture contains non-hex byte"),
    }
}

fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, dhat::HeapStats) {
    let _measurement =
        MEASUREMENT_LOCK.lock().expect("allocation measurement lock should not be poisoned");
    let _profiler = dhat::Profiler::builder().testing().build();
    let result = operation();
    let stats = dhat::HeapStats::get();
    (result, stats)
}

#[test]
fn parsers_allocate_zero_times_after_warm_up() {
    let bytes = canonical_triangle_ply();
    let compressed_bytes = canonical_compressed_triangle();
    parse_binary_ply(&bytes).expect("warm-up binary PLY should parse");
    parse_obj_text(OBJ_TRIANGLE).expect("warm-up OBJ should parse");
    decode_compressed_mesh(&compressed_bytes).expect("warm-up compressed mesh should parse");

    let (ply_mesh, ply_stats) = allocations_during(|| parse_binary_ply(&bytes));

    assert_eq!(ply_stats.total_blocks, 0, "binary PLY parser allocated after warm-up");
    assert_eq!(ply_mesh.expect("canonical binary PLY should parse").face_count(), 1);

    let (obj_counts, obj_stats) = allocations_during(|| {
        let mesh = parse_obj_text(OBJ_TRIANGLE)?;
        let vertices = mesh.vertices().try_fold(0, count_obj_record)?;
        let triangles = mesh.triangles().try_fold(0, count_obj_record)?;
        Ok::<_, ObjError>((vertices, triangles))
    });

    assert_eq!(obj_stats.total_blocks, 0, "OBJ parser allocated after warm-up");
    assert_eq!(obj_counts.expect("canonical OBJ should parse"), (3, 1));

    let (compressed_mesh, compressed_stats) =
        allocations_during(|| decode_compressed_mesh(&compressed_bytes));

    assert_eq!(compressed_stats.total_blocks, 0, "compressed mesh decoder allocated after warm-up");
    assert_eq!(
        compressed_mesh.expect("canonical compressed mesh should parse").triangle_count(),
        1
    );
}

fn count_obj_record<T>(count: usize, record: Result<T, ObjError>) -> Result<usize, ObjError> {
    record.map(|_| count + 1)
}
