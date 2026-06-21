# Receipt: Mat3 Mat4 Min Cofactor

Task:
Fix self-discovered matrix inverse boundary behavior where `FixedMat3` and
`FixedMat4` negated negative cofactors before determinant division, rejecting
representable inverse entries for raw `i64::MIN` cofactors.

Files read:

- `crates/bunny-linalg/src/fixed_mat3.rs`
- `crates/bunny-linalg/src/fixed_mat4_inverse.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/topics/matrix-types/test-plan.md`
- `CHANGELOG.md`

Files edited:

- `.repo-respect/receipts/2026-06-21-mat3-mat4-min-cofactor.md`
- `CHANGELOG.md`
- `crates/bunny-linalg/src/fixed_mat3.rs`
- `crates/bunny-linalg/src/fixed_mat4_inverse.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/topics/matrix-types/test-plan.md`

Topic docs:

- `docs/topics/matrix-types/test-plan.md`

Generated artifacts:
None.

Checks run:

- `cargo test --locked -p bunny-linalg --test matrix_tests mt_tp_013_fixed_mat3_inverse_divides_min_cofactor_before_negating`
- `cargo test --locked -p bunny-linalg --test matrix_tests mt_tp_014_fixed_mat4_inverse_divides_min_cofactor_before_negating`
- `cargo test --locked -p bunny-linalg --test matrix_tests`
- `cargo fmt --all`
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D warnings`
- `cargo run --locked -p xtask -- topic-docs`
- `markdownlint-cli2 CHANGELOG.md docs/topics/matrix-types/test-plan.md .repo-respect/receipts/2026-06-21-mat3-mat4-min-cofactor.md`
- `git diff --check`
- `cargo run --locked -p xtask -- repo-respect check --staged`
- `cargo run --locked -p xtask -- topic-docs --staged`
- `git diff --cached --check`
- `cargo run --locked -p xtask -- code-dojo --all`

Known risks:
GitHub CI and PR thread closure still need to run after this commit is pushed.

Human reviewer:
Self-audit finding posted to PR #191 with `@codex` for second opinion.
