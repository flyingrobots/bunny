# Receipt: Perf Evidence Export

Task:
Implement flyingrobots/bunny#238: an opt-in `--evidence <file>` option on `cargo xtask perf` that writes every round of both sides (case, side, round, p50, p95, p99) and the build identity (base reference, full base and candidate SHAs, target triple, flags, whether the working tree was dirty) as `bunny-perf-evidence/1` JSON Lines, from which the report table can be recomputed. The first commit adds the planned cases and failing tests against stub bodies; the second implements the writer and the living docs.

Files read:
- `xtask/src/perf.rs`
- `xtask/src/perf_report.rs`
- `xtask/src/main.rs`
- `xtask/src/git_helpers.rs`
- `xtask/src/topic_docs.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `CODE_STANDARDS.md`

Files edited:
- `xtask/src/perf.rs`
- `xtask/src/perf_evidence.rs`
- `xtask/src/perf_evidence/tests.rs`
- `xtask/src/main.rs`
- `docs/topics/performance/test-plan.md`
- `.repo-respect/receipts/2026-11-23-perf-evidence-export.md`

Topic docs:
- `docs/topics/performance/test-plan.md`: PERF-REQ-011 to PERF-REQ-013 and planned cases PERF-TP-058 to PERF-TP-065.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask perf`: RED, the six `perf_evidence` tests fail against the stubs; the two new `perf` tests and every existing test pass.
- `cargo run --locked -p xtask -- topic-docs`: clean, zero duplicate IDs.

Known risks:
- No real `cargo xtask perf` run was made (no measurement is claimed); the default path is covered by `the_report_is_rendered_as_before` and by construction.

Human reviewer:
@flyingrobots
