# Receipt: Bunny Lerp

Task:
Implement `bunny-lerp/1` (flyingrobots/bunny#303): `FixedQ32_32::lerp` and `FixedQ32_32::from_ratio` in `bunny-num`, `FixedVec3::lerp` and the three-channel `lerp_channels` in `bunny-linalg`, each rounded once to nearest with ties to even from an exact wide intermediate, with independent oracles, golden vectors and mutation checks. This commit adds the planned cases and the failing tests against stub bodies.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/u32_div_round_even.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/u32_div_round_even_tests.rs`
- `crates/bunny-num/tests/tie_rounding_tests.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `docs/topics/fixed-point-rounding/README.md`
- `docs/topics/fixed-point-rounding/test-plan.md`
- `docs/topics/gpu-u32-div-round-even/README.md`
- `docs/topics/gpu-u32-div-round-even/test-plan.md`
- `docs/topics/optics/test-plan.md` (incidence-cosine mutation checks)
- `docs/topics/performance/README.md`
- `.github/workflows/code-dojo.yml`

Files edited:
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-linalg/src/lerp.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/lerp_tests.rs`
- `crates/bunny-num/src/lerp.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/lerp_tests.rs`
- `docs/topics/lerp/test-plan.md`
- `.repo-respect/receipts/2026-11-23-bunny-lerp.md`

Topic docs:
- `docs/topics/lerp/test-plan.md`: LERP-REQ-001 to -004 and LERP-TP-001 to -010, planned.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-num --test lerp_tests` (RED: 6 of 7 fail against the stubs; LERP-TP-005 passes vacuously while every result is `None`)
- `cargo test --locked -p bunny-linalg --test lerp_tests` (RED: both fail)
- `cargo run --locked -p xtask -- topic-docs`

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
