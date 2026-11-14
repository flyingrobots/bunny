# Receipt: ASCII PLY Rules Pinned And Documented

Task:
flyingrobots/bunny#323. Add the two missing tests (the certain-overflow shortcut of the decimal
conversion and the scalar-type check of skipped vertex properties) and settle three places where
the ASCII PLY chapter and the code disagreed: which line a missing coordinate is reported on, which
kind a malformed face count has (and whether `03` is accepted), and which kind a face element before
the vertex element has. Decision for all three: the code is right or harmless, so the chapter changes;
no code changes.

Files read:
crates/bunny-codec/src/ply_ascii/decimal.rs, header.rs, body.rs, error.rs
crates/bunny-codec/tests/ply_ascii_tests.rs, ply_ascii_decimal_tests.rs
docs/topics/ascii-ply/README.md and test-plan.md

Files edited:
crates/bunny-codec/tests/ply_ascii_rules_tests.rs (new)
docs/topics/ascii-ply/README.md, docs/topics/ascii-ply/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-ascii-ply-rules.md

Topic docs:
docs/topics/ascii-ply/README.md (profile item 2 and 4, Faces, error lines, error table, conversion
rule) and test-plan.md (APLY-REQ-016, APLY-REQ-017, APLY-TP-061 to APLY-TP-065, a mutation table).
The IDs skip 014 to 060 on purpose: an open pull request (#328) uses APLY-REQ-014, 015 and
APLY-TP-055 to 060.

Generated artifacts:
None.

Checks run:
Container: the five new tests pass; five mutants under `timeout 120`: the overflow shortcut 11 to
12 and the disabled scalar-type check pass every older test and fail the new ones; the other three
constants mutants fail older tests too and the new one. The full gate result is in the pull request.

Known risks:
Documentation matches the code, not the other way round: a face count such as `x` reports
`InvalidIndex`, a misleading kind for a count, kept so that no public error changes.

Human reviewer:
Pending: the project lead reviews this pull request.
