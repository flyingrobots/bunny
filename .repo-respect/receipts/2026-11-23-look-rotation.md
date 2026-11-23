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
- `crates/bunny-linalg/src/fixed_mat4_look_at.rs`
- `crates/bunny-linalg/src/fixed_mat4_look_at/look_rotation.rs`
- `crates/bunny-linalg/README.md`
- `docs/topics/look-rotation/README.md`
- `docs/topics/look-rotation/test-plan.md`
- `docs/topics/matrix-types/README.md`
- `docs/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-look-rotation.md`

Topic docs:
- `docs/topics/look-rotation/README.md`: new chapter (convention, rounding and bounds, refusals).
- `docs/topics/look-rotation/test-plan.md`: LR-REQ-001 to LR-REQ-003, LR-TP-001 to LR-TP-005, the Python model's results, seven recorded mutations.
- `docs/topics/matrix-types/README.md`: a pointer from the look-at section.

Generated artifacts:
- None. The pinned LR-TP-003 golden and LR-TP-005 digest come from the separate Python model.

Checks run:
- `cargo test --locked -p bunny-linalg --test look_rotation_tests` (RED: `try_look_rotation` does not exist yet, three E0599 errors).
- Seven kernel mutations, each applied and restored, each failing the cases recorded in `docs/topics/look-rotation/test-plan.md#mutations`.
- After the change: `cargo test --locked --workspace --all-targets --all-features` (108 test result lines, all ok; look_rotation_tests 5 passed; look_at_tests and look_at_fidelity_tests still pass), `cargo fmt --all --check`, `code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs` (0 duplicate IDs), `repo-respect check --branch`, `cargo clippy -p bunny-linalg --all-targets --all-features`.
- Python Decimal model of the 10 000-case corpus (not committed, as with the repository's other Python models): all bounds hold, counts and digest equal the Rust test's.
- The wasm32 check was not run locally; CI's WebAssembly job runs it.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
