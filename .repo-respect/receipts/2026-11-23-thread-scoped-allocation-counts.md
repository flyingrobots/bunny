# Receipt: Thread-Scoped Allocation Counts

Task:
Fix `flyingrobots/bunny#439`: the zero-allocation tests count every allocation in the process, so an allocation by the libtest harness or another thread fails them although the kernel allocated nothing. First commit: the failing tests only.

Files read:
- `crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs`
- `crates/bunny-broadphase/tests/bvh_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_ascii_allocation_tests.rs`
- `CODE_STANDARDS.md`

Files edited:
- `crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs`
- `.repo-respect/receipts/2026-11-23-thread-scoped-allocation-counts.md`

Topic docs:
None yet.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-broadphase --test thread_scoped_allocation_tests` (RED: with `dhat`, the foreign-thread test counted 1334 blocks, and the others fail because `dhat` allows one profiler at a time).

Known risks:
- Work in progress.

Human reviewer:
@flyingrobots
