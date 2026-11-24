# Receipt: Exact Separating-Axis Tests (first slice)

Task:
flyingrobots/bunny#147, "Geom: Add SAT-based convex collision helpers", first slice, under
the contract identity `bunny-sat/1` (`SAT_CONTRACT_ID`) in `bunny-geom`: exact
separating-axis classification of two oriented boxes (`sat_obb_obb`) and of a triangle
against an axis-aligned box (`sat_triangle_aabb`) into `Separated`, `Touching` and
`Overlapping`, each with a deterministic witness axis in a documented order. No floats, no
normalised axes, no rounding, no overflow. Additive: no existing public item, output bit,
golden, digest or ID changes. The minimum translation axis is not in this slice.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-geometric-predicates.md
crates/bunny-geom/{Cargo.toml,README.md,src/lib.rs,src/predicates.rs,src/predicates/wide.rs}
crates/bunny-geom/src/shapes.rs, src/shapes/{obb.rs,triangle.rs}, src/plane/span.rs,
src/frustum.rs
crates/bunny-geom/tests/{predicates_oracle/mod.rs,predicate_sweep_tests.rs,
shape_obb_tests.rs}
crates/bunny-broadphase/src/utils.rs (closed AABB overlap convention)
crates/bunny-linalg/src/{fixed_mat3.rs,fixed_mat3_rotation.rs}
docs/README.md, docs/topics/predicates/{README.md,test-plan.md}
flyingrobots/bunny#147, #149, #424

Files edited:
crates/bunny-geom/src/sat.rs (new: the two kernels, axis names, classification)
crates/bunny-geom/src/sat/wide.rs (new: private 256-bit two's complement integer)
crates/bunny-geom/src/lib.rs (one module line and one export line, appended)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic row in sorted position)
docs/topics/convex-collision/README.md (new chapter)
docs/topics/convex-collision/test-plan.md (new: SAT-REQ-001 to -005, SAT-TP-001 to -013)
crates/bunny-geom/tests/sat_oracle/mod.rs (new: corner-projection and Minkowski-hull
oracles)
crates/bunny-geom/tests/sat_tests.rs (new: constructed cases)
crates/bunny-geom/tests/sat_sweep_tests.rs (new: seeded sweeps)
.repo-respect/receipts/2026-11-23-sat-convex.md

Topic docs:
docs/topics/convex-collision/: new topic with its own prefix, SAT-REQ-001 to SAT-REQ-005
and SAT-TP-001 to SAT-TP-013. No other ledger uses the SAT- prefix.

Generated artifacts:
None.

Checks run:
First commit (tests only): the new tests fail to compile, as the API does not exist yet.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned
1.96.0): cargo fmt --all --check; cargo test --locked --workspace --all-targets
--all-features (155 test result lines, none failed); cargo run --locked -p xtask --
code-dojo-generated; code-dojo-rust --all (clean after replacing indexing with destructuring
and splitting one function); code-dojo-determinism --enforce; code-dojo-perf-groups;
topic-docs (zero duplicate IDs, 13 cases implemented); repo-respect check --branch. cargo
build and cargo test -p bunny-geom --no-default-features pass (22 result lines).
RUSTDOCFLAGS="-D warnings" cargo doc -p bunny-geom clean. cargo clippy -p bunny-geom on the
library and the two new test targets with -D warnings: clean (the pre-existing
assert_is_empty lint of the newer toolchain remains in the untouched polygon_clip_tests.rs).
Mutations: twenty-two, each applied to the kernel, run with the two new test files, and
restored; each failed at least one case (the test plan's table). Column-major edge order
first survived; a searched configuration was added to SAT-TP-003.
Not run locally: wasm-pack (CI's WebAssembly job), the bare-metal no-std gate (needs
rustup), cargo deny and markdownlint-cli2 (CI's Dojo job), the pinned 1.96.0 clippy.

Known risks:
One more private wide integer in the workspace (flyingrobots/bunny#424 tracks sharing one).
The constructed OBB cases at unit scale run the brute-force hull oracle in unbounded
integers, which is slow in debug builds (about 25 seconds for each of the two new test files).

Human reviewer:
James Ross (maintainer); review pending.
