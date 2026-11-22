# Receipt: bunny-optics Area-Weighted Vertex Normals

Task:
Part 1 of flyingrobots/bunny#289 (smooth shading normals): per-vertex normals for an indexed
triangle mesh in `bunny-optics`, as a new additive API with its own identity
(`bunny-vertex-normals/1`). Area weighting from the exact `i128` cross products, one
normalization per vertex with `TriangleSurface::new`'s rule, `None` for a vertex with no face or
a zero sum, typed refusals for meshes outside the domain. The interpolated shading normal and
its consistency rule with the geometric normal are part 2 and are not in this change; no existing
function's output changes.

Files read:
CODE_STANDARDS.md (Rules 0 to 12)
docs/CODE_DOJO.md
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/performance/README.md (Fidelity Rule, Measuring, ledger)
`crates/bunny-optics/src/**` (shading module, wide arithmetic)
crates/bunny-query/src/trace.rs (TraceTriangle, TriangleHit, TRACE_COORD_LIMIT_RAW)
crates/bunny-linalg/src/lib.rs and fixed_vec3.rs (FixedUnitVec3::new, normalize)
crates/bunny-num/src/fixed_q32_32.rs (division rounding)
crates/bunny-mesh/src/lib.rs (Triangle32)
`crates/bunny-perf/src/**` (cases, shade group)
xtask/src/code_dojo.rs, xtask/src/topic_docs.rs, xtask/src/perf.rs
Lumen's crates/lumen/src/render.rs and mesh/ply.rs on origin/main (read only: the first caller)

Files edited:
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/shading.rs
crates/bunny-optics/src/shading/vertex_normals.rs (new)
crates/bunny-optics/src/shading/vertex_normals/tests/mod.rs (new)
crates/bunny-optics/tests/vertex_normal_tests.rs (new)
crates/bunny-optics/README.md
crates/bunny-perf/Cargo.toml, crates/bunny-perf/src/main.rs, crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/normals.rs (new)
docs/topics/optics/README.md
docs/topics/optics/test-plan.md (OPTICS-REQ-044 to -046; OPTICS-TP-066 to -073)
docs/topics/performance/README.md (ledger entry 12)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-vertex-normals.md

Topic docs:
docs/topics/optics/README.md (Vertex Normals, Accuracy And Cost, Status)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: with the new tests in place and the module not yet exported, `cargo test --locked -p
bunny-optics --test vertex_normal_tests` fails to build (unresolved imports `VertexNormals`,
`VertexNormalError`, `MAX_NORMAL_TRIANGLES`, `VERTEX_NORMALS_ID`).
Green: the seven new cases pass; the golden vectors of OPTICS-TP-072 were produced by the crate,
checked by the test's big-integer oracle and separately by an exact Python computation (each is
the correctly rounded unit vector).
Mutations (test plan, Vertex-Normal Mutation Checks): seven, each with `timeout 120`; each fails
at least one case; the restored source passes.
cargo fmt, clippy -D warnings (workspace, all targets) and the bunny-optics tests.
Timing: `cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
--filter shade/` through devrun with --exclusive, two runs (Apple M5 Pro, rust-dev:1.96.0, base
3c395f9950ac). shade/vertex_normals (new) 68.08 and 68.48 ns p50 per vertex; the unchanged
shade/ cases moved -0.5 % to +0.6 % in median p50. Recorded as ledger entry 12 in the
performance chapter.
code-dojo --all at the final commit (see the PR).

Known risks:
The triangle-count refusal (`TooManyTriangles`, more than 2^32 triangles) cannot be reached
through the public API in a test; OPTICS-TP-069 tests the private check directly. Vertex
normals are computed from the positions given: a caller that transforms its mesh must compute
them from the transformed positions. Faces wound inconsistently cancel at shared vertices.

Human reviewer:
James Ross (maintainer); review pending.
