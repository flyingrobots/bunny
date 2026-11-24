# Receipt: Uniform Grid Broadphase

Task:
flyingrobots/bunny#121: a deterministic uniform grid broadphase in `bunny-broadphase`
(`uniform_grid_pairs`, contract identity `bunny-uniform-grid/1`). Cells are identified by their
exact integer coordinates `floor(raw / cell_raw)`; candidates come from sorting (cell, object)
keys in caller scratch, not from hashing. Pairs have the form and order of `sweep_and_prune`.
Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, docs/CODE_DOJO.md (limits)
.repo-respect/receipts/2026-11-23-unproject-ray.md
crates/bunny-broadphase/{Cargo.toml,README.md,src/lib.rs,src/sweep_and_prune.rs,src/utils.rs}
crates/bunny-broadphase/tests/{determinism.rs,bvh_sah_allocation_tests.rs,common/mod.rs}
crates/bunny-geom/src/{lib.rs,bounds.rs} (FixedAabb3::new/try_new, BoundsError::InvertedBox)
docs/topics/bvh-construction/{README.md,test-plan.md}, docs/README.md
flyingrobots/bunny#485 (crates/bunny-mesh/src/topology/: the caller-scratch model)
xtask/src/{code_dojo.rs,topic_docs.rs} (limits, case statuses)

Files edited:
crates/bunny-broadphase/src/uniform_grid.rs (new: API, contract ID, the call)
crates/bunny-broadphase/src/uniform_grid/cell.rs (new: GridCellSize, floor quantization, cell ranges)
crates/bunny-broadphase/src/uniform_grid/key.rs (new: GridKey, grid_key_count, key fill and sort)
crates/bunny-broadphase/src/uniform_grid/pairs.rs (new: runs, first-shared-cell rule, report, sink)
crates/bunny-broadphase/src/uniform_grid/error.rs (new: UniformGridError)
crates/bunny-broadphase/src/lib.rs (module and exports, appended)
crates/bunny-broadphase/tests/uniform_grid_tests.rs (new)
crates/bunny-broadphase/tests/uniform_grid_allocation_tests.rs (new)
crates/bunny-broadphase/README.md, CHANGELOG.md, docs/README.md (topic row, sorted position)
docs/topics/uniform-grid/README.md (new chapter)
docs/topics/uniform-grid/test-plan.md (new: GRID-REQ-001 to -009, GRID-TP-001 to -015)
.repo-respect/receipts/2026-11-24-uniform-grid-broadphase.md

Topic docs:
docs/topics/uniform-grid/: a new topic with its own prefix GRID-, starting at 001. No other
ledger uses GRID-.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the grid tests fail to compile, because
`bunny_broadphase::uniform_grid` does not exist yet.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all; code-dojo-determinism
--enforce; code-dojo-perf-groups; topic-docs (zero duplicate IDs); repo-respect check --branch.
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p bunny-broadphase clean. cargo clippy --locked
-p bunny-broadphase --all-targets --all-features -- -D warnings clean.
Mutations: ten, each applied to the kernel, run with cargo test --no-fail-fast -p
bunny-broadphase --test uniform_grid_tests --test uniform_grid_allocation_tests, and restored;
each failed at least one case (the test plan's table).
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's Dojo
job), the pinned 1.96.0 clippy.

Known risks:
The candidate tests in one cell are quadratic in the boxes it holds, as an all-pairs scan is;
the cost is reported (`candidate_tests`) and documented, not bounded. A box many cells wide
needs many keys; the count is exact and refused when it does not fit, with no per-box cap.
`TooManyBoxes` is not exercised (it needs more than u32::MAX boxes).

Human reviewer:
James Ross (maintainer); review pending.
