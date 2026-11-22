# Receipt: Gpu U32 Div Round Even

Task:
Implement `bunny-u32-div-round-even/1` (`flyingrobots/bunny#428`): pure deterministic CPU reference in `crates/bunny-num`, portable WGSL helper + compute wrapper + ordered 1,720-vector fixture + content-addressed manifest + `wgpu` CPU-vs-GPU equivalence harness in quarantined `crates/bunny-gpu`, and living topic documentation under `docs/topics/gpu-u32-div-round-even/`.

Files read:
- `docs/design/0005-gpu-compute-backend.md`
- `docs/PROCESS.md`
- `docs/README.md`
- `CODE_STANDARDS.md`
- `Cargo.toml`
- `deny.toml`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/fixed_angle_tests.rs`
- `crates/bunny-perf/Cargo.toml`
- `xtask/src/code_dojo.rs`
- `xtask/src/crate_manifests.rs`
- `xtask/src/nondeterminism.rs`

Files edited:
- `Cargo.toml`
- `Cargo.lock`
- `deny.toml`
- `xtask/src/code_dojo.rs`
- `crates/bunny-num/src/u32_div_round_even.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/u32_div_round_even_tests.rs`
- `crates/bunny-gpu/Cargo.toml`
- `crates/bunny-gpu/README.md`
- `crates/bunny-gpu/shaders/bunny_u32_div_round_even.wgsl`
- `crates/bunny-gpu/shaders/bunny_u32_div_round_even_compute.wgsl`
- `crates/bunny-gpu/fixtures/u32_div_round_even_vectors.json`
- `crates/bunny-gpu/src/lib.rs`
- `crates/bunny-gpu/src/manifest.rs`
- `crates/bunny-gpu/src/receipt.rs`
- `crates/bunny-gpu/src/u32_div_round_even.rs`
- `crates/bunny-gpu/tests/u32_div_round_even_gpu.rs`
- `docs/topics/gpu-u32-div-round-even/README.md`
- `docs/topics/gpu-u32-div-round-even/test-plan.md`
- `docs/README.md`
- `.repo-respect/receipts/2026-11-22-gpu-u32-div-round-even.md`

Topic docs:
- `docs/topics/gpu-u32-div-round-even/README.md`
- `docs/topics/gpu-u32-div-round-even/test-plan.md`
- `docs/README.md`

Generated artifacts:
None.

Checks run:
- `cargo test -p bunny-num` (RED verified before impl; GREEN 49/49 pass after impl)
- `cargo test -p bunny-gpu -- --nocapture` (RED verified against stub WGSL on Apple M5 Metal; GREEN 3/3 bit-for-bit pass across all 1,720 vectors on Apple M5 Metal after impl)
- `cargo clippy -p bunny-num -p bunny-gpu --all-targets -- -D warnings` (passed)
- `cargo fmt --all -- --check` (passed)
- `cargo run --locked -p xtask -- code-dojo-rust --staged` (passed)
- `cargo run --locked -p xtask -- code-dojo-generated` (passed)
- `cargo run --locked -p xtask -- topic-docs` (passed)
- `cargo run --locked -p xtask -- repo-respect check --branch` (passed)
- `cargo run --locked -p xtask -- code-dojo-commit-msg` (passed)

Known risks:
- `bunny-u32-div-round-even/1` is a bounded unsigned-integer kernel and makes no claim of Q32.32 GPU conformance.
- Headless CI runners without a Vulkan/software GPU adapter report `Outcome::Unavailable` rather than `Outcome::Pass`.
- `deny.toml` `[licenses]` adds `CC0-1.0`, `ISC`, and `Zlib` for `wgpu 29.0.4` transitive crates (`hexf-parse`, `libloading`, `foldhash`); `[advisories]` is intentionally untouched.

Human reviewer:
@flyingrobots
