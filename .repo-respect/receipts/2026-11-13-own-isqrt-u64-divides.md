# Receipt: Own Isqrt With U64 Divides (re-measured)

Task:
flyingrobots/bunny#219. Merge origin/main into perf/bunny-isqrt, re-measure Bunny's
two-divide `u128` square root with the corrected benchmark harness, settle whether its
`linalg/normalize` regression is real, and record the outcome. Result: the regression is
real on aarch64 (about 4-5 % at best p50 in three runs), so `isqrt_u128` keeps core's
`u128::isqrt` on every target and the two-divide function is retained as a tested,
unselected implementation.

Files read:
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/tests/sqrt_equivalence_tests.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/repo-respect/README.md
Rust 1.96.0 core/src/num/imp/int_sqrt.rs (toolchain rust-src)

Files edited:
CHANGELOG.md
crates/bunny-num/src/lib.rs (dispatch, `isqrt_u128_two_divides`, unit tests)
crates/bunny-num/src/fixed_q32_32.rs (doc comment, call through the dispatch)
crates/bunny-num/tests/sqrt_equivalence_tests.rs (core-comparison tests moved to lib.rs)
docs/topics/performance/README.md (ledger entry 2, target-specific section)
docs/topics/performance/test-plan.md (PERF-TP-012 to PERF-TP-014)
.repo-respect/receipts/2026-11-13-own-isqrt-u64-divides.md

Topic docs:
docs/topics/performance/README.md (ledger entry 2)
docs/topics/performance/test-plan.md

Generated artifacts:
None.

Checks run (all inside the flyingrobots/rust-dev:1.96.0 container via devrun):
cargo test --locked --workspace (after the merge): passed
cargo test --locked -p bunny-num; cargo clippy --locked -p bunny-num --all-targets: clean
cargo run --locked -p xtask -- topic-docs: contract graph clean
mutation: replacing the correction `candidate - 1` with `candidate` failed all three new
unit tests; reverted
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code, twice,
and once with --rounds 21 --filter linalg/ (base 91ed71fc7a5e, Apple M5 Pro, 2026-11-13).
Best p50: num/sqrt 1.33x, 1.34x; linalg/length 1.28x, 1.29x, 1.26x; linalg/normalize +4.5 %,
+3.5 %, +5.5 %; controls num/mul, num/div, query/ray_triangle within 3.2 %.
code-dojo --all: see the pull request

Known risks:
x86-64 and wasm32 are unmeasured in this PR; both keep core's implementation. The cause of
the `normalize` loss is not established. The dispatched build was not re-measured; it is
`main`'s code path.

Human reviewer:
James Ross approved the researched exact speedups on 2026-11-13; review pending.
