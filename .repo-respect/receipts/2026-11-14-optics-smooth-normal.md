# Receipt: Exact Barycentric Numerators and the Interpolated Shading Normal

Task:
Part 2 of flyingrobots/bunny#289 (smooth shading normals). An additive public accessor on
`bunny-query`'s `TriangleHit` for the exact barycentric numerators and determinant
(`barycentric_numerators`, type `BarycentricNumerators`), and `bunny-optics`'s `smooth_normal`
(`bunny-smooth-normal/1`): the vertex normals interpolated with those weights, shifted right by a
common amount, combined exactly and normalized once, with a stated bound and a golden input on
the boundary of every rounding. The shading frame built from it, its rule against the geometric
normal, the bounce rejection and the cost measurement are later parts; no existing output changes.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
docs/topics/optics/README.md and test-plan.md
docs/topics/ray-tracing/README.md and test-plan.md
docs/topics/performance/README.md (Fidelity Rule)
crates/bunny-query/src/trace.rs, lib.rs; crates/bunny-query/tests/trace_tests.rs
`crates/bunny-optics/src/shading/**`
crates/bunny-linalg/src/fixed_vec3.rs and lib.rs (normalize, length, FixedUnitVec3)
crates/bunny-num/src/fixed_q32_32.rs (division rounding)
flyingrobots/bunny#298 and #288 (untested roundings of bunny-diffuse-shading/1; not fixed here)

Files edited:
crates/bunny-query/src/barycentric.rs (new), lib.rs, trace.rs
crates/bunny-query/tests/trace_tests.rs (TRACE-TP-313, TRACE-TP-314)
crates/bunny-query/README.md
crates/bunny-optics/src/shading/smooth_normal.rs (new), shading.rs, lib.rs
crates/bunny-optics/tests/smooth_normal_tests.rs (new)
crates/bunny-optics/tests/exact/mod.rs (new: the big-integer comparison, moved out of
vertex_normal_tests.rs, which now uses it)
crates/bunny-optics/tests/vertex_normal_tests.rs
crates/bunny-optics/README.md
docs/topics/optics/README.md (Smooth Normal), test-plan.md (OPTICS-REQ-047, -048;
OPTICS-TP-074 to -078)
docs/topics/ray-tracing/README.md (Exact Triangle Test), test-plan.md (TRACE-REQ-030;
TRACE-TP-313, -314)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-smooth-normal.md

Topic docs:
docs/topics/optics/README.md (Smooth Normal, Status, Accuracy And Cost)
docs/topics/ray-tracing/README.md (Status, Exact Triangle Test)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: with the new tests in place and the crates' exports unchanged, `trace_tests` and
`smooth_normal_tests` fail to build (unresolved `BarycentricNumerators`,
`barycentric_numerators`, `smooth_normal`, `SMOOTH_NORMAL_ID`).
Green: every bunny-optics and bunny-query test passes. Golden values came from a separate Python
model written from the chapter (unbounded integers, each rounding switchable), not committed;
the boundary inputs of OPTICS-TP-075 were constructed with it and each distinguishes its rule
in the model before the Rust test was run.
Mutations (test plan, Smooth-Normal Mutation Checks): eight, each with `timeout 120`; each fails
at least one case; the restored source passes.
cargo fmt, clippy -D warnings (workspace, all targets, all features).
code-dojo --all at the final commit (see the PR).

Known risks:
The bound grows as 2^-26 / rho for nearly cancelling normals; the oracle counts, but does not
check, shifted cases with rho below 2^-26. The oracle's exact sum is computed in i128, so its
seeded det stays below 2^93; larger dets (up to 2^117 from real hits) are covered by a golden
vector and the bound's derivation only. The shared-normal step makes a corner exact, so the
normal can move by up to a raw unit between a corner and a point next to it.

Human reviewer:
James Ross (maintainer); review pending.
