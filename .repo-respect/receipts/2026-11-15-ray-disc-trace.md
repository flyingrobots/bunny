# Receipt: Ray Disc Trace

Task:
Add a trace-domain ray-disc kernel to `bunny-query` (flyingrobots/bunny#363): the hit
distance, facing and normal of a `TraceRay` with a disc in the plane `z = height` about the
`z` axis, in the form of `SphereHit`, under the contract identity `bunny-ray-disc/1`. The
plane hit is followed by an exact radius test on the exact hit point.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/lib.rs
crates/bunny-query/src/trace.rs
crates/bunny-query/src/trace/sphere.rs
crates/bunny-query/src/trace/wide.rs
crates/bunny-query/tests/trace_sphere_tests.rs

Files edited:
crates/bunny-query/src/lib.rs
crates/bunny-query/src/trace.rs
crates/bunny-query/src/trace/disc.rs
crates/bunny-query/src/trace/wide.rs
crates/bunny-query/tests/trace_disc_tests.rs
crates/bunny-query/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-15-ray-disc-trace.md

Topic docs:
`docs/topics/ray-tracing/`: the README states the disc kernel, its domain, its typed error
and the contract `bunny-ray-disc/1` with its rounding rule; the test plan records the
requirements, cases, oracle and evidence.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The radius test squares numerators up to `2^75`, which needs the crate's 256-bit helper.

Human reviewer:
Pending: the project lead reviews this pull request.
