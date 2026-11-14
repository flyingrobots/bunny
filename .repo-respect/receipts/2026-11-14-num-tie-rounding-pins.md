# Receipt: Exact Tie-Rounding Pins In bunny-num

Task:
flyingrobots/bunny#287. Pin the exact ties-to-even result at every Q32.32 rounding site in
`bunny-num` with hand-derived raw values, and show by mutation that each wrong rounding rule
fails a `bunny-num` test. Test-only: no library function changes.

Files read:
docs/NUMERIC_CONSTITUTION.md
crates/bunny-num/src/fixed_q32_32.rs, fixed_q32_32/conversions.rs, fixed_divisor.rs,
fixed_angle.rs, lib.rs
crates/bunny-num/tests/*.rs
docs/topics/angle-trigonometry/ (README.md, test-plan.md), docs/topics/performance/test-plan.md
(PERF-TP-034 to -037)
docs/PROCESS.md, docs/README.md, docs/topics/repo-respect/README.md

Files edited:
crates/bunny-num/tests/tie_rounding_tests.rs (new)
docs/topics/fixed-point-rounding/README.md (new)
docs/topics/fixed-point-rounding/test-plan.md (new: FPR-REQ-001 to -003, FPR-TP-001 to -013)
docs/NUMERIC_CONSTITUTION.md (pointer from the required tests; audit rows for `FixedDivisor`,
`to_f32` and `FixedAngle`)
docs/README.md (chapter index)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-num-tie-rounding-pins.md

Topic docs:
docs/topics/fixed-point-rounding/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Survival reproduced first, on the unchanged tests: 16 mutants (four rules at each of five
sites), each under `timeout 120`, sources touched before each build. Every multiplication and
float mutant passed all `bunny-num` tests; division and prepared-divisor mutants failed only
the agreement tests between the two division implementations.
After: every mutant fails at least one FPR case (table in the test plan); the restored source
passes. Every vector was checked against exact rational arithmetic in a separate Python model.
cargo fmt (no changes), clippy -p bunny-num --all-targets -D warnings, cargo test -p bunny-num.
code-dojo --all at the final commit (see the PR).

Known risks:
The vectors are hand derived; a shared misreading of the rule would be in both the vectors and
the Python check. `FixedAngle`'s documented half-up conversions are not covered.

Human reviewer:
James Ross (maintainer); review pending.
