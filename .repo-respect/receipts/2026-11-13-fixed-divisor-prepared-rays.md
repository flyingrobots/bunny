# Receipt: Fixed Divisor And Prepared Rays

Task:
Add exact division by a prepared divisor (`FixedDivisor`, Möller–Granlund Algorithm 4) and
a prepared ray (`PreparedRay3`, `ray_intersects_aabb_prepared`) whose box test multiplies
instead of dividing, with results identical to the existing `checked_div` and
`ray_intersects_aabb`. Stacked on perf/bunny-isqrt.

Files read:
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/src/lib.rs
crates/bunny-query/src/ray.rs
crates/bunny-query/src/lib.rs
crates/bunny-geom/src/lib.rs
crates/bunny-perf/src/cases.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
Möller and Granlund, "Improved Division by Invariant Integers" (IEEE TC 60(2), 2011)

Files edited:
CHANGELOG.md
crates/bunny-num/src/fixed_divisor.rs
crates/bunny-num/src/lib.rs
crates/bunny-num/tests/fixed_divisor_tests.rs
crates/bunny-query/src/ray.rs
crates/bunny-query/src/ray/prepared.rs
crates/bunny-query/src/lib.rs
crates/bunny-query/tests/prepared_ray_tests.rs
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
.repo-respect/receipts/2026-11-13-fixed-divisor-prepared-rays.md

Topic docs:
docs/topics/performance/README.md (ledger entry 4)
docs/topics/performance/test-plan.md (PERF-TP-014 to PERF-TP-019)

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-num --test fixed_divisor_tests (4 passed)
cargo test --locked -p bunny-query --test prepared_ray_tests (2 passed)
bunny-perf builds with each feature group alone and together
cargo run --locked -p xtask -- perf --base origin/perf/bunny-isqrt --rounds 11 --align-code
(best p50: prepared/div 3.54 ns vs num/div 6.27 ns; prepared/ray_aabb 22.46 ns vs
query/ray_aabb 39.84 ns; prepared/ray_prepare 24.78 ns; existing cases within 1.5 %)
A scratch prototype matched checked_div on 1 000 000 random pairs before implementation.
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all

Follow-up: recorded the rejected division-free normalize experiment (branch
experiment/divide-free-normalize) under "Tried And Rejected" in
docs/topics/performance/README.md, with its measurements.

Known risks:
The correctness of Algorithm 4 rests on the paper's proof and the equivalence tests, which
sample rather than exhaust the input space. New public API names may change in review.
Speed measured on an Apple M5 Pro only.

Human reviewer:
James Ross approved the researched speedups as new APIs with stated bounds on 2026-11-13;
review pending.
