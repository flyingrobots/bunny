# Receipt: Curve Arc Length

Task:
flyingrobots/bunny#307: `bunny-curve/1`, part 3, the arc-length table and constant-speed
parameter lookup. Adds the `Curve3` interface (`span_count`, `at`), implemented by
`CubicBezier3` with one span by delegating to its unchanged `at`; `ArcLengthTable::build`
into caller-provided storage, `total`, `lengths`, `end`, `param_at_distance`, and the typed
refusals `ArcLengthError`. Additive: no existing public item, output bit, golden, digest or
ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-linalg/src/{lib.rs,cubic_bezier3.rs,fixed_vec3.rs,lerp.rs}
crates/bunny-linalg/tests/curve_tests.rs (style, seed and oracle conventions)
crates/bunny-num/src/{lerp.rs,fixed_q32_32.rs} (from_ratio, lerp, sqrt_u128)
crates/bunny-broadphase/src (caller-provided slice convention, by search only)
xtask/src/topic_docs.rs (test discovery)
docs/topics/curve/README.md and test-plan.md
flyingrobots/bunny#307 and #309 (the Catmull-Rom parameter range `[0, segment_count]`)

Files edited:
crates/bunny-linalg/src/curve3.rs (new: the `Curve3` trait and its `CubicBezier3` impl)
crates/bunny-linalg/src/curve_arc_length.rs (new: the kernel)
crates/bunny-linalg/src/lib.rs (modules and exports)
crates/bunny-linalg/tests/curve_arc_length_tests.rs (new)
crates/bunny-linalg/README.md, CHANGELOG.md, docs/README.md (curve row)
docs/topics/curve/README.md (new section: Arc Length)
docs/topics/curve/test-plan.md (CURVE-REQ-020 to -025, CURVE-TP-040 to -046; mutations)
.repo-respect/receipts/2026-11-23-curve-arc-length.md

Topic docs:
docs/topics/curve/ gains CURVE-REQ-020 to -025 and CURVE-TP-040 to -046, inside the range
assigned to this task (REQ-020 to -029, TP-040 to -059); flyingrobots/bunny#309 uses
REQ-010 to -019 and TP-020 to -039.

Generated artifacts:
None. The Python `decimal` reference that produced `PYTHON_ARC_RAW` is posted in the pull
request, not committed.

Checks run:
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check
--branch; cargo clippy --locked -p bunny-linalg --all-targets --all-features -D warnings
(local Rust 1.100.0; CI uses the pinned 1.96.0).
Mutations: thirteen, each applied to the kernel, run with cargo test --locked -q -p
bunny-linalg --no-fail-fast --test curve_arc_length_tests, and restored; each failed at
least one case (the test plan's table).
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

Known risks:
Each chord is kept to 2^-16 raw rather than rounded by `FixedVec3::length`, which the issue
proposed: floored chords miss the issue's 2 raw unit straight-line criterion by 1618 raw
units at 4095 segments. `build` takes a storage slice the issue's signature does not have,
following the crate's no-allocation convention. The `Curve3` shape is a public-API choice
for the owner to confirm. The table can exceed the true arc length by the proven
3 sqrt(3) K + 1/2 raw units; that it stayed below in every measured case is measurement.

Human reviewer:
James Ross (maintainer); review pending.
