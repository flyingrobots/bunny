# Receipt: bunny-optics Typed Shading Refusals, Lint Header, Core-Crate List

Task:
Implement flyingrobots/bunny#280. Add `try_` forms of `SurfaceHit::new`, `SurfaceHit::bounce_ray`
and `ShadingFrame::cosine_direction` that return a typed `SecondaryRayError` (one variant per
refusal cause, carrying the observed values); keep the `Option` functions as thin wrappers with
every 0.7.1 output. Replace the blanket `clippy::nursery` deny with named lints, give every
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
docs/topics/optics/test-plan.md (OPTICS-REQ-041, -042; OPTICS-TP-057 to -063)
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
Shading timing, `cargo run --locked -p xtask -- perf --base origin/main --rounds 11
--align-code --filter shade/` through devrun with --exclusive (Apple M5 Pro, rust-dev:1.96.0,
base 8e3d2079e78d), best p50: at 1f82e42 shade/hit_shading +9.1 %; at 4a8e243 (#[inline])
+6.9 %; at f340827 (shared start-point core, lazy errors, bounce_ray on the Option direction)
+3.7 % (65.55 -> 67.95 ns), with shade/surface_new, whose code is unchanged, at -0.1 % in the
same run and shade/bounce_direction at +0.6 %; at the commit that split the start point to the
function limit, +4.1 % (66.12 -> 68.81 ns), surface_new -0.9 %, bounce_direction +0.4 %. The
subject of f340827 states its aim; the residual of about 4 % is a measured slowdown, reported
to the maintainer rather than claimed away.
code-dojo --all at the final commit (see the PR).

Known risks:
The one behaviour change is the debug-build panic above becoming `None`; release-build outputs
are unchanged. `BounceRayRefused` is defensive and has no constructed case. The per-hit shading
step measures about 2.7 ns (4 %) slower than 0.7.1; see Checks run.

Human reviewer:
James Ross (maintainer); review pending.
