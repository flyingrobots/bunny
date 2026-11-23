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
- `.repo-respect/receipts/2026-11-23-perf-profile-counted-counts.md`

Topic docs:
None yet.

Generated artifacts:
None.

Checks run:
- Failing test first: `the_profile_takes_its_counts_from_the_counted_searches` does not compile until the counted source exists.

Known risks:
- Work in progress.

Human reviewer:
@flyingrobots
