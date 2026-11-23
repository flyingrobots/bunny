# Receipt: Optics Test Plan Table Bars

Task:
Three rows of `docs/topics/optics/test-plan.md` (OPTICS-REQ-047, OPTICS-REQ-053 and
OPTICS-TP-103) had unescaped `|` characters inside code spans (`|sum w_i n_i|`, `|d|`). In a
Markdown table an unescaped bar ends a cell, so each of those rows had two cells more than its
header and the cells after the formula were displaced. Found while reviewing
flyingrobots/bunny#406, where the same defect was fixed in that branch's own rows.

Files read:
docs/topics/optics/test-plan.md
docs/topics/*/test-plan.md (scanned for rows whose cell count differs from their header)

Files edited:
docs/topics/optics/test-plan.md
.repo-respect/receipts/2026-11-23-optics-test-plan-table-bars.md

Topic docs:
docs/topics/optics/test-plan.md: the bars inside the code spans of the three rows are escaped
as `\|`. No requirement, case, test name or oracle text changes.

Generated artifacts:
None.

Checks run:
- A scan of every `docs/topics/*/test-plan.md` for table rows whose cell count (split on
  unescaped bars) differs from their header -> 3 rows before, all in the optics test plan; 0 after
- `cargo run --locked -p xtask -- topic-docs` -> clean; 269 active requirements and 673
  implemented cases, the same as on `main` before the change
- `cargo run --locked -p xtask -- repo-respect check --branch` -> clean

Known risks:
None. Documentation only; no kernel, test or contract identity changes. markdownlint is not
installed locally; CI's Dojo job runs it.

Human reviewer:
@flyingrobots
