# Receipt: Docs Index Duplicates

Task:
Work `flyingrobots/bunny#451`: `docs/README.md` listed `topics/ray-tracing/` and `topics/plane-clipping/` twice each, with descriptions that disagreed. Keep one row per topic, with the description that matches the chapter, and put the topic rows in alphabetical order.

Files read:
- `docs/README.md`
- `docs/topics/ray-tracing/README.md`
- `docs/topics/plane-clipping/README.md`

Files edited:
- `docs/README.md`
- `.repo-respect/receipts/2026-11-23-docs-index-duplicates.md`

Topic docs:
None - no chapter or ledger changed; only the index that routes to them.

Generated artifacts:
None.

Checks run:
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`
- A comparison of the index's topic rows with the directories under `docs/topics/`: 23 rows, 23 directories, same names.

Known risks:
- Other open branches add rows to the same table, so they will conflict with the reordering and need both rows kept in alphabetical order when they merge.

Human reviewer:
@flyingrobots
