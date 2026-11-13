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
cargo test --locked -p bunny-num --test fixed_angle_tests (7 passed)
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all
Local error sweep: max 0.5001 raw units over 4 196 611 angles against f64 sin/cos.
Golden values for 30 degrees, 60 degrees, 1 radian and mid-segment angles checked against
60-digit decimal arithmetic.
Local timing: sin_cos about 10.3 ns per call (f64::sin_cos about 6.8 ns), Apple M5 Pro.

Known risks:
The error bound is checked on sampled angles, not exhaustively; the Hermite error analysis
covers the gaps. Radian conversion rounds once, so a Q32.32 `pi` is not exactly a half turn.
The API is new and unreleased; names and constructor set may change in review.

Human reviewer:
James Ross asked for lookup-table trig reproduced from Echo on 2026-11-13; review pending.
