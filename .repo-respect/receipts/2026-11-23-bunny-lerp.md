# Receipt: Bunny Lerp

Task:
Implement `bunny-lerp/1` (flyingrobots/bunny#303): `FixedQ32_32::lerp` and `FixedQ32_32::from_ratio` in `bunny-num`, `FixedVec3::lerp` and the three-channel `lerp_channels` in `bunny-linalg`, each rounded once to nearest with ties to even from an exact wide intermediate, with independent oracles, golden vectors and mutation checks. The first commit added the planned cases and failing tests against stub bodies; the second implements the kernels and the living docs.

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
- `docs/topics/lerp/README.md`
- `docs/topics/fixed-point-rounding/README.md`
- `docs/README.md`
- `crates/bunny-num/README.md`
- `crates/bunny-linalg/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-bunny-lerp.md`

Topic docs:
- `docs/topics/lerp/README.md`: new living reference for `bunny-lerp/1`.
- `docs/topics/lerp/test-plan.md`: LERP-REQ-001 to -004 and LERP-TP-001 to -010, implemented, with the mutation checks.
- `docs/topics/fixed-point-rounding/README.md`: the two new rounding sites.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-num --test lerp_tests` (RED: 6 of 7 fail against the stubs; LERP-TP-005 passes vacuously while every result is `None`)
- `cargo test --locked -p bunny-linalg --test lerp_tests` (RED: both fail)
- `cargo run --locked -p xtask -- topic-docs`
- After the implementation: `cargo test --locked -p bunny-num -p bunny-linalg --test lerp_tests` (9 passed), `cargo fmt --all --check`, `cargo test --locked --workspace --all-targets --all-features` (108 test binaries ok, none failed), `cargo run --locked -p xtask -- code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`.
- `cargo clippy --locked -p bunny-num -p bunny-linalg --all-targets --all-features -- -D warnings` clean (the workspace-wide run fails only on a 1.100.0-only lint in `bunny-perf`, untouched here).
- Mutations: thirteen, recorded in `docs/topics/lerp/test-plan.md#mutation-checks`; each run with `cargo test --locked --no-fail-fast -p bunny-num -p bunny-linalg --test lerp_tests` and the source restored after.
- The wasm32 tests were not run locally (`wasm-pack` and the target are not installed); CI's WebAssembly job runs every `lerp_tests` case.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.
- LERP-TP-003 runs 16 781 312 oracle comparisons; about 2 s natively in a debug build, slower under the WebAssembly job.
- The pull request is about 830 changed lines against the issue's guide of about 400: 132 are source, the rest tests and docs.
- No `bunny-perf` case is added: the performance chapter describes how to add one but does not require it for a new API.

Human reviewer:
@flyingrobots
