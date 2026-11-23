# Receipt: Perf Single Feature Group

Task:
Fix `flyingrobots/bunny#272`: `bunny-perf` did not build with a single feature group (shared helpers were dead code or unused imports under `warnings = "deny"`), so the `xtask perf` per-group fallback reported buildable groups as new. Gate the shared helpers on the groups that use them and add a gate that builds the harness once per group alone and once with all groups.

Files read:
- `crates/bunny-perf/Cargo.toml`
- `crates/bunny-perf/src/cases.rs`
- `crates/bunny-perf/src/scene.rs`
- `xtask/src/perf.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `docs/CODE_DOJO.md`

Files edited:
- `crates/bunny-perf/src/cases.rs`
- `crates/bunny-perf/src/scene.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/main.rs`
- `xtask/src/perf.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `docs/CODE_DOJO.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-perf-single-feature-group.md`

Topic docs:
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md` (PERF-REQ-010, PERF-TP-049 to PERF-TP-051)

Generated artifacts:
None.

Checks run:
- `cargo run --locked -p xtask -- code-dojo-perf-groups` (RED on the gate-only commit at the `num` group; GREEN after the fix, all 17 groups alone and all together)
- `cargo test --locked -p xtask feature_configs`
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

- Follow-up: `cargo clippy` on the pinned toolchain flagged `iter_cloned_collect` in `perf_group_commands` (CI Dojo); fixed with `clone()`. Local clippy cannot confirm the pinned lint set (1.100.0 vs 1.96.0).

Known risks:
- `cargo clippy` on the local 1.100.0 toolchain reports lints that exist only on that toolchain (for example `chunks_exact`); the pinned 1.96.0 toolchain runs in CI.
- `scene.rs` keeps its existing crate-level `allow(dead_code)` for the shared scene helpers; only its imports were gated here.
- `cargo deny` is not installed locally; CI runs it.

Human reviewer:
@flyingrobots
