# Receipt: Projection Matrices

Task:
Implement flyingrobots/bunny#201: `FixedMat4::try_orthographic` and
`FixedMat4::try_perspective` (tangent input, no trigonometry), a typed
`ProjectionError`, the NDC depth range in the coordinate law, and golden vectors that
run natively and under the repository's wasm test attribute.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
docs/CODE_DOJO.md
clippy.toml
docs/COORDINATE_LAW.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
docs/topics/repo-respect/README.md
crates/bunny-linalg/src/fixed_mat4.rs
crates/bunny-linalg/tests/matrix_tests.rs
crates/bunny-num/src/fixed_q32_32.rs

Files edited:
crates/bunny-linalg/src/fixed_mat4_projection.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/tests/projection_tests.rs
crates/bunny-linalg/README.md
docs/README.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-projection-matrices.md

Topic docs:
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md

Generated artifacts:
None.

Decision for the maintainer:
NDC depth is `[0, 1]` with near at 0 and far at 1, camera looking down `-Z` in the
right-handed frame. The coordinate law already reserved `[0, 1]` and a `-Z` default
camera; this PR activates that reservation (CL-REQ-011, new CL-REQ-013). Overrule by
changing the two depth rows and the golden vectors.

Checks run:
RED: with the builders stubbed to return `Err`, `cargo test -p bunny-linalg --test
projection_tests` failed 9 of 10 (devrun --worktree). GREEN: 10 of 10 pass.
Mutation: removing the negation of the perspective `m23` entry failed 5 of 10 tests
(CL-TP-004, CL-TP-005, MT-TP-017, MT-TP-018, MT-TP-022); reverted.
Final-commit gates are listed in the pull request.

Known risks:
Matrix entries round to nearest Q32.32, so depth endpoints of non-power-of-two
parameters are within a few raw units of 0 and 1, not exact (bound asserted in
MT-TP-022). `Overflow` also covers a divisor that rounds to zero. The wasm run is the
repository's headless gate; see the pull request for what ran.

Human reviewer:
Pending.
