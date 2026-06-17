# Receipt: Standards Review Documentation Fixes

Task:
Resolve the self-code-review findings from PR #106 by aligning Code Dojo CI
documentation with the installed workflow and removing stale local/uncommitted
goalpost wording.

Files read:

* `.github/workflows/code-dojo.yml`
* `.repo-respect/README.md`
* `CODE_STANDARDS.md`
* `docs/CODE_DOJO.md`
* `docs/goalposts/post-v0.4.0-standards-alignment.md`
* `xtask/src/code_dojo.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-standards-review-docs.md`
* `docs/CODE_DOJO.md`
* `docs/goalposts/post-v0.4.0-standards-alignment.md`

Checks run:

* `git diff --check origin/main...HEAD`
* `npx --yes markdownlint-cli2 README.md CHANGELOG.md ROADMAP.md docs/*.md docs/design/*.md docs/goalposts/*.md crates/*/README.md`
* `cargo run --locked -p xtask -- code-dojo --all`

Known risks:

* Documentation-only fix; no runtime behavior changed.
* `cargo deny check` still reports duplicate dependency warnings from the
  `wesley-core` graph, but the configured Code Dojo gate passes.

Human reviewer:
James
