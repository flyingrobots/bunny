# Receipt: Compact Node Layout Spike

Task:
Implement flyingrobots/bunny#215: a spike on a BVH node layout that stores both child boxes
in the parent. Measure first how much of a box test is memory access on the benchmark scene,
then build the layout beside the existing one, compare it with `closest_hit` within one run,
and record the outcome in the performance chapter either way.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/src/bvh/build.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-query/src/trace.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs

Files edited:
crates/bunny-broadphase/src/compact.rs
crates/bunny-broadphase/src/compact_hit.rs
crates/bunny-broadphase/src/lib.rs
crates/bunny-broadphase/tests/compact_tests.rs
crates/bunny-broadphase/README.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/memory.rs
crates/bunny-perf/src/compact.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/scene.rs
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-compact-node-layout.md

Topic docs:
docs/topics/performance/README.md (memory share of a box test, ledger entry 8)
docs/topics/performance/test-plan.md (PERF-REQ-008, PERF-TP-042..046)
docs/topics/ray-tracing/README.md (Compact Layout)
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-028, TRACE-TP-301..307, mutation checks)

Generated artifacts:
None.

Checks run:
See the pull request description; every build, test and timing ran in the rust-dev
container through devrun.

Known risks:
One synthetic scene; the memory share is estimated from replays, not measured inside the
shipped traversal. The compact search repeats closest_hit's loop, so the two must be kept
in step; call-for-call tests guard that.

Human reviewer:
James Ross (maintainer); review pending.
