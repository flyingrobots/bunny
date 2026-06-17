# Receipt: Staged Rust Index Check

Task:
Resolve PR #106 review thread `PRRT_kwDOS2Gurc6KTHRy` by making staged Rust
policy checks read the Git index blob instead of the worktree file.

Files read:

* `xtask/Cargo.toml`
* `xtask/src/code_dojo.rs`
* `xtask/src/main.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-staged-rust-index.md`
* `xtask/src/code_dojo.rs`

Checks run:

* `cargo test --locked -p xtask staged_rust_sources_are_read_from_index`
* `cargo fmt --check -p xtask`
* `cargo fmt -p xtask`
* `cargo clippy --locked -p xtask --all-targets -- -D warnings`
* `git diff --check`

Known risks:

* The source reader now shells out to `git show :<path>` for staged mode. This
  is intentionally limited to pre-commit/staged checks; full-repo checks still
  read the worktree.

Human reviewer:
James
