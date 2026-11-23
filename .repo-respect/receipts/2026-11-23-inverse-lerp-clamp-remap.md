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
- `crates/bunny-num/src/lerp.rs` (the `LERP_CONTRACT_ID` doc comment only)
- `crates/bunny-num/tests/inverse_lerp_remap_tests.rs`
- `crates/bunny-num/README.md`
- `docs/topics/lerp/README.md`
- `docs/topics/lerp/test-plan.md`
- `docs/README.md`
- `docs/NUMERIC_CONSTITUTION.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-inverse-lerp-clamp-remap.md`

Topic docs:
`docs/topics/lerp/README.md` gains the Clamp, Min and Max, Inverse Lerp (with Round Trips) and Remap sections; `docs/topics/lerp/test-plan.md` gains LERP-REQ-010 to -013, LERP-TP-020 to -030 and the new mutation table, and its out-of-scope line no longer lists the delivered functions. `topic-docs` reports zero duplicate IDs.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-num --test inverse_lerp_remap_tests` against a stub kernel: 9 of 10 cases fail (LERP-TP-021, on `Ord::min` and `Ord::max`, passes as it should); with the kernel, 10 of 10 pass.
- Twelve mutations of `remap.rs`, each failing at least one case (table in `docs/topics/lerp/test-plan.md#mutation-checks`).
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features` (124 test binaries, all ok)
- `cargo build --locked -p bunny-num --no-default-features` and `cargo test --locked -p bunny-num --no-default-features` on the host; the bare-metal `code-dojo-no-std` build runs only in CI (no `rustup` here).
- `cargo clippy --locked -p bunny-num --all-targets --all-features -- -D warnings` on Rust 1.100.0
- `cargo run --locked -p xtask -- code-dojo-rust --all`, `code-dojo-generated`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `code-dojo-rustdoc`, `topic-docs`, `repo-respect check --branch`
- Not run locally: the WebAssembly job (`wasm-pack` is not installed), clippy on the pinned 1.96.0, `cargo deny`, markdownlint.

Known risks:
- `remap` takes its ranges as `[lo, hi]` arrays because core functions are limited to four parameters.
- `checked_clamp` is named apart from `Ord::clamp`, which panics on an inverted range, so existing calls keep their meaning.
- `bunny-lerp/1` now also names the three new functions; the identity string and every existing bit are unchanged.
- LERP-TP-027 runs about a million cases through a bit-serial oracle; it takes under a second natively, longer under WebAssembly.

Human reviewer:
@flyingrobots
