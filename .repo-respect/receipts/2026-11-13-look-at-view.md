# Receipt: Look-At View Matrix

Task:
Implement flyingrobots/bunny#200: `FixedMat4::try_look_at(eye, target, up)`, a typed
`LookAtError`, golden vectors that run natively and under the repository's wasm test
attribute, and a stated orthonormality bound.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
docs/CODE_DOJO.md
clippy.toml
docs/COORDINATE_LAW.md
docs/topics/coordinate-law/README.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
docs/topics/repo-respect/README.md
crates/bunny-linalg/src/fixed_mat4.rs
crates/bunny-linalg/src/fixed_mat4_projection.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-linalg/tests/projection_tests.rs
crates/bunny-num/src/fixed_q32_32.rs

Files edited:
crates/bunny-linalg/src/fixed_mat4_look_at.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/tests/look_at_tests.rs
crates/bunny-linalg/README.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
docs/topics/coordinate-law/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-look-at-view.md

Topic docs:
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
docs/topics/coordinate-law/README.md

Generated artifacts:
None.

Decision for the maintainer:
See the pull request: `ParallelUp` means zero or exactly parallel after normalization
(near-parallel is accepted, orthonormal bound still holds, roll is rounding noise);
`Overflow` covers only translation entries; the bound is 2 raw units.

Checks run:
RED: with `try_look_at` stubbed to return `Err`, `cargo test -p bunny-linalg --test
look_at_tests` failed 6 of 7 (devrun --worktree); the Display test passes on the stub.
GREEN: 7 of 7 pass, and the whole `bunny-linalg` suite passes, on the first run (the pinned
diagonal rows were cross-checked beforehand against an independent decimal reference).
Mutation: making `checked_negate` return its input (camera looks down +Z instead of -Z) failed 4
of 7 tests (MT-TP-023, 024, 025, 026); reverted.

Known risks:
Pending until the final commit.

Human reviewer:
Pending.
