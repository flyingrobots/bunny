# Receipt: ASCII PLY Reader Limits

Task:
flyingrobots/bunny#300. Give the ASCII PLY reader caller-visible limits (declared vertex count,
declared face count, line length in bytes) through an additive entry point with a small limits
value, documented defaults and a typed error naming the limit and the observed value, with
`parse_ascii_ply` unchanged. Per the review correction, this is sized as limits that refuse early,
not a resource fix: the reader already used constant memory and linear time. Decide the status of
skipped vertex columns (counted, never parsed: unchanged).

Files read:
crates/bunny-codec/src/ply_ascii.rs and ply_ascii/{body,header,decimal,error}.rs
crates/bunny-codec/src/ply.rs, obj.rs (neither enforces any limit; no precedent to follow)
crates/bunny-codec/tests/ply_ascii_tests.rs, ply_ascii_allocation_tests.rs
docs/topics/ascii-ply/README.md and test-plan.md
CHANGELOG.md (precedent: a public enum without non_exhaustive gets no new variant)

Files edited:
crates/bunny-codec/src/ply_ascii/limits.rs (new), ply_ascii.rs, ply_ascii/header.rs, ply_ascii/body.rs, lib.rs
crates/bunny-codec/tests/ply_ascii_limits_tests.rs (new)
crates/bunny-codec/README.md
docs/README.md, docs/topics/ascii-ply/README.md, docs/topics/ascii-ply/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-ascii-ply-limits.md

Topic docs:
docs/topics/ascii-ply/README.md (Limits section) and test-plan.md (APLY-REQ-014, APLY-REQ-015,
APLY-TP-055 to APLY-TP-060, a mutation table).

Generated artifacts:
None.

Checks run:
Container: tests of the new file (six tests); five mutants of limits.rs under `timeout 120`, each
caught (table in the test plan). The full gate result is in the pull request.

Known risks:
New public API (`parse_ascii_ply_with_limits` and four types). `HeaderSpec` gained two line
fields, which only the new entry point reads; `parse_ascii_ply` results are unchanged and the
differential test compares them. The line limit is checked after a line has been scanned to its
end, so it bounds later work, not that one scan.

Human reviewer:
Pending: the project lead reviews this pull request.
