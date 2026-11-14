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

- `pre-commit` — checks staged Rust files with the AST policy gate and the core
  crates' Clippy lint policy, then runs topic documentation contract checks,
  the generated-artifact regeneration check, repo-respect receipt coverage,
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
| `network` | `network` | `std::net`, `reqwest`, `ureq`; the same crates as dependencies; since #321 `std::os::<platform>::net` | 1 of 6 (as `ambient-state`) | `network_rejects_every_seeded_form` |
| `unordered-container` | `unordered-container` | `HashMap`, `HashSet`, `FxHashMap`, `FxHashSet` by name; `std::collections::hash_map` and `hash_set` except `DefaultHasher`; `std::hash::RandomState`; `hashbrown`, `rustc_hash`; crates `hashbrown`, `rustc-hash` | 2 of 10 (as `nondeterministic-map`) | `unordered_container_rejects_every_seeded_form` |
| `hand-hashed-table` | `hand-hashed-table` | a line, comments included, naming open addressing | 0 of 2 | `hand_hashed_table_rejects_every_seeded_form` |
| `json-serialization` | `json-serialization` | `serde_json` paths and macros; crate `serde_json` | 0 of 5 | `json_serialization_rejects_every_seeded_form` |
| `wasm-serde` | `wasm-serde` | `serde_wasm_bindgen`; crate `serde-wasm-bindgen` | 0 of 2 | `wasm_serde_rejects_every_seeded_form` |
| `float-sentinel` | `float-boundary` | any `f32`/`f64` path in core, so `NAN`, `INFINITY`, `NEG_INFINITY`, `consts` | 0 of 5 | `float_sentinels_are_rejected_as_float_boundary` |
| `float-op` | `float-boundary` | float types, float literals and float paths in core, also inside macros; since #321 names ending in `_f32`/`_f64` (`Duration::mul_f64`, `as_secs_f64`, `to_f32`) | 1 of 3 | `float_operations_are_rejected_as_float_boundary` |
| `std-env` | `std-env` | `std::env`; `env!` and `option_env!` since #321 | 1 of 6 (as `ambient-state`) | `std_env_rejects_every_seeded_form` |
| `std-fs` | `std-fs` | `std::fs`, and the names `fs::…` and `File::open`; since #321 `Path` methods that read the filesystem, `std::os::<platform>::fs`, `io`, `prelude`, and `std::os::fd` | 2 of 5 (as `ambient-state`) | `std_fs_rejects_every_seeded_form` |
| `std-process` | `std-process` | `std::process`; since #321 `std::os::<platform>::process` and `std::io::pipe` | 0 of 5 | `std_process_rejects_every_seeded_form` |

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
| `#[path]` emitted by a macro, under nested `cfg_attr`, second in a `cfg_attr`, into `vendor`, `third_party` or `target` (found in review) | `include-source` | 0 of 6 | `path_attributes_in_macros_nested_cfg_attr_and_unscanned_directories_are_rejected` |
| `include_str!`/`include_bytes!` outside the crate, absolute, computed, renamed, in a macro | `include-data` (Bunny's own) | 0 of 5 | `included_data_must_come_from_inside_the_crate` |
| `env!`, `option_env!`, `std::env!`, `core::option_env!`, `env!("CARGO_PKG_VERSION")`, in a macro | `std-env` | 1 of 6 (`std::env!`, through the `std::env` prefix) | `compile_time_environment_macros_are_std_env` |

| `std::thread::sleep_ms` under `#[allow(deprecated)]`, `park`, `park_timeout_ms`, `yield_now`, `park_timeout` by alias; `SystemTimeError` | `thread-sleep`, `time-system` | 0 of 5 | `sibling_names_in_banned_families_are_rejected` |
| `d.mul_f64(d.as_secs_f64())`, `Duration::from_secs_f64(text.parse().ok()?)`, `div_duration_f32`, `as_secs_f32` in `format!`, `x.to_f32()` | `float-boundary` | 0 of 5 | `float_methods_with_no_float_token_are_float_boundary` |
| `use core::f64::consts::PI as P`, a renamed `to_f32`, the same in a group, a `core::f32::consts` glob (found in review) | `float-boundary` | 0 of 4 | `float_names_in_use_declarations_are_float_boundary` |

| `p.exists()`, `try_exists`, `is_file`/`is_dir`, `metadata`, `canonicalize`, `read_dir`, `Path::exists(p)`, `PathBuf::is_symlink`, in `format!` | `std-fs` | 0 of 9 | `path_methods_that_read_the_filesystem_are_std_fs` |
| `std::os::{unix,windows,linux}::fs`, `std::os::fd`, a platform `prelude` glob; `std::os::unix::net`; `std::os::unix::process`, `std::io::pipe`; `std::path::absolute`, `.absolute()` | `std-fs`, `network`, `std-process`, `std-env` | 0 of 10 | `platform_filesystem_modules_are_banned` |
| `std::io::stdin`, `Stdin`, `IsTerminal`, `.is_terminal()`, by alias, in `format!` | `host-input` (Bunny's own) | 0 of 6 | `terminal_input_is_host_input` |
| banned code in `///` and `//!` doc tests, behind a hidden `# ` line, in `rust,no_run`, in `/** */` | `std-fs`, `time-instant`, `unordered-container`, `thread-sleep`, `std-env` | 0 of 5 | `doc_tests_are_scanned` |

With these the rule tests seed 180 forms; the gate before #321 caught 93.

flyingrobots/bunny#347 adds the constructions a second review found, seeded in
`xtask/src/nondeterminism/macro_tests.rs`:

| Construction | Rules seeded | Caught before #347 | Test |
| --- | --- | --- | --- |
| a metavariable next to `::` (`std::time::$t::now()`, `std::$m::read`, `$root::time`, a `::` repetition, in a `use`, in a nested `format!`), as a macro name (`$m!($p)`, `$m![..]`), as a method name, in an attribute (`#[path = $p]`, `#[$a]`, `#![$a]`, `cfg_attr(.., path = $p)`) | `macro-metavariable` (Bunny's own) | 2 of 15 (`$root::time::Instant::now()` as `time-instant`, the `cfg_attr` form as `include-source`) | `metavariables_in_unresolvable_positions_are_rejected` |
| `include!`, `env!`, `option_env!`, `include_str!` through `std::prelude::v1`, `core::prelude::*` and a prelude alias, with and without an import; `file!` bare, under `core`, renamed; `is_x86_feature_detected!`, `is_aarch64_feature_detected!` | `include-source`, `std-env`, `include-data`, `host-cpu` | 0 of 13 | `banned_macros_are_found_through_every_std_path` |
| `Backtrace::capture`, `force_capture`, `use std::backtrace::Backtrace`, `std::panic::Location`, `core::panic::Location`, `get_backtrace_style`; `std::arch`, `core::arch`; `std::random`; `std::os::unix::thread`, `std::os::windows::ffi`, `use std::os;`; `io::Error::last_os_error` | `std-env`, `host-cpu`, `random`, `host-platform`, `std-process` | 0 of 13 | `host_state_beyond_the_earlier_lists_is_rejected` |
| a module or macro directly below `std`, `core` or `alloc` that no toolchain documents | `unclassified-std` (Bunny's own) | 0 of 4 | `an_unclassified_std_module_is_reported` |

That is 45 more forms, of which the gate before #347 caught 2, both under
another rule. Every module and macro at the roots of `std`, `core` and
`alloc` is now classified ([the std
classification](topics/code-dojo-determinism/README.md#the-std-classification));
`std_modules_tests.rs` probes the 245 documented names and the eight platform
modules of `std::os` against the gate.
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

### What The Rules Do Not Cover

The source rules read syntax and each file's own imports. After
flyingrobots/bunny#321 these remain outside them:

- A float whose type is only inferred, with no float type, literal or
  `_f32`/`_f64` name on the way (`text.parse()` passed to a float parameter,
  `x.into()` to an `f64`); pinned as a known gap by
  `an_inferred_float_with_no_float_name_is_not_seen`.
- A banned type that reaches a file only as another crate's return type, or
  only through a `crate::` re-export declared in another file (the re-export
  itself is reported where it is declared), and code that a dependency's
  procedural macro generates.
- Symbolic links behind an `include_str!`, `include_bytes!` or `#[path]`
  target, and doc text brought in with `#[doc = include_str!(..)]`.
- What a macro of another crate expands to: a dependency's `macro_rules!`
  that builds a banned path from its arguments is read only where it is
  invoked, from the invocation's tokens. Inside the deterministic crates a
  metavariable cannot build a path, a macro or method name or an attribute
  (`macro-metavariable`), so this concerns dependencies, which the dependency
  check restricts.
- A standard macro reached through a re-export in another crate (`pub use
  std::include as inc;` in a dependency): the `use` is reported in the crate
  that writes it, if that crate is scanned.
- Writing to standard output or standard error, and threads: both permitted on
  purpose (see Lumen below for the difference on threads).
- Dev-dependencies, in the dependency check.

Known over-reports, each resolvable with a waiver: a zero-argument method of
Bunny's own named `exists`, `metadata`, `absolute`, `is_terminal` or like the
other filesystem `Path` methods; a user macro called by the bare name
`include`, `env`, `option_env` or `file`; a metavariable that a macro uses
safely next to `::` (`$t::MAX`; `<$t>::MAX` is accepted); a `#[path]` inside an inline module, which
the compiler resolves against a deeper directory; any name ending in `_f32`
or `_f64` that involves no float.

### Compared With Lumen's Gate

Lumen's gate (flyingrobots/lumen, `docs/adr/0006-nondeterminism-gate.md`)
uses the same Echo rule IDs. Since flyingrobots/bunny#321 the two lists
agree on `Path::exists` (Bunny also bans the nine other `Path` methods that
read the filesystem), `env!` and `option_env!`, module imports and renames,
glob imports, `std::hash::RandomState`, and float literals and float names
inside macro bodies. They still differ:

- **Threads.** Lumen bans `std::thread::spawn`, `scope` and `Builder`
  (`thread-spawn`) outside its scheduler. Bunny permits them: Rule 6 admits
  parallelism that does not change results, and tests, not the gate, show
  that. Both ban `available_parallelism` and sleeping.
- **How floats are caught.** Lumen bans the `f32` and `f64` types through
  Clippy and denies `clippy::float_arithmetic`, which also sees arithmetic on
  an inferred float. Bunny's `float-boundary` is syntactic: float types,
  literals, paths and `_f32`/`_f64` names, with waivers for boundary
  conversions; an inferred float with no float name is not seen.
- **Mechanism.** Lumen's first layer is Clippy's `disallowed-types`,
  `disallowed-methods` and `disallowed-macros`, which resolve paths exactly,
  re-exports included; Bunny reads syntax with `syn` and resolves through
  the file's own imports. Lumen's dependency rule is `cargo-deny` allowing no
  registry at all; Bunny lists banned crates over `cargo tree`.
- **Bunny only.** `include-source`, `include-data`, `host-input`, the
  platform modules of `std::os`, `std::path::absolute`, `std::io::pipe`, and
  doc tests.
- **Macros (since flyingrobots/bunny#347).** Bunny bans metavariables where
  they would build a path, a macro or method name or an attribute
  (`macro-metavariable`), recognises `include!`, `env!`, `option_env!` and
  `file!` under every `std`, `core` and `alloc` path including the preludes,
  and classifies every root module and macro of the three crates
  (`unclassified-std`). Lumen's main branch (9ae9d84) has none of these in
  its syntax scan; its Clippy layer lints code after macro expansion, which a
  syntax scan cannot, for the items its disallowed lists name.
  flyingrobots/lumen#155 to #158 (open) cover the same ground there:
  `std::os` items and CPU features, `c_double`, aliases, and code outside the
  scanned crates. `std::backtrace`, `Location` and `file!` are banned in
  Bunny; whether Lumen bans them is for its #155.
- **Waivers.** Lumen's waiver covers its line or the line below, and a waiver
  that waives nothing fails; Bunny's covers the item or statement below it,
  and an unused waiver is not reported.

### Keeping The Name Lists Current

`std::time` and `std::thread` are banned item by item, not as modules, so a
sibling such as the deprecated `sleep_ms` cannot slip past a prefix. Every
module and macro at the roots of `std`, `core` and `alloc`, and every module
of `std::os`, is classified the same way, and a path through an unclassified
root name is an `unclassified-std` finding. The item
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
  done
  for root in std core alloc std/os; do
    page=$(curl -fsSL https://doc.rust-lang.org/$V/$root/index.html)
    if [ "$root" = std/os ]; then echo "[std::os]"; else echo "[$root modules]"; fi
    echo "$page" | grep -oE "class=\"mod\" href=\"[a-z0-9_]+/index.html\"" |
      sed -E "s/class=\"mod\" href=\"([a-z0-9_]+)\/index.html\"/mod \1/"
    [ "$root" = std/os ] && break; echo; echo "[$root macros]"
    echo "$page" | grep -oE "class=\"macro\" href=\"macro\.[a-z0-9_]+\.html\"" |
      sed -E "s/class=\"macro\" href=\"macro\.([a-z0-9_]+)\.html\"/macro \1/"
    echo
  done'
```

prefix its three comment lines and save the output as the new
`std_api_<version>.txt` (it reproduces the 1.96.0 file exactly), classify
each new item in the test's table, each new root module or macro in
`std_modules.rs`, and both in the rules, and read the release notes for
renamed items.

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

The full gate runs `cargo tree --locked --edges normal,build --target all
--no-dedupe` for the deterministic library crates twice, with the features the
workspace enables and with `--all-features`, and rejects any of these in either
graph: `rand`, `getrandom`, `fastrand` (`random`); `js-sys`, `wasm-bindgen`,
`web-sys` (`host-callback`); `reqwest`, `ureq` (`network`); `hashbrown`,
`rustc-hash` (`unordered-container`); `serde_json` (`json-serialization`);
`serde-wasm-bindgen` (`wasm-serde`); `tokio`, `async-std` (`thread-sleep`).
`--all-features` finds a banned crate behind an optional dependency that no
workspace crate enables, which a downstream crate could still turn on. Build
edges find a banned crate a build script could use; a build script's own
source (`build.rs`) is a Rust file of the crate and is read by the source
rules. No deterministic crate has a build script or build-dependencies today.
A finding names the deterministic crate whose manifest declares the path to
the banned crate and the feature set that found it first; a crate reached
through another deterministic crate is reported against that crate only.
Dev-dependencies are not part of the check: core tests depend on
`wasm-bindgen-test`, which brings `wasm-bindgen` and `js-sys` into test builds
only.

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

## Lint Policy

The `blanket-nursery` check (`xtask/src/lint_policy.rs`) holds the core crates
to Rule 10 of `CODE_STANDARDS.md`: it fails the full gate and the pre-commit
gate on any `deny`, `warn` or `forbid` of `clippy::nursery` as a group in a
core crate's `src/lib.rs`, its manifest or the workspace manifest, naming the
file and line. It runs right after the Rust source rules and has no waiver.
The chapter is [`topics/lint-policy/`](topics/lint-policy/).

## Generated Artifacts

The full gate and the pre-commit gate regenerate `bunny-wesley`'s contract
artifacts in a per-run directory under `target/dojo-generated/` and fail when
a committed artifact differs (the pre-commit gate compares the staged schema
and artifacts, the full gate the working tree), naming the file, its first differing line and
`cargo run --locked -p xtask -- generate`. Run it alone with
`cargo run --locked -p xtask -- code-dojo-generated`. See the
[contract-generation chapter](topics/contract-generation/README.md#regeneration-check).

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
whatever text no token covers is whitespace or a comment, and each comment is
read whole, so a block comment over several lines with two clauses waives
neither rule.

A dependency-rule waiver is the same clause in a whole-line TOML comment
(`#` first on the line) anywhere in the manifest of the crate the finding
names; the same text inside a TOML string (a line inside a multi-line
`"""` or `'''` string included), or after a value on the line, is not a
waiver:

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
