# Receipt: Ray Packets For Coherent Camera Rays (Spike, Rejected)

Task:
Spike for flyingrobots/bunny#217: build the simplest correct packet traversal
(`closest_hits` in `bunny-broadphase`), measure it against `closest_hit` one ray at a time
on the benchmark scene, and record the result either way. Outcome: rejected on the Apple
M5 Pro; the packet code is removed again and the measurement is recorded as entry 7 under
"Tried And Rejected" in the performance chapter.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-query/src/trace.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/src/profile.rs
xtask/src/perf.rs
xtask/src/code_dojo.rs

Files edited (spike commits, removed again in the last commit):
crates/bunny-broadphase/src/packet/mod.rs
crates/bunny-broadphase/src/packet/frustum.rs
crates/bunny-broadphase/src/packet/tests/mod.rs
crates/bunny-broadphase/tests/packet_tests.rs
crates/bunny-broadphase/src/lib.rs
crates/bunny-perf/src/packet.rs
crates/bunny-perf/src/packet_profile.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/scene.rs
crates/bunny-perf/Cargo.toml
docs/topics/ray-tracing/test-plan.md (TRACE-REQ-028, TRACE-TP-111..116; removed with the tests)

Files edited (kept):
docs/topics/performance/README.md (entry 7 under "Tried And Rejected"; profile item 4)
CHANGELOG.md
.repo-respect/receipts/2026-11-14-ray-packets.md

Topic docs:
docs/topics/performance/README.md (Tried And Rejected, entry 7)

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host.
cargo fmt --all -- --check; cargo clippy --locked -p bunny-broadphase -p bunny-perf --all-targets -- -D warnings
cargo test --locked -p bunny-broadphase packet (3 unit, 4 integration cases) at 7c65c5e
cargo test --locked --release -p bunny-perf packet (6 cases) at 7c65c5e
Mutations of the whole-packet test at 7c65c5e: six caught, three survive with reasons
(listed in the ledger entry).
bunny-perf --stats at 7c65c5e (packet counters).
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code, twice, with
--exclusive, at 7c65c5e (base 57bc035f69f2).
PR job "Perf / before and after" (x86-64, 7 rounds) read twice at 7c65c5e.
At the final commit: cargo test --locked --workspace and code-dojo --all (see the PR).

Known risks:
The spike commit 7c65c5e fails Code Dojo on one 101-character waiver line in
crates/bunny-broadphase/src/packet/mod.rs; the file is removed in the final commit. The
x86-64 readings favour the shared descent (14 and 20 % faster at 8 x 8 on shared CI runners)
and were not followed up here; whether to pursue a target-specific packet path is the
maintainer's call.

Human reviewer:
James Ross (maintainer); review pending.
