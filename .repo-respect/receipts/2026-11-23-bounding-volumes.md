# Receipt: Bounding Volumes

Task:
flyingrobots/bunny#151: box helpers on `FixedAabb3` in `bunny-geom` under the contract identity
`bunny-bounds/1` (`from_points`, `from_triangle`, `merge`, `expand`, `expand_axes`,
`from_sphere`, `bounding_sphere`, with the typed `BoundsError`), `mesh_bounds` in `bunny-mesh`
(with `MeshBoundsError`), and dedicated tests of the existing `union_aabb`,
`TraceSphere::bounds` and `TraceTriangle::bounds` as they behave today. Additive: no existing
public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, docs/NUMERIC_CONSTITUTION.md
.repo-respect/receipts/2026-11-23-transform-composition.md
scripts/publish-crates.sh, crates/*/Cargo.toml (dependency direction)
crates/bunny-geom/src/{lib.rs,conversions.rs}, crates/bunny-geom/tests/geom_tests.rs
crates/bunny-broadphase/src/utils.rs, crates/bunny-broadphase/tests/common/mod.rs
crates/bunny-query/src/trace.rs, trace/{sphere.rs,box_hit.rs}
crates/bunny-query/tests/trace_sphere_tests.rs
crates/bunny-mesh/src/{lib.rs,hash.rs}
crates/bunny-num/src/{lib.rs,fixed_q32_32.rs} (sqrt_u128)
crates/bunny-linalg/tests/transform_tests.rs (seeded-test pattern)
xtask/src/{topic_docs.rs,code_dojo.rs} (ledger rules, file limits)
docs/topics/{transform,lerp}/test-plan.md, docs/topics/ray-tracing/README.md (box inversion)
flyingrobots/bunny#151; open pull requests flyingrobots/bunny#469 and #471 (file lists)

Files edited:
crates/bunny-geom/src/bounds.rs (new: the helpers, BoundsError, BOUNDS_CONTRACT_ID)
crates/bunny-geom/src/lib.rs (module and export, at the end of the lists)
crates/bunny-mesh/src/bounds.rs (new: mesh_bounds, MeshBoundsError)
crates/bunny-mesh/src/lib.rs (module and export, at the end of the lists)
crates/bunny-geom/tests/bounds_tests.rs (new)
crates/bunny-mesh/tests/mesh_bounds_tests.rs (new)
crates/bunny-broadphase/tests/union_aabb_tests.rs (new)
crates/bunny-query/tests/trace_bounds_tests.rs (new)
crates/bunny-geom/README.md, crates/bunny-mesh/README.md, CHANGELOG.md
docs/README.md (topic index row, sorted)
docs/NUMERIC_CONSTITUTION.md (one overflow-audit row)
docs/topics/bounding-volumes/README.md (new chapter)
docs/topics/bounding-volumes/test-plan.md (new: BOUNDS-REQ-001 to -008, BOUNDS-TP-001 to -016)
.repo-respect/receipts/2026-11-23-bounding-volumes.md

Topic docs:
docs/topics/bounding-volumes/ is a new topic with its own BOUNDS- prefix; no other topic uses
it. No other topic's ledger is edited.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): cargo test --locked -p bunny-geom --test
bounds_tests fails to compile (the helpers do not exist); topic-docs and repo-respect check
--branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (128
test result lines, all ok); cargo run --locked -p xtask -- code-dojo-generated;
code-dojo-rust --all; code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs;
repo-respect check --branch. cargo clippy --locked -D warnings on bunny-geom and bunny-mesh
(lib and the new test) and on the new bunny-query and bunny-broadphase tests: clean.
RUSTDOCFLAGS="-D warnings" cargo doc for bunny-geom and bunny-mesh: clean. cargo build -p
bunny-mesh --no-default-features: clean. git diff --check: clean.
Mutations: fifteen, each applied to the kernel, the four new test files run with cargo test
--locked -q --no-fail-fast, and restored; each failed at least one case (the test plan's
table).
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

Known risks:
The existing bunny_broadphase::utils::union_aabb does not refuse inverted boxes and can turn
two inverted boxes into an ordered one; it is pinned by test and not changed here (a finding
for the owner). bunny-broadphase's union_aabb and the new FixedAabb3::merge overlap on
ordered boxes.

Human reviewer:
James Ross (maintainer); review pending.
