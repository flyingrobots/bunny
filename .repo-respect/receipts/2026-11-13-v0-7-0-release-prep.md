# Receipt: V0 7 0 Release Prep

Task:
Prepare the Bunny `v0.7.0` release (flyingrobots/bunny#205) as an early cut of the
0.7 line: bump the workspace and every inter-crate requirement to 0.7.0, regenerate
contract witnesses, turn the changelog's Unreleased section into 0.7.0 with the
ray-box behaviour change (#211) first, and align README, ROADMAP and signposts
without claiming the unfinished milestone goalposts. No tag, GitHub release or
publish.

Files read:
docs/PROCESS.md
ROADMAP.md
CHANGELOG.md
docs/BEARING.md
docs/VISION.md
docs/topics/performance/README.md
scripts/publish-crates.sh
.github/workflows/release.yml
Cargo.toml
crates/*/Cargo.toml
crates/*/README.md
.repo-respect/receipts/2026-06-26-v0-6-0-release-prep.md

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
crates/bunny-num/README.md
crates/bunny-contract/src/generated/graphics.rs
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts

Topic docs:
None changed.

Generated artifacts:
The `bunny-wesley/0.6.0` generator string became `bunny-wesley/0.7.0` in the three
generated files (edited by substitution, then checked by `xtask generate` and the
code-dojo drift check; see the pull request for results).

Checks run:
See the pull request description for the exact commands and results at the final
commit.

Known risks:
`v0.7.0` is an early cut: quaternions, interpolation, curves, robust predicates,
shapes, topology and mass properties are not shipped. The registry check cannot
resolve unpublished 0.7.0 siblings, so `verify` fully packages only the root crates.

Human reviewer:
Pending: the project lead reviews this pull request and performs the tag and release.
