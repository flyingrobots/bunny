# Receipt: The Depth Contract Between The BVH Builders And The Traversal Stack

Task:
flyingrobots/bunny#293, with the decision the maintainer's delegate made: restate the contract
so that "equals a test of every primitive" holds for every tree whose depth the traversal
stack holds, with the bound stated exactly for each traversal and for the compact conversion;
hold `build_bvh_sah` to depth 60 with tests that do not compare against the constant under
test; give `build_bvh` no cap, document what a caller gets, and settle whether inputs inside
the trace domain can make it exceed the stack. No production behaviour changes.

Files read:
crates/bunny-broadphase/src/ (bvh.rs, bvh/build.rs, bvh/split.rs, bvh/sah.rs,
bvh/sah/tests/mod.rs, traversal.rs, closest_hit.rs, compact.rs, compact_hit.rs)
crates/bunny-broadphase/tests/ (common/mod.rs, bvh_sah_tests.rs, compact_tests.rs,
traversal_guard_tests.rs)
docs/topics/bvh-construction/README.md and test-plan.md
docs/topics/ray-tracing/README.md and test-plan.md
CONTRIBUTING.md, docs/PROCESS.md, docs/CODE_DOJO.md

Files edited:
crates/bunny-broadphase/tests/traversal_depth_tests.rs (new: TRACE-TP-412, TRACE-TP-413)
crates/bunny-broadphase/tests/bvh_depth_tests.rs (new: BVH-TP-022, BVH-TP-024)
crates/bunny-broadphase/tests/common/mod.rs, bvh_sah_tests.rs (depth bounds as literals)
crates/bunny-broadphase/src/ (documentation comments only: bvh.rs, compact.rs,
closest_hit.rs, compact_hit.rs, traversal.rs)
docs/topics/bvh-construction/README.md and test-plan.md
docs/topics/ray-tracing/README.md and test-plan.md
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-bvh-depth-contract.md

Topic docs:
docs/topics/bvh-construction/ and docs/topics/ray-tracing/ (README.md and test-plan.md)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Planned; see the pull request.

Known risks:
Planned; see the pull request.

Human reviewer:
James Ross (maintainer); review pending.
