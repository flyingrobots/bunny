# Receipt: Rustdoc Links

Task:
Fix the broken intra-doc links on `main` and add a rustdoc step to the full Code Dojo gate (flyingrobots/bunny#467). Documentation comments and the gate only: no code behaviour, signature or visibility changes.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/main.rs`
- `docs/CODE_DOJO.md`
- `.github/workflows/code-dojo.yml`
- `crates/bunny-query/src/ray/prepared.rs`
- `crates/bunny-broadphase/src/compact.rs`
- `crates/bunny-broadphase/src/compact_hit.rs`
- `crates/bunny-broadphase/src/counts.rs`

Files edited:
- `crates/bunny-query/src/ray/prepared.rs`
- `crates/bunny-broadphase/src/compact.rs`
- `crates/bunny-broadphase/src/compact_hit.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/main.rs`
- `docs/CODE_DOJO.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-rustdoc-links.md`

Topic docs:
None. The Code Dojo chapter `docs/CODE_DOJO.md` has no requirement ledger.

Generated artifacts:
None.

Checks run:
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features` failed on `main` (10 errors in 3 files), clean after the fix.
- `cargo test --locked -p xtask rustdoc` (the command-list unit test)
- Gate proof: a deliberately broken link in `prepared.rs` made `cargo run --locked -p xtask -- code-dojo-rustdoc` exit 1 naming the file and line; restored, it exits 0.
- Other gates are listed in the pull request.

Known risks:
- Local rustdoc is 1.100.0; CI pins 1.96.0, so the first CI run is the real test of the step.
- Open pull requests that carry a broken link will fail the new step until fixed.

Human reviewer:
@flyingrobots
