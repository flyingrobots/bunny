# Receipt: Shape Library

Task:
flyingrobots/bunny#115 (the remainder): geometry-level shape types in `bunny-geom` beside the
existing ray, box, sphere, segment and plane. `FixedLine3` (point and `FixedUnitVec3`, the
ray's normalization rule), `FixedTriangle3` (vertex order kept; exact degeneracy test of the
raw cross product), `FixedCapsule3` (segment and non-negative radius) and `FixedObb3`
(centre, axis columns within the `bunny-rotation/1` column tolerances of 11 and 12 raw units,
non-negative half extents), refusing with a new `#[non_exhaustive]` `ShapeError`; float
boundary constructors for those four and for `FixedSegment3` and `FixedPlane3`; a new
shape-library chapter listing all six shapes' invariants. Additive: no existing public item,
output bit, golden, digest or ID changes; `GeomError` is unchanged.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-geom/src/{lib.rs,conversions.rs,plane.rs,plane/span.rs,plane/polygon.rs}
crates/bunny-geom/tests/geom_tests.rs and the plane test files (conventions only)
crates/bunny-linalg/src/{lib.rs,fixed_vec3.rs,fixed_mat3.rs}
crates/bunny-num/src/fixed_q32_32/conversions.rs
crates/bunny-query/src/{trace.rs,trace/sphere.rs,trace/box_hit.rs,trace/disc.rs,trace/cylinder.rs}
crates/bunny-linalg/tests/rotation_tests.rs (the bound definitions)
docs/NUMERIC_CONSTITUTION.md, docs/topics/{rotation,look-rotation,shape-frame,plane-clipping,lerp}/
xtask/src/topic_docs.rs (ledger rules)
flyingrobots/bunny#115

Files edited:
crates/bunny-geom/src/shapes.rs and crates/bunny-geom/src/shapes/{line,triangle,capsule,obb,float}.rs (new)
crates/bunny-geom/src/lib.rs (one module line and one export block, at the end)
crates/bunny-geom/tests/shape_oracle/mod.rs (new: independent 256-bit oracle)
crates/bunny-geom/tests/{shape_line_capsule_tests,shape_triangle_tests,shape_obb_tests,shape_float_tests}.rs (new)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic index),
docs/MATH_GEOMETRY_CAPABILITY_MAP.md (current surface list)
docs/topics/shape-library/README.md and test-plan.md (new)
.repo-respect/receipts/2026-11-23-shape-library.md

Topic docs:
docs/topics/shape-library/ is a new topic with its own SHAPE- prefix (SHAPE-REQ-001 to -008,
SHAPE-TP-001 to -012); no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the shape tests fail to compile (the types do not
exist); topic-docs and repo-respect check --branch clean.
Implementation commit, each its own command (local Rust 1.100.0; CI pins 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch. cargo clippy --locked -p
bunny-geom --all-targets --all-features -D warnings reports nothing in the new files (it
reports newer-toolchain lints in the existing polygon_clip_tests.rs only).
Mutations: sixteen applied to the shape sources with the four shape test files run and the
sources restored; fourteen fail at least one case, two are equivalent (test-plan table).
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them), code-dojo-no-std (needs rustup).

Known risks:
The oriented-box tolerances are a choice (the widest rotation contract, 11 and 12 raw units);
a caller with a looser basis is refused. Acceptance of every rotation constructor's output
rests on those chapters' proven bounds; the tests check seeded samples, not every input.

Human reviewer:
James Ross (maintainer); review pending.
