# Receipt: Frustum Culling

Task:
flyingrobots/bunny#146: a six-plane `FixedFrustum` in `bunny-geom` (left, right, bottom, top,
near, far, each front facing into the frustum, inside the closed set), `contains_point`,
`classify_aabb` and `classify_sphere` returning `FrustumClass` (`Inside`, `Intersecting`,
`Outside`) by a stated rule built on `FixedPlane3::classify`, `classify_aabb` and `PlaneSpan`,
and `from_clip_matrix`, the exact row-sum extraction for Bunny's `[0, 1]` depth range.
Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-transform-composition.md
crates/bunny-geom/src/{lib.rs,plane.rs,plane/span.rs}
crates/bunny-geom/tests/{plane_span_tests.rs,plane_span_sweep_tests.rs}
crates/bunny-linalg/src/{fixed_mat4.rs,fixed_mat4_projection.rs,fixed_mat4_look_at.rs,lib.rs}
crates/bunny-num/src/fixed_q32_32.rs (accessors)
xtask/src/{code_dojo.rs,topic_docs.rs} (limits and ledger rules)
docs/topics/{plane-clipping,coordinate-law,transform}/ chapters and test plans
flyingrobots/bunny#146 (no comments)

Files edited:
crates/bunny-geom/src/frustum.rs (new: FixedFrustum, FrustumClass, FrustumSphereError, the
combination rule and the clip-matrix extraction)
crates/bunny-geom/src/frustum/sphere.rs (new: the sphere-plane comparison in 256 bits)
crates/bunny-geom/src/lib.rs (module and exports, appended after the plane exports)
crates/bunny-geom/tests/frustum_tests.rs (new: constructed cases)
crates/bunny-geom/tests/frustum_sweep_tests.rs (new: seeded sweeps and independent oracles)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (topic index, sorted position)
docs/topics/frustum/README.md (new chapter)
docs/topics/frustum/test-plan.md (new: FRUSTUM-REQ-001 to -007, FRUSTUM-TP-001 to -016;
mutations)
.repo-respect/receipts/2026-11-23-frustum-culling.md

Topic docs:
docs/topics/frustum/ is a new topic with its own FRUSTUM- prefix; no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the frustum tests fail to compile, since
`FixedFrustum` does not exist; topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (136
test result lines, all ok; frustum_tests 10 passed, frustum_sweep_tests 4 passed);
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs (0 duplicate IDs);
repo-respect check --branch. cargo build --locked -p bunny-geom --no-default-features clean.
cargo clippy --locked -p bunny-geom --all-targets --all-features -- -D warnings: clean in every
file this branch adds; the only findings are `assert_is_empty` in polygon_clip_tests.rs, a lint
of the local 1.100 toolchain only.
Mutations: thirteen, each applied to the kernel, run with cargo test --locked -q -p
bunny-geom --no-fail-fast --test frustum_tests --test frustum_sweep_tests, and restored; twelve
fail at least one case (the test plan's table), one is equivalent (the middle carry of the
256-bit product, unreachable for these widths; stated in sphere.rs and the plan).
Not run locally: code-dojo-no-std (needs rustup; CI runs it), wasm-pack (not installed; CI's
WebAssembly job runs it), cargo deny and markdownlint-cli2 (not installed; CI's Dojo job runs
them).

Known risks:
The six-plane test is conservative: `Intersecting` includes shapes that meet no point of the
frustum. The chapter says so and a test pins an example of each. Extraction is exact against
the stored matrix, not against the exact camera; the seeded camera margin (1/64 of a face's
extent) is a tested property of stated parameter ranges, not a bound. Frustum against frustum
is not provided, so flyingrobots/bunny#146 stays open (Refs, not Closes).

Human reviewer:
James Ross (maintainer); review pending.
