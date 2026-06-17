# Receipt: Panic Macro Gate

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHSG` by making the Code Dojo
AST policy catch assertion and unreachable panic macros in deterministic core
crates.

Files read:

* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-panic-macro-gate.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask core_policy_flags_assertion_and_unreachable_macros`
* `cargo test --locked -p xtask`
* `cargo fmt --check -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* Statement-position macros and expression-position macros are checked through
  the same predicate to avoid divergent panic-path coverage.

Human reviewer:
James
