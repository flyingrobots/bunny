# Receipt: V0 7 1 Release Prep

Task:
Prepare the Bunny `v0.7.1` release (flyingrobots/bunny#277, refs flyingrobots/bunny#205).
`v0.7.0` was tagged and has a GitHub release but nothing was published to crates.io (the
upload of the first crate was refused with an authentication error), so `0.7.1` is built
from `main` and is the first 0.7 release on the registry. Bump the workspace and every
inter-crate requirement to 0.7.1, regenerate contract witnesses, turn the changelog's
Unreleased section into 0.7.1 (behaviour changes first), and align README, ROADMAP and
signposts. No tag, GitHub release or publish.

Files read:
docs/PROCESS.md
ROADMAP.md
CHANGELOG.md
docs/BEARING.md
docs/VISION.md
docs/topics/performance/README.md (ledger entries 8, 10, 11)
scripts/publish-crates.sh
.github/workflows/release.yml
Cargo.toml
crates/*/Cargo.toml
crates/*/README.md
.repo-respect/receipts/2026-11-13-v0-7-0-release-prep.md

Files edited:
Cargo.toml
Cargo.lock
CHANGELOG.md
README.md
ROADMAP.md
docs/BEARING.md
docs/VISION.md
docs/TECHNICAL_TEARDOWN.md
crates/*/Cargo.toml (every crate with an inter-crate requirement, including bunny-perf)
crates/bunny-linalg/README.md
crates/bunny-contract/src/generated/graphics.rs
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts
.repo-respect/receipts/2026-11-14-v0-7-1-release-prep.md

Topic docs:
None changed.

Generated artifacts:
`cargo run -p xtask -- generate` in the container, exported with `--export-patch`; the only
change is `bunny-wesley/0.7.0` to `bunny-wesley/0.7.1` in the three generated files.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
`verify` fully packages only the crates with no unpublished siblings; `cargo package
--workspace --locked` is run as well to build every crate from its archive.

Human reviewer:
Pending: the project lead reviews this pull request and performs the tag and release.
