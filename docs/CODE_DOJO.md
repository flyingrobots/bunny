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

## Nondeterminism Rules

Code Dojo enforces the rules of Echo's `ban-nondeterminism` scan
(`scripts/ban-nondeterminism.sh` in Echo) and uses Echo's rule IDs as their
names, so a rule ID means the same ban in both repositories. Bunny has one
mechanism for them: the Rust AST gate (`xtask/src/nondeterminism.rs`, applied
by `xtask/src/code_dojo.rs`) and one dependency check in the full gate. There
is no shell scan and no separate allowlist file. The topic chapter is
[`topics/code-dojo-determinism/`](topics/code-dojo-determinism/).

### Scope

The source rules apply to every Rust file in the deterministic library crates:
the core crates (`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-query`,
`bunny-broadphase`, `bunny-mesh`, `bunny-optics`) plus `bunny-codec` and
`bunny-contract`, which are WASM libraries producing canonical data (decoded
meshes and the contract DTOs). Their tests are included, because a golden
vector that depends on the clock or the environment is not a golden vector.

Outside the rules, each for a stated reason:

- `bunny-perf` measures time; its job is `Instant`.
- `xtask` runs Git and Cargo, reads and writes repository files, and times
  benchmarks.
- `bunny-wesley` is a generator: it reads schema files and writes artifacts and
  a JSON manifest, so files and `serde_json` are its job. Its determinism is a
  property of its outputs: the generated Rust, TypeScript and manifest are
  committed, and `bunny-contract`'s tests check their witnesses. No gate
  regenerates them and compares the bytes.

`float-boundary` keeps its older, narrower scope: core crates only, test files
excluded. `bunny-codec` parses float text and bytes by design.

### Mapping

Every Echo rule, the Code Dojo rule that enforces it, and what the gate
caught before flyingrobots/bunny#279. "Before" counts the seeded forms in the
rule's test that the earlier gate reported under any rule name; the forms are
a qualified path, a `use`, a `use` group, a renamed import, a module alias, a
type position, a macro body and a glob import, as applicable. The tests are in
`xtask/src/nondeterminism/tests.rs`.

| Echo rule ID | Code Dojo rule | What it bans in Bunny | Before #279 | Test |
| --- | --- | --- | --- | --- |
| `time-system` | `time-system` | `std::time::SystemTime`, `UNIX_EPOCH` | 2 of 9 (as `ambient-state`) | `time_system_rejects_every_seeded_form` |
| `time-instant` | `time-instant` | `std::time::Instant` | 2 of 9 (as `ambient-state`) | `time_instant_rejects_every_seeded_form` |
| `thread-sleep` | `thread-sleep` | `std::thread::sleep`, `tokio`/`async_std` sleeps; crates `tokio`, `async-std` | 0 of 7 | `thread_sleep_rejects_every_seeded_form` |
| `host-parallelism` | `host-parallelism` | `available_parallelism`, qualified or bare | 0 of 5 | `host_parallelism_rejects_every_seeded_form` |
| `random` | `random` | `rand`, `getrandom`, `fastrand`, `thread_rng`; the same crates as dependencies | 2 of 7 (as `ambient-state`) | `random_rejects_every_seeded_form` |
| `host-callback` | `host-callback` | `js_sys`, `wasm_bindgen`, `web_sys` paths; crates `js-sys`, `wasm-bindgen`, `web-sys` | 0 of 4 | `host_callback_rejects_every_seeded_form` |
| `network` | `network` | `std::net`, `reqwest`, `ureq`; the same crates as dependencies | 1 of 6 (as `ambient-state`) | `network_rejects_every_seeded_form` |
| `unordered-container` | `unordered-container` | `HashMap`, `HashSet`, `FxHashMap`, `FxHashSet` by name; `std::collections::hash_map` and `hash_set` except `DefaultHasher`; `std::hash::RandomState`; `hashbrown`, `rustc_hash`; crates `hashbrown`, `rustc-hash` | 2 of 10 (as `nondeterministic-map`) | `unordered_container_rejects_every_seeded_form` |
| `hand-hashed-table` | `hand-hashed-table` | a line, comments included, naming open addressing | 0 of 2 | `hand_hashed_table_rejects_every_seeded_form` |
| `json-serialization` | `json-serialization` | `serde_json` paths and macros; crate `serde_json` | 0 of 5 | `json_serialization_rejects_every_seeded_form` |
| `wasm-serde` | `wasm-serde` | `serde_wasm_bindgen`; crate `serde-wasm-bindgen` | 0 of 2 | `wasm_serde_rejects_every_seeded_form` |
| `float-sentinel` | `float-boundary` | any `f32`/`f64` path in core, so `NAN`, `INFINITY`, `NEG_INFINITY`, `consts` | 0 of 5 | `float_sentinels_are_rejected_as_float_boundary` |
| `float-op` | `float-boundary` | float types, float literals and float paths in core, also inside macros | 1 of 3 | `float_operations_are_rejected_as_float_boundary` |
| `std-env` | `std-env` | `std::env` | 1 of 6 (as `ambient-state`) | `std_env_rejects_every_seeded_form` |
| `std-fs` | `std-fs` | `std::fs`, and the names `fs::…` and `File::open` | 2 of 5 (as `ambient-state`) | `std_fs_rejects_every_seeded_form` |
| `std-process` | `std-process` | `std::process` | 0 of 5 | `std_process_rejects_every_seeded_form` |

Where the names differ:

- Bunny's earlier `ambient-state` rule covered clocks, `thread_rng`,
  `rand::random`, `std::env::args`/`var`, the filesystem and two network
  calls under one name. It is split into `time-system`, `time-instant`,
  `random`, `std-env`, `std-fs` and `network`, so a waiver names one ban.
- Bunny's earlier `nondeterministic-map` rule is renamed `unordered-container`.
  No waiver in the repository named either retired rule.
- Echo's `float-sentinel` and `float-op` map to Bunny's `float-boundary`, which
  is broader: core crates may not touch floats at all without a waiver. Echo's
  `float-op` matches the method names `sin`, `cos`, `tan`, `sqrt` and `pow`;
  Bunny cannot, because its fixed-point types have `sqrt` and `sin` methods
  (`FixedQ32_32::sqrt`, `FixedAngle::sin`) and integer `pow` is ordinary
  arithmetic. Bunny bans the float instead of the operation.
- `host-callback` cannot fire in a compiling core crate today, since no
  deterministic crate has `js-sys` or `wasm-bindgen` as a normal dependency
  (`wasm-bindgen-test` is a dev-dependency of their tests and is accepted).
  The dependency check is what keeps it that way.

### How the source rules read code

Each path is resolved through the file's `use` declarations before it is
matched: `use std::time as clock;` makes `clock::Instant::now()` a
`time-instant` finding, and `use std::collections::HashMap as Map;` makes
`Map::new()` an `unordered-container` finding. Every `use` leaf is checked
itself, inside groups and with `self` resolved, and a glob import is reported
for every banned item it would bring into scope (`use std::time::*;` is both
`time-system` and `time-instant`). Macro bodies, which `syn` leaves as tokens,
are searched for paths and, in core, for float literals. A rule is reported at
most once per line.

Resolution is file-wide, not per module, so it can only over-report. A type
that reaches a file only as the inferred return type of another crate's
function is invisible to the source rules; the dependency check covers
third-party crates for that reason.

### Dependency Check

The full gate runs `cargo tree --locked --edges normal --target all
--no-dedupe` for the deterministic library crates and rejects any of these in
their normal dependency graph: `rand`, `getrandom`, `fastrand` (`random`);
`js-sys`, `wasm-bindgen`, `web-sys` (`host-callback`); `reqwest`, `ureq`
(`network`); `hashbrown`, `rustc-hash` (`unordered-container`); `serde_json`
(`json-serialization`); `serde-wasm-bindgen` (`wasm-serde`); `tokio`,
`async-std` (`thread-sleep`). A finding names the deterministic crate whose
manifest declares the path to the banned crate; a crate reached through
another deterministic crate is reported against that crate only. Dev and
build dependencies are not part of the check.

### No-Default-Features Check

`bunny-mesh` and `bunny-codec` reach crates.io only through their default
`mesh-hash` feature. The full gate lists `cargo tree --locked -p <crate>
--no-default-features --edges normal,build --target all --prefix none --format
{p}` for each and fails on any crate that is not a local path crate. It then
runs Clippy (`-D warnings`), `cargo test` (doctests included) and a
`wasm32-unknown-unknown` check of `-p bunny-mesh -p bunny-codec
--no-default-features`. Each command selects only these two packages, so no
other workspace member re-enables the feature through feature unification. The
check is `xtask/src/feature_configs.rs`.

### Where It Runs

- `cargo run --locked -p xtask -- code-dojo --all` runs the source rules on
  every tracked Rust file and then the dependency check. CI runs it in the
  `dojo-ubuntu` job of `.github/workflows/code-dojo.yml`; that job is the
  enforcing gate.
- `code-dojo-rust --all` and `--staged` run the source rules only.
- The `pre-commit` hook runs the source rules on staged Rust files; the
  `pre-push` hook runs the full gate, dependency check included. The hooks run
  only in a clone where `scripts/install-githooks.sh` has set
  `core.hooksPath` to `.githooks`; nothing installs them automatically, so a
  fresh clone or a new worktree commits and pushes without them.

## Waivers

Waivers are allowed only when they are local, specific, and justified.

Examples:

```rust
// dojo: allow indexing -- loop bounds prove i < len; covered by golden vector ray_box_touching_face
let point = points[i];

// dojo: allow float-boundary -- DTO ingress only; converted to Q32x32 before canonical math
pub x: f32,
```

A waiver covers its own line and the two lines below it, names exactly one
rule, and needs a reason of at least eight characters after `--`. A waiver
with no reason, a shorter reason, or another rule's name leaves the finding
reported. A dependency-rule waiver is the same comment, with `#`, anywhere in
the manifest of the crate the finding names:

```toml
# dojo: allow random -- seeded generator behind the reviewed `fuzz` feature
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
