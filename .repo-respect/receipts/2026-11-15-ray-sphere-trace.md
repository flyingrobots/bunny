# Receipt: Ray Sphere Trace

Task:
Add a trace-domain ray-sphere kernel to `bunny-query` (flyingrobots/bunny#358): the nearest
hit distance, facing and outward normal of a `TraceRay` with a sphere, in the same form as
`TraceTriangle`'s hit, built on Bunny's exact integer square root, under the contract
identity `bunny-ray-sphere/1`. `ray_intersects_sphere` is unchanged.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/lib.rs
crates/bunny-query/src/trace.rs
crates/bunny-query/src/ray.rs
crates/bunny-query/tests/trace_edge_oracle_tests.rs
crates/bunny-query/tests/support/trace_reference.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-geom/src/lib.rs

Files edited:
crates/bunny-query/src/lib.rs
crates/bunny-query/src/trace.rs
crates/bunny-query/src/trace/sphere.rs
crates/bunny-query/src/trace/wide.rs
crates/bunny-query/tests/trace_sphere_tests.rs
crates/bunny-query/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-15-ray-sphere-trace.md

Topic docs:
`docs/topics/ray-tracing/`: the README states the sphere kernel, its domain, its typed error
and the contract `bunny-ray-sphere/1` with its rounding rule; the test plan records the
requirements, cases, oracle and evidence.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The discriminant needs 256-bit intermediates; the crate's own two-limb helper computes them,
and the floor square root of a 256-bit value starts from `FixedQ32_32::sqrt_u128` on the
high limb and settles the low 64 bits one at a time.

Human reviewer:
Pending: the project lead reviews this pull request.
