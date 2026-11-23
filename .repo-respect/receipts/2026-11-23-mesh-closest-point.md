# Receipt: Mesh Closest Point

Task:
flyingrobots/bunny#246: closest-point-to-mesh queries through the BVH, and a mesh input layout
connecting `bunny-mesh` buffers to a BVH and the trace kernels. `PointQuery::closest_on_triangle`
(`bunny-query`) finds the exact closest point of a closed triangle to a trace-domain point
(degenerate triangles as their segment or point) in 256-bit integers, compares exactly in 512
bits, and rounds the point and the squared distance once each, to nearest, ties to even;
`PointQuery::box_distance_bound` is the conservative box bound. `closest_point` (`bunny-broadphase`)
is a nearest-first BVH search equal to a test of every primitive, lowest index on exact ties.
`MeshView`, `MeshBvh` and `MeshError` (`bunny-mesh`) are a borrowed Rust view, not a serialized
format. Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-query/src/{lib.rs,closest.rs,trace.rs,trace/wide.rs,barycentric.rs}
crates/bunny-broadphase/src/{lib.rs,bvh.rs,bvh/build.rs,closest_hit.rs,traversal.rs,compact.rs,utils.rs}
crates/bunny-broadphase/tests/{closest_hit_tests.rs,bvh_allocation_tests.rs}
crates/bunny-mesh/src/lib.rs, crates/bunny-codec/src/ply_ascii.rs, crates/bunny-codec/Cargo.toml
crates/bunny-codec/tests/ply_ascii_stanford_tests.rs and its fixture README
crates/bunny-optics/tests/exact/mod.rs (oracle pattern only)
xtask/src/{code_dojo.rs,feature_configs.rs,topic_docs.rs}, scripts/publish-crates.sh
docs/topics/{ray-tracing,bvh-construction,release-packaging,rotation,lint-policy}/ (chapters and plans)
docs/README.md, docs/NUMERIC_CONSTITUTION.md, flyingrobots/bunny#246

Files edited:
crates/bunny-query/src/point_query.rs, point_query/feature.rs, point_query/wide.rs (new)
crates/bunny-query/src/lib.rs (module and exports, at the end)
crates/bunny-broadphase/src/closest_point.rs (new), crates/bunny-broadphase/src/lib.rs (exports)
crates/bunny-mesh/src/mesh_bvh.rs (new), crates/bunny-mesh/src/lib.rs, crates/bunny-mesh/Cargo.toml
(new dependencies bunny-query and bunny-broadphase, path crates), Cargo.lock
crates/bunny-codec/Cargo.toml (dev-dependencies; the new fixture test excluded from the package)
Tests (new): crates/bunny-query/tests/point_triangle_tests.rs,
crates/bunny-query/tests/support/point_triangle_oracle.rs,
crates/bunny-broadphase/tests/closest_point_tests.rs, closest_point_allocation_tests.rs,
crates/bunny-mesh/tests/mesh_bvh_tests.rs, crates/bunny-codec/tests/mesh_bvh_stanford_tests.rs
docs/topics/mesh-queries/README.md and test-plan.md (new: MQ-REQ-001 to -010, MQ-TP-001 to -040)
docs/README.md (topic row), docs/topics/release-packaging/README.md (file-rule row),
docs/NUMERIC_CONSTITUTION.md (overflow-policy row), CHANGELOG.md, crate READMEs of bunny-query,
bunny-broadphase and bunny-mesh
.repo-respect/receipts/2026-11-23-mesh-closest-point.md

Topic docs:
docs/topics/mesh-queries/ is a new topic with its own MQ- prefix; no other topic uses it. The
ray-tracing and bvh-construction chapters are not edited.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (the kernels do not
exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch; cargo clippy on the four touched
crates with -D warnings. Nineteen mutations applied, run and restored (the test plan's table).
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's Dojo
job), the code-dojo-no-std gate (needs rustup) and scripts/publish-crates.sh.

Known risks:
`bunny-mesh` gains two normal dependencies (path crates published before it). The mesh layout is
a Rust API shape chosen here; the issue fixed none of its details.

Human reviewer:
James Ross (maintainer); review pending.
