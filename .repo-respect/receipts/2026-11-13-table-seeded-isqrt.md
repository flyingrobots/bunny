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
Local release-mode measurements on an Apple M5 Pro (single thread, mean):
`sqrt` 47.9 ns -> 18.4 ns; build and normalize a ray 205 ns -> 116 ns;
`ray_intersects_sphere` on a hit 132 ns -> 74.5 ns.

Known risks:
Relies on `u128::isqrt` (stable and const since Rust 1.84, below the 1.96 MSRV)
being an exact floor square root, which the Rust documentation states and the
equivalence tests check against the retained reference. The timings are local
single-machine measurements, not a checked-in benchmark.

Human reviewer:
James Ross requested the speedup ("let's use game dev tricks to speed up the
bunny math") on 2026-11-13; review pending.
