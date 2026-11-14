# Receipt: Binary Angle Trigonometry

Task:
Answer #110 (canonical angle and trigonometry policy): add `FixedAngle` (binary angles,
`2^64` units per turn), conversions from turns, degrees and radians, and deterministic
table-based `sin`/`cos`, reproducing the design of Echo's `warp-math` trig (quarter-wave
table, quadrant symmetry, exact negation symmetry, golden vectors) natively in integer
arithmetic, with cubic Hermite interpolation for sub-raw-unit error.

Files read:
CODE_STANDARDS.md
ROADMAP.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
docs/README.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
crates/bunny-num/src/lib.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/tests/property_tests.rs
xtask/src/code_dojo.rs
Echo origin/main: crates/warp-math/src/trig.rs, trig_lut.rs, bin/gen_sin_qtr_lut.rs,
tests/trig_golden_vectors.rs, docs/determinism/SPEC_DETERMINISTIC_MATH.md

Files edited:
CHANGELOG.md
docs/README.md
crates/bunny-num/src/lib.rs
crates/bunny-num/src/fixed_angle.rs
crates/bunny-num/tests/fixed_angle_tests.rs
docs/topics/angle-trigonometry/README.md
docs/topics/angle-trigonometry/test-plan.md
.repo-respect/receipts/2026-11-13-binary-angle-trig.md

Topic docs:
docs/topics/angle-trigonometry/README.md
docs/topics/angle-trigonometry/test-plan.md

Generated artifacts:
None. The sine table is computed at compile time by a const fn.

Checks run:
cargo test --locked -p bunny-num --test fixed_angle_tests (8 passed, in the container)
cargo test --locked --workspace (container)
Mutation: h11 = t3 + t2 in hermite fails at_tp_001, at_tp_004 and at_tp_004b.
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all
Earlier local error sweep (max 0.5001 raw units over 4 196 611 angles) had no retained
tool; the claim is withdrawn from the docs until reproduced.
Golden values for 30 degrees, 60 degrees, 1 radian and mid-segment angles checked against
60-digit decimal arithmetic.
Harness timing (merged perf/bunny-perf-harness; angle group added to crates/bunny-perf):
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code --filter angle/
re-measured 2026-11-13 after merging main's black_box harness, in the container, two runs,
base be55424fef16: sin_cos p50 11.15/11.07 ns, from_radians 0.94/0.94 ns, f64 sin_cos
reference 4.27/4.19 ns. The earlier 10.21/0.89/2.81 ns figures are superseded. Ledger entry 2 and
PERF-TP-011 added to docs/topics/performance/; crates/bunny-perf/Cargo.toml and
src/cases.rs edited.

Also edited after merging main: bunny-perf cases black_box whole results; probe of every
segment added to the error test; AT-TP-008 (pythagorean identity).

Known risks:
The error bound is checked on sampled angles, not exhaustively; the Hermite error analysis
covers the gaps. Radian conversion rounds once, so a Q32.32 `pi` is not exactly a half turn.
The API is new and unreleased; names and constructor set may change in review.

Human reviewer:
James Ross asked for lookup-table trig reproduced from Echo on 2026-11-13; review pending.
