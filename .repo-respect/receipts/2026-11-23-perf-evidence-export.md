# Receipt: Perf Evidence Export

Task:
Implement flyingrobots/bunny#238: an opt-in `--evidence <file>` option on `cargo xtask perf` that writes every round of both sides (case, side, round, p50, p95, p99) and the build identity (base reference, full base and candidate SHAs, target triple, flags, whether the working tree was dirty) as `bunny-perf-evidence/1` JSON Lines, from which the report table can be recomputed. The first commit adds the planned cases and failing tests against stub bodies; the second implements the writer and the living docs; the third, at the coordinator's request, adds a JSON Schema per line kind under `schemas/bunny/perf-evidence/v1/` (following the repository's convention that Bunny-owned schemas live under `schemas/bunny/` in a versioned directory) and validates real writer output against them.

Files read:
- `xtask/src/perf.rs`
- `xtask/src/perf_report.rs`
- `xtask/src/main.rs`
- `xtask/src/git_helpers.rs`
- `xtask/src/topic_docs.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `CODE_STANDARDS.md`
- `README.md` (the `schemas/bunny` ownership convention)
- `docs/topics/contract-generation/README.md`

Files edited:
- `xtask/src/perf.rs`
- `xtask/src/perf_evidence.rs`
- `xtask/src/perf_evidence/tests.rs`
- `xtask/src/perf_evidence/tests/schema.rs`
- `xtask/src/perf_evidence/json.rs`
- `schemas/bunny/perf-evidence/v1/build.schema.json`
- `schemas/bunny/perf-evidence/v1/round.schema.json`
- `xtask/src/main.rs`
- `docs/topics/performance/test-plan.md`
- `docs/topics/performance/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-perf-evidence-export.md`

Topic docs:
- `docs/topics/performance/test-plan.md`: PERF-REQ-011 to PERF-REQ-013 and cases PERF-TP-058 to PERF-TP-065, planned in the first commit and implemented in the second; scope, fixtures, determinism and gaps updated.
- `docs/topics/performance/README.md`: the `--evidence` option row, a new "Evidence files" section (location, naming, every field of `bunny-perf-evidence/1`, ordering, how to recompute the table), and the ledger introduction saying new entries link an evidence file. No existing ledger entry was changed. The third commit adds that the schema files are the definition and the tables the explanation, and PERF-REQ-014 with PERF-TP-066 to PERF-TP-069.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask perf`: RED, the six `perf_evidence` tests fail against the stubs; the two new `perf` tests and every existing test pass.
- `cargo run --locked -p xtask -- topic-docs`: clean, zero duplicate IDs.
- After the implementation: `cargo test --locked -p xtask perf` 29 passed; `cargo test --locked --workspace --all-targets --all-features` 116 suites, 845 passed, 0 failed.
- Mutations, each restored: numbers written with two decimals fails PERF-TP-058 and PERF-TP-059; dropping round 1 from the file fails PERF-TP-058, -059 and -062; treating untracked files as clean fails PERF-TP-061; taking the upper middle value as the even-count median in `perf_report` fails PERF-TP-058.
- `cargo fmt --all --check`; `cargo run --locked -p xtask -- code-dojo-generated`; `code-dojo-rust --all`; `code-dojo-determinism --enforce`; `code-dojo-perf-groups`; `topic-docs`; `repo-respect check --branch`: all clean.
- Schema commit: `cargo test --locked -p xtask perf` 33 passed; mutations, each restored: dropping `max_regression` from the build schema's `required` fails PERF-TP-068; renaming `align_code` in the writer fails PERF-TP-059, -066, -067 and -068; adding a `toolchain` field to the writer fails the same four.
- `cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings` on local Rust 1.100.0: no finding in the new code; it reports `assert!(x.is_empty())` findings in untouched lines of `perf.rs`, `perf_report.rs`, `nondeterminism/tests.rs` and `topic_docs.rs`, a lint the pinned 1.96.0 does not have.

Known risks:
- No real `cargo xtask perf` run was made (no measurement is claimed); the default path is covered by `the_report_is_rendered_as_before` and by construction.

Human reviewer:
@flyingrobots
