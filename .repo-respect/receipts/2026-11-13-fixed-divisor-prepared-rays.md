# Receipt: Fixed Divisor And Prepared Rays

Task:
Add exact division by a prepared divisor (`FixedDivisor`, Möller–Granlund Algorithm 4) and
a prepared ray (`PreparedRay3`, `ray_intersects_aabb_prepared`) whose box test multiplies
instead of dividing, with results identical to the existing `checked_div` and
`ray_intersects_aabb` (including its overflow handling, flyingrobots/bunny#211). Re-measured
and merged with main for flyingrobots/bunny#220; no longer stacked on the two-divide square
root, which main retains but does not select.

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
crates/bunny-query/README.md
docs/topics/ray-tracing/README.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
.repo-respect/receipts/2026-11-13-fixed-divisor-prepared-rays.md

Topic docs:
docs/topics/performance/README.md (ledger entry 5)
docs/topics/performance/test-plan.md (PERF-TP-034 to PERF-TP-041)

Generated artifacts:
None.

Checks run:
cargo fmt --all --check; cargo clippy --locked --workspace --all-targets -- -D warnings;
cargo test --locked --workspace (all passed, in the rust-dev:1.96.0 container)
prepared_ray_tests: 4 passed, including the overflow corpus and the constructed overflow
case; a deliberate mutation (discarding the wide path's hit) fails both
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code, twice, with
--exclusive, base cc0521e15084, 2026-11-13, Apple M5 Pro in the container; best p50, runs 1
and 2: prepared/div 3.99 and 3.50 ns vs num/div 6.43 and 6.47; prepared/ray_aabb 21.73 and
21.53 vs query/ray_aabb 52.25 and 51.47 (same runs); prepared/ray_prepare 26.00 and 26.04;
trace/box_entry 5.98 and 5.98; trace/box_ray_prepare 1.83 and 1.83. Two earlier runs against
base b2fe36638429 agree. The earlier host numbers are superseded. Full tables: ledger
entry 5 and the pull request.
Harness: the case helper in bunny-perf compiled only with the scene or trace group on main,
so the other groups failed to build alone against a base; repaired in this branch.
cargo run --locked -p xtask -- topic-docs
cargo run --locked -p xtask -- code-dojo --all

Superseded follow-up: the division-free normalize experiment (branch
experiment/divide-free-normalize) stays recorded under "Tried And Rejected" with its
original measurements, which predate the black_box harness and were not repeated.

Known risks:
The correctness of Algorithm 4 rests on the paper's proof and the equivalence tests, which
sample rather than exhaust the input space. New public API names may change in review.
Speed measured on an Apple M5 Pro only; the x86-64 CI job is reported in the pull request.
PreparedRay3 has no caller in the workspace; box_entry is faster for traversal, and the
maintainer decides whether to keep an exact box API.

Human reviewer:
James Ross approved the researched speedups as new APIs with stated bounds on 2026-11-13;
review pending.
