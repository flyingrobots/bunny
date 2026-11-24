# Receipt: 2D Ray Queries And Point In Polygon

Task:
flyingrobots/bunny#149, second slice, on top of the first (flyingrobots/bunny#495): exact
closed ray queries for `FixedRay2` against `FixedAabb2`, `FixedCircle2` and `FixedSegment2`
(a hit decision and the first hit parameter rounded once, ties to even, in a new `RayHit2`),
and a borrowed `FixedPolygon2` with an exact three-way point in polygon (`PolygonLocation`)
on `orientation2` signs. Additive: no existing public item, output bit, golden, digest or ID
changes; `GeomError` gains no variant.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-planar-primitives.md
crates/bunny-geom/src/{lib.rs,planar.rs,planar/segment.rs,wide.rs,predicates.rs,predicates/barycentric.rs}
crates/bunny-geom/tests/{planar_oracle/mod.rs,predicates_oracle/mod.rs,planar_tests.rs,planar_sweep_tests.rs}
crates/bunny-linalg/src/lib.rs (the unit-vector tolerance), crates/bunny-query/src/ray.rs (3D ray conventions)
docs/topics/planar-geometry/, xtask/src/topic_docs.rs (ledger statuses)
flyingrobots/bunny#149, flyingrobots/bunny#495

Files edited:
crates/bunny-geom/src/planar/{ray.rs,ray/root.rs,polygon.rs} (new kernels; root.rs holds a unit test)
crates/bunny-geom/src/planar.rs, crates/bunny-geom/src/lib.rs (module and export lines)
crates/bunny-geom/src/wide.rs (adds `from_unsigned` and `isqrt`, and a unit test; existing code unchanged)
crates/bunny-geom/tests/planar_cast_oracle/mod.rs (new: independent oracle)
crates/bunny-geom/tests/{planar_ray_tests,planar_polygon_tests,planar_cast_sweep_tests}.rs (new)
docs/topics/planar-geometry/README.md and test-plan.md (PLANAR-REQ-010 to -015, PLANAR-TP-013 to -022)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic row), docs/MATH_GEOMETRY_CAPABILITY_MAP.md
.repo-respect/receipts/2026-11-24-planar-rays-polygons.md

Topic docs:
docs/topics/planar-geometry/ continues its own ranges: PLANAR-REQ-010 to -015 and
PLANAR-TP-013 to -022 (planned in the first commit, implemented in the second). No existing
ID changes.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (the ray
queries, `RayHit2`, `FixedPolygon2` and `PolygonLocation` do not exist); topic-docs and
repo-respect check --branch clean.
Implementation commit, each its own command (local Rust 1.100.0; CI pins 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features
(171 `test result: ok` lines, none failed); code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check
--branch; cargo build and cargo test -p bunny-geom --no-default-features. cargo clippy
--locked -p bunny-geom on the lib, its unit tests and the five planar test targets with -D
warnings reports nothing; the whole-crate run stops on newer-toolchain lints in the existing
polygon_clip_tests.rs only.
Mutations: thirty-three applied to planar/ray.rs, planar/ray/root.rs, planar/polygon.rs and
wide.rs with the lib and the three new test files run (RUSTFLAGS=--cap-lints=warn) and the
sources restored; thirty-two fail at least one case, one is equivalent (test-plan table,
with the argument).
A Python model (Fractions) reproduces every constructed ray golden; it is posted on the pull
request, not kept in the tree.
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's
Dojo job), code-dojo-no-std (needs rustup).

Known risks:
Returning a parameter as `RayHit2` (with `Beyond` for a parameter past `i64::MAX`), the
parameter zero for an origin in the shape, and the nonzero winding rule for unvalidated
polygons are choices the issue leaves open. The sweeps check seeded samples, not every input;
exactness rests on the widths stated in the rustdoc and wide.rs, and on the FixedUnitVec2
invariant that its length is within one raw unit of one.

Human reviewer:
James Ross (maintainer); review pending.
