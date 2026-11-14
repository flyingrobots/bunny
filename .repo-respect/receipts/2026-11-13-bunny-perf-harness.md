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

Follow-up (--align-code):
Measuring the sqrt branch showed `query/ray_aabb` 15-19 % slower although it takes no
square root and reads bit-identical inputs; with forced 64-byte code alignment on both
sides it measured 51.19 ns before and 51.15 ns after. `--align-code` makes that the
comparison mode, and CI uses it. Re-ran cargo test -p xtask and code-dojo --all: clean.

Follow-up (best of rounds):
Even with --align-code, `query/ray_aabb` alone flipped between about 40 and 52 ns across
eight runs of the same binary (process placement), so its median could read -21 % or +19 %
on identical code. The report now also takes the best p50 across rounds; speedups and the
regression gate use it, with a test for the bimodal case. Re-ran cargo test -p xtask,
topic-docs and code-dojo --all: clean.

Follow-up (cross-target):
Added --target/--runner to cargo xtask perf, target-named report headings, a report-only
platform matrix job (workflow_dispatch with platforms=true) and the "Measuring other
targets" and "Target-specific implementations" sections. Verified locally: wasm32-wasip1
under wasmtime 50.0.1 against origin/main (3 rounds; identical code, changes within 6 %),
and with --align-code. cargo test -p xtask (42 passed); workflow YAML parsed with Ruby.
The platform matrix itself has not run on GitHub yet.

Follow-up (Windows CI, PR #209):
The Windows determinism job failed harness_manifest_points_at_the_given_crates: Path::join
adds backslashes there, and the manifest used Debug formatting, which also escapes
non-ASCII characters as \u{...} (invalid TOML). Manifest paths are now written with forward
slashes and escaped quotes (toml_path), with a Windows-shaped test (PERF-TP-021).
cargo test -p xtask: 43 passed. The Dojo job's cargo-deny failure (RUSTSEC-2026-0386,
tokio 1.48.0 via ninelives 0.2.0 -> wesley-core) is not caused by this branch and is not
fixed here.

Follow-up (RUSTSEC-2026-0386, PR #209 Dojo job):
cargo deny failed on tokio 1.48.0 (RUSTSEC-2026-0386), reached through wesley-core ->
ninelives 0.2.0, whose "~1.48.0" requirement blocked any patched tokio. Fixed upstream in
flyingrobots/ninelives#8, released as ninelives 0.2.1 (tokio ^1.54.1). Cargo.lock now has
ninelives 0.2.1 and tokio 1.54.1, produced by `cargo update -p ninelives -p tokio` inside
flyingrobots/rust-dev:1.96.0 through devcontainers' devrun, exported as a patch and applied.

Follow-up (CodeRabbit review of PR #209, seven findings, all valid):
- Benchmarks pass each call's whole result through black_box; linalg/normalize read only
  n.x, so the optimizer could drop the y and z divisions (and similar partial reads in the
  query cases). Ledger numbers measured with the old harness need re-measuring.
- Each harness build passes --target-dir, so an inherited CARGO_TARGET_DIR cannot make both
  sides share (or run stale) binaries.
- The gate fails when no case was measured on both sides (a mistyped --filter passed before)
  and its message reports the best-p50 change it tested (PERF-TP-022, PERF-TP-023).
- The README says best-of-rounds reduces bimodal noise rather than removing it, with the
  rerun advice for a suspected bimodal CI failure.
- The Perf workflow pins its actions to commit SHAs and sets CARGO_INCREMENTAL=0.
Checks: cargo fmt applied from the dev container; full code-dojo --all via devrun.

Known risks:
Timing on shared CI runners is noisy; the 20 % threshold is chosen to catch real
regressions and may still need tuning after the first runs. The workflow has not run on
GitHub yet. `bunny-perf` is registered as tooling, so Code Dojo applies tooling limits to
it rather than core limits.

Human reviewer:
James Ross asked for before/after perf of every speedup, reusable benchmarks that run as
CI jobs, and thorough documentation on 2026-11-13; review pending.
