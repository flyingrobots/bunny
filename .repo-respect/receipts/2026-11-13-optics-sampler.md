# Receipt: Deterministic Sample Hash, Pixel Sampler And Sequential Generator

Task:
Implement flyingrobots/bunny#221: a counter-based sample hash, an Owen-scrambled Sobol
pixel sampler and dimension pairs for `bunny-optics`, with golden vectors, an exact
stratification property and evaluation-order independence; evaluate Echo's `warp-math`
generator and carry it bit for bit as `Xoroshiro128Plus`. Hemisphere mapping (#160) is
out of scope; no existing function changes.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/NUMERIC_CONSTITUTION.md
docs/design/0001-bunny-graphics-commons.md
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/performance/README.md
crates/bunny-optics/src/*.rs
crates/bunny-optics/tests/camera_tests.rs
crates/bunny-perf/src/*.rs
xtask/src/perf.rs
Echo (read only): crates/warp-math/src/prng.rs, crates/warp-math/tests/prng_golden_regression.rs,
crates/warp-math/tests/fixtures/math-fixtures.json, docs/adr/0019-bunny-owns-reusable-geometry.md

Files edited:
crates/bunny-optics/tests/sampler_tests.rs
crates/bunny-optics/tests/xoroshiro_tests.rs
docs/topics/optics/test-plan.md
.repo-respect/receipts/2026-11-13-optics-sampler.md

Topic docs:
docs/topics/optics/test-plan.md (OPTICS-REQ-010..016, OPTICS-TP-010..021)

Generated artifacts:
None.

Checks run:
In progress.

Known risks:
In progress.

Human reviewer:
James Ross (maintainer); review pending.
