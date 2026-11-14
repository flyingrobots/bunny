# Receipt: Echo Nondeterminism Rules In Code Dojo

Task:
Implement flyingrobots/bunny#279: map every rule of Echo's `ban-nondeterminism` scan onto
Code Dojo, add the rules Bunny lacked as syntax-aware Code Dojo rules with Echo's rule IDs,
and document the mapping. This commit holds the planned test cases and the failing
evidence; the rules follow in the next commit.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
docs/CODE_DOJO.md
docs/PROCESS.md
docs/topics/repo-respect/README.md
docs/topics/repo-respect/test-plan.md
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
.repo-respect/receipts/2026-06-17-core-float-boundaries.md
.repo-respect/receipts/2026-06-17-float-literal-gate.md
.repo-respect/receipts/2026-06-17-map-constructor-ban.md
.repo-respect/receipts/2026-06-17-std-fs-ambient-ban.md
Echo at origin/main (read only): scripts/ban-nondeterminism.sh,
scripts/lib/determinism-scan.sh, .ban-nondeterminism-allowlist,
docs/adr/ADR-0006-Ban-Non-Determinism.md, .githooks/pre-push-parallel,
.github/workflows/ci.yml, .github/workflows/det-gates.yml

Files edited:
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
docs/topics/code-dojo-determinism/test-plan.md
.repo-respect/receipts/2026-11-14-dojo-nondeterminism-rules.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-001..005, ND-TP-001..029, planned)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test --locked -p xtask nondeterminism at this commit: 24 of 29 tests fail. The
failure messages record, form by form, what the existing gate reported for each seeded
violation (for example `std::fs::read` was reported as `ambient-state`, while
`use std::fs::read_to_string` followed by a bare call was not reported at all).

Known risks:
The per-file gate entry point `file_violations` is a pure extraction of the loop body of
`check_rust`; `dependency_violations` is a stub returning no findings until the next commit.

Human reviewer:
James Ross (maintainer); review pending.
