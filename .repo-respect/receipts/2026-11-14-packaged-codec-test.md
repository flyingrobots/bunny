# Receipt: Packaged Codec Test

Task:
Fix flyingrobots/bunny#283: `bunny-codec`'s package ships `tests/ply_ascii_stanford_tests.rs`
but excludes the fixture it reads with `include_bytes!`, so `cargo test` on the downloaded
crate cannot compile. Exclude the test with its fixture and add a release check that
compiles the tests of every publishable crate from its unpacked archive. No release.

Files read:
* `crates/bunny-codec/Cargo.toml`, `crates/bunny-contract/Cargo.toml`
* `crates/*/tests/*.rs` (every `include_str!`, `include_bytes!` and `../` path)
* `scripts/publish-crates.sh`, `.github/workflows/release.yml`
* `docs/PROCESS.md`, `docs/TESTING.md`, `docs/topics/repo-respect/README.md`
* `xtask/src/topic_docs.rs` (the test-plan contract)

Files edited:
* `scripts/publish-crates.sh` (new `packaged-tests` mode, called by `verify`)
* `crates/bunny-codec/Cargo.toml`, `crates/bunny-contract/Cargo.toml` (`exclude`)
* `crates/bunny-codec/README.md`, `crates/bunny-contract/README.md`
* `docs/topics/release-packaging/README.md`, `docs/topics/release-packaging/test-plan.md` (new topic)
* `docs/README.md`, `docs/PROCESS.md`, `docs/TESTING.md`, `CHANGELOG.md`
* `.repo-respect/receipts/2026-11-14-packaged-codec-test.md`

Topic docs:
New topic `docs/topics/release-packaging/` with requirements REL-REQ-001 and REL-REQ-002
and cases REL-TP-001 to REL-TP-003 (static-check evidence anchored on the script).

Generated artifacts:
None.

Checks run:
Before the fix, in the container with network: `ALLOW_DIRTY=1 scripts/publish-crates.sh
packaged-tests` exits 101 and names `bunny-codec` `ply_ascii_stanford_tests` and
`bunny-contract` `generated_version_tests` (the issue named only the first). After the fix
the same command exits 0. The full Code Dojo gate result is in the pull request.

Known risks:
The check needs network for third-party crates and uses `cargo check`, so it finds
compile-time mismatches only, not test failures at run time. `bunny-contract` has no test
left in its package.

Human reviewer:
Pending: the project lead reviews this pull request.
