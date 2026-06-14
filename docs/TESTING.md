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

Run these before opening or updating a goalpost PR:

```bash
cargo +1.96.0 fmt --all -- --check
git diff --check
cargo +1.96.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.96.0 test --locked --workspace --all-targets
cargo +1.96.0 check --locked -p bunny-num -p bunny-linalg -p bunny-geom \
  -p bunny-contract -p bunny-query -p bunny-broadphase -p bunny-mesh \
  -p bunny-codec --target wasm32-unknown-unknown
```

Documentation changes should also run Markdown lint over every touched Markdown
file:

```bash
markdownlint-cli2 <changed-markdown-files>
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
| `.github/workflows/ci.yml` | Cross-platform and WASM enforcement |

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
* Workspace native tests across Linux, macOS, and Windows.
* WASM compile checks for all WASM-compatible library crates.
* Headless Node.js WASM tests for all WASM-compatible library crates.

If local and CI behavior diverge, CI wins until the difference is understood and
documented.
