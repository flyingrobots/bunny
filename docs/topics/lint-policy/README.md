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

- an inner attribute of the crate's `src/lib.rs` (`#![deny(clippy::nursery)]`,
  also with other lints in the list, and inside `cfg_attr`);
- the `nursery` key of the crate manifest's `[lints.clippy]` table, or the
  dotted key `clippy.nursery` under `[lints]`;
- the same keys in the workspace manifest's `[workspace.lints.clippy]` and
  `[workspace.lints]` tables, which every core crate inherits.

`allow` and `expect` of the group, named nursery lints, attributes on items
other than the crate, and the group named in a comment or string are not
findings. A `lib.rs` that does not parse is reported with line 0. The rule has
no waiver.

Limits: the manifest scan reads one key per line, as the repository's
manifests are written; an inline table such as
`lints = { clippy = { nursery = "deny" } }` is not read. Lints passed on the
command line (`-D clippy::nursery`) are outside the files the check reads.

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
