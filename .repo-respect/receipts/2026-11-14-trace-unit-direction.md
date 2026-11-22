# Receipt: from_ray And The Unit Direction Bound

Task:
flyingrobots/bunny#292. `TraceRay::from_ray` returned `None` for the unit vector
`(2^32 + 1, 0, 0)` and its negation on every axis, which `FixedUnitVec3::try_from_unit`
accepts, although its documentation said it refuses only an origin outside the trace domain.
Decision: option (a), accept the component in `from_ray` only (`new` keeps its bound), after
redoing every line of the width argument; additive, no existing `Some` changes.

Files read:
crates/bunny-query/src/trace.rs, src/barycentric.rs, src/lib.rs
crates/bunny-linalg/src/lib.rs, src/fixed_vec3.rs (unit tolerance, normalization)
crates/bunny-query/tests/trace_tests.rs, ray_aabb_overflow_tests.rs, support/volume_oracle.rs
docs/topics/ray-tracing/README.md and test-plan.md
crates/bunny-optics/src/camera.rs, shading/smooth_hit.rs (users of the bound)

Files edited:
crates/bunny-query/src/trace.rs, src/barycentric.rs, src/lib.rs
crates/bunny-query/tests/trace_unit_direction_tests.rs (new)
docs/topics/ray-tracing/README.md, test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-trace-unit-direction.md

Topic docs:
docs/topics/ray-tracing/README.md (trace domain, widths, numerators) and test-plan.md
(TRACE-REQ-001 reworded, TRACE-REQ-032, TRACE-TP-408 to TRACE-TP-411).

Generated artifacts:
None.

Checks run:
Red first: with `from_ray` using the old bound, TRACE-TP-408, 410 and 411 fail ("from_ray refused
[4294967297, 0, 0] with origin 0"). Mutants (each under `timeout 120`, sources touched before the
rebuild): `new` given the wider bound fails trace_tp_006; both functions given a bound one raw
unit smaller fail seven trace_tests cases. Container: `cargo fmt --check`, clippy `-D warnings`
on bunny-query, and the tests of bunny-query, bunny-broadphase and bunny-optics pass. The full
gate result is in the pull request.

Known risks:
The new public constant is additive API. `TraceRay::new` and every kernel function are
unchanged, so only `from_ray` changes behaviour, and only for inputs it refused.

Human reviewer:
Pending: the project lead reviews this pull request.
