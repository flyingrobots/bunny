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
Planned; see the pull request.

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

Known risks:
Planned; see the pull request.

Human reviewer:
James Ross (maintainer); review pending.
