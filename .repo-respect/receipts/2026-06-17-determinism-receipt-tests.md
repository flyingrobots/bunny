# Receipt: Determinism Receipt Test Validation

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHSO` by requiring Code Dojo
determinism receipts to contain actual Rust test functions instead of trusting
placeholder filenames.

Files read:

* `crates/*/tests/*.rs`
* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-determinism-receipt-tests.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask deterministic_receipts_require_real_tests`
* `cargo test --locked -p xtask`
* `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
* `cargo fmt --check -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* Receipt files that use special deterministic filenames now fail unless the
  Rust AST contains a native `#[test]` or `#[wasm_bindgen_test]` function.

Human reviewer:
James
