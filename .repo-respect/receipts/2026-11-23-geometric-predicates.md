# Receipt: Exact Geometric Predicates

Task:
flyingrobots/bunny#145, "Math: Add robust fixed geometric predicates", under the contract
identity `bunny-predicates/1` (`PREDICATES_CONTRACT_ID`) in `bunny-geom`: exact 2D and 3D
orientation (the sign of the exact determinant for every representable input), the exact
doubled signed area and sextupled signed volume with separately named forms rounded once to
Q32.32, exact barycentric numerators over a positive denominator for a point and a 2D
triangle (and a point exactly in the plane of a 3D triangle), the exact location
classification with its precedence, closed and open containment, and the weights rounded
once each. Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-unproject-ray.md
crates/bunny-geom/{Cargo.toml,README.md,src/lib.rs,src/plane.rs}
crates/bunny-geom/tests/{determinism.rs,plane_span_sweep_tests.rs}
crates/bunny-linalg/{src/lib.rs,src/fixed_vec2.rs,src/unproject/wide.rs}
crates/bunny-query/src/{lib.rs,barycentric.rs}
crates/bunny-num/tests/lerp_tests.rs
docs/topics/coordinate-law/README.md (canonical 3D frame, canonical 2D plane, viewport)
docs/topics/plane-clipping/{README.md,test-plan.md}
docs/topics/ray-tracing/README.md (exact triangle test)
docs/topics/lerp/{README.md,test-plan.md}
xtask/src/topic_docs.rs
flyingrobots/bunny#145, #424

Files edited:
crates/bunny-geom/src/predicates.rs (new: orientations, exact and rounded area and volume)
crates/bunny-geom/src/predicates/barycentric.rs (new: exact barycentrics and location)
crates/bunny-geom/src/predicates/wide.rs (new: private 256-bit signed integer)
crates/bunny-geom/src/lib.rs (module and exports, appended after the plane exports)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic row in sorted position)
docs/topics/predicates/README.md (new chapter)
docs/topics/predicates/test-plan.md (new: PRED-REQ-001 to -009, PRED-TP-001 to -012)
crates/bunny-geom/tests/predicates_oracle/mod.rs (new: independent big-integer oracle)
crates/bunny-geom/tests/predicate_orientation_tests.rs (new)
crates/bunny-geom/tests/predicate_barycentric_tests.rs (new)
crates/bunny-geom/tests/predicate_sweep_tests.rs (new)
.repo-respect/receipts/2026-11-23-geometric-predicates.md

Topic docs:
docs/topics/predicates/: new topic with its own prefix, PRED-REQ-001 to PRED-REQ-009 and
PRED-TP-001 to PRED-TP-012. No other ledger uses the PRED- prefix.

Generated artifacts:
None.

Checks run:
First commit (tests and planned ledger only): the predicate tests fail to compile (the API
does not exist). cargo fmt --all; topic-docs clean (12 planned cases, zero duplicate IDs).
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned
1.96.0): cargo fmt --all --check; cargo test --locked --workspace --all-targets
--all-features (139 test result lines, none failed); cargo run --locked -p xtask --
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs (zero duplicate IDs, 12 cases implemented); repo-respect
check --branch. cargo build and cargo test -p bunny-geom --no-default-features pass.
RUSTDOCFLAGS="-D warnings" cargo doc -p bunny-geom clean. cargo clippy --locked -p
bunny-geom --all-targets --all-features -D warnings: clean in every added file (the
pre-existing assert_is_empty lint of the newer toolchain remains in the untouched
polygon_clip_tests.rs).
Mutations: twenty-four, each applied to the kernel, run with the three predicate test
files, and restored; each failed at least one case (the test plan's table). The Python model
of the pinned goldens (posted on the pull request) reproduces every value.
Not run locally: wasm-pack (CI's WebAssembly job; no wasm32 standard library here), the
bare-metal no-std gate (needs rustup), cargo deny and markdownlint-cli2 (CI's Dojo job), the
pinned 1.96.0 clippy.

Known risks:
The exact types are a new public surface (ExactDeterminant, ExactBarycentric) holding a
256-bit value; if flyingrobots/bunny#424 settles a shared wide integer, the private helper
behind them can move without changing these types' API. barycentric3_coplanar accepts only
points exactly in the plane; most points of a general 3D triangle's plane are not on the raw
grid and are refused.

Human reviewer:
James Ross (maintainer); review pending.
