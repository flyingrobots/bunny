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
crates/bunny-query/src/frame.rs (new), crates/bunny-query/src/lib.rs,
crates/bunny-query/src/trace.rs (`TraceRay::prepare` made crate-visible)
crates/bunny-query/tests/shape_frame_tests.rs (new)
crates/bunny-query/README.md
docs/topics/shape-frame/README.md, docs/topics/shape-frame/test-plan.md (new)
docs/README.md, docs/NUMERIC_CONSTITUTION.md
CHANGELOG.md
.repo-respect/receipts/2026-11-15-shape-frame.md

Topic docs:
docs/topics/shape-frame/README.md (contract identity, frame refusals, the rounding rule of
both maps, degenerate cases); docs/topics/shape-frame/test-plan.md (SF-REQ-001 to 007,
SF-TP-001 and 003 to 011, mutations).

Generated artifacts:
None.

Checks run:
All inside the bunny container through devrun; nothing on the host.
`cargo test --locked -p bunny-query`: all pass, the eight new tests included. Clippy with
`-D warnings` on `bunny-query --all-targets`: clean. Mutations of `frame.rs` (ties half up,
floor, handedness accepting negatives, exclusive orthogonality limit, origin domain check
removed, exclusive direction limit, `to_world` transposed): each fails the cases listed in
the test plan. The full gate, `code-dojo --all` with `--history origin/main..HEAD`: see the
pull request.

Coordinator decision (2026-11-16): a direction lengthened by the rotation is divided by
`2^k` (k 0 or 1) inside the one rounding, not refused; `ObjectRay` carries k and maps
distances. Tests SF-TP-010 and SF-TP-011 and six more mutations (no shift, always shift,
round then shift, world_distance nearest or unscaled, object_distance unscaled), all killed.

Known risks:
The axis tolerance (`2^-24`) and the right-handedness requirement were this change's
choices and the coordinator kept them; the issue says only "orthonormal basis from unit
vectors". The world parameter of an object hit is exact for the object ray; relative to the
world ray it carries the mapping's rounding.

Human reviewer:
James Ross (maintainer); review pending.
