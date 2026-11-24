# Receipt: 2D Primitives And Queries

Task:
flyingrobots/bunny#149, first slice: 2D primitives in `bunny-geom` beside the 3D ones
(`FixedSegment2`, `FixedRay2`, `FixedAabb2`, `FixedCircle2`, contract `bunny-planar/1`) and
exact closed-boundary queries: box containment, overlap and closest point; circle
containment and overlap with a circle or a box; segment containment, intersection and
closest point. Orientation is the exact `orientation2` of flyingrobots/bunny#145; the
predicates' private 256-bit integer moves from `src/predicates/wide.rs` to `src/wide.rs`
(code unchanged; items `pub` with `unreachable_pub` allowed on the module, the
`bunny-optics` pattern) so both modules can use it. Additive: no existing public
item, output bit, golden, digest or ID changes; `GeomError` gains no variant.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-shape-library.md
crates/bunny-geom/src/{lib.rs,predicates.rs,predicates/wide.rs,predicates/barycentric.rs,shapes.rs,shapes/triangle.rs,shapes/capsule.rs,plane.rs,bounds.rs}
crates/bunny-geom/tests/{predicates_oracle/mod.rs,predicate_orientation_tests.rs,predicate_sweep_tests.rs}
crates/bunny-linalg/src/{lib.rs,fixed_vec2.rs}
crates/bunny-query/src/closest.rs, crates/bunny-broadphase/src/utils.rs (3D conventions)
docs/README.md, docs/topics/predicates/, xtask/src/topic_docs.rs (ledger rules)
flyingrobots/bunny#149

Files edited:
crates/bunny-geom/src/planar.rs and crates/bunny-geom/src/planar/segment.rs (new)
crates/bunny-geom/src/wide.rs (moved from src/predicates/wide.rs; header and visibility only)
crates/bunny-geom/src/{lib.rs,predicates.rs,predicates/barycentric.rs} (module and import lines)
crates/bunny-geom/tests/planar_oracle/mod.rs (new: independent oracle on the predicates' big integer)
crates/bunny-geom/tests/{planar_tests,planar_sweep_tests}.rs (new)
docs/topics/planar-geometry/README.md and test-plan.md (new)
docs/topics/predicates/README.md and test-plan.md (the moved wide.rs path only)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic index),
docs/MATH_GEOMETRY_CAPABILITY_MAP.md (current surface list)
.repo-respect/receipts/2026-11-23-planar-primitives.md

Topic docs:
docs/topics/planar-geometry/ is a new topic with its own PLANAR- prefix (PLANAR-REQ-001 to
-009, PLANAR-TP-001 to -012); no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the planar tests fail to compile (the types do
not exist); topic-docs and repo-respect check --branch clean.
Implementation commit, each its own command (local Rust 1.100.0; CI pins 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch; cargo build and cargo test
-p bunny-geom --no-default-features. cargo clippy --locked -p bunny-geom --all-targets
--all-features -D warnings reports nothing in the new or changed files (it reports
newer-toolchain lints in the existing polygon_clip_tests.rs only).
Mutations: twenty-eight applied to planar.rs, planar/segment.rs and wide.rs with the two
planar test files run and the sources restored; twenty-seven fail at least one case, one is
equivalent (test-plan table, with the argument).
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them), code-dojo-no-std (needs rustup).

Known risks:
The closed boundary, the empty-set reading of inverted boxes and negative radii, the reuse
of `GeomError::NegativeSphereRadius` for a circle, and putting the 2D queries on the types
in bunny-geom rather than in bunny-query are choices the issue leaves open. The sweeps check
seeded samples, not every input; exactness rests on the 256-bit widths stated in wide.rs.

Human reviewer:
James Ross (maintainer); review pending.
