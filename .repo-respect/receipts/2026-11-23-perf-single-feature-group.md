# Receipt: Perf Single Feature Group

Task:
Fix `flyingrobots/bunny#272`: `bunny-perf` does not build with a single feature group, so the `xtask perf` fallback reports groups the base can build as new. First step: add the gate that builds the harness once per group alone and once with all groups; it fails on `main` before the harness is fixed.

Files read:
- `crates/bunny-perf/Cargo.toml`
- `crates/bunny-perf/src/cases.rs`
- `xtask/src/perf.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `docs/topics/performance/README.md`

Files edited:
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/main.rs`
- `xtask/src/perf.rs`
- `.repo-respect/receipts/2026-11-23-perf-single-feature-group.md`

Topic docs:
None yet - updated with the fix.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask feature_configs` (pass)
- `cargo run --locked -p xtask -- code-dojo-perf-groups` (RED: fails on the `num` group alone, `method vec3 is never used`)

Known risks:
The harness fix is not in this commit; the new gate is expected to fail until it lands.

Human reviewer:
@flyingrobots
