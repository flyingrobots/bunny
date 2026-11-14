# Receipt: Code Dojo Name Resolution Holes

Task:
Part of flyingrobots/bunny#321: close the name-resolution holes an independent review
found in the nondeterminism gate (flyingrobots/bunny#290, batch 3): `extern crate`
renames never entered the import map, raw identifiers compared as `r#name`, `r#f64`
passed `float-boundary`, and no test failed when `self` in a `use` group was treated as a
path segment. The first commit holds the planned cases and the failing evidence; the
second the fix and the documentation.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
docs/CODE_DOJO.md
docs/PROCESS.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
docs/topics/repo-respect/README.md
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
xtask/src/feature_configs.rs
.repo-respect/receipts/2026-11-14-dojo-nondeterminism-rules.md

Files edited:
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs (helpers shared with the new module)
xtask/src/nondeterminism/evasion_tests.rs (new)
xtask/src/code_dojo.rs
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-gate-resolution.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-TP-030..034)
docs/topics/code-dojo-determinism/README.md (how paths are read)
docs/CODE_DOJO.md (mapping table and coverage count)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
rustc 1.96.0 --edition 2021 --crate-type lib on each new snippet that names only std,
core or alloc: all 17 compile.
cargo test --locked -p xtask evasion at the failing-evidence commit: 3 of 5 fail. With
`assert_rejects` made to report instead of panic, the unfixed gate misses 14 of the 16
new forms; the two `self` forms pass, since that test exists to kill a mutation.
Mutation, under `timeout 120`: `named_leaf` keeping `self` as a segment fails
`self_in_a_use_group_names_the_module_itself` (exit 101); it survived every test before.
cargo fmt --all; cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings;
cargo test --locked -p xtask: 83 passed.
cargo run --locked -q -p xtask -- code-dojo-rust --all over the repository with the fix:
clean, no new finding on main.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD --network
at the final commit.

Known risks:
Name resolution is still syntactic and file-wide: a path that reaches a banned item only
through another crate's re-export or an inferred type is not seen; the dependency check
covers third-party crates. The other holes of flyingrobots/bunny#321 (waivers, included
files, sibling names, float methods) are separate pull requests.

Human reviewer:
James Ross (maintainer); review pending.
