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
cargo test --locked -p xtask evasion at the failing-evidence commit: 3 of 5 fail
(`r#std::fs::read`, both `r#f64` forms and `host::env::var` through `extern crate std as
host` slip through); the `self` case and the look-alikes pass, as they must on the unfixed
gate, since the `self` case exists to kill a mutation.

Known risks:
The fix is not in this commit; its tests fail here by design.

Human reviewer:
James Ross (maintainer); review pending.
