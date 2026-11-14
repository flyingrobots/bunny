# Receipt: Table-Seeded Integer Square Root

Task:
Speed up `FixedQ32_32::sqrt_u128` (and so `sqrt`, `FixedVec3::length`,
`normalize` and the ray queries built on them) without changing a single output
bit. The bit-by-bit loop is replaced by the standard library's `u128::isqrt`,
which seeds from a lookup table and refines with integer arithmetic only. Also
fixes the `chunks_exact_to_as_chunks` clippy lint that newer toolchains report
in two `bunny-codec` test helpers.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/tests/property_tests.rs
crates/bunny-num/tests/fixed_q32_32_tests.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-query/src/ray.rs
crates/bunny-codec/tests/ply_allocation_tests.rs
crates/bunny-codec/tests/compressed_tests.rs

Files edited:
CHANGELOG.md
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/tests/sqrt_equivalence_tests.rs
crates/bunny-codec/tests/ply_allocation_tests.rs
crates/bunny-codec/tests/compressed_tests.rs
.repo-respect/receipts/2026-11-13-table-seeded-isqrt.md

Topic docs:
None. The public contract (exact floor square root) is unchanged.

Generated artifacts:
None.

Checks run:
cargo run --locked -p xtask -- code-dojo --all (toolchain 1.96.0: clean, 49 test suites passing)
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings (Homebrew 1.98.1: clean)
cargo test --locked -p bunny-num --test sqrt_equivalence_tests
Equivalence: the new `sqrt_u128` equals the previous loop on all power-of-two
boundaries, perfect squares and their neighbours, 20 000 seeded inputs spread
across every magnitude, and every raw value 0..=65 536; a local check of
2 000 000 seeded inputs also matched.
Re-measurement (flyingrobots/bunny#218), superseding the earlier host numbers taken
with the older harness (which let the optimizer drop work), none of which are kept.
Merged origin/main (be55424fef16) into the branch, then, on an Apple M5 Pro inside the
flyingrobots/rust-dev:1.96.0 container (Linux VM), 2026-11-13, one run:
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
Best p50 speedups: num/sqrt 62.54 -> 12.82 ns (4.88x), linalg/length 4.29x,
query/ray_build 2.39x, query/ray_sphere_miss 2.32x, linalg/normalize 2.09x,
query/ray_sphere_hit 1.58x. Controls without a square root: num/div +0.7 %, num/mul 0.0 %,
query/ray_triangle +0.6 %, query/ray_aabb -1.0 % (best p50). Full table in
docs/topics/performance/README.md (ledger entry 1). Not measured: x86-64, WASM, the host.
Also ran cargo test --locked --workspace in the container after the merge: all passed.
Docs edited for this: docs/topics/performance/README.md, docs/topics/performance/test-plan.md.

Known risks:
Relies on `u128::isqrt` (stable and const since Rust 1.84, below the 1.96 MSRV)
being an exact floor square root, which the Rust documentation states and the
equivalence tests check against the retained reference. The timings are a single-machine
measurement, not a checked-in benchmark.

Human reviewer:
James Ross requested the speedup ("let's use game dev tricks to speed up the
bunny math") on 2026-11-13; review pending.
