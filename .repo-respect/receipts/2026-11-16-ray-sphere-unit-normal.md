# Receipt: Ray Sphere Unit Normal

Task:
Correct the sphere kernel's normal under `bunny-ray-sphere/1` (flyingrobots/bunny#422): a
`FixedUnitVec3` at the exact root by Bunny's unit-vector rule, with a derived per-component
bound, an oracle that shares no expression with the kernel, range-limit tests and a public
`RAY_SPHERE_ID`. Distances are unchanged.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/optics/README.md (unit-vector rule)
crates/bunny-query/src/trace/sphere.rs
crates/bunny-query/src/trace/wide.rs
crates/bunny-query/src/trace/box_hit.rs
crates/bunny-query/tests/trace_box_tests.rs
crates/bunny-optics/src/shading/wide.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/src/fixed_vec3.rs

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
.repo-respect/receipts/2026-11-16-ray-sphere-unit-normal.md

Topic docs:
`docs/topics/ray-tracing/`: the Ray-Sphere section states the corrected normal, its derived
bound and the identity constant; the test plan gains the requirements, cases and mutations.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The normal costs one more 256-bit square root per hit.

Human reviewer:
Pending: the project lead reviews this pull request.
