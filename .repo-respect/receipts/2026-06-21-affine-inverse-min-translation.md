# Receipt: Affine Inverse Min Translation

Task:
Resolve ChatGPT Code Reviewer feedback on PR #192: avoid pre-negating minimum
raw affine translations before applying the inverse linear scale.

Files read:

- `crates/bunny-linalg/src/fixed_affine2.rs`
- `crates/bunny-linalg/src/fixed_affine3.rs`
- `crates/bunny-linalg/tests/affine_transform_tests.rs`
- `CHANGELOG.md`
- `docs/topics/matrix-types/test-plan.md`
- PR #192 review thread `PRRT_kwDOS2Gurc6LE17_`

Files edited:

- `.repo-respect/receipts/2026-06-21-affine-inverse-min-translation.md`
- `CHANGELOG.md`
- `crates/bunny-linalg/src/fixed_affine2.rs`
- `crates/bunny-linalg/src/fixed_affine3.rs`
- `crates/bunny-linalg/tests/affine_transform_tests.rs`
- `docs/topics/matrix-types/test-plan.md`

Topic docs:

- `docs/topics/matrix-types/test-plan.md`

Generated artifacts:

- None.

Checks run:

- RED: `cargo test --locked -p bunny-linalg --test affine_transform_tests
  mt_tp_011_affine_inverse_scales_min_translation_before_negating` failed
  before the implementation change because `FixedAffine2::try_inverse`
  returned `None`.
- GREEN: `cargo test --locked -p bunny-linalg --test affine_transform_tests
  mt_tp_011_affine_inverse_scales_min_translation_before_negating` passed
  after applying inverse scale before negation.
- `cargo test --locked -p bunny-linalg --test affine_transform_tests`
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D
  warnings`
- `cargo run --locked -p xtask -- topic-docs`
- `markdownlint-cli2 CHANGELOG.md docs/topics/matrix-types/test-plan.md
  .repo-respect/receipts/2026-06-21-affine-inverse-min-translation.md`
- `git diff --cached --check`
- `cargo run --locked -p xtask -- code-dojo --all`

Known risks:

- The PR review thread remains unresolved until explicitly resolved on GitHub.
- This branch remains stacked on PR #191.

Human reviewer:
ChatGPT Code Reviewer on PR #192.
