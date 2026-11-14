# Receipt: Ray Packets For Coherent Camera Rays (Spike)

Task:
Spike for flyingrobots/bunny#217: build the simplest correct packet traversal
(`closest_hits` in `bunny-broadphase`), measure it against `closest_hit` one ray at a time
on the benchmark scene, and record the result in the performance chapter either way.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/test-plan.md
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-query/src/trace.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/profile.rs

Files edited:
(in progress; completed in the last commit of the branch)

Topic docs:
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-028, TRACE-TP-111..116)

Generated artifacts:
None.

Checks run:
(in progress)

Known risks:
(in progress)

Human reviewer:
James Ross (maintainer); review pending.
