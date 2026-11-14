# Receipt: Smooth-Shaded Bounces and the Wider Smooth-Normal Oracle

Task:
Part 3b of flyingrobots/bunny#289: `SmoothHit::bounce` and `SmoothHit::is_above_surface`
(`bunny-smooth-shading/1`), which sample the bounce around the shading normal and reject a
direction at or below the exact geometric plane (`SmoothBounce::BelowSurface`, no ray, zero
contribution); the estimator's difference from `bunny-diffuse-shading/1` documented, energy loss
included; the shared test oracle extended to signed big integers so `smooth_normal` is checked
for determinants up to 2^117 and for every rho; an independently derived radiance for the smooth
light term. No existing output changes; the benchmark is the last part.

Files read:
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (smooth_hit, frame, hit)
`crates/bunny-optics/tests/**` (exact, smooth_normal_tests, smooth_hit_tests, vertex_normal_tests)

Files edited:
crates/bunny-optics/src/shading/smooth_hit.rs, shading.rs, lib.rs
crates/bunny-optics/tests/exact/mod.rs (signed big integers, a slack term)
crates/bunny-optics/tests/smooth_normal_tests.rs, vertex_normal_tests.rs, smooth_hit_tests.rs
crates/bunny-optics/README.md
docs/topics/optics/README.md (Smooth Shading: the bounce and the estimator)
docs/topics/optics/test-plan.md (OPTICS-REQ-051; OPTICS-TP-083 to -085; TP-076, -077, -081)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-smooth-bounce.md

Topic docs:
docs/topics/optics/README.md (Status, Smooth Shading)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: the new cases fail to build on the crate without `SmoothBounce`, `bounce` and
`is_above_surface`.
Green: every bunny-optics test passes. The smooth light's radiance (raw 2 187 408 882 per
channel) is derived by hand in the test's comment; the rejection counts of OPTICS-TP-085 (49 of
256) were read from one run and locked.
Mutations (test plan, Smooth-Shading Mutation Checks): four new for the bounce, and three of the
smooth-normal mutations rerun against the widened oracle; each fails at least one case.
cargo fmt, clippy -D warnings.
code-dojo --all at the final commit (see the PR).

Known risks:
No seeded case of OPTICS-TP-076 reaches the accepted "None for rho < 2^-59" outcome. The 49 of
256 is a regression lock, not an independent count.

Human reviewer:
James Ross (maintainer); review pending.
