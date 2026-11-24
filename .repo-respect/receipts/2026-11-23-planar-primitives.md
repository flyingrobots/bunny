# Receipt: 2D Primitives And Queries

Task:
flyingrobots/bunny#149, first slice: 2D primitives in `bunny-geom` beside the 3D ones
(`FixedSegment2`, `FixedRay2`, `FixedAabb2`, `FixedCircle2`, contract `bunny-planar/1`) and
exact closed-boundary queries: box containment, overlap and closest point; circle
containment and overlap with a circle or a box; segment containment, intersection and
closest point. Orientation is the exact `orientation2` of flyingrobots/bunny#145; the
256-bit integer of the predicates is shared crate-internally. Additive: no existing public
item, output bit, golden, digest or ID changes; `GeomError` gains no variant.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-shape-library.md
crates/bunny-geom/src/{lib.rs,predicates.rs,predicates/wide.rs,predicates/barycentric.rs,shapes.rs,shapes/triangle.rs,shapes/capsule.rs,plane.rs,bounds.rs}
crates/bunny-geom/tests/{predicates_oracle/mod.rs,predicate_orientation_tests.rs,predicate_sweep_tests.rs}
crates/bunny-linalg/src/{lib.rs,fixed_vec2.rs}
crates/bunny-query/src/closest.rs, crates/bunny-broadphase/src/utils.rs (3D conventions)
docs/README.md, docs/topics/predicates/, xtask/src/topic_docs.rs (ledger rules)
flyingrobots/bunny#149

Files edited:
crates/bunny-geom/tests/planar_oracle/mod.rs (new: independent oracle on the predicates' big integer)
crates/bunny-geom/tests/{planar_tests,planar_sweep_tests}.rs (new)
docs/topics/planar-geometry/test-plan.md (new)
.repo-respect/receipts/2026-11-23-planar-primitives.md

Topic docs:
docs/topics/planar-geometry/ is a new topic with its own PLANAR- prefix (PLANAR-REQ-001 to
-009, PLANAR-TP-001 to -012); no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the planar tests fail to compile (the types do
not exist); topic-docs and repo-respect check --branch clean.

Known risks:
The closed boundary, the empty-set reading of inverted boxes and negative radii, and the
reuse of `GeomError::NegativeSphereRadius` for a circle are choices the issue leaves open.

Human reviewer:
James Ross (maintainer); review pending.
