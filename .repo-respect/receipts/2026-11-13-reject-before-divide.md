# Receipt: Reject Triangle Hits Beyond The Limit Before Dividing

Task:
Implement flyingrobots/bunny#214: a conservative pre-check in `TraceRay::hit_triangle` that
rejects a crossing certainly beyond `t_max` before the long division, as an exact
replacement (no output bit changes), with a retained reference oracle, counts of how often it
fires on the benchmark scene, mutation checks and a ledger entry.

Outcome:
The pre-check was implemented, proven equal to the reference, mutation checked and timed
twice, then withdrawn in a revert commit: on the scene only 24 of 714 crossings are beyond
the limit, so it can save about 0.3 ns per ray while adding about 2 ns per crossing, and
`trace/closest_hit` moved within noise (-1.5 %, -2.0 % best p50). `hit_triangle` on this
branch is byte-identical to `origin/main`. The tests, counting evidence, micro cases and
the ledger entry (performance chapter, entry 5, under Tried And Rejected) remain.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/trace.rs
crates/bunny-query/tests/trace_tests.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/scene.rs

Files edited:
crates/bunny-query/tests/support/trace_reference.rs
crates/bunny-query/tests/trace_equivalence_tests.rs
crates/bunny-query/src/trace.rs (pre-check added, then reverted; no net change)
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/cases.rs
docs/topics/performance/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-reject-before-divide.md

Topic docs:
docs/topics/performance/README.md (Tried And Rejected, entry 5; the new trace/triangle_* cases)
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-027, TRACE-TP-201..205, mutation note)
docs/topics/ray-tracing/README.md unchanged: the kernel's contract and code did not change.

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host.
cargo test --locked -p bunny-perf --release count_the (before any kernel change):
calls 1673, divided 714, beyond_limit 24, pre_checked 24.
cargo fmt --all; cargo clippy --locked --workspace --all-targets -- -D warnings;
cargo test --locked -p bunny-query -p bunny-perf -p bunny-broadphase (with the pre-check).
Deliberate mutations of the pre-check, each failing at least one case: t_max for
t_max + 1; floor for ceil; t + 1 >= bound; ceil with 2^32 - 2. The last two survived until
TRACE-TP-204 was added.
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code, twice,
with devrun --exclusive, base 42279ed2825f (tables in the performance chapter).
Final state: fmt check, clippy, cargo test --locked --workspace and
cargo run --locked -p xtask -- code-dojo --all (see the pull request).

Known risks:
The equivalence tests compare the kernel with a copy of itself today; they become a real
oracle the moment the kernel is optimized. The count test locks the scene's numbers, so a
change to the scene, camera or traversal must update the ledger entry with it.

Human reviewer:
James Ross (maintainer); review pending.
