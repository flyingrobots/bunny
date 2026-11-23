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
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-query/src/trace/cylinder.rs
crates/bunny-query/src/trace/cylinder/span.rs
crates/bunny-query/tests/trace_disc_tests.rs

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
bound and the identity constant; the test plan corrects TRACE-REQ-041 and TRACE-REQ-042 and TRACE-TP-501, adds
TRACE-TP-701 to TRACE-TP-707 and records the mutations. The crate README and the changelog
state the corrected rule and the type change.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
The normal costs one more 256-bit square root per hit. `SphereHit::normal` changes type from
`FixedVec3` to `FixedUnitVec3`; no caller exists in the workspace. The ray-cylinder kernel
calls the sphere's `roots`, now a thin wrapper over the same quadratic, and its tests pass
unchanged. A prescale of `[2^60, 2^61)` instead of `[2^61, 2^62)` is not detected: it gives
the same normal on every generated hit.

Human reviewer:
Pending: the project lead reviews this pull request.
