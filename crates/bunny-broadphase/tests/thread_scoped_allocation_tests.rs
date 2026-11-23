//! The allocation count of a measured scope belongs to the measuring thread alone.

#![cfg(not(target_arch = "wasm32"))]

use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

/// Blocks allocated by the calling thread while `operation` runs.
fn blocks_during(operation: impl FnOnce()) -> u64 {
    let _profiler = dhat::Profiler::builder().testing().build();
    operation();
    dhat::HeapStats::get().total_blocks
}

#[test]
fn tscope_tp_001_foreign_thread_allocations_are_not_counted() {
    let go = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let foreign_blocks = Arc::new(AtomicU64::new(0));
    let foreign = {
        let (go, stop, foreign_blocks) = (go.clone(), stop.clone(), foreign_blocks.clone());
        thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                if go.load(Ordering::Acquire) {
                    drop(black_box(Box::new([0_u64; 8])));
                    foreign_blocks.fetch_add(1, Ordering::AcqRel);
                } else {
                    thread::yield_now();
                }
            }
        })
    };

    let measured = blocks_during(|| {
        go.store(true, Ordering::Release);
        // The measurement window stays open until the other thread has certainly allocated in it.
        while foreign_blocks.load(Ordering::Acquire) < 1000 {
            thread::yield_now();
        }
    });
    stop.store(true, Ordering::Release);
    foreign.join().expect("foreign thread");

    assert!(foreign_blocks.load(Ordering::Acquire) >= 1000);
    assert_eq!(measured, 0, "an allocation by another thread was counted");
}

#[test]
fn tscope_tp_002_own_thread_allocations_are_counted() {
    let measured = blocks_during(|| {
        drop(black_box(Vec::<u64>::with_capacity(16)));
    });
    assert_eq!(measured, 1);
}

#[test]
fn tscope_tp_003_a_reallocation_is_counted_on_the_measuring_thread() {
    let measured = blocks_during(|| {
        let mut v: Vec<u64> = Vec::with_capacity(1);
        for i in 0..64 {
            v.push(black_box(i));
        }
        drop(black_box(v));
    });
    // Capacity 1, then growth to 4, 8, 16, 32 and 64: one allocation and five growths.
    assert_eq!(measured, 6);
}

#[test]
fn tscope_tp_004_a_block_freed_on_another_thread_does_not_change_the_count() {
    let block = Box::new([7_u64; 8]);
    let measured = blocks_during(|| {
        let handle = thread::spawn(move || drop(black_box(block)));
        handle.join().expect("freeing thread");
    });
    // `thread::spawn` allocates on the measuring thread; the free on the other thread adds nothing.
    let spawn_only = blocks_during(|| {
        thread::spawn(|| ()).join().expect("empty thread");
    });
    assert_eq!(measured, spawn_only);
}

#[test]
fn tscope_tp_005_two_measuring_threads_do_not_see_each_other() {
    let barrier = Arc::new(Barrier::new(2));
    let run = |blocks: usize| {
        let barrier = barrier.clone();
        thread::spawn(move || {
            blocks_during(|| {
                // Both scopes are open here, so each thread allocates inside the other's window.
                barrier.wait();
                let kept: Vec<Box<u64>> =
                    (0..blocks as u64).map(|i| Box::new(black_box(i))).collect();
                barrier.wait();
                drop(black_box(kept));
            })
        })
    };
    let (a, b) = (run(10), run(25));
    // Each count is its own boxes plus the one vector holding them.
    assert_eq!(a.join().expect("thread a"), 10 + 1);
    assert_eq!(b.join().expect("thread b"), 25 + 1);
}
