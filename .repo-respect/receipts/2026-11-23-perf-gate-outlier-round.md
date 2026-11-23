# Receipt: Perf Gate Outlier Round

Task:
Fix `flyingrobots/bunny#254`: the `xtask perf` regression gate failed a pull request on one lucky base round. First step: add the failing tests (the pull request 252 shape, a consistent regression, a lucky head round that hides a regression, and a floor/median disagreement).

Files read:
- `xtask/src/perf_report.rs`
- `xtask/src/perf.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`

Files edited:
- `xtask/src/perf_report.rs`
- `.repo-respect/receipts/2026-11-23-perf-gate-outlier-round.md`

Topic docs:
None yet - updated with the fix.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask perf_report` (RED: three of the four new tests fail on the best-p50 rule)

Known risks:
The gate rule is not changed in this commit; the new tests are expected to fail until it lands.

Human reviewer:
@flyingrobots
