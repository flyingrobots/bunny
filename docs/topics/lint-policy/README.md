# Lint Policy

This chapter describes how Rule 10 of
[`CODE_STANDARDS.md`](../../CODE_STANDARDS.md#rule-10-clippy-is-a-guardrail-not-a-random-opinion-generator)
applies to the core crates and how Code Dojo enforces it.

## Status

Living topic chapter. The check is `xtask/src/lint_policy.rs`, run by
`xtask/src/code_dojo.rs` in the full gate and the pre-commit gate. The
verification design is [`test-plan.md`](test-plan.md).

## The Rule

Clippy's `nursery` group holds lints that are not yet stable: new ones arrive
with toolchain updates, and some change behaviour or are withdrawn. Denying
the whole group would let a toolchain update fail the build for reasons nobody
chose. Rule 10 therefore bans setting `clippy::nursery` as a group to `deny`,
`warn` or `forbid` in a core crate (`warn` counts, because the workspace denies
warnings). Nursery lints are named one by one, each with its reason.

The core crates are those in `CORE_CRATES` in `xtask/src/code_dojo.rs`:
`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-query`, `bunny-broadphase`,
`bunny-mesh` and `bunny-optics`. Other crates are outside Rule 10;
`bunny-codec` still denies the group.

## The Check

The `blanket-nursery` rule reports every place that sets the group for a core
crate, with its file and line:

- a lint attribute anywhere in any Rust file under the crate's directory
  (outside `target/`): inner or outer, on the crate, a module, an item, an
  impl, a method or a statement, also inside `cfg_attr`. The group is banned
  at every scope, because a toolchain update that adds a nursery lint breaks
  a module the same way it breaks a crate. Every file is read, so a crate
  root other than `src/lib.rs` (`[lib] path`), its modules and the crate's
  integration tests are covered;
- the `nursery` key of the crate manifest's `lints.clippy` table and of the
  workspace manifest's `workspace.lints.clippy` table, which every core crate
  inherits;
- `-D`, `-W`, `-F`, `--deny`, `--warn` or `--forbid` with `clippy::nursery`
  (joined or as two words) in the `build.rustflags` or any
  `target.<triple or cfg>.rustflags` of `.cargo/config.toml` or
  `.cargo/config`, at the repository root or in the crate's directory.

Manifests and configurations are parsed as TOML (`toml_edit`, already in the
lockfile through `bunny-wesley`'s `toml`), so every spelling of a table is the
same table: `nursery.level = "deny"`, `[ workspace . lints . clippy ]`,
`[workspace.lints] clippy = { nursery = "deny" }`, `lints = { clippy = { .. } }`,
`nursery = { level = "deny", priority = -1 }`, quoted keys, and
`[workspace.lints.clippy.nursery]` with a `level` key. The finding's line is
the `nursery` key's.

`allow` and `expect` of the group, named nursery lints, and the group named in
a comment or string are not findings. A file that does not parse, or a
manifest or configuration that is not TOML, is reported with line 0. The rule
has no waiver.

Limits: lints passed on the command line (`cargo clippy -- -D
clippy::nursery`), in the `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`
environment variables, in a Cargo configuration outside the repository (for
example `~/.cargo/config.toml`) or through a `[alias]`, and a lint attribute
that a macro emits, are outside the files the check reads. Before
flyingrobots/bunny#350 the check read only inner attributes of `src/lib.rs`
and one `nursery` key per line, and missed the dotted, spaced and inline
spellings and attributes on modules.

## Named Nursery Lints

Each core crate denies these three nursery lints by name, with the reasons in
a comment above them in its `src/lib.rs`:

| Lint | Reason |
| --- | --- |
| `clippy::missing_const_for_fn` | Every function that can be `const` stays usable in constant expressions, as the crates' value constructors and accessors are. |
| `clippy::redundant_pub_crate` | Crate-private items stay `pub` inside private modules, so the module's own visibility is the one boundary to read. |
| `clippy::use_self` | `Self` in impls, so a renamed type changes one line. |

They were chosen because the code already follows them throughout: with the
group removed and every nursery lint enabled, no core library has a finding,
and the crates use these patterns heavily (from 7 `const fn` in `bunny-mesh`
to 71 in `bunny-linalg`). In the crates' integration tests, which the group
never covered, the same group finds `missing_const_for_fn` in every crate and
`redundant_pub_crate` in three. A crate that needs another nursery lint names
it the same way.
