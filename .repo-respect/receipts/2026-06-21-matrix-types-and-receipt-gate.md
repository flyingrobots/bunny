# Receipt: Matrix Types And Receipt Gate

Task:
Implement the v0.6.0 matrix-types slice and tighten repo-respect enforcement so
every contributor, not only AI-assisted contributors, must provide PR receipts
and commit trailers.

Files read:

- `.githooks/commit-msg`
- `.githooks/pre-commit`
- `.githooks/pre-push`
- `.github/workflows/code-dojo.yml`
- `.repo-respect/README.md`
- `.repo-respect/receipts/2026-06-17-staged-rust-index.md`
- `CHANGELOG.md`
- `CODE_STANDARDS.md`
- `CONTRIBUTING.md`
- `README.md`
- `docs/CODE_DOJO.md`
- `docs/PROCESS.md`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `xtask/src/topic_docs.rs`

Files edited:

- `.repo-respect/README.md`
- `CHANGELOG.md`
- `CODE_STANDARDS.md`
- `CONTRIBUTING.md`
- `README.md`
- `crates/bunny-linalg/README.md`
- `crates/bunny-linalg/src/fixed_mat2.rs`
- `crates/bunny-linalg/src/fixed_mat3.rs`
- `crates/bunny-linalg/src/fixed_mat4.rs`
- `crates/bunny-linalg/src/fixed_mat4_inverse.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/src/matrix_common.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/CODE_DOJO.md`
- `docs/PROCESS.md`
- `docs/README.md`
- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `xtask/src/repo_respect.rs`
- `xtask/src/topic_docs.rs`

Topic docs:

- Updated `docs/topics/matrix-types/README.md`.
- Updated `docs/topics/matrix-types/test-plan.md`.
- No separate repo-respect topic shelf yet; this change updates
  `.repo-respect/README.md`, `CODE_STANDARDS.md`, `CONTRIBUTING.md`,
  `docs/CODE_DOJO.md`, and `docs/PROCESS.md` as the current process references.

Generated artifacts:

- None.

Checks run:

- `cargo test --locked -p xtask`
- `cargo run --locked -p xtask -- repo-respect check --staged`
- `cargo clippy --locked -p xtask --all-targets -- -D warnings`
- `markdownlint-cli2 .repo-respect/README.md
  .repo-respect/receipts/2026-06-21-matrix-types-and-receipt-gate.md
  CHANGELOG.md CODE_STANDARDS.md CONTRIBUTING.md docs/CODE_DOJO.md
  docs/PROCESS.md`
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features --
  -D warnings`
- `cargo test --locked -p bunny-linalg`
- `cargo run --locked -p xtask -- code-dojo-pre-commit`

Known risks:

- The matrix-types slice has passed the pre-commit gate but has not completed
  the full pre-push gate with the WASM leg.
- TypeScript generated-contract compilation remains a separate gap to close in a
  follow-up slice.

Human reviewer:
James
