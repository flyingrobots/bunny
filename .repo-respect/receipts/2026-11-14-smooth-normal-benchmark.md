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
.repo-respect/receipts/2026-11-14-smooth-normal-benchmark.md

Topic docs:
Pending in this branch.

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Pending in this branch.

Known risks:
Pending in this branch.

Human reviewer:
James Ross (maintainer); review pending.
