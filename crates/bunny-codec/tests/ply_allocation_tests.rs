#![cfg(not(target_arch = "wasm32"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bunny_codec::{parse_binary_ply, parse_obj_text};

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

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TRACKING: AtomicBool = AtomicBool::new(false);

struct CountingAllocator;
struct AllocationTrackingGuard;

impl Drop for AllocationTrackingGuard {
    fn drop(&mut self) {
        TRACKING.store(false, Ordering::SeqCst);
    }
}

// SAFETY: This test allocator forwards all allocation and deallocation requests
// to the standard system allocator while counting allocation calls.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::SeqCst) {
            ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        }
        // SAFETY: The layout is forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout were produced by the system allocator.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn canonical_triangle_ply() -> Vec<u8> {
    let mut bytes = Vec::from(HEADER.as_bytes());
    bytes.extend_from_slice(VERTEX_BYTES);
    bytes.extend_from_slice(FACE_BYTES);
    bytes
}

fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);
    let guard = AllocationTrackingGuard;
    let result = operation();
    drop(guard);
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    (result, allocations)
}

#[test]
fn binary_ply_parse_allocates_zero_times() {
    let bytes = canonical_triangle_ply();
    parse_binary_ply(&bytes).expect("warm-up binary PLY should parse");

    let (mesh, allocations) = allocations_during(|| parse_binary_ply(&bytes));

    assert_eq!(allocations, 0, "binary PLY parser allocated after warm-up");
    assert_eq!(
        mesh.expect("canonical binary PLY should parse")
            .face_count(),
        1
    );
}

#[test]
fn obj_text_parse_allocates_zero_times() {
    parse_obj_text(OBJ_TRIANGLE).expect("warm-up OBJ should parse");

    let (mesh, allocations) = allocations_during(|| parse_obj_text(OBJ_TRIANGLE));

    assert_eq!(allocations, 0, "OBJ parser allocated after warm-up");
    assert_eq!(mesh.expect("canonical OBJ should parse").face_count(), 1);
}
