# Receipt: bunny-optics Typed Shading Refusals, Lint Header, Core-Crate List

Task:
Implement flyingrobots/bunny#280. Add `try_` forms of `SurfaceHit::new`, `SurfaceHit::bounce_ray`
and `ShadingFrame::cosine_direction` that return a typed `SecondaryRayError` (one variant per
refusal cause, carrying the observed values); keep the `Option` functions as the fast path with
their 0.7.1 bodies and cost; the `try_` forms are the diagnostic path. Replace the blanket `clippy::nursery` deny with named lints, give every
allowance a reason, and list `bunny-optics` among the core crates in `CODE_STANDARDS.md`.
`CameraError` values were not added: changing its unit variants is not additive.

Files read:
CODE_STANDARDS.md (Rules 3, 4, 9, 10)
docs/CODE_DOJO.md
docs/PROCESS.md
docs/topics/performance/README.md (Fidelity Rule)
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
xtask/src/code_dojo.rs (CORE_CRATES)
crates/bunny-optics/src/** and crates/bunny-optics/tests/**
crates/bunny-query/src/trace.rs (TraceRay, TriangleHit)
crates/bunny-num/src/fixed_q32_32.rs (sqrt)
crates/bunny-perf/src/shade.rs

Files edited:
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/pixel.rs
crates/bunny-optics/src/shading.rs
crates/bunny-optics/src/shading/frame.rs
crates/bunny-optics/src/shading/hit.rs
crates/bunny-optics/src/shading/refusal.rs (new)
crates/bunny-optics/tests/shading_refusal_tests.rs (new)
crates/bunny-optics/tests/shading_reference/mod.rs (new, 0.7.1 code over the public API)
crates/bunny-optics/README.md
docs/topics/optics/README.md
docs/topics/optics/test-plan.md (OPTICS-REQ-041 to -043; OPTICS-TP-057 to -064)
docs/topics/performance/README.md (contract change cost check, Tried And Rejected)
CODE_STANDARDS.md (Rule 9 core list)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-typed-refusals.md

Topic docs:
docs/topics/optics/README.md (Facing And Secondary Rays, Inputs And Refusals)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: the new tests fail to build on the unmodified crate (no `try_new`, `try_bounce_ray`,
`try_cosine_direction` or `SecondaryRayError`). The 0.7.1 `SurfaceHit::new` on a hit point
2^30 units above the ground panics in a debug build ("attempt to multiply with overflow" in
the plane test) and returns `None` in a release build.
cargo fmt, clippy -D warnings and the bunny-optics tests in one command.
Mutations (test plan, Shading Mutation Checks): each of three fails a case.
Shading timing: `cargo run --locked -p xtask -- perf --base origin/main --rounds 11
--align-code --filter shade/` through devrun with --exclusive (Apple M5 Pro, rust-dev:1.96.0,
base 8e3d2079e78d). Every version and its numbers are in the performance chapter under
"Typed shading refusals (flyingrobots/bunny#280)", and the shared-core versions under
"Tried And Rejected". Selected fast-path version, three runs: shade/hit_shading best p50
-0.2 %, +0.7 %, +0.4 %; median p50 +0.9 %, +1.5 %, +1.8 %; the unchanged control
shade/surface_new moved -2.6 % to +2.2 % in the same runs.
Inaccurate commit subject: f340827 ("bunny-optics: keep the per-hit shading step at its 0.7.1
cost") states its aim, not its result. Measured at that commit, shade/hit_shading was 3.5 %
(median) and 3.7 % (best) slower than 0.7.1; the cost was recovered only by 8d081e8. Commits
cannot be amended, so the record is corrected here and in the PR body.
Mutations (test plan, Shading Mutation Checks): six, four of them breaking one side of the
fast/diagnostic pair alone; each fails a case. 30 consecutive fresh-seed runs of OPTICS-TP-064
passed.
code-dojo --all at the final commit (see the PR).

Known risks:
The one behaviour change is the debug-build panic above becoming `None`; release-build outputs
are unchanged. `BounceRayRefused` is defensive and has no constructed case. The fast and
diagnostic paths are separate code; OPTICS-REQ-043 and its differential cases keep them in
agreement. The overflow guard's cost is inside the noise band, not shown to be zero.

Human reviewer:
James Ross (maintainer); review pending.
