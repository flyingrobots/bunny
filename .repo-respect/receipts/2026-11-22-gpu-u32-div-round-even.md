# Receipt: Gpu U32 Div Round Even

Task:
Implement `bunny-u32-div-round-even/1` (`flyingrobots/bunny#428`): pure deterministic CPU reference in `crates/bunny-num`, portable WGSL helper + compute wrapper + ordered golden/seeded vector fixture + content-addressed manifest + `wgpu` CPU-vs-GPU equivalence harness in quarantined `crates/bunny-gpu`, and living topic documentation under `docs/topics/gpu-u32-div-round-even/`.

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
- `crates/bunny-num/src/u32_div_round_even.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/u32_div_round_even_tests.rs`
- `.repo-respect/receipts/2026-11-22-gpu-u32-div-round-even.md`

Topic docs:
- `docs/topics/gpu-u32-div-round-even/README.md` (added in implementation slice)
- `docs/topics/gpu-u32-div-round-even/test-plan.md` (added in implementation slice)
- `docs/README.md` (updated in implementation slice)

Generated artifacts:
None.

Checks run:
- `cargo test -p bunny-num --test u32_div_round_even_tests` (RED verified before implementation, GREEN verified after)
- `cargo run --locked -p xtask -- code-dojo-rust --staged`
- `cargo run --locked -p xtask -- repo-respect check --staged`

Known risks:
- `bunny-u32-div-round-even/1` is a bounded unsigned-integer kernel and makes no claim of Q32.32 GPU conformance.
- Headless CI runners without a Vulkan/software GPU adapter report `Outcome::Unavailable` rather than `Pass`.

Human reviewer:
@flyingrobots
