# Receipt: Affine Transforms

Task:
Implement ROADMAP v0.6.0 Goalpost 2 Slice 2.2: deterministic affine
transform types for `bunny-linalg`, stacked on the matrix-types branch.

Files read:

- `crates/bunny-linalg/src/fixed_mat2.rs`
- `crates/bunny-linalg/src/fixed_mat3.rs`
- `crates/bunny-linalg/src/fixed_vec2.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/matrix_tests.rs`
- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`
- `ROADMAP.md`

Files edited:

- `.repo-respect/receipts/2026-06-21-affine-transforms.md`
- `CHANGELOG.md`
- `README.md`
- `ROADMAP.md`
- `crates/bunny-linalg/README.md`
- `crates/bunny-linalg/src/fixed_affine2.rs`
- `crates/bunny-linalg/src/fixed_affine3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/affine_transform_tests.rs`
- `docs/MATH_GEOMETRY_CAPABILITY_MAP.md`
- `docs/README.md`
- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`

Topic docs:

- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`

Generated artifacts:

- None.

Checks run:

- RED: `cargo test --locked -p bunny-linalg --test affine_transform_tests`
  failed with unresolved `FixedAffine2` and `FixedAffine3` imports before
  implementation.
- GREEN: `cargo test --locked -p bunny-linalg --test affine_transform_tests`
  passed after implementation.
- `cargo fmt --all`
- `git diff --check`
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D
  warnings`
- `cargo run --locked -p xtask -- topic-docs`
- `markdownlint-cli2 README.md CHANGELOG.md ROADMAP.md
  crates/bunny-linalg/README.md docs/README.md
  docs/MATH_GEOMETRY_CAPABILITY_MAP.md docs/topics/matrix-types/README.md
  docs/topics/matrix-types/test-plan.md
  .repo-respect/receipts/2026-06-21-affine-transforms.md`
- `cargo run --locked -p xtask -- code-dojo --all`

Known risks:

- This branch is stacked on PR #191 because `main` does not yet contain the
  matrix types used by the affine wrappers.
- Normal transforms, bounds propagation, projection, and viewport mapping remain
  future roadmap slices and are explicitly out of scope.

Human reviewer:
Pending PR review.
