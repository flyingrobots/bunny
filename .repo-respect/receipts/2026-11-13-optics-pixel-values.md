# Receipt: Optics Pixel Values

Task:
Implement flyingrobots/bunny#222: pixel-value math in `bunny-optics`: exact sample
accumulation, the mean's rounding, an sRGB and a linear tone curve, 8-bit and 16-bit
quantization and an RGBA8 writer over caller-provided slices, integer-only, with versioned
identities, golden vectors, an `f64` reference oracle in tests and deliberate mutations.

Files read:
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/NUMERIC_CONSTITUTION.md
docs/CODE_DOJO.md
docs/topics/performance/README.md
docs/topics/repo-respect/README.md
CODE_STANDARDS.md
CHANGELOG.md
clippy.toml
rustfmt.toml
Cargo.toml
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
xtask/src/perf.rs
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/camera.rs
crates/bunny-optics/src/error.rs
crates/bunny-optics/src/point.rs
crates/bunny-optics/tests/camera_tests.rs
crates/bunny-optics/README.md
crates/bunny-broadphase/src/bvh.rs
crates/bunny-num/tests/fixed_angle_tests.rs
crates/bunny-num/src/fixed_angle.rs
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
.repo-respect/receipts/2026-11-13-optics-camera.md

Files edited:
crates/bunny-optics/Cargo.toml
crates/bunny-optics/README.md
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/pixel.rs
crates/bunny-optics/src/pixel/accumulate.rs
crates/bunny-optics/src/pixel/srgb.rs
crates/bunny-optics/src/pixel/srgb_knots.rs
crates/bunny-optics/src/pixel/tone.rs
crates/bunny-optics/tests/pixel_tests.rs
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
Cargo.lock
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/README.md
docs/TESTING.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-optics-pixel-values.md

Topic docs:
docs/topics/optics/README.md (new Pixel Values sections: accumulation and its bit budget,
resolve, tone curves with the knot table, hold and error bound, output codes, accuracy of the
codes against correct rounding, versioned identities)
docs/topics/optics/test-plan.md (OPTICS-REQ-017..025, OPTICS-TP-022..034 implemented, the sRGB
reference oracle, pixel mutation checks, timing)

Generated artifacts:
crates/bunny-optics/src/pixel/srgb_knots.rs: 1153 knot values of `1.055 x^(1/2.4) - 0.055`
rounded to nearest raw Q32.32, computed with Python's standard `decimal` module at 60 digits
in a scratch directory (not committed); OPTICS-TP-026 recomputes each one with `f64`.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo test --locked -p bunny-optics --test pixel_tests against stubs (accumulator, curve,
quantization and writer returning constants): 2 passed, 9 failed. OPTICS-TP-025 and
OPTICS-TP-030 pass vacuously on the stub (zero sums have a zero mean; constant outputs are
monotone).
cargo fmt --all (exported patch applied), cargo clippy --locked --workspace --all-targets
--all-features -- -D warnings, strict library Clippy for bunny-optics, cargo test --locked -p
bunny-optics (camera 8 + pixel 11 passed), cargo test --locked -p bunny-perf,
cargo run --locked -p xtask -- code-dojo-rust --all (clean), topic-docs (clean).
One golden vector in OPTICS-TP-031 was wrong as first written (16-bit code of display raw
8 421 505 is 129, not 128: 65535 = 257 * 255); the implementation exposed it and it was
corrected.
Eight deliberate mutations of src/pixel/ (see the test plan's Pixel Mutation Checks), run
with lints capped to warnings; each failed at least one case; restored source passed 11 of 11.
Merged origin/main (0.7.0 release preparation): conflicts in crates/bunny-perf/Cargo.toml and
src/cases.rs resolved by keeping both feature groups; the CHANGELOG entry moved under
[Unreleased].

Timing (a reading, no speed claim, no ledger entry), Apple M5 Pro in rust-dev:1.96.0, devrun
--exclusive at 3814e17: cargo xtask perf --base origin/main --rounds 11 --align-code --filter
pixel/ built both sides but exits with "no case was measured on both sides" because the base has
no pixel functions. The harness run directly (cargo build --release -p bunny-perf, then five
runs each of --filter pixel/ and --filter num/div) gave pixel/resolve_srgb_rgba8 p50 8.71 to
8.95 ns per pixel (three channels resolved, toned and quantized, four bytes written), against
num/div p50 6.59 to 6.84 ns in the same runs.

Renumbered after the sampler (flyingrobots/bunny#221) merged with OPTICS-REQ-010..016 and
OPTICS-TP-010..021: this work's requirements are now OPTICS-REQ-017..025 and its cases
OPTICS-TP-022..034, test functions renamed to match, before merging origin/main.
Merged origin/main with the sampler: lib.rs keeps both modules and re-exports; the chapter,
test plan, crate README, TESTING.md, capability map, root README, bunny-perf (sample and pixel
groups) and CHANGELOG keep both sides, the sampler's first. The Pixel Values section now says
once what a sample is (one sample index, the ray of PixelSampler::camera_sample) and that the
1 024-sample accumulator limit matches the sampler's tested stratification for power-of-two
counts up to 1 024.

Known risks:
The 8-bit and 16-bit sRGB codes are not exactly the correctly rounded codes: 245 of 255 8-bit
codes begin up to 9 447 raw inputs late (486 760 inputs of 2^32 in [0, 1] differ by one). An
exact 8-bit encoding (255 thresholds) would be a different identity; the maintainer may prefer
it for the 8-bit output contract. The knot table is a checked-in literal generated outside the
repository; OPTICS-TP-026 rechecks it with f64, not with the 60-digit generator. Knots 0..76
are never read. The f64 oracle relies on libm powf being accurate to about one ulp on every
target, including wasm32. No speed claim is made, so there is no ledger entry.

Human reviewer:
James Ross (maintainer); review pending.
