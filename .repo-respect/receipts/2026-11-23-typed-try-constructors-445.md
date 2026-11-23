# Receipt: Typed Try Constructors 445

Task:
Work `flyingrobots/bunny#445`: add typed `try_` forms of `TraceRay::new`, `TraceTriangle::new`, `build_bvh_sah`, `Albedo::reflect` and `PinholeCamera::ray`, each with an error that names the refused condition, following the fast-path / diagnostic-path pattern of `OPTICS-REQ-043`; keep every existing `Option` function's body and behaviour; implement `Display` and `core::error::Error` for `TraversalError` and `CompactError`; document the `build_bvh_sah` primitive-index buffer rule. Additive only; variant names await the owner's approval, so the pull request stays in draft.

Files read:
- `crates/bunny-query/src/trace.rs`, `crates/bunny-query/src/trace/*.rs`, `crates/bunny-query/src/lib.rs`
- `crates/bunny-broadphase/src/bvh.rs`, `crates/bunny-broadphase/src/bvh/*.rs`, `crates/bunny-broadphase/src/bvh/sah/*.rs`, `crates/bunny-broadphase/src/traversal.rs`, `crates/bunny-broadphase/src/compact.rs`, `crates/bunny-broadphase/src/lib.rs`
- `crates/bunny-optics/src/camera.rs`, `crates/bunny-optics/src/error.rs`, `crates/bunny-optics/src/shading.rs`, `crates/bunny-optics/src/shading/hit.rs`, `crates/bunny-optics/src/shading/light.rs`, `crates/bunny-optics/src/shading/refusal.rs`, `crates/bunny-optics/src/lib.rs`
- `docs/topics/ray-tracing/`, `docs/topics/bvh-construction/`, `docs/topics/optics/`, `xtask/src/code_dojo.rs`

Files edited:
- `crates/bunny-query/src/trace.rs` (module, export, rustdoc only), `crates/bunny-query/src/trace/refusal.rs` (new), `crates/bunny-query/src/lib.rs`
- `crates/bunny-broadphase/src/bvh.rs` (new `try_build_bvh_sah`, rustdoc), `crates/bunny-broadphase/src/bvh/build.rs` (`validated_range` forwards its three lengths to `validated_lengths`), `crates/bunny-broadphase/src/bvh/refusal.rs` and `refusal/tests.rs` (new), `crates/bunny-broadphase/src/traversal.rs`, `crates/bunny-broadphase/src/compact.rs` (`Display` and `Error` only), `crates/bunny-broadphase/src/lib.rs`
- `crates/bunny-optics/src/camera.rs` (module, export, rustdoc only), `crates/bunny-optics/src/camera/refusal.rs` (new), `crates/bunny-optics/src/shading.rs`, `crates/bunny-optics/src/shading/albedo_refusal.rs` (new), `crates/bunny-optics/src/shading/light.rs` (rustdoc only), `crates/bunny-optics/src/lib.rs` (two lines at the end)
- `crates/bunny-query/tests/trace_refusal_tests.rs`, `crates/bunny-broadphase/tests/bvh_refusal_tests.rs`, `crates/bunny-broadphase/tests/traversal_error_display_tests.rs`, `crates/bunny-optics/tests/typed_refusal_tests.rs` (new)
- `crates/bunny-query/README.md`, `crates/bunny-broadphase/README.md`, `crates/bunny-optics/README.md`, `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-typed-try-constructors-445.md`

Topic docs:
- `docs/topics/ray-tracing/`: new section Typed Ray And Triangle Refusals, `Display` text of `TraversalError` and `CompactError`; TRACE-REQ-070 to -073, TRACE-TP-720 to -730, Typed Refusal Mutation Checks.
- `docs/topics/bvh-construction/`: new sections The Primitive-Index Buffer and Typed Refusals; BVH-REQ-012 to -015, BVH-TP-029 to -035, Typed Refusal Mutation Checks.
- `docs/topics/optics/`: new sections Typed Camera Ray Refusals and Typed Reflection Refusals; OPTICS-REQ-170 to -172, OPTICS-TP-330 to -338 (rows next to OPTICS-REQ-043 and OPTICS-TP-064, not at the ends of the tables), Typed Refusal Mutation Checks.
- No existing requirement or case ID changed.

Generated artifacts:
None.

Checks run:
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo run --locked -p xtask -- code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`
- `cargo clippy --locked -p <crate> --lib --test <new test> --all-features -- -D warnings` for the three crates (local 1.100.0)
- 18 mutation checks, recorded in the three test plans.

Known risks:
- New public API: every error type, variant and field name is a contract the owner approves; the pull request stays in draft.
- Clippy at the pinned 1.96.0, the wasm32 and no-std builds, `cargo deny` and markdownlint run only in CI.

Human reviewer:
@flyingrobots
