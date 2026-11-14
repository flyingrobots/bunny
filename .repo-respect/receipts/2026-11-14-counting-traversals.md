# Receipt: Counting Traversals

Task:
Implement flyingrobots/bunny#335: counting forms of `compact_closest_hit` and
`compact_any_hit` that return the same result, error and primitive calls as the uncounted
functions together with a record of the work the search did (box tests, boxes entered,
skipped pops, interior and leaf visits, primitive tests, the deepest stack), for Lumen's
statistics report (flyingrobots/lumen#29) and BVH-cost view (flyingrobots/lumen#16). The
uncounted functions keep every output bit and their measured cost.

Files read:
crates/bunny-broadphase/src/compact_hit.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/src/compact.rs
crates/bunny-broadphase/tests/compact_tests.rs
crates/bunny-broadphase/tests/traversal_guard_tests.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/compact.rs
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
CODE_STANDARDS.md
CONTRIBUTING.md
docs/PROCESS.md

Files edited:
docs/topics/ray-tracing/test-plan.md
.repo-respect/receipts/2026-11-14-counting-traversals.md

Topic docs:
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-033, TRACE-TP-412 to TRACE-TP-417, planned).

Generated artifacts:
None.

Checks run:
Planning commit only; checks are recorded with the implementation.

Known risks:
None yet: no code has changed.

Human reviewer:
James Ross (maintainer); review pending.
