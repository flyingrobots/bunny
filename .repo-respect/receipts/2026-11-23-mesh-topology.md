# Receipt: Mesh Topology

Task:
flyingrobots/bunny#122: topology validation for quantized triangle meshes under the contract
identity `bunny-mesh-topology/1` (`MESH_TOPOLOGY_CONTRACT_ID`). `validate_topology` reports
index-degenerate triangles, geometrically degenerate triangles (exact `i64` cross product on
the `u16` coordinates, no tolerance), exact and flipped duplicate faces, and boundary,
winding-inconsistent and non-manifold edges as `TopologyFinding` records in the derived `Ord`
order, with a fixed severity per kind. It does not allocate: the caller provides `n` face
slots, `3 n` edge slots and the output slice, and a `TopologyReport` counts every finding.
Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-mesh/{Cargo.toml,README.md,src/lib.rs,src/hash.rs,tests/*.rs}
crates/bunny-codec/{Cargo.toml,README.md,src/ply_ascii.rs,src/ply_ascii/*.rs}
crates/bunny-codec/tests/ply_ascii_stanford_tests.rs and tests/fixtures/ (README files)
scripts/publish-crates.sh (packaged-tests), xtask/src/code_dojo.rs (category limits),
xtask/src/topic_docs.rs (ledger rules)
docs/topics/{code-dojo-determinism,rotation,release-packaging,coordinate-law}/README.md,
docs/topics/rotation/test-plan.md, docs/README.md
flyingrobots/bunny#122

Files edited:
crates/bunny-mesh/src/topology.rs and crates/bunny-mesh/src/topology/{edges,error,faces,
finding,scratch,triangles}.rs (new: the kernel)
crates/bunny-mesh/src/lib.rs (module and export lines at the end)
crates/bunny-mesh/tests/topology_tests.rs (new)
crates/bunny-codec/tests/ply_ascii_stanford_topology_tests.rs (new: the Stanford fixture test)
crates/bunny-codec/Cargo.toml (the new test joins the asset in the package `exclude` list)
crates/bunny-codec/README.md, docs/topics/release-packaging/README.md (the exclusion list)
crates/bunny-mesh/README.md, CHANGELOG.md, docs/README.md (topic index)
docs/topics/mesh-topology/README.md (new chapter)
docs/topics/mesh-topology/test-plan.md (new: MTOPO-REQ-001 to -007, MTOPO-TP-001 to -016;
mutations)
.repo-respect/receipts/2026-11-23-mesh-topology.md

Topic docs:
docs/topics/mesh-topology/ is a new topic with its own MTOPO- prefix; no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the topology tests fail to compile (the kernel
does not exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check
--branch. cargo clippy --locked -p bunny-mesh -p bunny-codec --all-targets --all-features
-D warnings clean.
Mutations: eighteen, each applied to the kernel, run with the two topology test files and
restored; seventeen failed at least one case, and M03 is an equivalent mutant (the test
plan's table and note).
Python count over the raw Stanford PLY face list: the six combinatorial counts the test pins.
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them), the packaged-tests step of
scripts/publish-crates.sh (it calls cargo +1.96.0, which needs rustup).

Known risks:
The severities, the exclusion of index-degenerate faces from later checks, and the reporting
of each duplicate against the lowest-indexed face are choices the issue does not fix; the
pull request lists them for the owner.

Human reviewer:
James Ross (maintainer); review pending.
