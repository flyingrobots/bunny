# Bunny - Testing Doctrine

Bunny tests prove deterministic behavior, safe boundaries, and honest claims.
Passing tests are not enough if they do not cover the acceptance criteria that
the roadmap says are complete.

## Determinism Rules

* Fixed-point outputs use exact assertions on raw values or value objects.
* Floating-point values are allowed at ingress and egress boundaries only.
* Non-finite float inputs must be rejected before they enter deterministic
  geometry or mesh contracts.
* Randomized corpora must have fixed seeds and committed expected outputs.
* Allocation claims require direct witnesses, not inference from code shape.

## Required Local Gates

Run the Code Dojo gate before opening or updating a goalpost PR:

```bash
python3 scripts/code-dojo/dojo.py --all
```

The gate runs:

* repo-respect source checks from `scripts/code-dojo/check_files.py`, backed by
  the `xtask` Rust AST checker across tracked and untracked nonignored Rust
  files;
* deterministic test receipt checks from
  `scripts/code-dojo/check_determinism_manifest.py`;
* workspace formatting;
* workspace Clippy with warnings denied;
* strict package-scoped Clippy for unwrap/expect/panic/todo/unimplemented and
  unchecked indexing in library targets where those are standards violations;
* dependency policy through `cargo deny check`;
* workspace native tests;
* `wasm32-unknown-unknown` checks for every WASM-compatible library crate.

The full gate has no local skip flags for Cargo, deterministic receipts, or
WASM. If a rule needs to change, change the standard and enforcement in review.

Documentation changes should also run Markdown lint over every touched Markdown
file when the tool is available:

```bash
markdownlint-cli2 <changed-markdown-files>
```

Release candidates must also verify every crates.io archive before tagging:

```bash
scripts/publish-crates.sh verify
```

## WebAssembly Gates

Every WASM-compatible library crate must run headless under Node.js:

```bash
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-num --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-linalg --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-geom --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-contract --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-query --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-broadphase --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-mesh --locked
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-codec --locked
```

Host-side tooling crates (`bunny-wesley`, `xtask`) are covered by native
workspace tests and are intentionally not described as WASM library crates.

## Test Layout

Source files stay small. Public behavior is tested through integration tests:

| Location | Use |
| --- | --- |
| `crates/<crate>/src/` | Library implementation |
| `crates/<crate>/tests/` | Public API and regression tests |
| `docs/goalposts/` | Acceptance evidence and completion notes |
| `.github/workflows/code-dojo.yml` | Code Dojo, cross-platform, and WASM enforcement |
| `.github/workflows/release.yml` | crates.io release publication |
| `.githooks/` | Repo-local pre-commit, commit-msg, and pre-push hooks |
| `scripts/code-dojo/` | Local and CI repository-respect policy checks |
| `scripts/publish-crates.sh` | Local and CI package publication gate |

## Boundary Fixtures

Fixtures need provenance. A fixture should record:

* Source URL or generation procedure.
* Download or generation date when applicable.
* Any source record IDs, vertices, faces, or raw bytes used.
* Any local remapping performed for reduced fixtures.

For accepted zero-copy parser paths, tests should prove both parsed values and
borrowed source or payload retention. For rejected paths, tests should assert the
specific error variant.

## CI Expectations

GitHub Actions must run:

* Formatting.
* Workspace Clippy with warnings denied.
* Strict package-scoped Clippy for semantic library panic-path and indexing bans.
* Dependency policy through `cargo deny check`.
* Workspace native tests across Linux, macOS, and Windows.
* Code Dojo repo-respect source and deterministic receipt checks.
* WASM compile checks for all WASM-compatible library crates.
* Headless Node.js WASM tests for all WASM-compatible library crates.
* Release archive verification before crates.io publication.

If local and CI behavior diverge, CI wins until the difference is understood and
documented.
