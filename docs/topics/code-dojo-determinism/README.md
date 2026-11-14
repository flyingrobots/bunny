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
| `time-system` | `std::time::SystemTime`, `SystemTimeError`, `UNIX_EPOCH` |
| `time-instant` | `std::time::Instant` |
| `thread-sleep` | `std::thread::sleep`, `sleep_ms`, `sleep_until`, `park`, `park_timeout`, `park_timeout_ms`, `yield_now`; `tokio::time::sleep`, `async_std::time::sleep`, `async_std::task::sleep` |
| `host-parallelism` | `std::thread::available_parallelism`, also as a bare name |
| `random` | the `rand`, `getrandom` and `fastrand` crates, and the name `thread_rng` |
| `host-callback` | the `js_sys`, `wasm_bindgen` and `web_sys` crates |
| `network` | `std::net`, `std::os::<platform>::net`, the `reqwest` and `ureq` crates, and the names `TcpStream::connect` and `UdpSocket::bind` |
| `unordered-container` | the names `HashMap`, `HashSet`, `FxHashMap`, `FxHashSet`; `std::collections::hash_map` and `hash_set` except `hash_map::DefaultHasher`; `std::hash::RandomState`; the `hashbrown` and `rustc_hash` crates |
| `hand-hashed-table` | any line, comments included, that names open addressing |
| `json-serialization` | the `serde_json` crate |
| `wasm-serde` | the `serde_wasm_bindgen` crate |
| `std-env` | `std::env`, `std::path::absolute` and a zero-argument `.absolute()` call (they read the current directory), and the macros `env!` and `option_env!` (also under `std::` and `core::`) |
| `std-fs` | `std::fs`, `std::os::<platform>::fs`, `io` and `prelude`, `std::os::fd`; the names `fs::…` and `File::open`; the `Path`/`PathBuf` methods that read the filesystem (`canonicalize`, `exists`, `is_dir`, `is_file`, `is_symlink`, `metadata`, `read_dir`, `read_link`, `symlink_metadata`, `try_exists`) as zero-argument calls or `Path::` paths |
| `std-process` | `std::process`, `std::os::<platform>::process`, `std::io::pipe`, `PipeReader`, `PipeWriter` |
| `host-input` | terminal input: `std::io::stdin`, `Stdin`, `StdinLock`, `IsTerminal` and a zero-argument `.is_terminal()` call |
| `include-source` | `include!` in every spelling, and a `#[path]` module (also under nested `cfg_attr` or emitted by a macro) naming anything but a `.rs` file the gate scans inside the crate directory |
| `include-data` | `include_str!` and `include_bytes!` whose argument is not one string literal naming a file inside the crate directory |
| `float-boundary` | in core crates only: `f32` and `f64` types, float literals, any path through `f32` or `f64` (for example `f32::NAN`, `f64::from_bits`, `core::f64::consts::PI`), and any path segment or method named with an `_f32` or `_f64` suffix (`Duration::mul_f64`, `d.as_secs_f64()`, Bunny's own `to_f32`), also inside macro bodies and in `use` declarations, so a rename (`use core::f64::consts::PI as P`) cannot hide one |

A banned path bans everything below it: `std::fs` covers `std::fs::read` and
`std::fs::File::open`. `std::time` and `std::thread` are not banned as
modules; every item in them is named one by one, banned or permitted, and a
test checks that list against the pinned toolchain's documentation
([keeping the lists current](../../CODE_DOJO.md#keeping-the-name-lists-current)).
Threads themselves (`spawn`, `scope`, `Builder`) are permitted: Rule 6 lets
parallelism in when it does not change results, which tests show.

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

### Compile-Time File Access

The gate scans every tracked `.rs` file of a crate, so code can reach a
crate unscanned only through `include!` or a `#[path]` module that names
another kind of file or a file outside the crate.

- `include!` is banned outright (`include-source`), whatever its argument: a
  `.rs` file inside the crate can be a `mod`, and anything else (a non-Rust
  file, a `concat!` or `env!` path, a build-script output) is code the gate
  never reads. Because the first `include!` is itself the finding, a second
  level of indirection behind it changes nothing.
- A `#[path]` module, also one set through `cfg_attr` at any depth or
  emitted by a macro, must name a `.rs` file inside the crate directory and
  outside the `target`, `vendor` and `third_party` directories, which the gate
  does not list; that file is then scanned as a file of the crate. Anything
  else is an `include-source` finding at the attribute.
- `include_str!` and `include_bytes!` must name one string literal that
  resolves inside the crate directory (`include-data`): fixed data in the
  package, not a file on the host.
- `env!` and `option_env!` read the build environment and are `std-env`
  findings, `CARGO_PKG_VERSION` included; a waiver says why a variable Cargo
  sets from the manifest is safe.

Macro names are resolved through imports (`use std::include_str as text;`)
and found inside other macros' tokens. Paths resolve lexically against the
including file's directory, as the compiler resolves `include_str!` and a
top-level `#[path]`; inside an inline module the compiler uses a deeper
directory, so the lexical answer can only call an inside path outside.
Symbolic links are not followed, and a path with a backslash is rejected.

### Host Access Beyond Echo

`CODE_STANDARDS.md` Rule 4 forbids the filesystem, the environment, process
arguments and the current directory in core crates; Echo's list does not name
every way to reach them. The gate also bans `Path` methods that read the
filesystem, the platform modules of `std::os` that do (`*` stands for
`unix`, `windows`, `linux`, `wasi` and the rest), `std::path::absolute`,
`std::io::pipe`, and terminal input (`host-input`). A method call is matched
by its name when it takes no arguments, since the receiver's type is unknown;
a type of Bunny's with an `exists()` or `metadata()` method would need
another name or a waiver. Writing to standard output or standard error is not
banned: `println!` in a test cannot change a result, and tests print the
seeds that replay them.

### Doc Tests

`cargo test` compiles and runs doc tests, so the rules read them too: each
fenced block in a doc comment that rustdoc treats as Rust (no info string, or
only `rust`, `no_run`, `ignore`, `should_panic`, `compile_fail`,
`test_harness`, `standalone_crate` or an edition) is parsed with its hidden
`# ` lines, as a file or else as the body of `fn main`, and scanned like the
file around it. A finding is reported at the line of the doc comment that
holds it; a waiver above the documented item covers it. A `text` or other
non-Rust block is not read, nor is doc text brought in with
`#[doc = include_str!(..)]`.

### Dependency Rules

The full gate also lists the dependency graph of every deterministic library
crate with `cargo tree --locked --edges normal,build --target all --no-dedupe
--prefix depth --format {p}`, once with the workspace's features and once with
`--all-features`, and rejects these crates in either listing:

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
names the crate whose manifest declares the path to the banned crate. The
all-features listing finds a banned crate behind an optional dependency that
nothing in the workspace enables; the build edges find one a build script
could use. A finding found by both listings is reported once.
Dev-dependencies are outside the check: core tests depend on
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

A source waiver is a `//` or `/* */` comment holding exactly one clause:

```rust
// dojo: allow std-fs -- fixture loader for the golden vectors, reviewed in #NNN
```

It names one rule by its exact ID and needs a reason of at least eight
characters after ` -- `. A waiver without a reason, with a shorter reason,
naming another rule, or sharing its comment with a second clause leaves the
finding reported; it does not hide a valid waiver for the same finding.

Comments are what the token stream does not cover, so the same text inside a
string, raw-string, byte-string or character literal, a doc comment, a
`#[doc]` attribute or a macro's tokens is not a waiver. Each comment is read
whole: a block comment over several lines holding two clauses waives
neither.

A waiver after code covers its own line. A waiver on a line of its own covers
that line, the comment lines below it, and the whole of the outermost item,
statement, field, variant, match arm or struct-literal field that starts on
the next line holding code; if none starts there, that line only. A node
starts on its first line and again on its first line after its attributes
and doc comments, so a waiver between a doc comment and `fn` covers the
function. A
`float-boundary` waiver above a function therefore covers every float in it,
and a float in the next function is reported.

A dependency waiver is the same clause in a whole-line TOML comment, anywhere
in the manifest of the crate the finding names, but not on a line inside a
multi-line string. There are no dependency
waivers in the repository.

## Where The Rules Run

The source rules run in `code-dojo-rust --all`, in `code-dojo-rust
--staged` and the `pre-commit` hook (staged files only), and in the full gate
`code-dojo --all`. The dependency rules run only in the full gate, which is
what the `pre-push` hook and the CI `dojo-ubuntu` job run. The hooks run only
where `scripts/install-githooks.sh` has been run, so CI is the gate that
always applies.
