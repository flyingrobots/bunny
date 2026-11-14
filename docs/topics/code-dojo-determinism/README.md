# Code Dojo Determinism Rules

This chapter describes the Code Dojo rules that keep host nondeterminism out of
Bunny's deterministic library crates. They enforce every rule of Echo's
`ban-nondeterminism` scan under Echo's rule IDs.

## Status

This is a living topic chapter. The rules are implemented in
`xtask/src/nondeterminism.rs` and applied by `xtask/src/code_dojo.rs`. The
rule-by-rule mapping from Echo, with what the gate caught before these rules,
is the table in [`docs/CODE_DOJO.md`](../../CODE_DOJO.md#mapping).

The verification design is [`test-plan.md`](test-plan.md).

## Rules

| Rule | Bans |
| --- | --- |
| `time-system` | `std::time::SystemTime`, `std::time::UNIX_EPOCH` |
| `time-instant` | `std::time::Instant` |
| `thread-sleep` | `std::thread::sleep`, `tokio::time::sleep`, `async_std::time::sleep`, `async_std::task::sleep` |
| `host-parallelism` | `std::thread::available_parallelism`, also as a bare name |
| `random` | the `rand`, `getrandom` and `fastrand` crates, and the name `thread_rng` |
| `host-callback` | the `js_sys`, `wasm_bindgen` and `web_sys` crates |
| `network` | `std::net`, the `reqwest` and `ureq` crates, and the names `TcpStream::connect` and `UdpSocket::bind` |
| `unordered-container` | the names `HashMap`, `HashSet`, `FxHashMap`, `FxHashSet`; `std::collections::hash_map` and `hash_set` except `hash_map::DefaultHasher`; `std::hash::RandomState`; the `hashbrown` and `rustc_hash` crates |
| `hand-hashed-table` | any line, comments included, that names open addressing |
| `json-serialization` | the `serde_json` crate |
| `wasm-serde` | the `serde_wasm_bindgen` crate |
| `std-env` | `std::env` |
| `std-fs` | `std::fs`, and the names `fs::…` and `File::open` |
| `std-process` | `std::process` |
| `float-boundary` | in core crates only: `f32` and `f64` types, float literals, and any path through `f32` or `f64` (for example `f32::NAN`, `f64::from_bits`, `core::f64::consts::PI`), also inside macro bodies |

A banned path bans everything below it: `std::fs` covers `std::fs::read` and
`std::fs::File::open`.

`DefaultHasher` is accepted because `DefaultHasher::new()` uses fixed SipHash
keys; the host seeds a hasher only through `RandomState`. Its output can still
change between Rust releases, so it must not reach a golden vector.

### How Paths Are Read

The source rules read `syn` syntax trees, not text:

1. Every `use` leaf is collected from the whole file, including `use` groups,
   nested groups, `self` inside a group, and renames. An `extern crate`
   item is a leaf too: `extern crate std as host;` binds `host` to `std`, and
   `extern crate rand;` is itself a `random` finding.
2. Each leaf is checked as written. A glob import is reported for every banned
   item it would bring into scope, so `use std::thread::*;` is both
   `thread-sleep` and `host-parallelism`.
3. Every other path (expressions, types, patterns, trait bounds, macro
   invocation paths) is expanded through the imports: its first segment is
   replaced by the path it was imported from. The expanded path is matched
   against the banned prefixes; the path as written is matched against the
   banned names.
4. Macro bodies are token streams that `syn` does not parse. Every
   `::`-separated identifier sequence inside them is checked as a path.

Identifiers are compared as the compiler compares them, with any `r#` prefix
removed: `r#std::fs::read`, `std::r#env::var`, `use std::collections::r#HashMap`
and `r#f64` match like their plain spellings, in paths, `use` trees, macro
bodies, method names (`x.r#unwrap()`) and macro names (`r#panic!`).

A rule is reported at most once per line. The import map is file-wide, not
per module, so a name imported in one module also resolves in a sibling
module; this can only over-report.

### Dependency Rules

The full gate also lists the normal dependency graph of every deterministic
library crate with `cargo tree --locked --edges normal --target all
--no-dedupe --prefix depth --format {p}` and rejects these crates:

| Crate | Rule |
| --- | --- |
| `rand`, `getrandom`, `fastrand` | `random` |
| `js-sys`, `wasm-bindgen`, `web-sys` | `host-callback` |
| `reqwest`, `ureq` | `network` |
| `hashbrown`, `rustc-hash` | `unordered-container` |
| `serde_json` | `json-serialization` |
| `serde-wasm-bindgen` | `wasm-serde` |
| `tokio`, `async-std` | `thread-sleep` |

The listing is read as a tree. Below another deterministic crate the subtree
is skipped, because that crate's own tree reports it; a finding therefore
names the crate whose manifest declares the path to the banned crate. Dev and
build dependencies are outside the check: core tests depend on
`wasm-bindgen-test`, which brings `wasm-bindgen` and `js-sys` into test
builds only.

## Scope

The rules cover every Rust file, tests included, in `bunny-num`,
`bunny-linalg`, `bunny-geom`, `bunny-query`, `bunny-broadphase`, `bunny-mesh`,
`bunny-optics`, `bunny-codec` and `bunny-contract`. `float-boundary` applies
only to the non-test files of the first seven, the core crates.

`bunny-perf`, `xtask` and `bunny-wesley` are outside the rules: they time
code, run Git and Cargo, and read and write files by design.
[`docs/CODE_DOJO.md`](../../CODE_DOJO.md#scope) gives the reason for each.

## Waivers

A source waiver is a comment on the finding's line or one of the two lines
above it:

```rust
// dojo: allow std-fs -- fixture loader for the golden vectors, reviewed in #NNN
```

It names one rule and needs a reason of at least eight characters after `--`.
A waiver without a reason, with a shorter reason, or naming another rule
leaves the finding reported.

A dependency waiver is the same comment with `#`, anywhere in the manifest of
the crate the finding names. There are no dependency waivers in the
repository.

## Where The Rules Run

The source rules run in `code-dojo-rust --all`, in `code-dojo-rust
--staged` and the `pre-commit` hook (staged files only), and in the full gate
`code-dojo --all`. The dependency rules run only in the full gate, which is
what the `pre-push` hook and the CI `dojo-ubuntu` job run. The hooks run only
where `scripts/install-githooks.sh` has been run, so CI is the gate that
always applies.
