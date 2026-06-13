#![cfg(not(target_arch = "wasm32"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bunny_broadphase::{build_bvh, intersect_aabb, intersect_ray, BvhNode};
use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TRACKING: AtomicBool = AtomicBool::new(false);

struct CountingAllocator;

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

fn allocations_during<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);
    let result = operation();
    TRACKING.store(false, Ordering::SeqCst);
    (result, ALLOCATIONS.load(Ordering::SeqCst))
}

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * bunny_num::fixed_q32_32::ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn aabb(min_x: i32, max_x: i32) -> FixedAabb3 {
    FixedAabb3::new(vec3(min_x, -1, -1), vec3(max_x, 1, 1))
}

fn empty_node() -> BvhNode {
    BvhNode {
        bounds: FixedAabb3::new(vec3(0, 0, 0), vec3(0, 0, 0)),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }
}

#[test]
fn build_bvh_and_traversal_allocate_zero_times() {
    let primitives = [aabb(0, 1), aabb(3, 4), aabb(6, 7), aabb(9, 10)];
    let mut nodes = [empty_node(); 7];
    let mut primitive_indices = [0_u32; 4];

    let (node_count, allocations) =
        allocations_during(|| build_bvh(&mut nodes, &mut primitive_indices, &primitives));

    assert_eq!(allocations, 0);
    let node_count = node_count.expect("build should succeed");
    assert!(node_count > 0);

    let query_box = FixedAabb3::new(vec3(-1, -2, -2), vec3(11, 2, 2));
    let mut overlap_hits = [u32::MAX; 4];
    let ((overlap_result, overlap_count), overlap_allocations) = allocations_during(|| {
        let mut count = 0;
        let result = intersect_aabb(
            &nodes[..node_count],
            &primitive_indices,
            &query_box,
            |idx| {
                overlap_hits[count] = idx;
                count += 1;
            },
        );
        (result, count)
    });
    assert_eq!(overlap_allocations, 0);
    assert_eq!(overlap_result, Ok(()));
    assert_eq!(overlap_count, 4);

    let ray = FixedRay3::new(vec3(-1, 0, 0), FixedUnitVec3::UNIT_X);
    let mut ray_hits = [u32::MAX; 4];
    let ((ray_result, ray_count), ray_allocations) = allocations_during(|| {
        let mut count = 0;
        let result = intersect_ray(&nodes[..node_count], &primitive_indices, &ray, |idx| {
            ray_hits[count] = idx;
            count += 1;
        });
        (result, count)
    });
    assert_eq!(ray_allocations, 0);
    assert_eq!(ray_result, Ok(()));
    assert_eq!(ray_count, 4);
}
