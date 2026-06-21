# Code Dojo™

Code Dojo is the repository-respect enforcement layer for this standard. It combines local Git hooks,
CI gates, a Rust AST policy checker, topic documentation contract checks,
strict Clippy passes, and Rust `xtask` orchestration commands.

## Local Hooks

Install:

```sh
bash scripts/install-githooks.sh
```

Hooks installed:

- `pre-commit` — checks staged Rust files with the AST policy gate, then runs
  topic documentation contract checks, repo-respect receipt coverage,
  `cargo fmt --check`, workspace Clippy, strict package-scoped library Clippy,
  dependency policy, `cargo test`, and doctests.
- `commit-msg` — enforces focused commit subjects and repo-respect receipt trailers.
- `pre-push` — runs the full dojo, including repo-respect receipt coverage,
  deterministic manifest checks, and WASM check.

The stable local and CI entrypoint is:

```sh
cargo run --locked -p xtask -- code-dojo --all
```

Rust source-shape policy is parsed through `cargo run --locked -p xtask -- code-dojo-rust`.
Topic documentation contract metadata is checked through
`cargo run --locked -p xtask -- topic-docs`.
Repo-respect receipt templates and checks are handled through
`cargo run --locked -p xtask -- repo-respect`.

## CI

Workflow:

```text
.github/workflows/code-dojo.yml
```

Jobs:

- `dojo-ubuntu` — runs `cargo run --locked -p xtask -- code-dojo --all --ci`,
  covering formatting, Rust AST policy, dependency policy, Clippy, native tests,
  doctests, topic documentation contract metadata, repo-respect receipt
  coverage, deterministic receipts, and wasm32 library checks.
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
