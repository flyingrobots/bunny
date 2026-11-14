# Receipt: Reject Triangle Hits Beyond The Limit Before Dividing

Task:
Implement flyingrobots/bunny#214: a conservative pre-check in `TraceRay::hit_triangle` that
rejects a crossing certainly beyond `t_max` before the long division, as an exact
replacement (no output bit changes), with a retained reference oracle, counts of how often it
fires on the benchmark scene, mutation checks and a ledger entry.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/trace.rs
crates/bunny-query/tests/trace_tests.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/scene.rs

Files edited:
(in progress)

Checks run:
(in progress)

Human reviewer:
James Ross (maintainer); review pending.
