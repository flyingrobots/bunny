# Receipt: Orthographic Camera And Projection Round Trip

Task:
flyingrobots/bunny#257: the remainder of flyingrobots/bunny#138. The thin-lens model it names
is already on main (flyingrobots/bunny#364, `bunny-thin-lens/1`). This adds the orthographic
camera `bunny-orthographic-camera/1` (`OrthographicCamera`, `OrthographicSpec`,
`OrthographicError`, `ORTHOGRAPHIC_CAMERA_ID`, `ORTHOGRAPHIC_HALF_HEIGHT_LIMIT_RAW`), a
projection round-trip check through `FixedMat4::try_look_at`, `try_perspective`,
`try_orthographic` and `Viewport` with a per-case bound derived from each step's stated
rounding, and the chapter's account of the two construction safeguards (the second
orthogonalization and the positive-forward corner check): which are proven and which are
tested. Additive: no existing public item, output bit, golden, digest or ID changes;
`round_div` moved from `thin_lens.rs` to `plane.rs` unchanged, and three private helpers of
`camera.rs` became module-visible.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-ray-footprint.md
crates/bunny-optics/src/{lib.rs,camera.rs,plane.rs,basis.rs,thin_lens.rs,error.rs,point.rs}
crates/bunny-optics/tests/{camera_tests.rs,thin_lens_tests.rs,exact/mod.rs}
crates/bunny-linalg/src/{fixed_mat4_look_at.rs,fixed_mat4_projection.rs,viewport.rs,fixed_vec3.rs,lib.rs,fixed_mat4.rs}
crates/bunny-num/src/fixed_q32_32.rs (multiplication and division rounding)
crates/bunny-query/src/trace.rs (TraceRay::new)
docs/topics/optics/README.md and test-plan.md
flyingrobots/bunny#257, #138 and pull request #253

Files edited:
crates/bunny-optics/src/orthographic.rs (new)
crates/bunny-optics/src/{lib.rs,camera.rs,plane.rs,thin_lens.rs}
crates/bunny-optics/tests/orthographic_tests.rs (new)
crates/bunny-optics/tests/projection_round_trip_tests.rs (new)
crates/bunny-optics/tests/camera_safeguard_tests.rs (new)
crates/bunny-optics/README.md, CHANGELOG.md, docs/README.md
docs/topics/optics/README.md (Status, Camera Frame, Direction Scaling, Thin-Lens Camera;
new Orthographic Camera, Projection Round Trip and Construction Safeguards sections)
docs/topics/optics/test-plan.md (OPTICS-REQ-160 to -168; OPTICS-TP-310 to -324; mutation
checks; open gaps)
.repo-respect/receipts/2026-11-23-orthographic-camera-round-trip.md

Topic docs:
docs/topics/optics/README.md and test-plan.md. The IDs are in the range assigned to this
task (OPTICS-REQ-160 to -169, OPTICS-TP-310 to -329).

Generated artifacts:
None.

Checks run:
First commit (tests only): the new tests fail to compile, the camera does not exist.
Kernel and docs commits, each its own command on the host (local Rust 1.100.0; CI uses the
pinned 1.96.0): cargo fmt --all --check; cargo test --locked --workspace --all-targets
--all-features (123 test result lines, every one `ok`); cargo run --locked -p xtask --
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch; git diff --check. All clean.
cargo clippy --locked -p bunny-optics --lib and the three new test targets with -D warnings:
clean.
Mutations: fourteen, each applied to the source, run with cargo test --locked -q -p
bunny-optics --no-fail-fast over the orthographic, round-trip, safeguard, camera and thin-lens
tests, and restored; each failed at least one case (the test plan's table).
Not run locally: wasm-pack (CI's WebAssembly job runs it), cargo deny and markdownlint-cli2
(not installed; CI's Dojo job runs them), the no_std gate (bunny-num is not touched).

Known risks:
The round-trip bound is proven per case, from each step's stated rounding and the exact
basis mismatch of that case's two frames; no uniform bound over every accepted camera is
claimed. The orthographic view check uses the closed rectangle's corners, so it can refuse a
camera whose last sample is up to one pixel inside the trace domain.

Human reviewer:
James Ross (maintainer); review pending.
