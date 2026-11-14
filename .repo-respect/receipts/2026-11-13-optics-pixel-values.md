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
crates/bunny-optics/src/lib.rs
crates/bunny-optics/src/pixel.rs
crates/bunny-optics/src/pixel/accumulate.rs
crates/bunny-optics/src/pixel/srgb.rs
crates/bunny-optics/src/pixel/srgb_knots.rs
crates/bunny-optics/src/pixel/tone.rs
crates/bunny-optics/tests/pixel_tests.rs
docs/topics/optics/test-plan.md
.repo-respect/receipts/2026-11-13-optics-pixel-values.md

Topic docs:
docs/topics/optics/test-plan.md (OPTICS-REQ-010..018, OPTICS-TP-010..022, the sRGB
reference oracle)

Generated artifacts:
crates/bunny-optics/src/pixel/srgb_knots.rs: 1153 knot values of `1.055 x^(1/2.4) - 0.055`
rounded to nearest raw Q32.32, computed with Python's standard `decimal` module at 60 digits
in a scratch directory (not committed); OPTICS-TP-014 recomputes each one with `f64`.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devcontainers' devrun; nothing on the host.
cargo test --locked -p bunny-optics --test pixel_tests against stubs (accumulator, curve,
quantization and writer returning constants): 2 passed, 9 failed. OPTICS-TP-013 and
OPTICS-TP-018 pass vacuously on the stub (zero sums have a zero mean; constant outputs are
monotone).

Known risks:
In progress.

Human reviewer:
James Ross (maintainer); review pending.
