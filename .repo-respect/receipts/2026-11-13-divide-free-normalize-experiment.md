# Receipt: Divide-Free Normalize Experiment

Task:
Experiment, not for merging: replace the three divisions in `FixedVec2/3::normalize` with
`UnitDivisor` (table-seeded Newton reciprocal, multiply estimate, exact remainder
correction), keeping results bit-identical. Kept on experiment/divide-free-normalize as
evidence for the performance ledger's rejected-experiment entry.

Files read:
crates/bunny-linalg/src/fixed_vec2.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/src/fixed_q32_32.rs

Files edited:
crates/bunny-num/src/unit_divisor.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/tests/unit_divisor_tests.rs
crates/bunny-linalg/src/fixed_vec2.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-linalg/tests/normalize_equivalence_tests.rs
.repo-respect/receipts/2026-11-13-divide-free-normalize-experiment.md

Topic docs:
None on this branch; the rejected-experiment record is in the performance ledger on
perf/fixed-divisor.

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-num --test unit_divisor_tests (4 passed: exact vs checked_div)
cargo test --locked -p bunny-linalg --test normalize_equivalence_tests (3 passed: exact vs
the previous normalize on 100 000 vec3 and vec2 cases)
Reciprocal accuracy: within 0-2 units of 2^127/x after three Newton steps (1 000 000 x).
cargo run --locked -p xtask -- perf --base origin/perf/fixed-divisor --rounds 11 --align-code
--filter linalg/: normalize best p50 42.72 -> 41.46 ns (-2.9 %), median 45.49 -> 55.66 ns
(+22.4 %); with branch-free correction, best 42.60 -> 58.39 ns (+37.1 %).

Known risks:
Rejected: not faster on an Apple M5 Pro, whose 64-bit hardware divide makes compiler-rt's
128-bit division fast for these operand sizes. Not measured on x86-64 or wasm32, where it
might pay off.

Human reviewer:
James Ross approved trying the researched speedups on 2026-11-13; not proposed for merge.
