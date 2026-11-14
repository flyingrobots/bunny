# Receipt: Binned SAH BVH Builder

Task:
Implement flyingrobots/bunny#213: add `build_bvh_sah`, a binned surface-area-heuristic
BVH builder, to `bunny-broadphase` as a new function writing the same buffers and node
format as `build_bvh`; leave `build_bvh`'s output unchanged; measure traversal through
both trees on the benchmark scene and record the result as ledger entry 6.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/README.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/repo-respect/README.md
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/src/bvh/build.rs
crates/bunny-broadphase/src/bvh/split.rs
crates/bunny-broadphase/src/utils.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/tests/bvh_builder_tests.rs
crates/bunny-broadphase/tests/bvh_allocation_tests.rs
crates/bunny-broadphase/tests/determinism.rs
crates/bunny-broadphase/tests/closest_hit_tests.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/timing.rs
xtask/src/code_dojo.rs
xtask/src/perf.rs
xtask/src/topic_docs.rs

Files edited:
crates/bunny-broadphase/src/bvh.rs
crates/bunny-broadphase/src/bvh/build.rs (visibility of four helpers only)
crates/bunny-broadphase/src/bvh/sah.rs
crates/bunny-broadphase/src/bvh/sah/bins.rs
crates/bunny-broadphase/src/bvh/sah/cost.rs
crates/bunny-broadphase/src/bvh/sah/partition.rs
crates/bunny-broadphase/src/bvh/sah/tests/mod.rs
crates/bunny-broadphase/src/lib.rs
crates/bunny-broadphase/tests/common/mod.rs
crates/bunny-broadphase/tests/bvh_sah_tests.rs
crates/bunny-broadphase/tests/bvh_sah_golden.rs
crates/bunny-broadphase/tests/bvh_sah_allocation_tests.rs
crates/bunny-broadphase/README.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/profile.rs (`--stats` also reports the SAH tree)
docs/README.md
docs/topics/bvh-construction/README.md
docs/topics/bvh-construction/test-plan.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-sah-bvh-builder.md

Topic docs:
docs/topics/bvh-construction/README.md and test-plan.md (new chapter: node format, both
builders' contracts, BVH-REQ-001..009, BVH-TP-001..021)
docs/topics/performance/README.md (ledger entry 6, adding-a-case) and test-plan.md
(PERF-REQ-008, PERF-TP-042, PERF-TP-043)

Generated artifacts:
None.

Checks run:
All in the rust-dev container through devrun: cargo fmt --all -- --check; cargo clippy
--workspace --all-targets --all-features -D warnings; strict package clippy for
bunny-broadphase; cargo test --locked --workspace; topic-docs; code-dojo --all; two
`perf --base origin/main --rounds 11 --align-code` runs with --exclusive; parameter sweeps
and deliberate mutations recorded in the topic docs. Results are in the pull request.

Known risks:
The `trace/kernel_tri_*` replay cases read 7 to 14 % slower on the head side in both
runs although no code they run changed; not reproduced in workspace builds, cause not
established (see ledger entry 6).
The traversal measurement is one synthetic scene on one machine. The SAH tree is a
different tree, so `intersect_ray` and `intersect_aabb` report primitives in a different
order on it than on a `build_bvh` tree; `closest_hit` and `any_hit` answers are the same.

Human reviewer:
James Ross (maintainer); review pending.
