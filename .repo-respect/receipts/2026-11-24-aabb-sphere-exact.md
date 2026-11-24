# Receipt: Exact AABB-Sphere Overlap

Task:
Fix `aabb_intersects_sphere` in `bunny-query` (flyingrobots/bunny#501), which reported a sphere as meeting any box once its rounded Q32.32 squared radius saturated (radius magnitude 199 032 864 766 431 raw and up, about 46 341 units). Decide overlap by the exact comparison of the squared distance and squared radius on raw integers, keep the returned point, the box reading and the radius sign as they were, and state which inputs change. Classify the two siblings the issue names (`get_centroid`, `surface_area`).

Files read:
- `crates/bunny-query/src/closest.rs`
- `crates/bunny-query/tests/closest_raw_tests.rs`
- `crates/bunny-query/tests/query_tests.rs`
- `crates/bunny-query/tests/box_entry_equivalence_tests.rs`
- `crates/bunny-query/README.md`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-broadphase/src/utils.rs`
- `crates/bunny-broadphase/src/bvh/split.rs`
- `crates/bunny-broadphase/src/sweep_and_prune.rs`
- `docs/topics/bvh-construction/README.md`
- `docs/topics/performance/README.md`
- `docs/topics/lerp/test-plan.md`
- `docs/topics/bounding-volumes/README.md`
- `xtask/src/topic_docs.rs`
- `CONTRIBUTING.md`

Files edited:
- `crates/bunny-query/src/closest.rs` (only `aabb_intersects_sphere` and its rustdoc)
- `crates/bunny-query/tests/aabb_sphere_exact_tests.rs`
- `crates/bunny-query/README.md` (one line)
- `docs/topics/aabb-sphere-overlap/README.md`
- `docs/topics/aabb-sphere-overlap/test-plan.md`
- `docs/README.md` (one row)
- `CHANGELOG.md` (one bullet under Fixed)
- `.repo-respect/receipts/2026-11-24-aabb-sphere-exact.md`

Topic docs:
New topic `docs/topics/aabb-sphere-overlap/` with ledger ASO-REQ-001 to -003 and ASO-TP-001 to -009, all implemented, and recorded mutation checks.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-query --test aabb_sphere_exact_tests` (on `main`'s kernel: seven of eight cases fail, reproducing the defect; ASO-TP-007 passes; with the fix all eight pass)
- Eight mutations of the kernel, each failing at least one case (test plan, Mutation Checks)
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo clippy --locked -p bunny-query --all-targets --all-features -- -D warnings` (Rust 1.100.0)
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- code-dojo-perf-groups`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- A public function's answer changes: for radii whose rounded square saturated, and for inputs within `2^33` raw units squared of touching, where the old rounded products could order the squares wrongly. Callers depending on either old answer see a different result.
- The degeneracy policy of flyingrobots/bunny#116 is undecided; inverted boxes and negative radii keep their current reading.

Human reviewer:
@flyingrobots
