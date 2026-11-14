# Receipt: Counted Traversal Benchmark

Task:
Second part of flyingrobots/bunny#335: time the counted compact searches added by the first
pull request on the benchmark scene, state their cost, and check their counts against the
profile replica of `closest_hit` on every benchmark ray (TRACE-TP-418).

Files read:
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/compact.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/scene.rs
xtask/src/perf.rs
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md

Files edited:
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/counted.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/scene.rs
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-counted-traversal-bench.md

Topic docs:
docs/topics/performance/README.md (the `trace_counted` group and the counted forms' cost);
docs/topics/ray-tracing/README.md and test-plan.md (TRACE-TP-418 implemented).

Generated artifacts:
None.

Checks run:
TRACE-TP-418 and the bunny-perf tests; a build of the `trace_counted` group alone, as
`cargo xtask perf` builds groups; perf runs with `--exclusive`; the full Code Dojo gate.

Known risks:
The counted cases add code to the harness binary, which can move unrelated cases between
readings (code-layout noise); the ledger states how much the existing cases moved.

Human reviewer:
James Ross (maintainer); review pending.
