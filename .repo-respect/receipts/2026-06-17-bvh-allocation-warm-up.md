# Receipt: BVH Allocation Test Warm-Up

Task:
Resolve the PR #106 Ubuntu determinism failure where the BVH allocation test
counted one first-run allocation before measuring the intended zero-allocation
build and traversal paths.

Files read:

* `.github/workflows/code-dojo.yml`
* `crates/bunny-broadphase/src/bvh/build.rs`
* `crates/bunny-broadphase/src/bvh/split.rs`
* `crates/bunny-broadphase/src/traversal.rs`
* `crates/bunny-broadphase/tests/bvh_allocation_tests.rs`
* `crates/bunny-codec/tests/ply_allocation_tests.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-bvh-allocation-warm-up.md`
* `crates/bunny-broadphase/tests/bvh_allocation_tests.rs`

Checks run:

* `cargo test --locked -p bunny-broadphase --test bvh_allocation_tests --all-features`
* `cargo run --locked -p xtask -- code-dojo-rust --all`
* `cargo run --locked -p xtask -- code-dojo --all`

Known risks:

* The failure was observed only on the fresh Ubuntu GitHub Actions
  determinism job. The fix keeps the zero-allocation assertions unchanged and
  mirrors the existing codec allocation-test warm-up pattern.
* Production BVH code was not changed.

Human reviewer:
James
