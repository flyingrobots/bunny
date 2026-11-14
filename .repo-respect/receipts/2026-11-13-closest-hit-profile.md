# Receipt: Closest-Hit Profile

Task:
Implement flyingrobots/bunny#212: harness-only counters for `closest_hit` and `any_hit` on the
benchmark scene (a replica of the search loop in `bunny-perf`, checked against the shipped
functions on every ray), kernel replay timing cases, and a written profile in the performance
chapter. No optimization and no change to any library crate.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/src/bvh/build.rs
crates/bunny-query/src/trace.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/cases.rs
xtask/src/perf.rs

Files edited:
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/cases.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-closest-hit-profile.md

Topic docs:
docs/topics/performance/README.md (profile section, harness options, kernel cases)
docs/topics/performance/test-plan.md (PERF-REQ-007, PERF-TP-028..033)

Generated artifacts:
None.

Checks run:
See the pull request description and the final commit; all runs are in the rust-dev
container through devrun.

Known risks:
The profile describes one synthetic scene on one machine. The time split is an estimate
bracketed by two replay methods, not a measurement of the shipped traversal.

Human reviewer:
James Ross (maintainer); review pending.
