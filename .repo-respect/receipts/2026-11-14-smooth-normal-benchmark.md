# Receipt: bunny-perf Smooth-Normal Benchmark And Ledger Entry

Task:
Last part of flyingrobots/bunny#289 (smooth shading normals): measure the per-hit smooth path
with the perf harness and record it in the performance ledger. A new `smooth` feature group in
`bunny-perf` times the shipped interpolation (`barycentric_numerators` then `smooth_normal`), a
replica of the rejected alternative (two floored `barycentrics` weights), and the whole smooth
hit beside a flat hit on the same inputs. No production code changes.

Files read:
docs/topics/performance/README.md (Fidelity Rule, Measuring, ledger, Tried And Rejected)
docs/topics/performance/test-plan.md
docs/topics/optics/README.md (Smooth Normal, Smooth Shading, Accuracy And Cost)
docs/topics/optics/test-plan.md (Timing, OPTICS-TP-074 to -085)
docs/CODE_DOJO.md
`crates/bunny-perf/src/**`, crates/bunny-perf/Cargo.toml
`crates/bunny-optics/src/shading/**` (smooth_normal.rs, smooth_hit.rs, wide.rs)
crates/bunny-query/src/trace.rs and barycentric.rs
xtask/src/perf.rs

Files edited:
crates/bunny-perf/Cargo.toml, crates/bunny-perf/src/main.rs, crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/scene.rs (`camera_grid`), crates/bunny-perf/src/shade.rs (`lighting`)
crates/bunny-perf/src/smooth.rs (new)
docs/topics/performance/README.md (Adding a case, ledger entry 13, Tried And Rejected)
docs/topics/optics/README.md (Smooth Normal and Smooth Shading cost, Accuracy And Cost)
docs/topics/optics/test-plan.md (OPTICS-TP-086, Timing)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-smooth-normal-benchmark.md

Topic docs:
docs/topics/optics/README.md, docs/topics/performance/README.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: the agreement and batch tests were written with empty expected counts and failed,
printing the counts the inputs produce; those are now locked.
Mutations of the replica, each failing the agreement test on its bound assertion (shot 0): the
weight `w0 = 2^32 - b1` (b2 dropped), and the weights of `v1` and `v2` swapped.
cargo fmt --check, clippy -D warnings on bunny-perf (all targets), the bunny-perf tests.
Timing: `cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
--filter shade/` with --exclusive, three runs (Apple M5 Pro, rust-dev:1.96.0, base e7a4fabf55bb),
and one run with `--base HEAD`. Recorded as ledger entry 13 and under Tried And Rejected.
code-dojo --all at the final commit (see the PR).

Known risks:
The comparison of (a) and (b) is from one machine (aarch64); x86-64 and WebAssembly divide
`i128` at different costs. The inputs leave out the scene's ground, whose hits take the
shared-normal step in both paths. Where the extra time of a smooth hit goes was not timed step
by step.

Human reviewer:
James Ross (maintainer); review pending.
