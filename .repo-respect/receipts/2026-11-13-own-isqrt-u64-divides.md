# Receipt: Own Isqrt With U64 Divides

Task:
Replace `u128::isqrt` in `FixedQ32_32::sqrt_u128` with a Bunny-local exact integer square
root whose last Karatsuba step divides with two hardware `u64` divisions instead of a
software `u128 / u128` division, as found by reading Rust 1.96's
`core/src/num/imp/int_sqrt.rs`. Stacked on perf/fast-exact-fixed-math.

Files read:
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/tests/sqrt_equivalence_tests.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
Rust 1.96.0 core/src/num/imp/int_sqrt.rs (toolchain rust-src)

Files edited:
CHANGELOG.md
crates/bunny-num/src/lib.rs (isqrt_u128 and helpers)
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/tests/sqrt_equivalence_tests.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
.repo-respect/receipts/2026-11-13-own-isqrt-u64-divides.md

Topic docs:
docs/topics/performance/README.md (ledger entry 3)
docs/topics/performance/test-plan.md (PERF-TP-012, PERF-TP-013)

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-num --test sqrt_equivalence_tests (5 passed: old-loop and core
isqrt oracles)
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all
cargo run --locked -p xtask -- perf --base origin/perf/fast-exact-fixed-math --rounds 11 --align-code
(best p50: num/sqrt 13.14 -> 9.89 ns, linalg/length 15.26 -> 12.13 ns, linalg/normalize
39.43 -> 41.14 ns)
cargo run --locked -p xtask -- perf ... --rounds 21 --align-code --filter linalg/
(linalg/normalize +5.0 % median, +7.4 % best)

WebAssembly measurement (after merging the cross-target harness):
cargo run --locked -p xtask -- perf --base origin/perf/fast-exact-fixed-math --rounds 9
--target wasm32-wasip1 --runner wasmtime: sqrt 1.56x, length 1.48x, normalize +1.8 %.

Known risks:
`linalg/normalize` is measurably slower (5-7 %) although its square root is faster; the
cause is not established. Recommend landing together with the planned reciprocal-reuse
change for normalize. Speed measured on an Apple M5 Pro only.

Human reviewer:
James Ross approved the researched exact speedups on 2026-11-13; review pending.
