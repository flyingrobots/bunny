# Receipt: A Gate Step That Regenerates The Contract Artifacts

Task:
flyingrobots/bunny#286, the part that remains after the waiver-scope work of #329: no gate
regenerated `bunny-wesley`'s output and compared it with the committed files, so they could
drift. Add a Code Dojo step that regenerates into a scratch directory and fails on any
difference, naming the file and the command that fixes it; show it failing on seeded drift and
passing on main; measure its cost and choose the tier.

Files read:
xtask/src/main.rs (handle_generate), xtask/src/code_dojo.rs
crates/bunny-wesley/src/main.rs, render.rs, build.rs
generated/bunny-graphics.manifest.json, generated/typescript/bunny-graphics.ts,
crates/bunny-contract/src/generated/graphics.rs, crates/bunny-contract/tests/generated_version_tests.rs
docs/topics/contract-generation/README.md and test-plan.md, docs/CODE_DOJO.md
.repo-respect/receipts/2026-11-14-v0-7-1-release-prep.md

Files edited:
xtask/src/generated_check.rs, xtask/src/generated_check/tests.rs (new)
xtask/src/main.rs (generate shares the argument list; code-dojo-generated), xtask/src/code_dojo.rs
docs/topics/contract-generation/README.md and test-plan.md, docs/CODE_DOJO.md, CHANGELOG.md

Topic docs:
docs/topics/contract-generation/README.md and test-plan.md

Generated artifacts:
None changed; the check reads them.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Before: regenerating from main's schema into a scratch directory (with the schema copied to
the same relative path, because the manifest records the paths it was given) gave output
byte-identical to the committed files. Cost, measured with `--exclusive`: 0.03 s warm, 0.36 s
after touching every source, 5.66 s from an empty target directory.
Failing evidence: with a one-byte change in the Rust and the TypeScript outputs, `cargo test -p
bunny-contract -p bunny-wesley` passed. After: the seeds of CG-TP-016 (one byte in each output,
a missing output, the schema edited without regenerating) each fail
`xtask code-dojo-generated` naming the file and line; the repository and the restored copy
pass. cargo fmt --check, cargo clippy -p xtask --all-targets -D warnings, cargo test -p xtask,
code-dojo-rust --all; code-dojo --all at the final commit (see the PR).

Known risks:
The check compares only the three files the generator writes; it does not find stray files
under generated/. It reuses the gate's build of bunny-wesley; a cold run builds wesley-core
(5.66 s measured). The scratch directory under target/ is removed and rewritten on every run.

Human reviewer:
James Ross (maintainer); review pending.
