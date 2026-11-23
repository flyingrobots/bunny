# Receipt: Ray Footprint

Task:
flyingrobots/bunny#383: the ray footprint at a triangle hit, `bunny-ray-footprint/1`.
`PinholeCamera::footprint` and `ThinLensCamera::footprint` intersect the camera's rays through
the same sub-pixel offset one column right and one row down (past the image edge by the same
image-plane formula; the thin lens holds its lens point fixed) with the hit triangle's plane,
difference their exact texture coordinates and round each component once to nearest, ties to
even. A neighbouring ray parallel to the plane, meeting it behind the origin or giving a
derivative outside Q32.32 is a typed `RayFootprint::Unbounded` result; invalid inputs are a
`RayFootprintError`. Additive: no existing public item, output bit, golden, digest or ID
changes; `ThinLensCamera::ray` was refactored to share its lens-point step, bit for bit.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/CODE_DOJO.md, .githooks/*
.repo-respect/receipts/2026-11-23-optics-incidence-cosine-rounding.md
crates/bunny-optics/src/{lib.rs,camera.rs,plane.rs,basis.rs,thin_lens.rs,texture.rs,error.rs}
crates/bunny-optics/src/shading/{wide.rs,wide256.rs,point_light.rs}
crates/bunny-optics/tests/{thin_lens_tests.rs,point_light_tests.rs,exact/mod.rs}
crates/bunny-query/src/{trace.rs,barycentric.rs}
crates/bunny-perf/src/{texture.rs,cases.rs}, xtask/src/code_dojo.rs, xtask/src/topic_docs.rs
docs/topics/optics/README.md and test-plan.md
flyingrobots/bunny#383, #384 and #400 (the issues that list #383 in `.blocked_by`)

Files edited:
crates/bunny-optics/src/footprint.rs (new), crates/bunny-optics/src/footprint/wide.rs (new)
crates/bunny-optics/src/camera.rs (PinholeCamera::footprint, neighbour_directions)
crates/bunny-optics/src/thin_lens.rs (ThinLensCamera::footprint; `ray` shares `lens_point`)
crates/bunny-optics/src/lib.rs (module and exports)
crates/bunny-optics/tests/ray_footprint_tests.rs (new)
crates/bunny-optics/tests/exact/mod.rs (Signed::mul)
crates/bunny-optics/README.md, CHANGELOG.md
docs/topics/optics/README.md (Status; Ray Footprint)
docs/topics/optics/test-plan.md (OPTICS-REQ-110 to -114; OPTICS-TP-210 to -221; the footprint
oracle; Ray-Footprint Mutation Checks; two open gaps)
.repo-respect/receipts/2026-11-23-ray-footprint.md

Topic docs:
docs/topics/optics/README.md (Ray Footprint) and test-plan.md. The ids are above every id used
on main (OPTICS-REQ-102, OPTICS-TP-206) and on every remote branch at the time of writing.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): cargo test --locked -q -p bunny-optics --test
ray_footprint_tests fails to compile (the kernel does not exist); topic-docs and repo-respect
check --staged clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (113 test
result lines, every one `ok`); cargo run --locked -p xtask -- code-dojo-generated;
code-dojo-rust --all; code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs;
repo-respect check --branch. All clean. cargo clippy --locked -p bunny-optics --lib and --test
ray_footprint_tests with -D warnings: clean (other test targets report lints that exist only
on 1.100.0).
Mutations: fifteen, each applied to the source, run with cargo test --locked -q -p
bunny-optics --no-fail-fast --test ray_footprint_tests --test thin_lens_tests --test
camera_tests --lib, and restored; each failed at least one case (the test plan's table).
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

Known risks:
The footprint is a difference across one pixel (a secant), not a derivative at the hit; the
chapter says so. The edge extrapolation is tested through a larger camera with the same per-pixel
steps and scale shift, so cameras whose shifts differ are skipped (counted). No seeded case is
exactly `Parallel`; constructed cases cover it.

Human reviewer:
James Ross (maintainer); review pending.
