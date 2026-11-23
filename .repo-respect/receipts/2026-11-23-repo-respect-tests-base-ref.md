# Receipt: Repo Respect Tests Base Ref

Task:
Fix `flyingrobots/bunny#343`: four `xtask` `repo_respect` tests read `GITHUB_BASE_REF` (and `CODE_DOJO_BASE_REF`) from the process environment through the code under test, so they failed on a stacked pull request whose base branch is absent from the tests' temporary repository. The base ref is now resolved once at the entry points and passed down, so the tests supply it themselves.

Files read:
- `xtask/src/repo_respect.rs`
- `xtask/src/code_dojo.rs`

Files edited:
- `xtask/src/repo_respect.rs`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-repo-respect-tests-base-ref.md`

Topic docs:
None, because no documented behavior changes; `repo-respect check` resolves the base the same way as before.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask repo_respect` with `GITHUB_BASE_REF=nope` (RED: four fixture tests failed) and with it unset (green), before the fix.
- `cargo test --locked -p xtask` after the fix: 163 passed; `repo_respect` tests also pass with `GITHUB_BASE_REF=nope`.
- `cargo fmt --all --check`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- None beyond the local toolchain difference (1.100.0 here, 1.96.0 pinned in CI).

Human reviewer:
@flyingrobots
