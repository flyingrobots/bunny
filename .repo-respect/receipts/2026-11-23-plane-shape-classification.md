# Receipt: Plane Shape Classification

Task:
flyingrobots/bunny#162, the remainder: classify a segment, a triangle and an AABB against a
`FixedPlane3`. New type `PlaneSpan` (Front, Behind, Straddling, Coplanar) and three methods,
`classify_segment`, `classify_triangle` and `classify_aabb`, each defined by the existing point
rule (`FixedPlane3::classify`) at the endpoints, vertices or eight corners. Additive: no existing
public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-geom/{Cargo.toml,README.md,src/lib.rs,src/plane.rs,src/plane/polygon.rs}
crates/bunny-geom/tests/{plane_clip_tests.rs,polygon_clip_tests.rs}
crates/bunny-num/src/fixed_q32_32.rs (checked_mul, checked_add, checked_sub)
docs/topics/plane-clipping/{README.md,test-plan.md}
docs/topics/ray-tracing/README.md and test-plan.md (inverted-box approach, flyingrobots/bunny#295)
docs/topics/rotation/test-plan.md (mutation ledger format)
xtask/src/code_dojo.rs (category limits), xtask/src/topic_docs.rs (case rules)
flyingrobots/bunny#162 and flyingrobots/bunny#295

Files edited:
- `crates/bunny-geom/src/plane/span.rs` (new: `PlaneSpan` and the three methods)
- `crates/bunny-geom/src/plane.rs` (module and re-export only; no kernel line changed)
- `crates/bunny-geom/src/lib.rs` (export)
- `crates/bunny-geom/tests/plane_span_tests.rs` (new: constructed cases)
- `crates/bunny-geom/tests/plane_span_sweep_tests.rs` (new: seeded sweeps, independent model)
- `crates/bunny-geom/README.md`, `CHANGELOG.md`, `docs/README.md` (topic row text)
- `docs/topics/plane-clipping/README.md` (new Shape Classification section; one sentence of the
  rounding section corrected to state the existing `clip_segment` overflow on a touching segment)
- `docs/topics/plane-clipping/test-plan.md`
- `.repo-respect/receipts/2026-11-23-plane-shape-classification.md`

Topic docs:
docs/topics/plane-clipping/: PC-REQ-014 to -019 and PC-TP-025 to -037, a free contiguous range
above the existing PC-REQ-013 and PC-TP-024; topic-docs reports 0 duplicate IDs.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (PlaneSpan and the
three methods do not exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (125 test
result lines, all ok); cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check --branch.
cargo clippy --locked -p bunny-geom --lib --test plane_span_tests --test plane_span_sweep_tests
--all-features -D warnings clean. Whole-crate clippy on 1.100 also reports "used `assert!` to
check that a value is empty" in the untouched polygon_clip_tests.rs, a lint newer than CI's 1.96. RUSTDOCFLAGS=-D warnings cargo doc -p
bunny-geom clean.
Mutations: eleven, each applied to span.rs, run with cargo test --locked -q -p bunny-geom
--no-fail-fast over the four plane test files, and restored; ten fail named cases, one is an
equivalent mutant (test plan, Mutation Checks).
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's Dojo
job).

Known risks:
Classification is exact in the rounded signed distance of `classify`, not against the
real-valued plane; the chapter says so. Finding, not changed: `clip_segment` returns None for a
segment touching the plane at one end when `end - start` overflows on an axis.

Human reviewer:
James Ross (maintainer); review pending.
