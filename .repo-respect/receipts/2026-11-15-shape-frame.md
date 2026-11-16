# Receipt: Rigid Frame For Analytic Shapes

Task:
Implement flyingrobots/bunny#357: a `ShapeFrame` in `bunny-query` (orthonormal unit axes and a
translation) that maps a `TraceRay` into a shape's object frame and a hit normal back to world
space, under the contract identity `bunny-shape-frame/1`, with typed refusals outside the
trace domain.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
crates/bunny-query/src/trace.rs, crates/bunny-query/src/ray.rs, crates/bunny-query/src/lib.rs
crates/bunny-linalg/src/lib.rs (`FixedUnitVec3`), crates/bunny-optics/src/basis.rs,
crates/bunny-optics/src/error.rs
docs/topics/ray-tracing/README.md, docs/topics/ray-tracing/test-plan.md

Files edited:
In progress; listed when the change is complete.

Topic docs:
In progress.

Generated artifacts:
None.

Checks run:
In progress.

Known risks:
In progress.

Human reviewer:
James Ross (maintainer); review pending.
