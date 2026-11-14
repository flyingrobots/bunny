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
| `time-system` | `time-system` | `std::time::SystemTime`, `UNIX_EPOCH`; `SystemTimeError` since #321 | 2 of 9 (as `ambient-state`) | `time_system_rejects_every_seeded_form` |
| `time-instant` | `time-instant` | `std::time::Instant` | 2 of 9 (as `ambient-state`) | `time_instant_rejects_every_seeded_form` |
| `thread-sleep` | `thread-sleep` | `std::thread::sleep`, `tokio`/`async_std` sleeps; crates `tokio`, `async-std`; since #321 also `sleep_ms`, `sleep_until`, `park`, `park_timeout`, `park_timeout_ms`, `yield_now` | 0 of 7 | `thread_sleep_rejects_every_seeded_form` |
| `host-parallelism` | `host-parallelism` | `available_parallelism`, qualified or bare | 0 of 5 | `host_parallelism_rejects_every_seeded_form` |
| `random` | `random` | `rand`, `getrandom`, `fastrand`, `thread_rng`; the same crates as dependencies | 2 of 7 (as `ambient-state`) | `random_rejects_every_seeded_form` |
| `host-callback` | `host-callback` | `js_sys`, `wasm_bindgen`, `web_sys` paths; crates `js-sys`, `wasm-bindgen`, `web-sys` | 0 of 4 | `host_callback_rejects_every_seeded_form` |
| `network` | `network` | `std::net`, `reqwest`, `ureq`; the same crates as dependencies | 1 of 6 (as `ambient-state`) | `network_rejects_every_seeded_form` |
| `unordered-container` | `unordered-container` | `HashMap`, `HashSet`, `FxHashMap`, `FxHashSet` by name; `std::collections::hash_map` and `hash_set` except `DefaultHasher`; `std::hash::RandomState`; `hashbrown`, `rustc_hash`; crates `hashbrown`, `rustc-hash` | 2 of 10 (as `nondeterministic-map`) | `unordered_container_rejects_every_seeded_form` |
| `hand-hashed-table` | `hand-hashed-table` | a line, comments included, naming open addressing | 0 of 2 | `hand_hashed_table_rejects_every_seeded_form` |
| `json-serialization` | `json-serialization` | `serde_json` paths and macros; crate `serde_json` | 0 of 5 | `json_serialization_rejects_every_seeded_form` |
| `wasm-serde` | `wasm-serde` | `serde_wasm_bindgen`; crate `serde-wasm-bindgen` | 0 of 2 | `wasm_serde_rejects_every_seeded_form` |
| `float-sentinel` | `float-boundary` | any `f32`/`f64` path in core, so `NAN`, `INFINITY`, `NEG_INFINITY`, `consts` | 0 of 5 | `float_sentinels_are_rejected_as_float_boundary` |
| `float-op` | `float-boundary` | float types, float literals and float paths in core, also inside macros; since #321 names ending in `_f32`/`_f64` (`Duration::mul_f64`, `as_secs_f64`, `to_f32`) | 1 of 3 | `float_operations_are_rejected_as_float_boundary` |
| `std-env` | `std-env` | `std::env`; `env!` and `option_env!` since #321 | 1 of 6 (as `ambient-state`) | `std_env_rejects_every_seeded_form` |
| `std-fs` | `std-fs` | `std::fs`, and the names `fs::…` and `File::open` | 2 of 5 (as `ambient-state`) | `std_fs_rejects_every_seeded_form` |
| `std-process` | `std-process` | `std::process` | 0 of 5 | `std_process_rejects_every_seeded_form` |

flyingrobots/bunny#321 adds the constructions an independent review found
getting past these rules, seeded in `xtask/src/nondeterminism/evasion_tests.rs`
and counted with the forms above:

| Construction | Rules seeded | Caught before #321 | Test |
| --- | --- | --- | --- |
| `extern crate std as host;` then `host::env`, `host::fs`, `host::time`; `extern crate rand` | `std-env`, `std-fs`, `time-instant`, `random` | 0 of 5 | `extern_crate_aliases_resolve_like_imports` |
| raw identifiers: `r#std`, `r#env`, `r#HashMap`, `r#HashSet` in a macro, a raw alias, `r#unwrap`, `r#panic!` | `std-fs`, `std-env`, `unordered-container`, `std-process`, `panic-path` | 0 of 7 | `raw_identifiers_match_their_plain_names` |
| `r#f64` as a type and a path | `float-boundary` | 0 of 2 | `raw_float_types_are_float_boundary` |
| `use std::{self as s};` then `s::fs` | `std-fs`, `std-env` | 2 of 2 (no test failed when `self` was treated as a segment) | `self_in_a_use_group_names_the_module_itself` |

| `include!` in every spelling: a non-Rust file, a `.rs` file, a `concat!` path, `std::`/`core::`, renamed, in a macro | `include-source` (Bunny's own) | 0 of 8 | `include_of_source_is_rejected_however_it_is_named` |
| `#[path]` to a non-`.rs` file, outside the crate, absolute, with backslashes, through `cfg_attr` | `include-source` | 0 of 5 | `path_attributes_must_name_a_scanned_file` |
| `include_str!`/`include_bytes!` outside the crate, absolute, computed, renamed, in a macro | `include-data` (Bunny's own) | 0 of 5 | `included_data_must_come_from_inside_the_crate` |
| `env!`, `option_env!`, `std::env!`, `core::option_env!`, `env!("CARGO_PKG_VERSION")`, in a macro | `std-env` | 1 of 6 (`std::env!`, through the `std::env` prefix) | `compile_time_environment_macros_are_std_env` |

| `std::thread::sleep_ms` under `#[allow(deprecated)]`, `park`, `park_timeout_ms`, `yield_now`, `park_timeout` by alias; `SystemTimeError` | `thread-sleep`, `time-system` | 0 of 5 | `sibling_names_in_banned_families_are_rejected` |
| `d.mul_f64(d.as_secs_f64())`, `Duration::from_secs_f64(text.parse().ok()?)`, `div_duration_f32`, `as_secs_f32` in `format!`, `x.to_f32()` | `float-boundary` | 0 of 5 | `float_methods_with_no_float_token_are_float_boundary` |

With these the rule tests seed 140 forms; the gate before #321 caught 93.
Separately, `xtask/src/nondeterminism/std_api_tests.rs` probes each of the 29
documented items of `std::thread` and `std::time` as a path and as a `use`
(58 probes, 14 of which the gate before #321 got wrong) and each of the 14
float methods of `Duration` as a method call and as a path (28 probes, all
missed before).
`env!` was an accepted look-alike before #321; it is banned now, as in
Lumen's gate (flyingrobots/lumen, ADR 0006), and the one use on main, in a
`bunny-contract` test, carries a waiver.

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

### Keeping The Name Lists Current

`std::time` and `std::thread` are banned item by item, not as modules, so a
sibling such as the deprecated `sleep_ms` cannot slip past a prefix. The item
lists of the pinned toolchain's documentation are committed as
`xtask/src/nondeterminism/std_api_<version>.txt`; `std_api_tests.rs` fails
when the documentation lists an item its classification table does not name,
when the gate reports anything but the classified rule for an item, and when
`rust-toolchain.toml` pins a toolchain other than the one the file was read
from. When the toolchain moves, regenerate the lists in the container (the
documentation pages are the source; the image has no `rust-src` or
`rust-docs`):

```sh
python3 ~/git/devcontainers/bin/devrun bunny <worktree> --worktree --network -- bash -c '
  V=<version>; echo "toolchain $V"; echo
  for m in thread time; do echo "[std::$m]"
    curl -fsSL https://doc.rust-lang.org/$V/std/$m/index.html |
      grep -oE "class=\"(fn|struct|macro|type|enum|trait|constant)\" href=\"[^\"]+\"" |
      sed -E "s/class=\"([a-z]+)\" href=\"[a-z]+\.([A-Za-z0-9_]+)\.html\"/\1 \2/"
    echo
  done
  for page in time/Duration path/Path; do echo "[std::${page%/*}::${page#*/} methods]"
    curl -fsSL https://doc.rust-lang.org/$V/std/${page%/*}/struct.${page#*/}.html |
      grep -oE "id=\"method\.[a-z0-9_]+\"" | sed -E "s/id=\"method\.(.*)\"/\1/" | LC_ALL=C sort -u
    echo
  done'
```

prefix its three comment lines and save the output as the new
`std_api_<version>.txt` (the first run reproduced the 1.96.0 file
exactly), classify each new item in
the test's table and in the rules, and read the release notes for renamed
items.

### How the source rules read code

Each path is resolved through the file's `use` declarations and `extern
crate` renames before it is matched, comparing identifiers without their `r#`
prefix: `use std::time as clock;` makes `clock::Instant::now()` a
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

A waiver is a `//` or `/* */` comment whose text is exactly one clause,
`dojo: allow <rule> -- <reason>`, naming one rule by its exact ID, with a
reason of at least eight characters after ` -- `. Text that only looks like
a waiver waives nothing: inside a string, raw-string, byte-string or
character literal, in a doc comment or `#[doc]` attribute, or in a macro's
tokens. A comment with two clauses waives neither rule. A waiver with no
reason, a shorter reason or another rule's name leaves the finding reported,
and does not hide a valid waiver for the same finding above or below it.

What a waiver covers:

- On a line that also holds code, that line only.
- On a line of its own, that line, the comment lines below it, and every line
  of the outermost item, statement, field, enum variant, match arm or
  struct-literal field that starts on the next line holding code. A node
  starts on its first line and again on its first line after its attributes
  and doc comments, so a waiver between a doc comment and `fn` covers the
  function. A waiver
  above a function covers the whole function and nothing after it; a waiver
  above one statement does not cover the next statement. When no such node
  starts on that line (inside a macro's tokens, or on a continuation line of
  an expression), it covers that one line.

The gate finds comments from the token stream (`xtask/src/waivers.rs`):
whatever part of a line no token covers is a comment.

A dependency-rule waiver is the same clause in a whole-line TOML comment
(`#` first on the line) anywhere in the manifest of the crate the finding
names; the same text inside a TOML string, or after a value on the line, is
not a waiver:

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
