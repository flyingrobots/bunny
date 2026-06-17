# Code Dojo™

Code Dojo is the repository-respect enforcement layer for this standard. It combines local Git hooks,
CI gates, a Rust AST policy checker, strict Clippy passes, and Rust `xtask` orchestration commands.

## Local Hooks

Install:

```sh
bash scripts/install-githooks.sh
```

Hooks installed:

- `pre-commit` — checks staged Rust files with the AST policy gate, then runs
  `cargo fmt --check`, workspace Clippy, strict package-scoped library Clippy,
  dependency policy, and `cargo test`.
- `commit-msg` — enforces focused commit subjects and AI receipt trailers when applicable.
- `pre-push` — runs the full dojo, including deterministic manifest checks and WASM check.

The stable local and CI entrypoint is:

```sh
cargo run --locked -p xtask -- code-dojo --all
```

Rust source-shape policy is parsed through `cargo run --locked -p xtask -- code-dojo-rust`.

## CI

Workflow:

```text
.github/workflows/code-dojo.yml
```

Jobs:

- `dojo-ubuntu` — runs `cargo run --locked -p xtask -- code-dojo --all --ci`,
  covering formatting, Rust AST policy, dependency policy, Clippy, native tests,
  deterministic receipts, and wasm32 library checks.
- `determinism-matrix` — runs workspace tests and deterministic receipt
  enforcement on `ubuntu-latest`, `macos-26`, and `windows-latest`.
- `wasm-headless` — runs headless Node `wasm-pack test` for each
  WASM-compatible library crate.

The workflow uses GitHub Actions with Rust configured by `rust-toolchain.toml`;
the third-party wasm-pack installer action is pinned by commit.

## Waivers

Waivers are allowed only when they are local, specific, and justified.

Examples:

```rust
// dojo: allow indexing -- loop bounds prove i < len; covered by golden vector ray_box_touching_face
let point = points[i];

// dojo: allow float-boundary -- DTO ingress only; converted to Q32x32 before canonical math
pub x: f32,
```

Bad waiver:

```rust
// dojo: allow unwrap -- safe
```

That is not a reason. That is a vibe in a trench coat.

## No Rollout Bypasses

The local and CI gates do not expose environment variables for skipping Cargo,
missing deterministic receipts, or WASM checks. If a gate is too expensive or
too broad, narrow the repository standard deliberately in review instead of
teaching the tooling to look away.
