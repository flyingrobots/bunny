# Receipt: ASCII PLY Triangle-Mesh Reader

Task:
flyingrobots/bunny#247: add an ASCII PLY triangle-mesh reader to `bunny-codec`
(`parse_ascii_ply`), with an exact integer-only decimal to Q32.32 conversion
(`bunny-decimal-q32-32/1`, round to nearest, ties to even), typed errors with line
numbers, and a stated rule for line endings, blank lines, comments and trailing
whitespace. The binary reader is unchanged.

Files read:
CONTRIBUTING.md
docs/PROCESS.md
docs/CODE_DOJO.md
crates/bunny-codec/src/lib.rs
crates/bunny-codec/src/ply.rs
crates/bunny-codec/src/ply/header.rs
crates/bunny-codec/src/obj.rs
crates/bunny-codec/src/obj/iter.rs
crates/bunny-codec/tests/ply_tests.rs
crates/bunny-codec/tests/ply_allocation_tests.rs
crates/bunny-codec/tests/fixtures/README.md
crates/bunny-num/src/fixed_q32_32.rs
docs/topics/coordinate-law/test-plan.md
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs

Files edited:
crates/bunny-codec/src/lib.rs
crates/bunny-codec/README.md
CHANGELOG.md
README.md
docs/README.md

Files added:
crates/bunny-codec/src/ply_ascii.rs
crates/bunny-codec/src/ply_ascii/body.rs
crates/bunny-codec/src/ply_ascii/decimal.rs
crates/bunny-codec/src/ply_ascii/error.rs
crates/bunny-codec/src/ply_ascii/header.rs
crates/bunny-codec/tests/ply_ascii_tests.rs
crates/bunny-codec/tests/ply_ascii_decimal_tests.rs
crates/bunny-codec/tests/ply_ascii_allocation_tests.rs
docs/topics/ascii-ply/README.md
docs/topics/ascii-ply/test-plan.md

Topic docs:
docs/topics/ascii-ply/README.md
docs/topics/ascii-ply/test-plan.md

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host.
Red evidence at the test-first commit (stubbed reader): cargo test --locked -p bunny-codec
--tests --no-fail-fast: 27 of 31 new tests fail, the existing tests pass.
Final checks are listed in the pull request.

Known risks:
The conversion refuses a value of magnitude in [1e-10, 1e10) with more than 28
significant digits (`NumberTooLong`) instead of converting it. Nothing in the
Stanford data approaches that. Positions are Q32.32 only; there is no f64 accessor.

Human reviewer:
James Ross (maintainer); review pending.
