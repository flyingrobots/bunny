# Receipt: Pr191 Review Fixes

Task:
Resolve five unresolved PR #191 review findings by hardening repo-respect
receipt enforcement and fixing the `FixedMat2::try_inverse` minimum raw
off-diagonal boundary case.

Files read:

- `xtask/src/repo_respect.rs`
- `xtask/src/code_dojo.rs`
- `crates/bunny-linalg/src/fixed_mat2.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/topics/matrix-types/test-plan.md`
- `CHANGELOG.md`

Files edited:

- `.repo-respect/receipts/2026-06-21-pr191-review-fixes.md`
- `CHANGELOG.md`
- `crates/bunny-linalg/src/fixed_mat2.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/topics/matrix-types/test-plan.md`
- `xtask/src/repo_respect.rs`

Topic docs:

- `docs/topics/matrix-types/test-plan.md`

Generated artifacts:

- None.

Checks run:

- `cargo test --locked -p xtask repo_respect`
- `cargo test --locked -p bunny-linalg --test matrix_tests mt_tp_012_fixed_mat2_inverse_divides_min_off_diagonal_before_negating`
- `cargo fmt --all`
- `cargo test --locked -p xtask`
- `cargo test --locked -p bunny-linalg --test matrix_tests`
- `cargo clippy --locked -p xtask --all-targets -- -D warnings`
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D warnings`
- `cargo run --locked -p xtask -- topic-docs`
- `markdownlint-cli2 CHANGELOG.md docs/topics/matrix-types/test-plan.md`
- `git diff --check`
- `markdownlint-cli2 .repo-respect/receipts/2026-06-21-pr191-review-fixes.md`
- `cargo run --locked -p xtask -- repo-respect check --staged`
- `cargo run --locked -p xtask -- topic-docs --staged`
- `git diff --cached --check`
- `cargo run --locked -p xtask -- code-dojo --all`

Known risks:
GitHub CI still needs to run after pushing the branch update.

Human reviewer:
Pending human PR review.
