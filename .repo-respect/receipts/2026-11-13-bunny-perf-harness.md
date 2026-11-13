# Receipt: Bunny Perf Harness

Task:
Add reusable before/after benchmarks for Bunny's kernels that run locally and as a CI
job: the `bunny-perf` harness crate, `cargo xtask perf --base <rev>` (same harness built
against both revisions, run interleaved, medians compared, optional regression gate and
Markdown summary), the `Perf` workflow, and the `docs/topics/performance/` chapter with
the fidelity rule and an optimization ledger.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
Cargo.toml
docs/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/repo-respect/test-plan.md
xtask/Cargo.toml
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/git_helpers.rs
xtask/src/topic_docs.rs
.github/workflows/code-dojo.yml
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-geom/src/lib.rs
crates/bunny-query/src/ray.rs

Files edited:
Cargo.toml
Cargo.lock
CHANGELOG.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/timing.rs
xtask/src/main.rs
xtask/src/perf.rs
xtask/src/perf_report.rs
xtask/src/code_dojo.rs
.github/workflows/perf.yml
docs/README.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
.repo-respect/receipts/2026-11-13-bunny-perf-harness.md

Topic docs:
docs/topics/performance/README.md
docs/topics/performance/test-plan.md

Generated artifacts:
None. `cargo xtask perf` writes build outputs and a temporary worktree under the ignored
`target/perf/` directory.

Checks run:
cargo test --locked -p xtask
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all
cargo run --locked -p xtask -- perf --base origin/main --rounds 6 (identical code on both
sides: most cases within 3 %, the fastest up to 8 %; Apple M5 Pro under desktop load)

Known risks:
Timing on shared CI runners is noisy; the 20 % threshold is chosen to catch real
regressions and may still need tuning after the first runs. The workflow has not run on
GitHub yet. `bunny-perf` is registered as tooling, so Code Dojo applies tooling limits to
it rather than core limits.

Human reviewer:
James Ross asked for before/after perf of every speedup, reusable benchmarks that run as
CI jobs, and thorough documentation on 2026-11-13; review pending.
