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
crates/bunny-optics/src/camera.rs
crates/bunny-optics/src/error.rs
crates/bunny-optics/src/point.rs
crates/bunny-optics/tests/camera_tests.rs
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
Cargo.toml
Cargo.lock
xtask/src/code_dojo.rs
scripts/publish-crates.sh
.github/workflows/code-dojo.yml
docs/TESTING.md
.repo-respect/receipts/2026-11-13-optics-camera.md

Topic docs:
docs/topics/optics/test-plan.md (new: OPTICS-REQ-001..009, OPTICS-TP-001..009, all
planned)
docs/topics/optics/README.md (new; states that the camera is not implemented yet)

Generated artifacts:
None. Golden vectors are hand computed and written into
crates/bunny-optics/tests/camera_tests.rs.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo test --locked -p bunny-optics --test camera_tests: 0 passed, 8 failed (expected: the camera is a stub)

Known risks:
The camera is a stub in this commit; every planned case fails by design.

Human reviewer:
James Ross (maintainer); review pending.
