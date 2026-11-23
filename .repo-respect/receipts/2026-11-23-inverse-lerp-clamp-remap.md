# Receipt: Inverse Lerp, Clamp and Remap

Task:
The remainder of flyingrobots/bunny#111 beside `bunny-lerp/1`: `FixedQ32_32::checked_clamp`, `FixedQ32_32::inverse_lerp` and `FixedQ32_32::remap` in `bunny-num`, with degenerate-range, tie, overflow-threshold and round-trip tests against independent oracles. Additive only: `lerp`, `from_ratio`, their goldens and the existing ledger IDs are unchanged.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `.githooks/commit-msg`
- `.githooks/pre-commit`
- `.githooks/pre-push`
- `.repo-respect/receipts/2026-11-23-rustdoc-links.md`
- `docs/NUMERIC_CONSTITUTION.md`
- `docs/topics/lerp/README.md`
- `docs/topics/lerp/test-plan.md`
- `crates/bunny-num/src/lerp.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/lerp_tests.rs`
- `crates/bunny-num/README.md`
- `xtask/src/code_dojo.rs`

Files edited:
- `crates/bunny-num/src/remap.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/inverse_lerp_remap_tests.rs`
- `.repo-respect/receipts/2026-11-23-inverse-lerp-clamp-remap.md`

Topic docs:
`docs/topics/lerp/README.md` and `docs/topics/lerp/test-plan.md` gain LERP-REQ-010 to -013 and LERP-TP-020 to -030 once the kernel lands.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-num --test inverse_lerp_remap_tests` against a stub kernel: 9 of 10 cases fail (LERP-TP-021, on `Ord::min` and `Ord::max`, passes as it should).

Known risks:
- `remap` takes its ranges as `[lo, hi]` arrays because core functions are limited to four parameters.
- `checked_clamp` is named apart from `Ord::clamp`, which panics on an inverted range, so existing calls keep their meaning.

Human reviewer:
@flyingrobots
