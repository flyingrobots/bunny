# Receipt: Closest-Hit And Any-Hit BVH Traversal

Task:
Implement the Lumen goal slice of flyingrobots/bunny#123: `bunny_broadphase::{closest_hit,
any_hit, ClosestHit}` (plus `BvhView`, which keeps both functions at four parameters),
front-to-back traversal over existing `BvhNode` trees that prunes subtrees beyond the best
hit, ties to the lowest primitive index, with the ray-tracing chapter, test plan, mutation
evidence, CHANGELOG, docs index and crate README. The benchmark and ledger entry (#210), a
new tree builder (#213) and closest-point queries are out of scope; no existing function
changes.

Files read:
CODE_STANDARDS.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
crates/bunny-query/src/trace.rs
crates/bunny-broadphase/Cargo.toml
crates/bunny-broadphase/README.md
crates/bunny-broadphase/src/lib.rs
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/src/bvh/build.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/src/utils.rs
.repo-respect/receipts/2026-11-13-trace-kernels.md

Files edited:
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/lib.rs
crates/bunny-broadphase/tests/closest_hit_tests.rs
crates/bunny-broadphase/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-closest-hit-traversal.md

Topic docs:
docs/topics/ray-tracing/README.md (new Closest-Hit Traversal section: API, search order and
pruning, the tree-independence argument and what the caller must guarantee)
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-021..016, TRACE-TP-101..110, traversal
mutation checks)

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo fmt --all (applied from the container as a patch)
cargo clippy --locked -p bunny-broadphase --all-targets -- -D warnings
cargo test --locked -p bunny-broadphase --test closest_hit_tests (9 passed)
cargo test --locked -p bunny-broadphase --doc
Deliberate mutations, each in a container copy (see the test plan's Mutation Checks):
non-strict skip comparison, tie-break dropped, children pushed in the wrong order, any_hit
stop removed. Each failed at least one case; three of them survived the five draft cases,
which is why TRACE-TP-107..109 were added.
cargo test --locked --workspace
cargo run --locked -p xtask -- code-dojo --all

Known risks:
Tree independence holds only under the caller conditions the chapter states (node bounds
contain primitive boxes; reported distances are at least the floor of a distance inside the
primitive's box; the closure is a function of its inputs). A closure that under-reports can
make the answer depend on tree shape. The traversal is unmeasured until #210.

Human reviewer:
James Ross (maintainer); review pending.
