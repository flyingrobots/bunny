# Release Packaging

This chapter describes what `scripts/publish-crates.sh` checks about the crate
archives before anything is published, and the rule for files that a test reads.

## Status

Living topic chapter. The script is `scripts/publish-crates.sh`; the release
workflow `.github/workflows/release.yml` runs its `verify` mode.

## Modes

| Mode | What it does |
| --- | --- |
| `verify` | Packages the independent root crates in full, lists the files of the others, then runs `packaged-tests`. |
| `packaged-tests` | Builds the tests of every publishable crate from its own archive. |
| `dry-run` | `cargo publish --dry-run` for crates whose siblings are visible on crates.io, file lists for the rest. |
| `publish` | Publishes every crate in dependency order; refuses a dirty worktree. |

## Packaged Tests

`cargo package` compiles the library of an archive but never its tests, so a
test file that ships in the archive while a file it reads does not would pass
every other release check and fail for anyone running `cargo test` on the
downloaded crate.

`packaged-tests` does this:

1. `cargo package --locked --no-verify` for every crate in `CRATES`, in one
   invocation, into a scratch target directory.
2. Unpack each `.crate` archive side by side into one scratch directory.
3. Write a workspace manifest there whose members are the unpacked crates and
   whose `[patch.crates-io]` points each Bunny crate at its unpacked copy. No
   sibling version needs to exist on crates.io.
4. Copy the repository `Cargo.lock`, so every third-party crate is the version
   the repository tests.
5. `cargo check --workspace --tests --keep-going` in that directory. This
   expands `include_bytes!` and `include_str!`, resolves every import and
   dev-dependency, and reports every failing crate, without linking.

The step needs network access to fetch third-party crates, as `verify` already
does for the root crates. It fails with Cargo's own message, for example
`couldn't read ... tests/fixtures/...: No such file or directory`.

## The File Rule

A test file in a published crate may read only files that the same archive
contains. When a test reads a fixture that the manifest `exclude`s, or a file
outside the crate directory, the test file is excluded in the same `exclude`
list:

| Crate | Excluded together | Why |
| --- | --- | --- |
| `bunny-codec` | `tests/fixtures/stanford-bunny-ascii/` and `tests/ply_ascii_stanford_tests.rs` | The 143 KB Stanford asset is the only large fixture. |
| `bunny-contract` | `tests/generated_version_tests.rs` | It compares the generated files at the repository root, outside the crate. |

The excluded tests still run in the repository, in `cargo test --workspace`.

## Limits

`cargo check` does not link or run anything, so a test that compiles from the
archive but fails when run is not found by this step; the repository's own test
run covers behaviour. The step does not check that the excluded files are the
only ones that differ: it checks that everything left compiles.
