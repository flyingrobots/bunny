# Receipt: Frustum Culling

Task:
flyingrobots/bunny#146: a six-plane `FixedFrustum` in `bunny-geom` (left, right, bottom, top,
near, far, each front facing into the frustum, inside the closed set), `contains_point`,
`classify_aabb` and `classify_sphere` returning `FrustumClass` (`Inside`, `Intersecting`,
`Outside`) by a stated rule built on `FixedPlane3::classify`, `classify_aabb` and `PlaneSpan`,
and `from_clip_matrix`, the exact row-sum extraction for Bunny's `[0, 1]` depth range.
Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-transform-composition.md
crates/bunny-geom/src/{lib.rs,plane.rs,plane/span.rs}
crates/bunny-geom/tests/{plane_span_tests.rs,plane_span_sweep_tests.rs}
crates/bunny-linalg/src/{fixed_mat4.rs,fixed_mat4_projection.rs,fixed_mat4_look_at.rs,lib.rs}
crates/bunny-num/src/fixed_q32_32.rs (accessors)
xtask/src/{code_dojo.rs,topic_docs.rs} (limits and ledger rules)
docs/topics/{plane-clipping,coordinate-law,transform}/ chapters and test plans
flyingrobots/bunny#146 (no comments)

Files edited:
crates/bunny-geom/tests/frustum_tests.rs (new: constructed cases)
crates/bunny-geom/tests/frustum_sweep_tests.rs (new: seeded sweeps and independent oracles)
docs/topics/frustum/test-plan.md (new: FRUSTUM-REQ-001 to -007, FRUSTUM-TP-001 to -016)
.repo-respect/receipts/2026-11-23-frustum-culling.md

Topic docs:
docs/topics/frustum/ is a new topic with its own FRUSTUM- prefix; no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the frustum tests fail to compile, since
`FixedFrustum` does not exist; topic-docs and repo-respect check --branch clean.

Known risks:
The six-plane test is conservative: `Intersecting` includes shapes that meet no point of the
frustum. The chapter says so and a test pins an example of each.

Human reviewer:
James Ross (maintainer); review pending.
