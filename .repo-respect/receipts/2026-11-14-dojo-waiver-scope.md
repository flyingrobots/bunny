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
ND-TP-046 (a waiver after code covers only its line) was added with the implementation;
it passes on the old window too, and exists to kill a mutation of the new coverage.
Mutations of `xtask/src/waivers.rs`, each under `timeout 120`, all caught: the covered node
not extended past the next code line (2 tests fail), one line past the node (3), comment
lines below the waiver not covered (1), a trailing waiver also covering the next line (1),
a second clause allowed (2), the rule matched as a prefix (1), no token masked (9), a
group's whole span masked instead of its delimiters (3). The review's surviving mutation,
shrinking the old two-line window, has no counterpart: the window is gone, and shrinking
the new coverage to the next code line is the first mutant above.
cargo fmt --all; cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings;
cargo test --locked -p xtask: 95 passed.
cargo run --locked -q -p xtask -- code-dojo-rust --all over the repository with the new
waivers: clean. All 26 waivers on main sit directly above their finding, so each still
covers it.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD --network
at the final commit.

Known risks:
A waiver above a large item (an `impl` block, a module) covers all of it; review must
read the waiver against the item it sits on. A float admitted by a waived item can leave
it as a return value and be operated on where its type is only inferred. Comments are
found with proc-macro2's lexer, the same one `syn` uses; a file it cannot lex has no
waivers, and the parse error is reported as `rust-parse`.

Human reviewer:
James Ross (maintainer); review pending.
