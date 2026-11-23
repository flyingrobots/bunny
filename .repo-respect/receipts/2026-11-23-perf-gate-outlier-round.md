# Receipt: Perf Gate Outlier Round

Task:
Fix `flyingrobots/bunny#254`: the `xtask perf` regression gate failed on one lucky base round (and one lucky head round could hide a regression). The gate now fails a case only when its floor p50 (lowest, or second-lowest from five rounds) and its median p50 both slow past the threshold.

Files read:
- `xtask/src/perf_report.rs`
- `xtask/src/perf.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`

Files edited:
- `xtask/src/perf_report.rs`
- `xtask/src/perf.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-perf-gate-outlier-round.md`

Topic docs:
- `docs/topics/performance/README.md` (new "One lucky round" section, gate wording)
- `docs/topics/performance/test-plan.md` (PERF-REQ-003 reworded, PERF-TP-052 to PERF-TP-056, PERF-TP-023 renamed test)

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask perf_report` (RED on the first commit, GREEN after)
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- With fewer than five rounds the floor is the plain minimum, so one outlier round can still trip the gate there; CI uses seven rounds.
- The existing test `the_gate_message_reports_the_best_p50_change_it_tested` encoded the old best-only rule (median -35 %, best +30 % failed). It is replaced by `the_gate_message_reports_the_floor_and_median_change_it_tested` because that rule is what the issue changes.
- Local clippy runs 1.100.0; the pinned 1.96.0 runs in CI. `cargo deny` is not installed locally.

Human reviewer:
@flyingrobots
