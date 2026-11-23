# Receipt: Fixed Angle Atan2 Conversions

Task:
First slice of flyingrobots/bunny#244: conversions out of `FixedAngle` (`to_turns`, `to_degrees`, `to_radians`) and `FixedAngle::atan2`, each with a stated rounding rule and bound in raw units, an independent high-precision oracle, golden vectors from a 60-digit decimal model and mutation checks. The first commit adds the planned cases and failing tests against stub bodies; the second implements the kernels and the living docs. Tangent, `asin`, `acos`, single-ratio `atan`, rotation APIs and the `sin_cos` error-sweep tool remain for later pull requests.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-num/src/fixed_angle.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/lerp.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/fixed_angle_tests.rs`
- `crates/bunny-num/tests/lerp_tests.rs`
- `crates/bunny-num/README.md`
- `docs/topics/angle-trigonometry/README.md`
- `docs/topics/angle-trigonometry/test-plan.md`
- `docs/topics/lerp/test-plan.md`
- `docs/topics/look-rotation/README.md`
- `docs/topics/fixed-point-rounding/README.md`
- `.repo-respect/receipts/2026-11-23-bunny-lerp.md`

Files edited:
- `crates/bunny-num/src/fixed_angle/atan2.rs`
- `crates/bunny-num/src/fixed_angle/conversions_out.rs`
- `crates/bunny-num/src/fixed_angle.rs`
- `crates/bunny-num/src/fixed_q32_32.rs` (`round_shift_right_u128` becomes `pub(crate)`; no public item changes)
- `crates/bunny-num/tests/angle_oracle/mod.rs`
- `crates/bunny-num/tests/fixed_angle_inverse_tests.rs`
- `docs/topics/angle-trigonometry/test-plan.md`
- `docs/topics/angle-trigonometry/README.md`
- `docs/topics/fixed-point-rounding/README.md`
- `crates/bunny-num/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-fixed-angle-atan2-conversions.md`

Topic docs:
- `docs/topics/angle-trigonometry/test-plan.md`: AT-REQ-010 to -014 and AT-TP-010 to -017, implemented, with the mutation checks and the oracle description.
- `docs/topics/angle-trigonometry/README.md`: new "Conversions Out" and "Atan2" sections with rules, bounds, range, branch and the error analysis; non-goals updated.
- `docs/topics/fixed-point-rounding/README.md`: rows for `to_turns`, `to_degrees`, `to_radians` and `atan2`.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-num --test fixed_angle_inverse_tests` (RED: 6 of 7 fail against the stubs; AT-TP-015 passes vacuously while every result is `None`)
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`
- After the implementation: `cargo test --locked -p bunny-num --test fixed_angle_inverse_tests` (7 passed), `cargo fmt --all --check`, `cargo test --locked --workspace --all-targets --all-features` (every binary ok), `cargo clippy --locked -p bunny-num --all-targets --all-features -- -D warnings` and the same with `--no-default-features` (clean), `cargo build --locked -p bunny-num --no-default-features` and `cargo test --locked -p bunny-num --no-default-features` (ok), `code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`.
- Mutations: fourteen, recorded in `docs/topics/angle-trigonometry/test-plan.md#mutation-checks`; each run with `cargo test --locked --no-fail-fast -p bunny-num --test fixed_angle_inverse_tests` and the source restored after.
- Not run locally: `code-dojo-no-std` (needs `rustup` for the `thumbv7em-none-eabihf` target; CI runs it), the wasm32 tests (`wasm-pack` not installed; CI's WebAssembly job runs them), `cargo deny` and markdownlint.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.
- The bounds of `to_radians` and `atan2` are proved by the error analysis in the living reference and checked on samples (20 000 seeded inputs plus boundary sets), not exhaustively.
- No `bunny-perf` case is added: the performance chapter does not require one for a new API, and no performance claim is made.

Human reviewer:
@flyingrobots
