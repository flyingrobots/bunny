# Receipt: Inferred Float Literal Gate

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHSN` by making the Code Dojo
AST policy catch inferred float literals in deterministic core crates.

Files read:

* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-float-literal-gate.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask core_policy_flags_inferred_float_literals`
* `cargo test --locked -p xtask`
* `cargo fmt --check -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* The rule intentionally reports each float literal expression. Explicitly
  reviewed boundary conversions should use local proof waivers.

Human reviewer:
James
