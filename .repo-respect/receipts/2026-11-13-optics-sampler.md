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
crates/bunny-optics/src/hash.rs
crates/bunny-optics/src/sampler.rs
crates/bunny-optics/src/xoroshiro.rs
crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/sampler_tests.rs
crates/bunny-optics/tests/xoroshiro_tests.rs
crates/bunny-optics/Cargo.toml
crates/bunny-optics/README.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/sample.rs
Cargo.lock
xtask/src/perf.rs
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/performance/README.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
docs/TESTING.md
README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-optics-sampler.md

Topic docs:
docs/topics/optics/README.md (Sampling: identities, sample hash, fractions, pixel sampler,
stratification and its proof, dimension pairs, determinism, statistical limits, sequential
generator)
docs/topics/optics/test-plan.md (OPTICS-REQ-010..016, OPTICS-TP-010..021, sampler mutations)
docs/topics/performance/README.md (the `sample` harness group)

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host except the
Python reference model that produced the golden vectors (it reproduces the published
SplitMix64 outputs and Echo's Prng fixtures first).
cargo test --locked -p bunny-optics --test sampler_tests --test xoroshiro_tests at the
test-only commit (does not compile: the API is missing)
cargo fmt --all; cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
strict package clippy on bunny-optics; cargo test --locked --workspace; code-dojo-rust --all
Deliberate mutations (seven, listed in the test plan): each made at least one case fail.
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code --filter sample/
(--exclusive), Apple M5 Pro in rust-dev:1.96.0, base origin/main 8cdbb1c: all three cases
are new (best p50 0.49 ns stream_bits, 19.65 ns sampler_point, 23.36 ns sampler_new_point);
xtask exits 1 because no case exists on both sides. A reading, not a speed claim.
cargo test --locked --workspace and code-dojo --all at the final commit

Echo and attribution:
Echo's warp-math Prng is a stateful xoroshiro128+; it cannot give random access by pixel and
sample, so it was not used for rendering. It is carried as Xoroshiro128Plus, reimplemented
from the published algorithm and Echo's documented behaviour, not copied: no Echo file or
licence header was copied. Echo's prng.rs and warp-math Cargo.toml are Apache-2.0 (the dual
Apache-2.0 OR MIND-UCAL notice applies to Echo's documentation corpus), Bunny is Apache-2.0,
and both have the same author; the docs name Echo's warp-math as the compatibility target.

Known risks:
The contract choices (key packing, domain constants, the Burley constants, pair 0 as the
sub-pixel offset, Xoroshiro128Plus living in bunny-optics) are the maintainer's to overrule
before any artifact records the identities. No statistical test suite was run.

Human reviewer:
James Ross (maintainer); review pending.
