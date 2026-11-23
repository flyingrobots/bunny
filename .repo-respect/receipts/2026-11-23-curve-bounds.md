# Receipt: Curve Bounds

Task:
flyingrobots/bunny#152, the remainder: conservative bounds helpers for `bunny-curve/1`.
`CurveBounds3 { min, max }` with `contains`; `CubicBezier3::control_bounds` (the exact
per-axis minimum and maximum of the control points, proven to contain every rounded `at(t)`
with no widening); `CubicBezier3::split_bounds(t)` (each half's control box widened by a
derived 3 raw units and clipped to the control box, proven to contain the original's
evaluations on the half's range); `CatmullRom3::control_bounds` (the union of the segments'
control boxes, not the box of the points). Additive: no existing public item, output bit,
golden, digest or ID changes. Criteria 1, 2 and 4 of the issue were already met on `main`.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-linalg/src/{lib.rs,cubic_bezier3.rs,catmull_rom3.rs,fixed_vec3.rs,lerp.rs}
crates/bunny-linalg/Cargo.toml, crates/bunny-num/src/lerp.rs, scripts/publish-crates.sh
crates/bunny-linalg/tests/{curve_tests.rs,catmull_rom_tests.rs}
docs/topics/curve/README.md and test-plan.md, docs/README.md
xtask/src/topic_docs.rs (ledger statuses)
flyingrobots/bunny#152

Files edited:
crates/bunny-linalg/src/curve_bounds3.rs (new: the bounds)
crates/bunny-linalg/src/lib.rs (module and export)
crates/bunny-linalg/tests/curve_bounds_tests.rs (new)
crates/bunny-linalg/README.md, CHANGELOG.md, docs/README.md (curve row)
docs/topics/curve/README.md (new sections at the end)
docs/topics/curve/test-plan.md (CURVE-REQ-030 to -033, CURVE-TP-060 to -066, mutations)
.repo-respect/receipts/2026-11-23-curve-bounds.md

Topic docs:
docs/topics/curve/ extended within the IDs assigned to this task (REQ 030 to 039, TP 060 to
079); no existing ID changes.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the bounds tests fail to compile (the API does
not exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (125
test result lines, all ok); code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; code-dojo-rustdoc; topic-docs;
repo-respect check --branch; git diff --check. cargo clippy --locked -p bunny-linalg
--all-targets --all-features -- -D warnings clean.
Mutations: fifteen, each applied to curve_bounds3.rs, run with cargo test --locked -q -p
bunny-linalg --no-fail-fast --test curve_bounds_tests, and restored; each failed at least one
case (the test plan's table), including each of the six faces shrunk by one raw unit.
Python search (an integer port of de Casteljau, 300 000 small polygons): the largest escape of
the original from a half's unwidened control box is 1 raw unit; the constructed case of
CURVE-TP-063 comes from it.
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

Known risks:
The split widening of 3 raw units is proven, not shown tight: the largest escape found from a
half's unwidened box is 1 raw unit. A box tighter than the control box (the exact cubic's
extrema) is not provided.

Human reviewer:
James Ross (maintainer); review pending.
