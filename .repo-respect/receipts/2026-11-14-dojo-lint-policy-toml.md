# Receipt: The Lint Policy Reads Every Spelling And Scope

Task:
Implement flyingrobots/bunny#350: the `blanket-nursery` check missed `nursery.level =
"deny"`, a table header with spaces around the dots, inline tables, and lint attributes on
modules, and did not read module files, a `[lib] path` root or Cargo configurations. The
first commit holds the planned cases; the second the tests, the check and the docs.
Stacked on flyingrobots/bunny#353 (#348).

Files read:
xtask/src/lint_policy.rs, xtask/src/lint_policy/tests.rs
xtask/src/code_dojo.rs
xtask/Cargo.toml, Cargo.toml, Cargo.lock, deny.toml
crates/bunny-wesley/Cargo.toml (the existing `toml` dependency)
CODE_STANDARDS.md (Rule 10)
docs/topics/lint-policy/README.md, docs/topics/lint-policy/test-plan.md
docs/CODE_DOJO.md
The reviewer's probe scripts for the issue (zz_lint.sh, zz_lint2.sh)

Files edited:
xtask/src/lint_policy.rs
xtask/src/lint_policy/tests.rs
xtask/Cargo.toml (toml_edit 0.22, default features off, `parse` only)
Cargo.lock (one dependency edge from xtask to toml_edit; no package added)
docs/topics/lint-policy/README.md
docs/topics/lint-policy/test-plan.md
docs/CODE_DOJO.md
CODE_STANDARDS.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-lint-policy-toml.md

Topic docs:
docs/topics/lint-policy/README.md (The Check: what is read, the TOML spellings, limits)
docs/topics/lint-policy/test-plan.md (LINT-REQ-003, LINT-TP-005..008)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
The new seeded forms against main's `attribute_lines` and `manifest_lines`: of 9 attribute
forms the old function finds 1 (an inner attribute, which the old check read only in
`src/lib.rs`, so 0 at the gate); of 12 manifest spellings it finds 2 (an inline level table
and a header with a comment). The old check read no Cargo configuration (0 of 6) and no
file but `src/lib.rs`. All four forms of the issue's table reproduce.
`cargo test --locked -p xtask`, sources touched: 149 passed, LINT-TP-003 (the repository)
included: nothing on main sets the group.
Mutations, each under `timeout 120` after touching the sources, all caught: only the file's
own attributes read; the workspace table not read; a table's `level` not read; the root
configuration not read; joined flags ignored; separate flags ignored; target rustflags
ignored; `target/` directories walked; `cfg_attr` ignored (9 mutants).
`toml_edit` 0.22.27 is already in Cargo.lock (through `toml` 0.8.23, a dependency of
`bunny-wesley`); the lock diff adds only the edge. `cargo deny check` runs in the full gate.
The full gate, `code-dojo --all` with `--history origin/main..HEAD --network`, at the
head commit: see the pull request.

Known risks:
Lints on the command line, in `RUSTFLAGS`, in a Cargo configuration outside the
repository, through `[alias]`, or in an attribute a macro emits are not read. `toml_edit`
is a direct dependency of `xtask` now; it is tooling, not a deterministic crate.

Human reviewer:
James Ross (maintainer); review pending.
