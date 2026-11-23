# Receipt: Typed Try Constructors 445

Task:
Work `flyingrobots/bunny#445`: add typed `try_` forms of `TraceRay::new`, `TraceTriangle::new`, `build_bvh_sah`, `Albedo::reflect` and `PinholeCamera::ray`, each with an error that names the refused condition, following the fast-path / diagnostic-path pattern of `OPTICS-REQ-043`; keep every existing `Option` function's body and behaviour; implement `Display` and `core::error::Error` for `TraversalError` and `CompactError`; document the `build_bvh_sah` primitive-index buffer rule. Additive only; variant names await the owner's approval.

Files read:
- `crates/bunny-query/src/trace.rs`, `crates/bunny-query/src/trace/*.rs`, `crates/bunny-query/src/lib.rs`
- `crates/bunny-broadphase/src/bvh.rs`, `crates/bunny-broadphase/src/bvh/*.rs`, `crates/bunny-broadphase/src/bvh/sah/*.rs`, `crates/bunny-broadphase/src/traversal.rs`, `crates/bunny-broadphase/src/compact.rs`, `crates/bunny-broadphase/src/lib.rs`
- `crates/bunny-optics/src/camera.rs`, `crates/bunny-optics/src/error.rs`, `crates/bunny-optics/src/shading.rs`, `crates/bunny-optics/src/shading/hit.rs`, `crates/bunny-optics/src/shading/light.rs`, `crates/bunny-optics/src/shading/refusal.rs`, `crates/bunny-optics/src/lib.rs`
- `docs/topics/ray-tracing/`, `docs/topics/bvh-construction/`, `docs/topics/optics/`

Files edited:
- `crates/bunny-query/tests/trace_refusal_tests.rs`
- `crates/bunny-broadphase/tests/bvh_refusal_tests.rs`
- `crates/bunny-broadphase/tests/traversal_error_display_tests.rs`
- `crates/bunny-optics/tests/typed_refusal_tests.rs`
- `.repo-respect/receipts/2026-11-23-typed-try-constructors-445.md`

Topic docs:
To follow with the implementation: `docs/topics/ray-tracing/`, `docs/topics/bvh-construction/`, `docs/topics/optics/`.

Generated artifacts:
None.

Checks run:
- First commit: the new tests only, which do not compile until the `try_` forms exist (red).

Known risks:
- New public API: every error type, variant and field name is a contract the owner approves; the pull request stays in draft.

Human reviewer:
@flyingrobots
