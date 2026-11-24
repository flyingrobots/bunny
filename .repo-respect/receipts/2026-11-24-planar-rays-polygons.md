# Receipt: 2D Ray Queries And Point In Polygon

Task:
flyingrobots/bunny#149, second slice, on top of the first (flyingrobots/bunny#495): exact
closed ray queries for `FixedRay2` against `FixedAabb2`, `FixedCircle2` and `FixedSegment2`
(a hit decision and the first hit parameter rounded once, ties to even, in a new `RayHit2`),
and a borrowed `FixedPolygon2` with an exact three-way point in polygon (`PolygonLocation`)
on `orientation2` signs. Additive: no existing public item, output bit, golden, digest or ID
changes; `GeomError` gains no variant.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-planar-primitives.md
crates/bunny-geom/src/{lib.rs,planar.rs,planar/segment.rs,wide.rs,predicates.rs,predicates/barycentric.rs}
crates/bunny-geom/tests/{planar_oracle/mod.rs,predicates_oracle/mod.rs,planar_tests.rs,planar_sweep_tests.rs}
crates/bunny-linalg/src/lib.rs (the unit-vector tolerance), crates/bunny-query/src/ray.rs (3D ray conventions)
docs/topics/planar-geometry/, xtask/src/topic_docs.rs (ledger statuses)
flyingrobots/bunny#149, flyingrobots/bunny#495

Files edited:
crates/bunny-geom/tests/planar_cast_oracle/mod.rs (new: independent oracle)
crates/bunny-geom/tests/{planar_ray_tests,planar_polygon_tests,planar_cast_sweep_tests}.rs (new)
docs/topics/planar-geometry/test-plan.md (PLANAR-REQ-010 to -015, PLANAR-TP-013 to -020)
.repo-respect/receipts/2026-11-24-planar-rays-polygons.md

Topic docs:
docs/topics/planar-geometry/ continues its own ranges: PLANAR-REQ-010 to -015 and
PLANAR-TP-013 to -020, planned in this commit. No existing ID changes.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (the ray
queries, `RayHit2`, `FixedPolygon2` and `PolygonLocation` do not exist); topic-docs and
repo-respect check --branch clean.

Known risks:
Returning a parameter as `RayHit2` (with `Beyond` for a parameter past `i64::MAX`), the
parameter zero for an origin in the shape, and the nonzero winding rule for unvalidated
polygons are choices the issue leaves open.

Human reviewer:
James Ross (maintainer); review pending.
