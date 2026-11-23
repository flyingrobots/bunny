# Receipt: Ray-Tracing Docs, Inverted Boxes And Shared Nodes

Task:
Work `flyingrobots/bunny#295`: state the inverted-box behaviour of `TraceRay::box_entry` in the contract and pin it with a test, correct `NotATree`'s documentation (a node count, not a tree check) and pin that with a test, and add the trace kernels' rounding and saturation rows to the Numeric Constitution's overflow audit. The `compact.rs` line 13 comment was already correct on `main`; the typed `try_` constructors are a public API addition and are left for the owner.

Files read:
- `crates/bunny-query/src/trace.rs`
- `crates/bunny-query/src/trace/*.rs`
- `crates/bunny-broadphase/src/compact.rs`
- `docs/NUMERIC_CONSTITUTION.md`
- `docs/topics/ray-tracing/README.md`
- `docs/topics/ray-tracing/test-plan.md`

Files edited:
- `crates/bunny-broadphase/src/compact.rs` (doc comments only)
- `crates/bunny-broadphase/tests/compact_tests.rs`
- `crates/bunny-query/tests/trace_tests.rs`
- `docs/NUMERIC_CONSTITUTION.md`
- `docs/topics/ray-tracing/README.md`
- `docs/topics/ray-tracing/test-plan.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-ray-tracing-docs-295.md`

Topic docs:
`docs/topics/ray-tracing/`: README contract text, and new cases TRACE-TP-317 and TRACE-TP-318 (no requirement or existing case ID changed).

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-query --test trace_tests trace_tp_317` and `cargo test --locked -p bunny-broadphase --test compact_tests trace_tp_318`
- Local gates listed in the pull request description.

Known risks:
- None beyond the local toolchain difference (1.100.0 here, 1.96.0 pinned in CI).

Human reviewer:
@flyingrobots
