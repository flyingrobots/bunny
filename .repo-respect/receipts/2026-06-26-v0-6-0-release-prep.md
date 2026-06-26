# Receipt: V0 6 0 Release Prep

Task:
Prepare the Bunny `v0.6.0` release after the deterministic contract-profile
cycle merged to `main`: bump publishable crate versions, regenerate contract
witnesses, move release notes out of `Unreleased`, and align signpost docs with
the frame-commons release cut.

Files read:
README.md
CHANGELOG.md
ROADMAP.md
Cargo.toml
Cargo.lock
crates/*/Cargo.toml
crates/bunny-contract/tests/generated_version_tests.rs
crates/bunny-wesley/src/main.rs
docs/BEARING.md
docs/VISION.md
docs/PROCESS.md
docs/TESTING.md
docs/TECHNICAL_TEARDOWN.md
docs/topics/matrix-types/test-plan.md
scripts/publish-crates.sh
.github/workflows/release.yml

Files edited:
README.md
CHANGELOG.md
ROADMAP.md
Cargo.toml
Cargo.lock
crates/bunny-broadphase/Cargo.toml
crates/bunny-codec/Cargo.toml
crates/bunny-geom/Cargo.toml
crates/bunny-linalg/Cargo.toml
crates/bunny-mesh/Cargo.toml
crates/bunny-query/Cargo.toml
crates/bunny-contract/src/generated/graphics.rs
docs/BEARING.md
docs/VISION.md
docs/TECHNICAL_TEARDOWN.md
docs/topics/matrix-types/test-plan.md
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts

Topic docs:
docs/topics/matrix-types/test-plan.md

Generated artifacts:
crates/bunny-contract/src/generated/graphics.rs
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts

Checks run:
cargo update --workspace
cargo run --locked -p xtask -- generate
cargo metadata --locked --no-deps --format-version 1
cargo test --locked -p bunny-contract --test generated_version_tests
cargo fmt --all
markdownlint-cli2 README.md CHANGELOG.md ROADMAP.md docs/BEARING.md docs/VISION.md docs/TECHNICAL_TEARDOWN.md docs/topics/matrix-types/test-plan.md .repo-respect/receipts/2026-06-26-v0-6-0-release-prep.md
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- repo-respect check --staged
RELEASE_TAG=v0.6.0 scripts/publish-crates.sh verify
cargo run --locked -p xtask -- code-dojo --all
git diff --check
git diff --cached --check

Known risks:
`v0.6.0` intentionally cuts the landed frame-commons and deterministic
contract-profile surface before projection, quaternion, angle, interpolation,
curve, and transform-aware bounds APIs. ROADMAP.md and docs/BEARING.md move
those unlanded items forward explicitly so the release does not overclaim them.

Human reviewer:
James Ross approved release prep with "make it so" on 2026-06-26.
