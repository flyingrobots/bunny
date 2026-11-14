# Receipt: Traversal Cycle Guard

Task:
Implement flyingrobots/bunny#269: `intersect_aabb`, `intersect_ray`, `closest_hit` and
`any_hit` must not loop forever on a malformed node buffer with a reference cycle. Bound
their interior visits by `nodes.len() / 2`, which no tree can exceed, and return
`TraversalError::InvalidNodeIndex` beyond it; change no result, error or callback on a tree.

Files read:
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/src/compact.rs
crates/bunny-broadphase/src/compact_hit.rs
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/tests/closest_hit_tests.rs
crates/bunny-broadphase/tests/compact_tests.rs
crates/bunny-broadphase/tests/bvh_tests.rs
crates/bunny-perf/src/scene.rs
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
CODE_STANDARDS.md
docs/PROCESS.md

Files edited:
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/tests/traversal_guard_tests.rs
crates/bunny-broadphase/tests/support/traversal_reference.rs
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-traversal-cycle-guard.md

Topic docs:
docs/topics/ray-tracing/README.md (malformed buffers, the visit bound and its argument);
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-030, TRACE-TP-401 to TRACE-TP-407, mutation
checks); docs/topics/performance/README.md (contract change cost check).

Generated artifacts:
None.

Checks run:
Recorded in the pull request and the test plan's mutation checks: the new cases on the
unfixed source under a 60-second limit per test, the same after the fix, the broadphase
tests, the mutation runs under `timeout 120`, the perf runs with `--exclusive`, and the full
Code Dojo gate.

Known risks:
A buffer that is not a tree and used to return `StackOverflow` or a result (a shared
subtree within 64 levels) can now return `InvalidNodeIndex` instead; trees are unaffected.
`InvalidNodeIndex` does not distinguish a cycle from an out-of-range index.

Human reviewer:
James Ross (maintainer); review pending.
