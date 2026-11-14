# Receipt: Code Dojo Waivers Are Comments With One Rule And An Item Scope

Task:
Part of flyingrobots/bunny#321, and the waiver-scope part of flyingrobots/bunny#286:
waivers found matching as text anywhere on a line (string literals included), one comment
waiving two rules, a two-line window that silenced separate statements and stopped short
of the rest of a function, `has_waiver` returning on the first match, a rule name matched
as a prefix, and manifest waivers inside TOML strings. The first commit holds the planned
cases and the failing evidence; the second the new waiver module and the documentation.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
every `dojo: allow` comment under crates/ (26, all directly above their finding)

Files edited:
xtask/src/main.rs
xtask/src/waivers.rs (new)
xtask/src/waivers/tests.rs (new)
xtask/src/code_dojo.rs
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-waiver-scope.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-002 restated, ND-TP-035..045)
docs/topics/code-dojo-determinism/README.md (waivers)
docs/CODE_DOJO.md (waivers)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test --locked -p xtask waivers at the failing-evidence commit: 8 of 11 fail. Every
one of the nine non-comment carriers (string, raw string, byte string, a raw-string line,
three doc-comment forms, a doc attribute, macro tokens) waived the finding; the two-clause
comment waived both rules; the refused-then-valid pair reported; the window waived the
second statement and missed line 6 of a statement and of a function; `std-fs-extra`
waived `std-fs`; manifest waivers in a TOML string and with two clauses were honoured.

Known risks:
The fix is not in this commit; its tests fail here by design.

Human reviewer:
James Ross (maintainer); review pending.
