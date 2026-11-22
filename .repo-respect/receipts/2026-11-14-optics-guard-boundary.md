# Receipt: The Optics Beyond-Reach Guard's Boundary

Task:
flyingrobots/bunny#324 (from the independent review in flyingrobots/bunny#290, batch 3). Pin the
boundary of the beyond-reach guard of `SurfaceHit::new` and `SurfaceHit::try_new` with
constructed hits whose expected values are derived by hand, so that moving either guard fails a
test that does not compare with the retained 0.7.1 copy. Test and documentation only: no
library function changes.

Files read:
crates/bunny-optics/src/shading/ (hit.rs, surface.rs, refusal.rs), src/point.rs
crates/bunny-optics/tests/shading_refusal_tests.rs, start_point_tests.rs,
shading_reference/mod.rs
crates/bunny-query/src/trace.rs (TRACE_COORD_LIMIT_RAW, TriangleHit)
docs/topics/optics/README.md, docs/topics/optics/test-plan.md
CONTRIBUTING.md, docs/PROCESS.md, docs/CODE_DOJO.md
.repo-respect/receipts/2026-11-14-optics-start-point-two-steps.md

Files edited:
crates/bunny-optics/tests/domain_guard_tests.rs (new: OPTICS-TP-096, OPTICS-TP-097)
docs/topics/optics/test-plan.md (OPTICS-TP-096 to -098; overflow-guard mutation checks)
docs/topics/optics/README.md (the guard's bound is exact)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-guard-boundary.md

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Survival reproduced first, on the unchanged tests, each mutant under `timeout 120` with every
source touched before the build (`cargo test --locked -q -p bunny-optics --no-fail-fast`): the
fast path's guard tightened (refusing one raw unit outside) and loosened (admitting two raw
units outside) each failed nothing; the diagnostic path's guard tightened failed only
OPTICS-TP-062 and -064 (the comparison with the retained 0.7.1 code); loosened, it failed
nothing.
After: the fast guard tightened fails OPTICS-TP-096; the diagnostic guard tightened fails
OPTICS-TP-096 (and -062, -064); the diagnostic guard loosened fails OPTICS-TP-097; the fast guard
loosened fails nothing and is shown equivalent in the test plan. The restored source passes
every bunny-optics test. A stale build cache once served the last mutant to a later run (the
copy's file times were older than the cached build); every result above was taken after
touching every source.
cargo fmt --all --check, cargo clippy -p bunny-optics --all-targets -D warnings, cargo test -p
bunny-optics; code-dojo --all at the final commit (see the PR).

Known risks:
The expected values are hand derived by the same author who wrote the tests; a misreading of
the start-point rule shared by the chapter, the code and the derivation would pass. The fast
path's loosened guard changes no output of `SurfaceHit::new` (a point two raw units outside
cannot step inside), so no test can catch it; it is recorded as an equivalent mutant.

Human reviewer:
James Ross (maintainer); review pending.
