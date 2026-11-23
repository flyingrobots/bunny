//! Allocation witness for the ASCII PLY reader.

#![cfg(not(target_arch = "wasm32"))]

use bunny_codec::{parse_ascii_ply, AsciiPlyError};

const MESH: &str = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\n\
property float z\nproperty float confidence\nelement face 1\n\
property list uchar int vertex_indices\nend_header\n\
0 0 0 1\n1.5 0 0 1\n0 -2.25e0 0 1\n3 0 1 2\n";

/// Runs `operation` and returns its result with the number of heap blocks the calling thread
/// allocated meanwhile. Allocations by other threads, including the test harness, are not counted.
fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let mut result = None;
    let counts = allocation_counter::measure(|| result = Some(operation()));
    (result.expect("the measured operation should have run"), counts.count_total)
}

#[test]
fn aply_tp_040_ascii_reader_allocates_zero_times_after_warm_up() {
    let bytes = MESH.as_bytes();
    parse_ascii_ply(bytes).expect("warm-up parse");

    let (counts, blocks) = allocations_during(|| {
        let mesh = parse_ascii_ply(bytes)?;
        let vertices = mesh.vertices().try_fold(0_usize, |count, item| item.map(|_| count + 1))?;
        let triangles =
            mesh.triangles().try_fold(0_usize, |count, item| item.map(|_| count + 1))?;
        Ok::<_, AsciiPlyError>((vertices, triangles))
    });

    assert_eq!(blocks, 0, "ASCII PLY reader allocated after warm-up");
    assert_eq!(counts.expect("canonical ASCII PLY should parse"), (3, 1));

    let huge = MESH.replace("vertex 3", "vertex 4294967295");
    let (refusal, blocks) = allocations_during(|| parse_ascii_ply(huge.as_bytes()).err());
    assert_eq!(blocks, 0, "a huge declared count allocated");
    assert!(refusal.is_some());
}
