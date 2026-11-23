# Receipt: Thread-Scoped Allocation Counts

Task:
Fix `flyingrobots/bunny#439`: the zero-allocation tests count every allocation in the process, so an allocation by the libtest harness or another thread fails them although the kernel allocated nothing. The four tests now count with `allocation-counter` (thread-scoped, no `unsafe` in this repository; it is a dev-dependency like `dhat` was) instead of `dhat`.

Files read:
- `crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs`
- `crates/bunny-broadphase/tests/bvh_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_ascii_allocation_tests.rs`
- `CODE_STANDARDS.md`

Files edited:
- `crates/bunny-broadphase/tests/thread_scoped_allocation_tests.rs`
- `crates/bunny-broadphase/tests/bvh_allocation_tests.rs`
- `crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_allocation_tests.rs`
- `crates/bunny-codec/tests/ply_ascii_allocation_tests.rs`
- `crates/bunny-broadphase/Cargo.toml`
- `crates/bunny-codec/Cargo.toml`
- `Cargo.lock`
- `docs/topics/bvh-construction/test-plan.md`
- `docs/topics/ascii-ply/test-plan.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-thread-scoped-allocation-counts.md`

Topic docs:
- `docs/topics/bvh-construction/test-plan.md`: measurement rule section, BVH-TP-027 and BVH-TP-028, BVH-TP-021 wording.
- `docs/topics/ascii-ply/test-plan.md`: APLY-TP-040 names the measurement rule.

Generated artifacts:
- `Cargo.lock`: `dhat` and its dependencies removed, `allocation-counter` 0.8.1 (no dependencies) added.

Checks run:
- `cargo test --locked -p bunny-broadphase --test thread_scoped_allocation_tests` (RED: with `dhat`, the foreign-thread test counted 1334 blocks, and the others fail because `dhat` allows one profiler at a time).

- After the change: `cargo test --locked --workspace --all-targets --all-features` (105 test binaries ok, no failure), `cargo fmt --all --check`, `code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`.
- The wasm32 check was not run locally (target not installed); CI's WebAssembly job runs it.

Known risks:
- `allocation-counter` installs its own `#[global_allocator]` in its library, so a test binary using it cannot install another.
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
