# Receipt: Optics Pinhole Camera

Task:
Implement the Lumen goal slice of flyingrobots/bunny#138: a new `bunny-optics` crate with
a deterministic pinhole camera (position, look direction, up hint, tangent of half the
vertical field of view, integer image size) that generates `bunny_query::TraceRay`s per
pixel sample with integer arithmetic only, golden vectors, an exact image-plane oracle and
a deliberate-mutation check of the direction scaling rule. Projection matrices (#201,
#202), sampling sequences (#221) and shading or pixel values (#222, #160) are out of scope.

Files read:
docs/COORDINATE_LAW.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/design/0001-bunny-graphics-commons.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
docs/README.md
docs/PROCESS.md
docs/CODE_DOJO.md
docs/TESTING.md
docs/topics/repo-respect/README.md
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
README.md
clippy.toml
rustfmt.toml
Cargo.toml
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
scripts/publish-crates.sh
.github/workflows/code-dojo.yml
crates/bunny-query/src/trace.rs
crates/bunny-query/tests/trace_tests.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-geom/Cargo.toml

Files edited:
crates/bunny-optics/Cargo.toml
crates/bunny-optics/README.md
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/basis.rs
crates/bunny-optics/src/camera.rs
crates/bunny-optics/src/error.rs
crates/bunny-optics/src/plane.rs
crates/bunny-optics/src/point.rs
crates/bunny-optics/tests/camera_tests.rs
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/README.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
README.md
CHANGELOG.md
Cargo.toml
Cargo.lock
xtask/src/code_dojo.rs
scripts/publish-crates.sh
.github/workflows/code-dojo.yml
docs/TESTING.md
.repo-respect/receipts/2026-11-13-optics-camera.md

Topic docs:
docs/topics/optics/README.md (new chapter: inputs, construction refusals, camera frame,
image convention, ray generation, direction scaling and its proof, accuracy bound,
distances along unnormalized rays, accepted limits)
docs/topics/optics/test-plan.md (OPTICS-REQ-001..009, OPTICS-TP-001..009, implemented;
image-plane oracle; mutation checks)
docs/topics/coordinate-law/README.md and test-plan.md (camera image convention named,
pointing at the optics chapter)

Generated artifacts:
None. Golden vectors are hand computed (checked against an independent Python model of the
formula in a scratch directory, not committed) and written into
crates/bunny-optics/tests/camera_tests.rs.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo test --locked -p bunny-optics --test camera_tests against the stub: 0 passed, 8 failed
(after bounding OPTICS-TP-003's loop; the first stub run never ended because the loop
counted only cameras that constructed)
cargo fmt --all -- --check
cargo clippy --locked -p bunny-optics --all-targets --all-features -- -D warnings
strict library Clippy for bunny-optics (unwrap, expect, panic, todo, unimplemented,
indexing denied)
cargo run --locked -p xtask -- code-dojo-rust --all (clean)
cargo test --locked -p bunny-optics --test camera_tests (8 passed)
Four deliberate mutations of plane.rs (see the test plan's Mutation Checks); each failed at
least one case; restored source passed 8 of 8
cargo test --locked --workspace (all passed)
cargo run --locked -p xtask -- code-dojo --all (full gate, at the final commit)

Known risks:
The pixel origin is bottom-left with rows growing upward (matching +Y and NDC); renderers
that store rows top first must flip. The positive-forward corner check and the
re-orthogonalized basis are justified by tests, not by a closed-form proof; failure would be
the defensive Unrepresentable error. Narrow fields of view are accepted down to one raw unit
of tangent, where neighbouring pixels can share a ray. No timing was measured.

Human reviewer:
James Ross (maintainer); review pending.
