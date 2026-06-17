# Goalpost Post-v0.4.0: Standards Alignment and Quality Gates

## Goal

Align with the new code standards and pass all quality gates.

## Status

Completed on 2026-06-17.

## Scope

This goalpost replaces the older repository standards and quality enforcement
flow with the Rust Code Standards Editor's Edition and Code Dojo.

## Acceptance Criteria

* `CODE_STANDARDS.md` is the Rust Code Standards Editor's Edition.
* `docs/NUMERIC_CONSTITUTION.md`, `docs/SENSEIS_WISDOM.md`, and
  `docs/CODE_DOJO.md` are present and linked from process/testing signposts.
* Repo-local hooks are installed through `scripts/install-githooks.sh`.
* `.github/workflows/code-dojo.yml` is the active quality workflow, with release
  publication kept in `.github/workflows/release.yml`.
* Workspace and member manifests inherit the new lint baseline.
* `python3 scripts/code-dojo/dojo.py --all` passes.
* Headless WASM tests pass for every WASM-compatible library crate.
* Markdown checks pass for touched Markdown files when the tool is available.
* Release archive verification passes before any release tag is cut.

All acceptance criteria are satisfied for the local standards-alignment gate.
Release archive verification was run with `ALLOW_DIRTY=1` because the
alignment work is still local and intentionally uncommitted.

## Initial Gate Findings

The first Code Dojo checks after installation are expected to fail until the
alignment work is complete. Known initial categories:

* deterministic receipt checks do not yet recognize each core crate's existing
  golden-vector or degeneracy evidence;
* the Rust AST source-shape gate reports unchecked indexing, panic-paths,
  float-boundary waivers, function-size limits, statement-count limits,
  parameter-count limits, nesting-depth limits, and complexity limits;
* package-scoped strict Clippy now enforces unwrap/expect/panic/todo,
  unimplemented, and unchecked-indexing bans in library targets where the
  standard applies;
* the installed `rustfmt.toml` changes formatting expectations across existing
  Rust files.

## Evidence Log

* `bash scripts/install-githooks.sh` installed `.githooks` as `core.hooksPath`.
* `python3 -m py_compile scripts/code-dojo/*.py` passed.
* `git diff --check` passed after installation edits.
* `cargo metadata --locked --no-deps` parsed the workspace manifests.
* `python3 scripts/code-dojo/check_determinism_manifest.py --enforce` failed
  with missing deterministic receipt findings for the six current core crates.
* `python3 scripts/code-dojo/check_files.py --all` failed through the
  `xtask` Rust AST checker with source-shape alignment findings across the
  existing Rust workspace.
* `cargo run --locked -p xtask -- code-dojo-rust --all` compiled and reported
  AST-backed violations instead of regex-derived Rust findings.
* `cargo fmt --all -- --check` passed after formatting the workspace under the
  installed policy.
* `cargo test --locked -p xtask` passed after adding the `code-dojo-rust`
  command route.
* `cargo clippy --locked -p xtask --all-targets -- -D warnings` reached the
  repository package-metadata failures after AST checker helper shape was
  fixed.
* `cargo clippy --locked --workspace --all-targets --all-features --
  -D warnings` failed on missing package metadata, missing docs in generated
  contract output, and build-script formatting/docs findings.
* `python3 scripts/code-dojo/dojo.py --all` passed after the AST checker,
  deterministic receipts, formatting, workspace Clippy, strict package Clippy,
  native tests, and wasm32 library checks were aligned.
* Code Dojo was tightened after completion review: full-gate source discovery
  now checks tracked and untracked nonignored Rust files, and rollout skip knobs
  for Cargo, deterministic receipts, and WASM checks were removed.
* Dependency policy was promoted into Code Dojo through `cargo deny check`; the
  stale `deny.toml` schema was repaired and `xtask` now inherits the workspace
  license.
* `cargo deny check` is active in Code Dojo. Duplicate transitive dependency
  versions from the current `wesley-core` graph remain visible warnings rather
  than hidden skip exemptions.
* `python3 scripts/code-dojo/check_files.py --all` passed after the full-gate
  file-discovery ratchet was tightened.
* Headless Node.js WASM tests passed for every WASM-compatible library crate:
  `bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-contract`, `bunny-query`,
  `bunny-broadphase`, `bunny-mesh`, and `bunny-codec`.
* `markdownlint-cli2 CHANGELOG.md ROADMAP.md docs/BEARING.md
  docs/goalposts/post-v0.4.0-standards-alignment.md` passed.
* `git diff --check` passed after the standards-alignment edits.
* `ALLOW_DIRTY=1 scripts/publish-crates.sh verify` passed, including full
  package verification for the root crates and archive file-list verification
  for the remaining publishable crates.
