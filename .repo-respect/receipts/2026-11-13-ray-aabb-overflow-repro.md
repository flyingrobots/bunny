# Receipt: ray_intersects_aabb overflow reproduction

Task:
Reproduce or refute the suspected false miss in `ray_intersects_aabb` (flyingrobots/bunny#211)
with a test, characterise it, and write a proposal. The function is deliberately not changed.

Files read:
CONTRIBUTING.md
docs/PROCESS.md
docs/CODE_DOJO.md
docs/topics/performance/README.md
docs/topics/repo-respect/test-plan.md
crates/bunny-query/src/ray.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-geom/src/lib.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/tests/bvh_tests.rs

Files edited:
crates/bunny-query/tests/ray_aabb_overflow_tests.rs
crates/bunny-broadphase/tests/bvh_tests.rs
docs/design/0005-ray-aabb-slab-overflow.md
.repo-respect/receipts/2026-11-13-ray-aabb-overflow-repro.md

Topic docs:
None. The finding is a proposal, so it lives in docs/design/ (historical proposals); no
topic README describes current behaviour that changed.

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-query -p bunny-broadphase (all pass; the known_defect tests assert
the current wrong output, so they pass today and must flip when a fix lands)

Known risks:
The tests pin a known defect as passing behaviour. They carry `known_defect` names and comments,
and the design note says which assertions flip. How often a renderer produces near-axis
directions was not measured.

Human reviewer:
Pending: the fix decision belongs to the maintainer.
