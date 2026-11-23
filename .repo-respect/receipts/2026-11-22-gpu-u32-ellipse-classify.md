# Receipt: Gpu U32 Ellipse Classify

Task:
Implement `bunny-u32-ellipse-classify/1` (`flyingrobots/bunny#430`): pure deterministic exact 64-bit unsigned ellipse trichotomy predicate in `crates/bunny-num`, portable 2-limb `u32` WGSL helper + compute wrapper + ordered golden/boundary/seeded vector fixture + content-addressed manifest + `wgpu` CPU-vs-GPU equivalence harness in `crates/bunny-gpu`, and living topic documentation under `docs/topics/gpu-u32-ellipse-classify/`.

Files read:
- `docs/design/0005-gpu-compute-backend.md`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/src/u32_div_round_even.rs`
- `crates/bunny-gpu/src/lib.rs`
- `crates/bunny-gpu/src/u32_div_round_even.rs`
- `docs/topics/gpu-u32-div-round-even/README.md`
- `docs/topics/gpu-u32-div-round-even/test-plan.md`

Files edited:
- `crates/bunny-num/src/u32_ellipse_classify.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/u32_ellipse_classify_tests.rs`
- `crates/bunny-gpu/shaders/bunny_u32_ellipse_classify.wgsl`
- `crates/bunny-gpu/shaders/bunny_u32_ellipse_classify_compute.wgsl`
- `crates/bunny-gpu/fixtures/u32_ellipse_classify_vectors.json`
- `crates/bunny-gpu/src/lib.rs`
- `crates/bunny-gpu/src/u32_ellipse_classify.rs`
- `crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs`
- `docs/topics/gpu-u32-ellipse-classify/README.md`
- `docs/topics/gpu-u32-ellipse-classify/test-plan.md`
- `docs/README.md`
- `.repo-respect/receipts/2026-11-22-gpu-u32-ellipse-classify.md`

Topic docs:
- `docs/topics/gpu-u32-ellipse-classify/README.md`
- `docs/topics/gpu-u32-ellipse-classify/test-plan.md`
- `docs/README.md`

Generated artifacts:
None.

Checks run:
- `cargo test -p bunny-num --test u32_ellipse_classify_tests` (RED verified before impl, GREEN after)
- `cargo test -p bunny-gpu --test u32_ellipse_classify_gpu` (RED verified against stub WGSL on Apple M5 Metal, GREEN bit-for-bit pass after)
- `cargo clippy --locked -p bunny-num -p bunny-gpu --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Review follow-up (`VirtualGpu` classification):
- `crates/bunny-gpu/src/u32_ellipse_classify.rs` drops its private copy of the device-type
  mapping and calls `ExecutionClass::from_device_type` (`crates/bunny-gpu/src/receipt.rs`,
  covered by `udre_gpu_006_only_a_cpu_adapter_is_classified_as_software_execution`), so a
  `VirtualGpu` adapter is recorded as `Unknown` here too.
- `cargo test --locked -p bunny-gpu` (pass on Apple Metal).
- The pinned WGSL and fixture files are untouched.

Known risks:
- `bunny-u32-ellipse-classify/1` is a bounded 2-limb `u32` (64-bit) predicate over `0..=46,340` and makes no claim of full Q32.32 GPU conformance.
- Headless CI runners without a Vulkan/software GPU adapter report `Outcome::Unavailable` rather than `Outcome::Pass`.

Human reviewer:
@flyingrobots
