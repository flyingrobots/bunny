# Receipt: Perf Profile Counted Counts

Task:
Fix `flyingrobots/bunny#344`: `bunny-perf`'s profile replicated the traversal loop to count nodes, boxes and leaves although `compact_closest_hit_counted` and `compact_any_hit_counted` return those counts. The profile now takes them from the counted searches and keeps the replica only for the per-stage triangle counters and the recorded calls the kernel replays need.

Files read:
- `crates/bunny-perf/src/profile.rs`
- `crates/bunny-perf/src/counted.rs`
- `crates/bunny-perf/src/main.rs`
- `crates/bunny-broadphase/src/counts.rs`
- `crates/bunny-broadphase/src/compact_hit.rs`
- `docs/topics/performance/README.md`

Files edited:
- `crates/bunny-perf/src/profile.rs`
- `crates/bunny-perf/src/main.rs`
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md`
- `docs/topics/ray-tracing/README.md`
- `docs/topics/ray-tracing/test-plan.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-perf-profile-counted-counts.md`

Topic docs:
- `docs/topics/performance/README.md`
- `docs/topics/performance/test-plan.md` (PERF-REQ-007 reworded, PERF-TP-057 added)
- `docs/topics/ray-tracing/README.md`
- `docs/topics/ray-tracing/test-plan.md` (TRACE-TP-418 reworded)

Generated artifacts:
None.

Checks run:
- Red first: the test of PERF-TP-057 did not build before the counted source existed.
- `cargo run --release -p bunny-perf -- --stats` before and after: identical output (156 lines, both trees), so no reported number changed.
- Mutation: `leaf_visited` set one too high fails `the_profile_takes_its_counts_from_the_counted_searches` and `counters_obey_their_identities`; restored.
- `cargo test --locked --workspace --all-targets --all-features`: 717 passed, 0 failed.
- `cargo fmt --all --check`
- `cargo run --locked -p xtask -- code-dojo-perf-groups` (every group alone and all together)
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- `--stats` now needs the `trace_counted` group (it errors without it); the `trace` group alone still builds and keeps the kernel cases, so a base revision without the counted searches still compares them.
- The counted searches run on the compact copy of the tree; TRACE-TP-418 and the identical `--stats` output cover the quartile and SAH trees.
- `cargo deny` and markdownlint are not installed locally; CI runs them.

Human reviewer:
@flyingrobots
