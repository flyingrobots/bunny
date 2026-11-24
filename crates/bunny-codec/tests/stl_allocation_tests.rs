//! Allocation witness for the binary STL reader.

#![cfg(not(target_arch = "wasm32"))]

mod stl_support;

use bunny_codec::{parse_binary_stl, StlError};
use stl_support::{stl, stl_with_count, tetrahedron};

/// Runs `operation` and returns its result with the number of heap blocks the calling thread
/// allocated meanwhile. Allocations by other threads, including the test harness, are not counted.
fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let mut result = None;
    let counts = allocation_counter::measure(|| result = Some(operation()));
    (result.expect("the measured operation should have run"), counts.count_total)
}

#[test]
fn stl_tp_030_binary_stl_reader_allocates_zero_times_after_warm_up() {
    let bytes = stl(&tetrahedron());
    parse_binary_stl(&bytes).expect("warm-up parse");

    let (counts, blocks) = allocations_during(|| {
        let mesh = parse_binary_stl(&bytes)?;
        let vertices = mesh.vertices().try_fold(0_usize, |count, item| item.map(|_| count + 1))?;
        let triangles =
            mesh.triangles().try_fold(0_usize, |count, item| item.map(|_| count + 1))?;
        Ok::<_, StlError>((vertices, triangles))
    });
    assert_eq!(blocks, 0, "binary STL reader allocated after warm-up");
    assert_eq!(counts.expect("tetrahedron should parse"), (12, 4));

    let huge = stl_with_count(&[0; 80], u32::MAX, &tetrahedron());
    let (refusal, blocks) = allocations_during(|| parse_binary_stl(&huge).err());
    assert_eq!(blocks, 0, "a huge declared count allocated");
    assert!(matches!(refusal, Some(StlError::LengthMismatch { .. })));
}
