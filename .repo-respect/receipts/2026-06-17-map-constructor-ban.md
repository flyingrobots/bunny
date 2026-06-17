# Receipt: Inferred Map Constructor Ban

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHSA` by making the Code Dojo
AST policy catch inferred `HashMap::new()` and `HashSet::new()` constructor
calls in deterministic core crates.

Files read:

* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-map-constructor-ban.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask core_policy_flags_inferred_hash_map_and_set_constructors`
* `cargo test --locked -p xtask`
* `cargo fmt --check -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* The duplicate path-call visitor was removed so constructor calls are not
  reported twice. `visit_expr_path` still covers function call paths.

Human reviewer:
James
