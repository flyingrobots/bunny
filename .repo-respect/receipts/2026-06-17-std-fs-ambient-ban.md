# Receipt: std::fs Ambient-State Ban

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHR2` by making the Code Dojo
AST policy catch direct `std::fs::*` helper calls in deterministic core crates.

Files read:

* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-std-fs-ambient-ban.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask core_policy_flags_std_fs_helper_calls`
* `cargo fmt --check -p xtask`
* `cargo fmt -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* The gate also catches `fs::*` aliases. That is intentional for strict
  deterministic-core enforcement; local proof waivers remain available for
  exceptional reviewed cases.

Human reviewer:
James
