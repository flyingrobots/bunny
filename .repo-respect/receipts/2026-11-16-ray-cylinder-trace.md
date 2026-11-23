# Receipt: Ray Cylinder Trace

Task:
Add a trace-domain ray-cylinder kernel to `bunny-query` (flyingrobots/bunny#360): the nearest
hit distance, facing and outward normal of a `TraceRay` with a capped cylinder whose axis is
the object-space `+Y` axis, in the form of `SphereHit`, under the contract identity
`bunny-ray-cylinder/1`, with a stated tie rule where the side meets a cap.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/coordinate-law/README.md
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
crates/bunny-query/src/trace/cylinder.rs
crates/bunny-query/src/trace/sphere.rs
crates/bunny-query/tests/trace_cylinder_tests.rs
crates/bunny-query/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-16-ray-cylinder-trace.md

Topic docs:
`docs/topics/ray-tracing/`: the README states the cylinder kernel, its axis, its domain, its
typed error, the tie rule and the contract `bunny-ray-cylinder/1` with its rounding rule; the
test plan records the requirements, cases, oracle and evidence.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The side's discriminant and the exact comparison of a cap crossing with a side root need
256-bit intermediates; both use the 256-bit helper of the sphere kernel.

Human reviewer:
Pending: the project lead reviews this pull request.
