# Receipt: Unprojection to a World Ray

Task:
flyingrobots/bunny#150, the remainder: acceptance criterion 2, "convert viewport coordinates to
deterministic rays", as matrix-side unprojection. `Viewport::try_unproject(coordinate,
clip_from_view, view_from_world)` under the contract identity `bunny-unproject/1`
(`UNPROJECT_CONTRACT_ID`) returns a `ViewportRay` (origin at NDC depth 0, direction to the
depth-1 point) or a typed `UnprojectError`. Criteria 1, 3 and 4 were already met on `main` by
the coordinate law, the projection builders and the viewport mapping. Additive: no existing
public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-transform-composition.md
crates/bunny-linalg/src/{lib.rs,viewport.rs,fixed_mat4.rs,fixed_mat4_projection.rs,
fixed_mat4_inverse.rs,fixed_mat4_look_at.rs,fixed_vec3.rs}
crates/bunny-linalg/tests/{projection_tests.rs,viewport_tests.rs,look_at_fidelity_tests.rs}
crates/bunny-optics/src/{camera.rs,orthographic.rs,shading/wide256.rs}
crates/bunny-optics/tests/{projection_round_trip_tests.rs,exact/mod.rs}
crates/bunny-query/src/trace.rs (TraceRay::new limits)
docs/topics/coordinate-law/{README.md,test-plan.md}, docs/topics/optics/README.md (round trip,
construction safeguards), docs/topics/matrix-types/README.md (inverse)
flyingrobots/bunny#150, #464

Files edited:
crates/bunny-linalg/src/unproject.rs (new: API, errors, adjugate inverse of the view)
crates/bunny-linalg/src/unproject/frustum.rs (new: analytic inverse of the two projection shapes)
crates/bunny-linalg/src/unproject/wide.rs (new: 512-bit signed integers)
crates/bunny-linalg/src/lib.rs (module and exports)
crates/bunny-linalg/tests/unproject_tests.rs (new)
crates/bunny-linalg/tests/unproject_oracle/mod.rs (new: general-inverse oracle by Cramer's rule)
crates/bunny-optics/tests/unproject_agreement_tests.rs (new: agreement with the cameras)
crates/bunny-linalg/README.md, CHANGELOG.md
docs/topics/coordinate-law/README.md (Unprojection section; one stale sentence corrected)
docs/topics/coordinate-law/test-plan.md (CL-REQ-019 to -023, CL-TP-013 to -021; mutations)
.repo-respect/receipts/2026-11-23-unproject-ray.md

Topic docs:
docs/topics/coordinate-law/: new IDs CL-REQ-019 to CL-REQ-023 and CL-TP-013 to CL-TP-021,
contiguous above the ledger's highest (CL-REQ-018, CL-TP-012). No other ledger uses the CL-
prefix.

Generated artifacts:
None.

Checks run:
First commit (tests only): the unprojection tests fail to compile (the API does not exist).
Kernel commit: see the pull request for the gate list and results.

Known risks:
Only the two projection shapes the builders produce are accepted (any entry values); an
off-centre frustum or any other projective matrix is refused as UnsupportedProjection. The
camera agreement tolerance is measured, not proven.

Human reviewer:
James Ross (maintainer); review pending.
