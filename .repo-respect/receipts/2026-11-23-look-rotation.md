# Receipt: Look Rotation

Task:
Implement `flyingrobots/bunny#310`: `FixedMat3::try_look_rotation(forward, up)`, the rotation whose columns are right, true up and back, so the object's local `-Z` maps onto `forward`, with the same refusals as `try_look_at`.

Files read:
- `crates/bunny-linalg/src/fixed_mat4_look_at.rs`
- `crates/bunny-linalg/src/fixed_mat3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/look_at_tests.rs`
- `crates/bunny-linalg/tests/look_at_fidelity_tests.rs`
- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`
- `docs/topics/coordinate-law/README.md`
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`

Files edited:
- `crates/bunny-linalg/tests/look_rotation_tests.rs`
- `.repo-respect/receipts/2026-11-23-look-rotation.md`

Topic docs:
- Pending: `docs/topics/look-rotation/README.md` and `test-plan.md`.

Generated artifacts:
- None.

Checks run:
- `cargo test --locked -p bunny-linalg --test look_rotation_tests` (RED: `try_look_rotation` does not exist yet, three E0599 errors).

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
