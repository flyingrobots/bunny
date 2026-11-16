# Receipt: Ray Box Trace

Task:
Add a trace-domain ray-box kernel to `bunny-query` (flyingrobots/bunny#359): the nearest hit
distance, facing and face normal of a `TraceRay` with an axis-aligned box in object space, in
the same form as `SphereHit`, under the contract identity `bunny-ray-box/1`.
`ray_intersects_aabb` and `TraceRay::box_entry` are unchanged.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-query/src/lib.rs
crates/bunny-query/src/trace.rs
crates/bunny-query/src/trace/sphere.rs
crates/bunny-query/tests/trace_sphere_tests.rs
crates/bunny-geom/src/lib.rs

Files edited:
CHANGELOG.md
.repo-respect/receipts/2026-11-15-ray-box-trace.md

Topic docs:
`docs/topics/ray-tracing/`: the README will state the box kernel, its domain, its typed
error, its tie rule, the parallel-ray rule and the contract `bunny-ray-box/1` with its
rounding rule; the test plan will record the requirements, cases, oracle and evidence.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The tie rule for edges and corners and the rule for a ray parallel to a slab are choices the
issue leaves to the implementation; both are stated in the README and tested.

Human reviewer:
Pending: the project lead reviews this pull request.
