# Receipt: Ray-Tracing Trace Kernels

Task:
Implement the Lumen goal slice of flyingrobots/bunny#159: the `bunny_query::{TraceRay,
TraceTriangle, TriangleHit}` kernels (a division-free conservative `box_entry` and an
exact integer `hit_triangle`), with the `docs/topics/ray-tracing/` chapter and test plan,
golden vectors, deliberate-mutation evidence, CHANGELOG and docs index entries. BVH
traversal (#123), the benchmark and any performance ledger entry (#210) are out of scope.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
clippy.toml
rustfmt.toml
docs/CODE_DOJO.md
docs/README.md
docs/topics/performance/README.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
docs/topics/coordinate-law/README.md
docs/topics/repo-respect/README.md
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
crates/bunny-query/Cargo.toml
crates/bunny-query/README.md
crates/bunny-query/src/lib.rs
crates/bunny-query/tests/ray_determinism_tests.rs
crates/bunny-geom/src/lib.rs

Files edited:
crates/bunny-query/src/trace.rs
crates/bunny-query/src/lib.rs
crates/bunny-query/tests/trace_tests.rs
crates/bunny-query/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-trace-kernels.md

Topic docs:
docs/topics/ray-tracing/README.md (new chapter: trace domain, conservative box test and
its bound, exact triangle test, edge rule, rounding, facing)
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-001..010, TRACE-TP-001..012, mutation
checks)

Generated artifacts:
None. Golden vectors are hand computed and written into
crates/bunny-query/tests/trace_tests.rs.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo fmt --all (applied from the container as a patch)
cargo clippy --locked -p bunny-query --all-targets -- -D warnings
cargo test --locked -p bunny-query --test trace_tests (11 passed)
Deliberate mutations, each in a container copy (see the test plan's Mutation Checks): slab
lower bound rounded up, slab upper bound rounded down, each of the three edge
comparisons made exclusive, and the `v1` corner made exclusive. Every mutation failed at
least one case; the upper-bound mutation first survived, which is why the corner-graze
golden case was added.
cargo test --locked --workspace
cargo run --locked -p xtask -- code-dojo --all

Known risks:
The kernels are not yet used by a traversal, so the box bound's practical cost (boxes
accepted that the ray narrowly misses) is unmeasured until #123 and #210. The stated box
bound (from the issue) is looser than the derived one; the test checks both. Distances are
floored, so a hit whose true distance is just above `t_max` but floors to it is reported.

Human reviewer:
James Ross (maintainer); review pending.
