# Receipt: Cheaper Box Entry

Task:
flyingrobots/bunny#258: make `TraceRay::box_entry` cheaper as an exact replacement (the same
`Option<FixedQ32_32>` for every ray, box and limit), keep the previous implementation as a
reference oracle in the tests, measure each change, and record the outcome in the
performance ledger.

Files read:
CODE_STANDARDS.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/trace.rs
crates/bunny-query/tests/trace_tests.rs
crates/bunny-query/README.md
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/cases.rs
xtask/src/code_dojo.rs
xtask/src/repo_respect.rs

Files edited:
crates/bunny-query/src/trace.rs
crates/bunny-query/tests/box_entry_equivalence_tests.rs (new)
crates/bunny-query/tests/support/box_entry_reference.rs (new)
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/profile.rs
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-cheaper-box-entry.md

Topic docs:
docs/topics/ray-tracing/README.md (how the box test computes its unchanged bounds:
i64 differences with bitwise-not flipping, parallel axes in the same path, straight-line
code)
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-028, TRACE-TP-301..305, mutation checks)
docs/topics/performance/README.md (ledger entry 8, including the two rejected steps: a
single 128-bit exit comparison and precomputed TraceRay fields)

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo fmt --all (applied from the container as a patch), cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p bunny-query (equivalence, trace and all other suites)
cargo test --locked -p bunny-perf (TRACE-TP-304 and the replica tests)
14 deliberate mutations of the new box test in a container copy (test plan, Mutation
Checks); each failed at least one case.
cargo run --locked -p xtask -- perf --base <rev> --rounds 11 --align-code [--filter trace/],
with --exclusive, once per step and twice for the final build (ledger entry 8).
cargo test --locked --workspace
cargo run --locked -p xtask -- code-dojo --all

Known risks:
x86-64 and wasm32 codegen of the straight-line version are not measured locally; the CI
perf job gives the x86-64 reading. TraceRay is unchanged. The replayed triangle-kernel cases read slower in full-table
runs only (see ledger entry 8); the function they time is unchanged.

Human reviewer:
James Ross (maintainer); review pending.
