# Receipt: Whole-Scene Ray Benchmark

Task:
Implement flyingrobots/bunny#210: the `scene` and `trace` feature groups in `bunny-perf`
(a 27 650-triangle scene, 1 024 camera rays, closest hit through the visitor API and through
`closest_hit`), a harness check that both choose the same primitive on every ray,
`bunny-broadphase` in the `xtask perf` base crates, and the closest-hit ledger entry.
No library function changes.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/timing.rs
crates/bunny-broadphase/src/closest_hit.rs
xtask/src/perf.rs

Files edited:
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
xtask/src/perf.rs
Cargo.lock
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-scene-benchmark.md

Topic docs:
docs/topics/performance/README.md (ledger entry 3, adding-a-case, feature groups)
docs/topics/performance/test-plan.md (PERF-REQ-006, PERF-TP-024..027)

Generated artifacts:
None.

Checks run:
See the pull request description and the final commit; all runs are in the rust-dev
container through devrun.

Known risks:
The ledger ratio is for one synthetic scene on one machine; it is not a general claim.

Human reviewer:
James Ross (maintainer); review pending.
